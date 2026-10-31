use super::*;

pub(super) fn matching_rules<'a>(
    stylesheet: &'a Stylesheet,
    target: &'a crate::css::SelectorTarget,
    viewport: Vec2,
) -> impl Iterator<Item = &'a crate::css::Rule> {
    stylesheet
        .rule_index
        .matching(&stylesheet.rules, target)
        .filter(move |rule| rule.pseudo.is_none() && rule.matches_media(viewport))
}

pub(super) fn applicable_rules<'a>(
    stylesheet: &'a Stylesheet,
    target: &'a crate::css::SelectorTarget,
    state: u8,
    viewport: Vec2,
) -> impl Iterator<Item = &'a crate::css::Rule> + 'a {
    matching_rules(stylesheet, target, viewport).filter(move |rule| rule.state.matches(state))
}

pub(super) fn matching_pseudo_rules<'a>(
    stylesheet: &'a Stylesheet,
    target: &'a crate::css::SelectorTarget,
    pseudo: crate::css::PseudoElementKind,
    state: u8,
    viewport: Vec2,
) -> Vec<&'a crate::css::Rule> {
    if !stylesheet.rule_index.has_pseudo_rules {
        return Vec::new();
    }
    stylesheet
        .rule_index
        .matching(&stylesheet.rules, target)
        .filter(|rule| {
            rule.pseudo == Some(pseudo) && rule.state.matches(state) && rule.matches_media(viewport)
        })
        .collect()
}

pub(super) fn pseudo_text_rects(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    pseudo: crate::css::PseudoElementKind,
    vertical: bool,
) -> Vec<Rect> {
    fn collect(shape: &egui::Shape, text: &mut Vec<(Arc<egui::Galley>, Pos2)>) {
        match shape {
            egui::Shape::Text(shape) if shape.angle == 0.0 => {
                text.push((shape.galley.clone(), shape.pos));
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let Some(text) = context.graphics(|layers| {
        let list = layers.get(layer)?;
        let mut text = Vec::new();
        for entry in list.all_entries().skip(start.0).take(end.0 - start.0) {
            collect(&entry.shape, &mut text);
        }
        Some(text)
    }) else {
        return Vec::new();
    };
    let lines = text
        .iter()
        .filter_map(|(galley, position)| {
            let row = galley.rows.iter().find(|row| !row.glyphs.is_empty())?;
            let rect = row
                .rect_without_leading_space()
                .translate(position.to_vec2());
            let baseline = row
                .glyphs
                .first()
                .map(|glyph| position.y + row.pos.y + glyph.pos.y)
                .unwrap_or(rect.bottom());
            rect.is_positive()
                .then_some((row, *position, rect, baseline))
        })
        .collect::<Vec<_>>();
    let Some((first_row, position, first_rect, first_baseline)) = lines.first() else {
        return Vec::new();
    };
    match pseudo {
        crate::css::PseudoElementKind::FirstLine => {
            let reference = if vertical {
                first_rect.left()
            } else {
                *first_baseline
            };
            let mut rect = Rect::NOTHING;
            for (_, _, line, baseline) in lines {
                let coordinate = if vertical { line.left() } else { baseline };
                if (coordinate - reference).abs() <= 1.0 {
                    rect = rect.union(line);
                }
            }
            rect.is_positive().then_some(rect).into_iter().collect()
        }
        crate::css::PseudoElementKind::FirstLetter => {
            let glyphs = &first_row.glyphs;
            let characters = glyphs.iter().map(|glyph| glyph.chr).collect::<Vec<_>>();
            let Some(range) = first_letter_character_range(&characters) else {
                return Vec::new();
            };
            let row_offset = first_row.pos.to_vec2() + position.to_vec2();
            let mut rect = Rect::NOTHING;
            for glyph in &glyphs[range] {
                let glyph_rect = if vertical {
                    let vertex_start = glyph.first_vertex as usize;
                    first_row
                        .visuals
                        .mesh
                        .vertices
                        .get(vertex_start..vertex_start.saturating_add(4))
                        .filter(|vertices| vertices.len() == 4)
                        .map(|vertices| {
                            vertices.iter().fold(Rect::NOTHING, |bounds, vertex| {
                                bounds.union(Rect::from_min_max(vertex.pos, vertex.pos))
                            })
                        })
                        .map(|rect| rect.translate(row_offset))
                } else {
                    Some(glyph.logical_rect().translate(row_offset))
                };
                if let Some(glyph_rect) = glyph_rect {
                    rect = rect.union(glyph_rect);
                }
            }
            rect.is_positive().then_some(rect).into_iter().collect()
        }
    }
}

pub(super) fn is_first_letter_punctuation(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::ClosePunctuation
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::OtherPunctuation
    )
}

pub(super) fn is_first_letter_character(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::CurrencySymbol
            | GeneralCategory::DecimalNumber
            | GeneralCategory::LetterNumber
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::MathSymbol
            | GeneralCategory::ModifierLetter
            | GeneralCategory::ModifierSymbol
            | GeneralCategory::OtherLetter
            | GeneralCategory::OtherNumber
            | GeneralCategory::OtherSymbol
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::UppercaseLetter
    )
}

pub(super) fn is_first_letter_space(character: char) -> bool {
    get_general_category(character) == GeneralCategory::SpaceSeparator && character != '\u{3000}'
}

pub(super) fn is_first_letter_suffix_punctuation(character: char) -> bool {
    is_first_letter_punctuation(character)
        && !matches!(
            get_general_category(character),
            GeneralCategory::DashPunctuation | GeneralCategory::OpenPunctuation
        )
}

pub(super) fn first_letter_character_range(characters: &[char]) -> Option<std::ops::Range<usize>> {
    let letter = characters
        .iter()
        .position(|character| is_first_letter_character(*character))?;

    let mut prefix_start = letter;
    while prefix_start > 0
        && (is_first_letter_punctuation(characters[prefix_start - 1])
            || is_first_letter_space(characters[prefix_start - 1])
            || is_combining_mark(characters[prefix_start - 1]))
    {
        prefix_start -= 1;
    }
    let from = (prefix_start..letter)
        .find(|index| is_first_letter_punctuation(characters[*index]))
        .unwrap_or(letter);

    let mut to = letter + 1;
    while to < characters.len() && is_combining_mark(characters[to]) {
        to += 1;
    }
    let mut suffix_end = None;
    let mut cursor = to;
    while cursor < characters.len()
        && (is_first_letter_suffix_punctuation(characters[cursor])
            || is_first_letter_space(characters[cursor]) && characters[cursor] != ' '
            || is_combining_mark(characters[cursor]))
    {
        if is_first_letter_suffix_punctuation(characters[cursor])
            || is_combining_mark(characters[cursor]) && suffix_end.is_some()
        {
            suffix_end = Some(cursor + 1);
        }
        cursor += 1;
    }
    if let Some(suffix_end) = suffix_end {
        to = suffix_end;
    }
    Some(from..to)
}

pub(super) fn is_combining_mark(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::EnclosingMark
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
    )
}

pub(super) fn pseudo_decoration_style(
    stylesheet: &Stylesheet,
    target: &crate::css::SelectorTarget,
    pseudo: crate::css::PseudoElementKind,
    state: u8,
    parent: &BoxStyle,
    units: Units,
    current: Color32,
) -> Option<(BoxStyle, Color32, Color32)> {
    let rules = matching_pseudo_rules(stylesheet, target, pseudo, state, units.viewport);
    if rules.is_empty() {
        return None;
    }
    let mut style = BoxStyle::default();
    style.layout.direction = parent.layout.direction;
    style.format = parent.format.clone();
    style.current_color = current;
    let ordered = crate::css::cascade::ordered(rules.iter().copied());
    let custom_declarations = ordered
        .iter()
        .map(|(_, declaration)| *declaration)
        .collect::<Vec<_>>();
    let custom_properties =
        crate::css::variables::compute(&parent.custom_properties, &custom_declarations);
    style.custom_properties = custom_properties.clone();
    let mut fill = Color32::TRANSPARENT;
    let mut color = current;
    let resolved = ordered
        .into_iter()
        .filter_map(|(state, source)| {
            crate::css::resolve_deferred(source, &style.custom_properties)
                .map(|declaration| (state, declaration))
        })
        .collect();
    let (rules, all_keyword) = super::apply_all_rules(
        resolved,
        &parent.computed_declarations,
        parent.background_color,
    );
    match all_keyword {
        Some(crate::css::AllKeyword::Inherit) => {
            style = parent.clone();
            style.custom_properties = custom_properties.clone();
            style.text_shadows.clear();
            color = parent.current_color;
        }
        Some(crate::css::AllKeyword::Unset) => color = parent.current_color,
        Some(crate::css::AllKeyword::Initial | crate::css::AllKeyword::Revert) => {
            style = BoxStyle::default();
            style.layout.direction = parent.layout.direction;
            style.custom_properties = custom_properties;
            color = Color32::BLACK;
        }
        None => {}
    }
    for (_, declarations) in rules {
        for declaration in declarations {
            match &declaration {
                Declaration::Color(value) => {
                    color = *value;
                    style.current_color = *value;
                }
                Declaration::Background(value) => {
                    fill = *value;
                    style.background_color = *value;
                    style.background_set = true;
                    style.background_current_color = false;
                }
                Declaration::BackgroundShorthand(value) => {
                    fill = value.color;
                    style.background_color = value.color;
                    style.background_set = true;
                    style.background_current_color = false;
                }
                Declaration::Layout(value)
                    if matches!(
                        value.name.as_str(),
                        "color" | "background" | "background-color"
                    ) =>
                {
                    value
                        .apply_with_parent(&mut style, parent, units)
                        .expect("validated pseudo-element color declaration");
                    color = style.current_color;
                    if matches!(value.name.as_str(), "background" | "background-color") {
                        fill = style.background_color;
                    }
                }
                Declaration::Layout(value)
                    if pseudo_decoration_property_applies(pseudo, &value.name) =>
                {
                    value
                        .apply_with_parent(&mut style, parent, units)
                        .expect("validated pseudo-element decoration declaration");
                }
                Declaration::BorderRadius(radius) => {
                    let radius = f32::from(*radius);
                    let radius = crate::box_decoration::CornerRadius {
                        x: crate::box_decoration::LengthPercentage::Length(radius),
                        y: crate::box_decoration::LengthPercentage::Length(radius),
                    };
                    style.decorations.radii = [radius; 4];
                }
                _ => {}
            }
        }
    }
    if style.background_current_color {
        style.background_color = style.current_color;
        fill = style.current_color;
    }
    style.finish();
    Some((style, fill, color))
}

pub(super) fn pseudo_decoration_property_applies(
    pseudo: crate::css::PseudoElementKind,
    name: &str,
) -> bool {
    match pseudo {
        crate::css::PseudoElementKind::FirstLetter => {
            name.starts_with("border")
                || name.starts_with("corner-")
                || name.starts_with("box-shadow")
                || name == "text-shadow"
                || name == "box-decoration-break"
                || name == "padding"
                || name.starts_with("padding-")
                || name == "margin"
                || name.starts_with("margin-")
        }
        crate::css::PseudoElementKind::FirstLine => {
            name.starts_with("box-shadow")
                || name == "text-shadow"
                || name.starts_with("border-image")
                || name == "border-radius"
                || name.ends_with("-radius")
        }
    }
}

pub(super) struct PseudoDecorationInput<'a> {
    pub(super) context: &'a Context,
    pub(super) stylesheet: &'a Stylesheet,
    pub(super) rect: Rect,
    pub(super) style: &'a BoxStyle,
    pub(super) percentage_basis: f32,
    pub(super) fill: Color32,
    pub(super) current: Color32,
    pub(super) blocked_element_ids: &'a HashSet<String>,
}

pub(super) fn pseudo_decoration_shapes(input: PseudoDecorationInput<'_>) -> Vec<egui::Shape> {
    let PseudoDecorationInput {
        context,
        stylesheet,
        rect,
        style,
        percentage_basis,
        fill,
        current,
        blocked_element_ids,
    } = input;
    let basis = percentage_basis;
    let edge = |value: taffy::LengthPercentageAuto, basis| {
        resolve(value.into_raw(), basis).unwrap_or(0.0).max(0.0)
    };
    let padding = egui::epaint::MarginF32 {
        top: edge(style.layout.padding.top.into(), basis),
        right: edge(style.layout.padding.right.into(), basis),
        bottom: edge(style.layout.padding.bottom.into(), basis),
        left: edge(style.layout.padding.left.into(), basis),
    };
    let margin = egui::epaint::MarginF32 {
        top: edge(style.layout.margin.top, basis),
        right: edge(style.layout.margin.right, basis),
        bottom: edge(style.layout.margin.bottom, basis),
        left: edge(style.layout.margin.left, basis),
    };
    let border_widths = style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0).max(0.0));
    let stroke = egui::epaint::MarginF32 {
        top: border_widths.top,
        right: border_widths.right,
        bottom: border_widths.bottom,
        left: border_widths.left,
    };
    let border = Rect::from_min_max(
        pos2(
            rect.left() - padding.left - stroke.left,
            rect.top() - padding.top - stroke.top,
        ),
        pos2(
            rect.right() + padding.right + stroke.right,
            rect.bottom() + padding.bottom + stroke.bottom,
        ),
    );
    let geometry = BoxDecorationGeometry::new(border, style, stroke, padding, margin);
    let mut shapes = crate::box_decoration::shadow_shapes(
        border,
        style,
        current,
        crate::box_decoration::ShadowPosition::Outset,
        stroke,
        padding,
        margin,
    );
    if fill.a() > 0 {
        shapes.push(box_surface_shape(
            border,
            &geometry,
            crate::css::BackgroundClip::Border,
            fill,
            egui::CornerRadius::ZERO,
        ));
    }
    shapes.extend(crate::box_decoration::shadow_shapes(
        border,
        style,
        current,
        crate::box_decoration::ShadowPosition::Inset,
        stroke,
        padding,
        margin,
    ));
    if let Some(border_image) = border_image_shapes(
        border,
        style,
        &stylesheet.background_images,
        context,
        blocked_element_ids,
    ) {
        shapes.extend(border_image);
    } else if let Some(border_shape) =
        border_shape_shapes(border, style, stroke, padding, margin, current)
    {
        shapes.extend(border_shape);
    } else {
        shapes.extend(border_shapes(border, style, current));
    }
    shapes
}
