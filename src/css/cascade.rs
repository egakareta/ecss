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

pub(crate) fn overrides_animation(important: &[Declaration], animated: &Declaration) -> bool {
    let Some(property) = animated_property(animated) else {
        return false;
    };
    important
        .iter()
        .any(|declaration| declaration_affects(declaration, property))
}

fn animated_property(declaration: &Declaration) -> Option<&str> {
    match declaration {
        Declaration::Color(_) => Some("color"),
        Declaration::Background(_) => Some("background-color"),
        Declaration::BackgroundShorthand(_) => Some("background"),
        Declaration::Layout(layout) => Some(&layout.name),
        Declaration::BorderRadius(_) => Some("border-radius"),
        _ => None,
    }
}

fn declaration_affects(declaration: &Declaration, property: &str) -> bool {
    if matches!(declaration, Declaration::All(_)) {
        return true;
    }
    let Some(important_property) = animated_property(declaration) else {
        return false;
    };
    properties_overlap(important_property, property)
}

fn properties_overlap(first: &str, second: &str) -> bool {
    first == second || shorthand_affects(first, second) || shorthand_affects(second, first)
}

fn shorthand_affects(shorthand: &str, property: &str) -> bool {
    match shorthand {
        "background" => property.starts_with("background-"),
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
