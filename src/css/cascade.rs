use super::{Declaration, Rule, RuleState};

struct Entry<'a> {
    state: RuleState,
    declaration: &'a Declaration,
    important: bool,
    specificity: u32,
    source_order: usize,
    declaration_order: usize,
}

pub(crate) fn ordered<'a>(
    rules: impl IntoIterator<Item = &'a Rule>,
) -> Vec<(RuleState, &'a Declaration)> {
    ordered_with_importance(rules)
        .into_iter()
        .map(|(state, declaration, _)| (state, declaration))
        .collect()
}

pub(crate) fn ordered_with_importance<'a>(
    rules: impl IntoIterator<Item = &'a Rule>,
) -> Vec<(RuleState, &'a Declaration, bool)> {
    let mut entries = Vec::new();
    for rule in rules {
        for (declaration_order, declaration) in rule.declarations.iter().enumerate() {
            entries.push(Entry {
                state: rule.state,
                declaration,
                important: rule
                    .importance
                    .get(declaration_order)
                    .copied()
                    .unwrap_or(false),
                specificity: rule.specificity,
                source_order: rule.source_order,
                declaration_order,
            });
        }
    }
    entries.sort_unstable_by(|first, second| {
        first
            .important
            .cmp(&second.important)
            .then(first.specificity.cmp(&second.specificity))
            .then(first.source_order.cmp(&second.source_order))
            .then(first.declaration_order.cmp(&second.declaration_order))
    });
    entries
        .into_iter()
        .map(|entry| (entry.state, entry.declaration, entry.important))
        .collect()
}

pub(crate) fn overrides_animation(
    important: &[Declaration],
    animated: &Declaration,
    writing: crate::formatting::Writing,
) -> bool {
    let Some(property) = animated_property(animated) else {
        return false;
    };
    important
        .iter()
        .any(|declaration| declaration_affects(declaration, property, writing))
}

fn animated_property(declaration: &Declaration) -> Option<&str> {
    match declaration {
        Declaration::Color(_) => Some("color"),
        Declaration::Background(_) => Some("background-color"),
        Declaration::BackgroundShorthand(_) | Declaration::BackgroundReset(_) => Some("background"),
        Declaration::BackgroundTbdShorthand(_) | Declaration::BackgroundTbdReset(_) => {
            Some("background-tbd")
        }
        Declaration::BackgroundImage(_) => Some("background-image"),
        Declaration::BackgroundAttachment(_) => Some("background-attachment"),
        Declaration::BackgroundClip(_) => Some("background-clip"),
        Declaration::BackgroundOrigin(_) => Some("background-origin"),
        Declaration::BackgroundPosition(_) | Declaration::BackgroundPositionKeyword(_) => {
            Some("background-position")
        }
        Declaration::BackgroundPositionX(_) | Declaration::BackgroundPositionComputedX(_) => {
            Some("background-position-x")
        }
        Declaration::BackgroundPositionY(_) | Declaration::BackgroundPositionComputedY(_) => {
            Some("background-position-y")
        }
        Declaration::BackgroundPositionInline(_) => Some("background-position-inline"),
        Declaration::BackgroundPositionBlock(_) => Some("background-position-block"),
        Declaration::BackgroundRepeat(_) | Declaration::BackgroundRepeatKeyword(_) => {
            Some("background-repeat")
        }
        Declaration::BackgroundRepeatX(_) => Some("background-repeat-x"),
        Declaration::BackgroundRepeatY(_) => Some("background-repeat-y"),
        Declaration::BackgroundRepeatInline(_) => Some("background-repeat-inline"),
        Declaration::BackgroundRepeatBlock(_) => Some("background-repeat-block"),
        Declaration::BackgroundSize(_) => Some("background-size"),
        Declaration::Layout(layout) => Some(&layout.name),
        Declaration::BorderRadius(_) => Some("border-radius"),
        _ => None,
    }
}

fn declaration_affects(
    declaration: &Declaration,
    property: &str,
    writing: crate::formatting::Writing,
) -> bool {
    if matches!(declaration, Declaration::All(_)) {
        return true;
    }
    let Some(important_property) = animated_property(declaration) else {
        return false;
    };
    properties_overlap(important_property, property, writing)
}

fn properties_overlap(first: &str, second: &str, writing: crate::formatting::Writing) -> bool {
    let first = physical_background_axis(first, writing);
    let second = physical_background_axis(second, writing);
    first == second || shorthand_affects(first, second) || shorthand_affects(second, first)
}

fn shorthand_affects(shorthand: &str, property: &str) -> bool {
    match shorthand {
        "background" => property.starts_with("background-"),
        "background-tbd" => property.starts_with("background-") && property != "background-color",
        "background-position" => property.starts_with("background-position-"),
        "background-repeat" => property.starts_with("background-repeat-"),
        "margin" => property.starts_with("margin-") && property != "margin-trim",
        "padding" => property.starts_with("padding-"),
        "inset" => matches!(
            property,
            "top"
                | "right"
                | "bottom"
                | "left"
                | "inset-block-start"
                | "inset-block-end"
                | "inset-inline-start"
                | "inset-inline-end"
        ),
        "border" => {
            property == "border-width"
                || property == "border-color"
                || property == "border-style"
                || ["top", "right", "bottom", "left"].iter().any(|side| {
                    property == format!("border-{side}")
                        || property == format!("border-{side}-width")
                        || property == format!("border-{side}-color")
                        || property == format!("border-{side}-style")
                })
        }
        "border-width" => ["top", "right", "bottom", "left"]
            .iter()
            .any(|side| property == format!("border-{side}-width")),
        "border-color" => ["top", "right", "bottom", "left"]
            .iter()
            .any(|side| property == format!("border-{side}-color")),
        "outline" => matches!(
            property,
            "outline-color" | "outline-width" | "outline-offset"
        ),
        "column-rule" => matches!(property, "column-rule-color" | "column-rule-width"),
        "columns" => matches!(property, "column-width" | "column-count"),
        "flex" => matches!(property, "flex-basis" | "flex-grow" | "flex-shrink"),
        "gap" => matches!(property, "row-gap" | "column-gap"),
        "border-top" | "border-right" | "border-bottom" | "border-left" => {
            let side = shorthand.strip_prefix("border-").unwrap();
            matches!(property, "border-width" | "border-color" | "border-style")
                || property == format!("border-{side}-width")
                || property == format!("border-{side}-color")
                || property == format!("border-{side}-style")
        }
        _ => false,
    }
}

fn physical_background_axis(property: &str, writing: crate::formatting::Writing) -> &str {
    match property {
        "background-position-inline" => {
            if writing.vertical() {
                "background-position-y"
            } else {
                "background-position-x"
            }
        }
        "background-position-block" => {
            if writing.vertical() {
                "background-position-x"
            } else {
                "background-position-y"
            }
        }
        "background-repeat-inline" => {
            if writing.vertical() {
                "background-repeat-y"
            } else {
                "background-repeat-x"
            }
        }
        "background-repeat-block" => {
            if writing.vertical() {
                "background-repeat-x"
            } else {
                "background-repeat-y"
            }
        }
        _ => property,
    }
}
