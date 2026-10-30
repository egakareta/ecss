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
        .map(|entry| (entry.state, entry.declaration))
        .collect()
}
