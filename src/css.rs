use std::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
};

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, ParseErrorKind, Parser as CssParser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, SourceLocation,
    StyleSheetParser, ToCss, Token, serialize_identifier,
};
use egui::Color32;
use precomputed_hash::PrecomputedHash;
use selectors::parser::{self, Component, ParseRelative, SelectorList};

use crate::CssError;

#[derive(Clone, Debug)]
pub(crate) struct Rule {
    pub(crate) class: Option<String>,
    pub(crate) state: RuleState,
    pub(crate) declarations: Vec<Declaration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuleState {
    Base,
    Hovered,
    Active,
}

#[derive(Clone, Debug)]
pub(crate) enum Declaration {
    Color(Color32),
    Background(Color32),
    Padding(egui::Vec2),
    BorderRadius(u8),
    FontFamily(Vec<String>),
    FontWeight(u16),
    FontStyle(bool),
}

enum ParsedRule {
    Import(String),
    Style(Vec<Rule>),
}

enum AtRulePrelude {
    Import(String),
}

#[derive(Clone, Copy, Debug)]
struct ParseIssue {
    message: &'static str,
    line: Option<u32>,
}

impl ParseIssue {
    fn new(message: &'static str) -> Self {
        Self {
            message,
            line: None,
        }
    }
}

pub(crate) fn parse(source: &str) -> Result<(Vec<Rule>, Vec<String>), CssError> {
    let mut parser = CssParser::new(source);
    let mut rule_parser = StylesheetRuleParser;
    let mut rules = Vec::new();
    let mut imports = Vec::new();

    for parsed in StyleSheetParser::new(&mut parser, &mut rule_parser) {
        match parsed {
            Ok(ParsedRule::Import(import)) => imports.push(import),
            Ok(ParsedRule::Style(mut parsed_rules)) => rules.append(&mut parsed_rules),
            Err((error, _, location)) => return Err(to_css_error(error, location)),
        }
    }

    Ok((rules, imports))
}

pub(crate) fn selector_class(selector: &str) -> Option<String> {
    let mut parser = CssParser::new(selector);
    parse_selectors(&mut parser)
        .ok()
        .filter(|selectors| selectors.len() == 1)
        .and_then(|mut selectors| {
            let (class, state) = selectors.pop()?;
            (state == RuleState::Base).then_some(class).flatten()
        })
}

struct StylesheetRuleParser;

impl<'i> QualifiedRuleParser<'i> for StylesheetRuleParser {
    type Prelude = Vec<(Option<String>, RuleState)>;
    type QualifiedRule = ParsedRule;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        parse_selectors(input)
    }

    fn parse_block(
        &mut self,
        selectors: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Self::QualifiedRule, ParseError<Self::Error>> {
        let declarations = parse_declarations(input).map_err(ParseError::custom)?;
        Ok(ParsedRule::Style(
            selectors
                .into_iter()
                .map(|(class, state)| Rule {
                    class,
                    state,
                    declarations: declarations.clone(),
                })
                .collect(),
        ))
    }
}

impl<'i> AtRuleParser<'i> for StylesheetRuleParser {
    type Prelude = AtRulePrelude;
    type AtRule = ParsedRule;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        if !name.eq_ignore_ascii_case("import") {
            return Err(ParseError::custom(ParseIssue::new("unsupported at-rule")));
        }
        parse_import(input).map(AtRulePrelude::Import)
    }

    fn rule_without_block(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
    ) -> Result<Self::AtRule, ()> {
        match prelude {
            AtRulePrelude::Import(import) => Ok(ParsedRule::Import(import)),
        }
    }
}

fn parse_import<'i>(input: &mut CssParser<'i>) -> Result<String, ParseError<ParseIssue>> {
    let token = input
        .next()
        .map_err(|_| ParseError::custom(ParseIssue::new("unsupported import rule")))?
        .clone();
    let url = match token {
        Token::QuotedString(value) | Token::UnquotedUrl(value) => value.to_string(),
        Token::Function(name) if name.eq_ignore_ascii_case("url") => {
            input.parse_nested_block(|url| {
                url.expect_string()
                    .map(|value| value.to_string())
                    .map_err(Into::into)
            })?
        }
        _ => {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported import rule",
            )));
        }
    };
    if url.trim().is_empty() {
        return Err(ParseError::custom(ParseIssue::new(
            "unsupported import rule",
        )));
    }
    Ok(url)
}

fn parse_selectors<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<(Option<String>, RuleState)>, ParseError<ParseIssue>> {
    let selector_parser = SimpleSelectorParser;
    let selectors = SelectorList::parse(&selector_parser, input, ParseRelative::No)
        .map_err(|_| ParseError::custom(ParseIssue::new("unsupported class selector")))?;

    let mut result = Vec::with_capacity(selectors.len());
    for selector in selectors.slice() {
        let mut class = None;
        let mut universal = false;
        let mut state = RuleState::Base;
        for component in selector.iter_raw_parse_order_from(0) {
            match component {
                Component::Class(identifier) if class.is_none() && !universal => {
                    class = Some(identifier.0.clone());
                }
                Component::ExplicitUniversalType if class.is_none() && !universal => {
                    universal = true;
                }
                Component::NonTSPseudoClass(StatePseudo::Hover) if state == RuleState::Base => {
                    state = RuleState::Hovered;
                }
                Component::NonTSPseudoClass(StatePseudo::Active) if state == RuleState::Base => {
                    state = RuleState::Active;
                }
                _ => {
                    return Err(ParseError::custom(ParseIssue::new(
                        "unsupported class selector",
                    )));
                }
            }
        }
        if universal && class.is_some() {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported class selector",
            )));
        }
        let selector = if universal {
            None
        } else if let Some(class) = class {
            Some(class)
        } else {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported class selector",
            )));
        };
        result.push((selector, state));
    }
    if result.is_empty() {
        return Err(ParseError::custom(ParseIssue::new(
            "unsupported class selector",
        )));
    }
    Ok(result)
}

struct Declarations;

impl<'i> DeclarationParser<'i> for Declarations {
    type Declaration = Option<Declaration>;
    type Error = &'static str;

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
        _declaration_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<Self::Error>> {
        let declaration = if name.eq_ignore_ascii_case("color") {
            Some(Declaration::Color(parse_color(input)?))
        } else if name.eq_ignore_ascii_case("background-color") {
            Some(Declaration::Background(parse_color(input)?))
        } else if name.eq_ignore_ascii_case("padding") {
            Some(Declaration::Padding(parse_padding(input)?))
        } else if name.eq_ignore_ascii_case("border-radius") {
            let radius = parse_length(input, "unsupported border radius")?;
            Some(Declaration::BorderRadius(
                radius.round().min(f32::from(u8::MAX)) as u8,
            ))
        } else if name.eq_ignore_ascii_case("font-family") {
            Some(Declaration::FontFamily(parse_font_family(input)?))
        } else if name.eq_ignore_ascii_case("font-weight") {
            Some(Declaration::FontWeight(parse_font_weight(input)?))
        } else if name.eq_ignore_ascii_case("font-style") {
            Some(Declaration::FontStyle(parse_font_style(input)?))
        } else {
            while input.next().is_ok() {}
            None
        };
        Ok(declaration)
    }
}

impl<'i> AtRuleParser<'i> for Declarations {
    type Prelude = ();
    type AtRule = Option<Declaration>;
    type Error = &'static str;
}

impl<'i> QualifiedRuleParser<'i> for Declarations {
    type Prelude = ();
    type QualifiedRule = Option<Declaration>;
    type Error = &'static str;
}

impl<'i> RuleBodyItemParser<'i, Option<Declaration>, &'static str> for Declarations {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        false
    }
}

fn parse_declarations<'i>(input: &mut CssParser<'i>) -> Result<Vec<Declaration>, ParseIssue> {
    let mut parser = Declarations;
    let mut declarations = Vec::new();
    for item in RuleBodyParser::new(input, &mut parser) {
        match item {
            Ok(Some(declaration)) => declarations.push(declaration),
            Ok(None) => {}
            Err((error, _, location)) => {
                let message = match error.kind {
                    ParseErrorKind::Custom(message) => message,
                    ParseErrorKind::Basic(_) => "invalid CSS syntax",
                };
                return Err(ParseIssue {
                    message,
                    line: Some(location.line),
                });
            }
        }
    }
    Ok(declarations)
}

fn parse_color<'i>(input: &mut CssParser<'i>) -> Result<Color32, ParseError<&'static str>> {
    match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("transparent") => {
            Ok(Color32::TRANSPARENT)
        }
        Token::Hash(hex) | Token::IDHash(hex) => {
            parse_hex_color(hex).ok_or_else(|| ParseError::custom("unsupported color value"))
        }
        _ => Err(ParseError::custom("unsupported color value")),
    }
}

fn parse_hex_color(hex: &str) -> Option<Color32> {
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |index: usize| u8::from_str_radix(&hex[index..index + 1], 16).ok();
    let byte = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    match hex.len() {
        3 => Some(Color32::from_rgb(
            nibble(0)?.saturating_mul(17),
            nibble(1)?.saturating_mul(17),
            nibble(2)?.saturating_mul(17),
        )),
        4 => Some(Color32::from_rgba_unmultiplied(
            nibble(0)?.saturating_mul(17),
            nibble(1)?.saturating_mul(17),
            nibble(2)?.saturating_mul(17),
            nibble(3)?.saturating_mul(17),
        )),
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}

fn parse_length<'i>(
    input: &mut CssParser<'i>,
    message: &'static str,
) -> Result<f32, ParseError<&'static str>> {
    let length = match input.next()? {
        Token::Number { value, .. } => *value,
        Token::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("px") => *value,
        _ => return Err(ParseError::custom(message)),
    };
    if length.is_finite() && length >= 0.0 {
        Ok(length)
    } else {
        Err(ParseError::custom(message))
    }
}

fn parse_padding<'i>(input: &mut CssParser<'i>) -> Result<egui::Vec2, ParseError<&'static str>> {
    let mut lengths = Vec::with_capacity(2);
    while !input.is_exhausted() {
        if lengths.len() == 2 {
            return Err(ParseError::custom(
                "padding must have one or two nonnegative lengths",
            ));
        }
        lengths.push(parse_length(
            input,
            "padding must have one or two nonnegative lengths",
        )?);
    }
    match lengths.as_slice() {
        [all] => Ok(egui::Vec2::splat(*all)),
        [vertical, horizontal] => Ok(egui::vec2(*horizontal, *vertical)),
        _ => Err(ParseError::custom(
            "padding must have one or two nonnegative lengths",
        )),
    }
}

fn parse_font_family<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<String>, ParseError<&'static str>> {
    let families = input.parse_comma_separated(|family| {
        let first = family.next()?.clone();
        match first {
            Token::QuotedString(name) if !name.trim().is_empty() => {
                family.expect_exhausted()?;
                Ok(name.to_string())
            }
            Token::Ident(name) => {
                let mut names = vec![name.to_string()];
                while !family.is_exhausted() {
                    match family.next()? {
                        Token::Ident(name) => names.push(name.to_string()),
                        _ => return Err(ParseError::custom("unsupported font family")),
                    }
                }
                Ok(names.join(" "))
            }
            _ => Err(ParseError::custom("unsupported font family")),
        }
    })?;
    if families.is_empty() {
        Err(ParseError::custom("unsupported font family"))
    } else {
        Ok(families)
    }
}

fn parse_font_weight<'i>(input: &mut CssParser<'i>) -> Result<u16, ParseError<&'static str>> {
    match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("normal") => Ok(400),
        Token::Ident(value) if value.eq_ignore_ascii_case("bold") => Ok(700),
        Token::Number {
            int_value: Some(weight),
            ..
        } if (1..=1000).contains(weight) => Ok(*weight as u16),
        _ => Err(ParseError::custom("unsupported font weight")),
    }
}

fn parse_font_style<'i>(input: &mut CssParser<'i>) -> Result<bool, ParseError<&'static str>> {
    match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("normal") => Ok(false),
        Token::Ident(value)
            if value.eq_ignore_ascii_case("italic") || value.eq_ignore_ascii_case("oblique") =>
        {
            Ok(true)
        }
        _ => Err(ParseError::custom("unsupported font style")),
    }
}

fn to_css_error(error: ParseError<ParseIssue>, location: SourceLocation) -> CssError {
    let (line, message) = match error.kind {
        ParseErrorKind::Custom(issue) => (issue.line.unwrap_or(location.line), issue.message),
        ParseErrorKind::Basic(_) => (location.line, "invalid CSS syntax"),
    };
    CssError::at_line(line, message)
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
struct Atom(String);

impl ToCss for Atom {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        serialize_identifier(&self.0, dest)
    }
}

impl<'i> From<CowRcStr<'i>> for Atom {
    fn from(value: CowRcStr<'i>) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for Atom {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Atom {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl Borrow<str> for Atom {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PrecomputedHash for Atom {
    fn precomputed_hash(&self) -> u32 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish() as u32
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum StatePseudo {
    Hover,
    Active,
}

impl parser::NonTSPseudoClass for StatePseudo {
    fn is_active_or_hover(&self) -> bool {
        true
    }

    fn is_user_action_state(&self) -> bool {
        true
    }
}

impl ToCss for StatePseudo {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(match self {
            Self::Hover => ":hover",
            Self::Active => ":active",
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum NoPseudoElement {}

impl parser::PseudoElement for NoPseudoElement {}

impl ToCss for NoPseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match *self {}
    }
}

#[derive(Clone, Debug)]
struct SimpleSelectorImpl;

impl parser::SelectorImpl for SimpleSelectorImpl {
    type ExtraMatchingData<'a> = ();
    type AttrValue = Atom;
    type Identifier = Atom;
    type LocalName = Atom;
    type NamespaceUrl = Atom;
    type NamespacePrefix = Atom;
    type BorrowedLocalName = str;
    type BorrowedNamespaceUrl = str;
    type NonTSPseudoClass = StatePseudo;
    type PseudoElement = NoPseudoElement;
}

struct SimpleSelectorParser;

impl<'i> parser::Parser<'i> for SimpleSelectorParser {
    type Impl = SimpleSelectorImpl;
    type Error = parser::SelectorParseErrorKind;

    fn parse_non_ts_pseudo_class(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<StatePseudo, ParseError<Self::Error>> {
        if name.eq_ignore_ascii_case("hover") {
            Ok(StatePseudo::Hover)
        } else if name.eq_ignore_ascii_case("active") {
            Ok(StatePseudo::Active)
        } else {
            Err(ParseError::custom(
                parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }
}
