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

#[derive(Clone, Copy)]
pub(crate) struct LogicalAxes<'a> {
    pub(crate) direction: taffy::Direction,
    pub(crate) writing: crate::formatting::Writing,
    pub(crate) text_orientation: &'a str,
}

#[derive(Clone, Copy)]
pub(crate) struct AnimationPriority<'a> {
    pub(crate) important: &'a [Declaration],
    pub(crate) axes: LogicalAxes<'a>,
}

pub(crate) fn overrides_animation(
    important: &[Declaration],
    animated: &Declaration,
    axes: LogicalAxes<'_>,
) -> bool {
    let Some(property) = animated_property(animated) else {
        return false;
    };
    important
        .iter()
        .any(|declaration| declaration_affects(declaration, property, axes))
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
        Declaration::FontFamily(_) => Some("font-family"),
        Declaration::FontWeight(_) => Some("font-weight"),
        Declaration::FontStyle(_) => Some("font-style"),
        Declaration::FontShorthand(_) => Some("font"),
        _ => None,
    }
}

fn declaration_affects(declaration: &Declaration, property: &str, axes: LogicalAxes<'_>) -> bool {
    if matches!(declaration, Declaration::All(_)) {
        return true;
    }
    if matches!(declaration, Declaration::FontShorthand(_)) {
        return property == "font" || property.starts_with("font-") || property == "line-height";
    }
    let Some(important_property) = animated_property(declaration) else {
        return false;
    };
    properties_overlap(important_property, property, axes)
}

fn properties_overlap(first: &str, second: &str, axes: LogicalAxes<'_>) -> bool {
    let first = atomic_properties(first, axes);
    let second = atomic_properties(second, axes);
    first.iter().any(|property| second.contains(property))
}

fn atomic_properties(property: &str, axes: LogicalAxes<'_>) -> Vec<String> {
    let mut result = Vec::new();
    for property in physical_properties(property, axes) {
        match property.as_str() {
            "background" | "background-tbd" => result.extend(
                [
                    "background-color",
                    "background-image",
                    "background-attachment",
                    "background-clip",
                    "background-origin",
                    "background-position",
                    "background-position-x",
                    "background-position-y",
                    "background-repeat",
                    "background-repeat-x",
                    "background-repeat-y",
                    "background-size",
                ]
                .into_iter()
                .filter(|name| property != "background-tbd" || *name != "background-color")
                .map(str::to_owned),
            ),
            "background-position" => {
                result.extend(["background-position-x", "background-position-y"].map(str::to_owned))
            }
            "background-repeat" => {
                result.extend(["background-repeat-x", "background-repeat-y"].map(str::to_owned))
            }
            "font" => result.extend(
                [
                    "font-family",
                    "font-size",
                    "font-style",
                    "font-weight",
                    "font-kerning",
                    "line-height",
                ]
                .map(str::to_owned),
            ),
            "margin" | "padding" | "inset" => {
                let prefix = if property == "inset" { "" } else { &property };
                result.extend(["top", "right", "bottom", "left"].map(|side| {
                    if prefix.is_empty() {
                        side.to_owned()
                    } else {
                        format!("{prefix}-{side}")
                    }
                }));
            }
            "border" => result.extend(border_properties(None)),
            "border-width" | "border-color" | "border-style" => {
                let suffix = property.strip_prefix("border-").unwrap();
                result.extend(border_properties(Some(suffix)));
            }
            "border-top" | "border-right" | "border-bottom" | "border-left" => {
                let side = property.strip_prefix("border-").unwrap();
                result.extend(border_side_properties(side));
            }
            "outline" => result
                .extend(["outline-width", "outline-style", "outline-color"].map(str::to_owned)),
            "column-rule" => result.extend(
                [
                    "column-rule-width",
                    "column-rule-style",
                    "column-rule-color",
                ]
                .map(str::to_owned),
            ),
            "columns" => result.extend(["column-width", "column-count"].map(str::to_owned)),
            "flex" => result.extend(["flex-basis", "flex-grow", "flex-shrink"].map(str::to_owned)),
            "gap" => result.extend(["row-gap", "column-gap"].map(str::to_owned)),
            _ => result.push(property),
        }
    }
    result
}

fn border_properties(component: Option<&str>) -> Vec<String> {
    ["top", "right", "bottom", "left"]
        .into_iter()
        .flat_map(|side| match component {
            Some(component) => vec![format!("border-{side}-{component}")],
            None => border_side_properties(side),
        })
        .collect()
}

fn border_side_properties(side: &str) -> Vec<String> {
    ["width", "style", "color"]
        .map(|component| format!("border-{side}-{component}"))
        .to_vec()
}

pub(crate) fn physical_properties(property: &str, axes: LogicalAxes<'_>) -> Vec<String> {
    let background_axis = match property {
        "background-position-inline" => Some(if axes.writing.vertical() {
            "background-position-y"
        } else {
            "background-position-x"
        }),
        "background-position-block" => Some(if axes.writing.vertical() {
            "background-position-x"
        } else {
            "background-position-y"
        }),
        "background-repeat-inline" => Some(if axes.writing.vertical() {
            "background-repeat-y"
        } else {
            "background-repeat-x"
        }),
        "background-repeat-block" => Some(if axes.writing.vertical() {
            "background-repeat-x"
        } else {
            "background-repeat-y"
        }),
        _ => None,
    };
    if let Some(property) = background_axis {
        return vec![property.to_owned()];
    }

    for group in ["margin", "padding", "inset"] {
        let Some(suffix) = property.strip_prefix(&format!("{group}-")) else {
            continue;
        };
        let sides = match suffix {
            "inline" => vec!["inline-start", "inline-end"],
            "block" => vec!["block-start", "block-end"],
            "inline-start" | "inline-end" | "block-start" | "block-end" => vec![suffix],
            _ => Vec::new(),
        };
        if !sides.is_empty() {
            return sides
                .iter()
                .map(|side| {
                    let physical = physical_side(side, axes);
                    if group == "inset" {
                        physical.to_owned()
                    } else {
                        format!("{group}-{physical}")
                    }
                })
                .collect();
        }
    }

    let Some(suffix) = property.strip_prefix("border-") else {
        return physical_size_property(property, axes);
    };
    for logical in [
        "inline-start",
        "inline-end",
        "block-start",
        "block-end",
        "inline",
        "block",
    ] {
        if suffix == logical || suffix.strip_prefix(&format!("{logical}-")).is_some() {
            let tail = &suffix[logical.len()..];
            let sides = match logical {
                "inline-start" | "inline-end" | "block-start" | "block-end" => vec![logical],
                "inline" => vec!["inline-start", "inline-end"],
                _ => vec!["block-start", "block-end"],
            };
            return sides
                .into_iter()
                .map(|side| format!("border-{}{tail}", physical_side(side, axes)))
                .collect();
        }
    }
    physical_size_property(property, axes)
}

fn physical_size_property(property: &str, axes: LogicalAxes<'_>) -> Vec<String> {
    let (prefix, axis) = if let Some(axis) = property.strip_prefix("min-") {
        ("min-", axis)
    } else if let Some(axis) = property.strip_prefix("max-") {
        ("max-", axis)
    } else {
        ("", property)
    };
    let physical = match axis {
        "inline-size" => Some(if axes.writing.vertical() {
            "height"
        } else {
            "width"
        }),
        "block-size" => Some(if axes.writing.vertical() {
            "width"
        } else {
            "height"
        }),
        _ => None,
    };
    physical.map_or_else(
        || vec![property.to_owned()],
        |physical| vec![format!("{prefix}{physical}")],
    )
}

fn physical_side(side: &str, axes: LogicalAxes<'_>) -> &'static str {
    ["top", "right", "bottom", "left"][crate::layout_style::logical_side(
        side,
        axes.direction,
        axes.writing,
        axes.text_orientation,
    )
    .expect("logical side is valid")]
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
