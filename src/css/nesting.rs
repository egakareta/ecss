use std::cell::RefCell;

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, ParseErrorKind, Parser as CssParser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser,
};
use selectors::parser::{self, Combinator, Component, ParseRelative, SelectorList};

use super::{
    Declaration, Declarations, MediaQueryList, ParseIssue, ParsedSelector, PseudoElementKind, Rule,
    RuleState, SelectorAncestor, SelectorRelation, SelectorTarget, SimpleSelectorImpl,
    SimpleSelectorParser,
};

const MAX_NESTING_DEPTH: usize = 64;
const MAX_SELECTOR_EXPANSION: usize = 4096;
const MAX_SELECTOR_COMPONENTS: usize = 64;

thread_local! {
    static SELECTOR_STACK: RefCell<Vec<SelectorAncestor>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn with_selector_ancestry<R>(
    mut target: SelectorTarget,
    f: impl FnOnce(&SelectorTarget) -> R,
) -> R {
    target.ancestors = SELECTOR_STACK.with(|stack| stack.borrow().clone());
    let parent = SelectorAncestor {
        classes: target.classes.clone(),
        id: target.id.clone(),
        relation: SelectorRelation::Child,
    };
    SELECTOR_STACK.with(|stack| stack.borrow_mut().push(parent));
    let _guard = SelectorStackGuard;
    f(&target)
}

struct SelectorStackGuard;

impl Drop for SelectorStackGuard {
    fn drop(&mut self) {
        SELECTOR_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

pub(super) fn parse_style_block(
    input: &mut CssParser<'_>,
    selectors: Vec<ParsedSelector>,
) -> Result<Vec<Rule>, ParseError<ParseIssue>> {
    parse_body(input, selectors, 0)
}

#[derive(Clone)]
enum GroupPrelude {
    Media(MediaQueryList),
    Supports(bool),
}

enum BodyItem {
    Declaration(Option<(Declaration, bool)>),
    Rules(Vec<Rule>),
}

struct NestedRuleParser {
    parents: Vec<ParsedSelector>,
    depth: usize,
}

impl<'i> DeclarationParser<'i> for NestedRuleParser {
    type Declaration = BodyItem;
    type Error = ParseIssue;

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
        declaration_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<Self::Error>> {
        let mut declarations = Declarations {
            allow_important: true,
        };
        DeclarationParser::parse_value(&mut declarations, name, input, declaration_start)
            .map(BodyItem::Declaration)
            .map_err(|error| match error.kind {
                ParseErrorKind::Custom(message) => ParseError::custom(ParseIssue::new(message)),
                ParseErrorKind::Basic(kind) => ParseError {
                    kind: ParseErrorKind::Basic(kind),
                },
            })
    }
}

impl<'i> QualifiedRuleParser<'i> for NestedRuleParser {
    type Prelude = Vec<ParsedSelector>;
    type QualifiedRule = BodyItem;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        let nested =
            SelectorList::parse(&SimpleSelectorParser, input, ParseRelative::ForNesting)
                .map_err(|_| ParseError::custom(ParseIssue::new("unsupported nested selector")))?;
        resolve_nested_selectors(&self.parents, &nested)
    }

    fn parse_block(
        &mut self,
        selectors: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Self::QualifiedRule, ParseError<Self::Error>> {
        parse_body(input, selectors, self.depth + 1).map(BodyItem::Rules)
    }
}

impl<'i> AtRuleParser<'i> for NestedRuleParser {
    type Prelude = GroupPrelude;
    type AtRule = BodyItem;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        if name.eq_ignore_ascii_case("media") {
            return MediaQueryList::parse(input).map(GroupPrelude::Media);
        }
        if name.eq_ignore_ascii_case("supports") {
            return super::supports::parse(input).map(GroupPrelude::Supports);
        }
        Err(ParseError::custom(ParseIssue::new("unsupported at-rule")))
    }

    fn rule_without_block(
        &mut self,
        _prelude: Self::Prelude,
        _start: &ParserState,
    ) -> Result<Self::AtRule, ()> {
        Err(())
    }

    fn parse_block(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Self::AtRule, ParseError<Self::Error>> {
        let rules = parse_body(input, self.parents.clone(), self.depth + 1)?;
        let rules = match prelude {
            GroupPrelude::Media(query) => {
                let mut rules = rules;
                for rule in &mut rules {
                    rule.media.push(query.clone());
                }
                rules
            }
            GroupPrelude::Supports(enabled) if enabled => rules,
            GroupPrelude::Supports(_) => Vec::new(),
        };
        Ok(BodyItem::Rules(rules))
    }
}

impl<'i> RuleBodyItemParser<'i, BodyItem, ParseIssue> for NestedRuleParser {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        true
    }
}

fn parse_body(
    input: &mut CssParser<'_>,
    parents: Vec<ParsedSelector>,
    depth: usize,
) -> Result<Vec<Rule>, ParseError<ParseIssue>> {
    if depth > MAX_NESTING_DEPTH {
        return Err(ParseError::custom(ParseIssue::new(
            "CSS nesting depth limit exceeded",
        )));
    }

    let declaration_selectors = parents.clone();
    let mut parser = NestedRuleParser { parents, depth };
    let mut rules = Vec::new();
    let mut declarations = Vec::new();
    let mut importance = Vec::new();

    for item in RuleBodyParser::new(input, &mut parser) {
        match item {
            Ok(BodyItem::Declaration(Some((declaration, declaration_importance)))) => {
                declarations.push(declaration);
                importance.push(declaration_importance);
            }
            Ok(BodyItem::Declaration(None)) => {}
            Ok(BodyItem::Rules(mut nested)) => {
                append_declaration_run(
                    &mut rules,
                    &mut declarations,
                    &mut importance,
                    &declaration_selectors,
                );
                rules.append(&mut nested);
            }
            Err((error, _, location)) => {
                let mut issue = match error.kind {
                    ParseErrorKind::Custom(issue) => issue,
                    ParseErrorKind::Basic(_) => ParseIssue::new("invalid CSS syntax"),
                };
                issue.line.get_or_insert(location.line);
                return Err(ParseError::custom(issue));
            }
        }
    }

    append_declaration_run(
        &mut rules,
        &mut declarations,
        &mut importance,
        &declaration_selectors,
    );
    Ok(rules)
}

fn append_declaration_run(
    rules: &mut Vec<Rule>,
    declarations: &mut Vec<Declaration>,
    importance: &mut Vec<bool>,
    selectors: &[ParsedSelector],
) {
    if declarations.is_empty() {
        return;
    }
    for (target, state, pseudo, specificity) in selectors {
        rules.push(Rule {
            target: target.clone(),
            state: *state,
            pseudo: *pseudo,
            specificity: *specificity,
            declarations: declarations.clone(),
            importance: importance.clone(),
            source_order: 0,
            media: Vec::new(),
        });
    }
    declarations.clear();
    importance.clear();
}

#[derive(Default)]
struct Compound {
    target: SelectorTarget,
    state: RuleState,
    pseudo: Option<PseudoElementKind>,
    parent_references: usize,
}

struct RelativeSelector {
    compounds: Vec<Compound>,
    relations: Vec<SelectorRelation>,
}

fn resolve_nested_selectors(
    parents: &[ParsedSelector],
    selectors: &SelectorList<SimpleSelectorImpl>,
) -> Result<Vec<ParsedSelector>, ParseError<ParseIssue>> {
    let expansion_size = parents
        .len()
        .checked_mul(selectors.len())
        .filter(|size| *size <= MAX_SELECTOR_EXPANSION)
        .ok_or_else(|| {
            ParseError::custom(ParseIssue::new("nested selector expansion limit exceeded"))
        })?;
    let parent_specificity = parents
        .iter()
        .map(|(_, _, _, specificity)| *specificity)
        .max()
        .unwrap_or_default();
    let mut resolved = Vec::with_capacity(expansion_size);

    for selector in selectors.slice() {
        let relative = parse_relative_selector(selector)?;
        let first = &relative.compounds[0];
        if relative
            .compounds
            .iter()
            .skip(1)
            .any(|compound| compound.parent_references > 0)
        {
            return Err(ParseError::custom(ParseIssue::new(
                "nesting selector must occur in the first compound",
            )));
        }
        let references = first.parent_references.max(1) as u32;
        let specificity = parent_specificity
            .saturating_mul(references)
            .saturating_add(selector.specificity());

        for (parent, parent_state, parent_pseudo, _) in parents {
            let (target, state, pseudo) =
                resolve_relative_selector(parent, *parent_state, *parent_pseudo, &relative)?;
            resolved.push((target, state, pseudo, specificity));
        }
    }

    Ok(resolved)
}

fn resolve_relative_selector(
    parent: &SelectorTarget,
    parent_state: RuleState,
    parent_pseudo: Option<PseudoElementKind>,
    relative: &RelativeSelector,
) -> Result<(SelectorTarget, RuleState, Option<PseudoElementKind>), ParseError<ParseIssue>> {
    let first = &relative.compounds[0];
    let has_parent = first.parent_references > 0;
    let mut ancestors = parent.ancestors.clone();

    if has_parent {
        if parent_pseudo.is_some() {
            return Err(unsupported_selector());
        }
        let mut subject = merge_targets(parent, &first.target)?;
        let state = merge_state(parent_state, first.state);
        if relative.compounds.len() == 1 {
            return Ok((subject, state, first.pseudo));
        }
        if state.bits() != RuleState::BASE.bits() || first.pseudo.is_some() {
            return Err(ParseError::custom(ParseIssue::new(
                "nested ancestor states are unsupported",
            )));
        }
        subject.ancestors.clear();
        ancestors.push(ancestor_from_target(&subject, relative.relations[0]));
    } else {
        if parent_state != RuleState::BASE || parent_pseudo.is_some() {
            return Err(ParseError::custom(ParseIssue::new(
                "nested ancestor states are unsupported",
            )));
        }
        ancestors.push(ancestor_from_target(parent, SelectorRelation::Descendant));
    }

    let first_child = usize::from(has_parent);
    for index in first_child..relative.compounds.len() - 1 {
        let compound = &relative.compounds[index];
        if compound.state != RuleState::BASE || compound.pseudo.is_some() {
            return Err(ParseError::custom(ParseIssue::new(
                "nested ancestor states are unsupported",
            )));
        }
        ancestors.push(ancestor_from_target(
            &compound.target,
            relative.relations[index],
        ));
    }

    let final_compound = relative.compounds.last().expect("relative selector");
    let mut target = final_compound.target.clone();
    target.ancestors = ancestors;
    Ok((target, final_compound.state, final_compound.pseudo))
}

fn parse_relative_selector(
    selector: &parser::Selector<SimpleSelectorImpl>,
) -> Result<RelativeSelector, ParseError<ParseIssue>> {
    let mut compounds = vec![Compound::default()];
    let mut relations = Vec::new();
    for component in selector.iter_raw_parse_order_from(0) {
        match component {
            Component::Class(identifier) => {
                compounds
                    .last_mut()
                    .unwrap()
                    .target
                    .classes
                    .push(identifier.0.clone());
            }
            Component::ID(identifier) => {
                let target = &mut compounds.last_mut().unwrap().target;
                if target.id.as_ref().is_some_and(|id| id != &identifier.0) {
                    return Err(unsupported_selector());
                }
                target.id = Some(identifier.0.clone());
            }
            Component::ExplicitUniversalType => {}
            Component::ParentSelector => {
                compounds.last_mut().unwrap().parent_references += 1;
            }
            Component::NonTSPseudoClass(pseudo) => {
                let state = match pseudo {
                    super::StatePseudo::Hover => RuleState::HOVERED,
                    super::StatePseudo::Active => RuleState::ACTIVE,
                    super::StatePseudo::Focus => RuleState::FOCUS,
                    super::StatePseudo::FocusVisible => RuleState::FOCUS_VISIBLE,
                    super::StatePseudo::Enabled => RuleState::ENABLED,
                    super::StatePseudo::Disabled => RuleState::DISABLED,
                    super::StatePseudo::FirstLetter => {
                        let compound = compounds.last_mut().unwrap();
                        if compound
                            .pseudo
                            .replace(PseudoElementKind::FirstLetter)
                            .is_some()
                        {
                            return Err(unsupported_selector());
                        }
                        continue;
                    }
                    super::StatePseudo::FirstLine => {
                        let compound = compounds.last_mut().unwrap();
                        if compound
                            .pseudo
                            .replace(PseudoElementKind::FirstLine)
                            .is_some()
                        {
                            return Err(unsupported_selector());
                        }
                        continue;
                    }
                };
                compounds.last_mut().unwrap().state.insert(state);
            }
            Component::PseudoElement(pseudo) => {
                let compound = compounds.last_mut().unwrap();
                if compound.pseudo.replace(*pseudo).is_some() {
                    return Err(unsupported_selector());
                }
            }
            Component::Combinator(Combinator::PseudoElement) => {}
            Component::Combinator(Combinator::Descendant) => {
                relations.push(SelectorRelation::Descendant);
                compounds.push(Compound::default());
            }
            Component::Combinator(Combinator::Child) => {
                relations.push(SelectorRelation::Child);
                compounds.push(Compound::default());
            }
            _ => return Err(unsupported_selector()),
        }
        if compounds.len() > MAX_SELECTOR_COMPONENTS || relations.len() > MAX_SELECTOR_COMPONENTS {
            return Err(ParseError::custom(ParseIssue::new(
                "nested selector complexity limit exceeded",
            )));
        }
    }
    for compound in &mut compounds {
        compound.target.classes.sort();
        compound.target.classes.dedup();
    }
    if compounds.is_empty() || relations.len() + 1 != compounds.len() {
        return Err(unsupported_selector());
    }
    Ok(RelativeSelector {
        compounds,
        relations,
    })
}

fn merge_targets(
    parent: &SelectorTarget,
    child: &SelectorTarget,
) -> Result<SelectorTarget, ParseError<ParseIssue>> {
    let id = match (&parent.id, &child.id) {
        (Some(first), Some(second)) if first != second => return Err(unsupported_selector()),
        (Some(id), _) | (_, Some(id)) => Some(id.clone()),
        (None, None) => None,
    };
    let mut classes = parent.classes.clone();
    classes.extend(child.classes.iter().cloned());
    classes.sort();
    classes.dedup();
    Ok(SelectorTarget {
        classes,
        id,
        ancestors: parent.ancestors.clone(),
    })
}

fn merge_state(mut first: RuleState, second: RuleState) -> RuleState {
    first.insert(second);
    first
}

fn ancestor_from_target(target: &SelectorTarget, relation: SelectorRelation) -> SelectorAncestor {
    SelectorAncestor {
        classes: target.classes.clone(),
        id: target.id.clone(),
        relation,
    }
}

fn unsupported_selector() -> ParseError<ParseIssue> {
    ParseError::custom(ParseIssue::new("unsupported nested selector"))
}
