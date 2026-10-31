use super::*;

pub(super) type BackgroundPaint = crate::css::BackgroundValues;

#[derive(Clone, Copy)]
pub(super) struct BackgroundContext {
    pub(super) writing: crate::formatting::Writing,
    pub(super) direction: taffy::Direction,
}

pub(super) fn resolved_background_from_rules(
    rules: &[(RuleState, Vec<Declaration>)],
    parent: Option<&BackgroundPaint>,
    parent_context: BackgroundContext,
    context: BackgroundContext,
) -> BackgroundPaint {
    background_from_declarations(
        rules
            .iter()
            .flat_map(|(_, declarations)| declarations.iter()),
        parent,
        parent_context,
        context,
    )
}

pub(super) fn resolved_background(
    stylesheet: &Stylesheet,
    target: &crate::css::SelectorTarget,
    state: u8,
    viewport: Vec2,
) -> BackgroundPaint {
    let ordered = crate::css::cascade::ordered(
        matching_rules(stylesheet, target, viewport).filter(move |rule| rule.state.matches(state)),
    );
    let context = BackgroundContext {
        writing: crate::formatting::Writing::Horizontal,
        direction: taffy::Direction::Ltr,
    };
    background_from_declarations(
        ordered.into_iter().map(|(_, declaration)| declaration),
        None,
        context,
        context,
    )
}

pub(super) fn background_from_declarations<'a>(
    declarations: impl Iterator<Item = &'a Declaration>,
    parent: Option<&BackgroundPaint>,
    parent_context: BackgroundContext,
    context: BackgroundContext,
) -> BackgroundPaint {
    let mut background = BackgroundPaint::default();
    for declaration in declarations {
        match declaration {
            Declaration::All(keyword) | Declaration::BackgroundReset(keyword) => {
                apply_background_keyword(&mut background, *keyword, parent);
            }
            Declaration::BackgroundTbdReset(keyword) => {
                apply_background_tbd_keyword(&mut background, *keyword, parent);
            }
            Declaration::Background(color) => background.color = *color,
            Declaration::BackgroundShorthand(value) => {
                background.color = value.color;
                set_background_layers(&mut background, value, context.writing);
            }
            Declaration::BackgroundTbdShorthand(value) => {
                set_background_layers(&mut background, value, context.writing);
            }
            Declaration::BackgroundImage(images) => background.images = images.clone(),
            Declaration::BackgroundAttachment(attachments) => {
                background.attachments = attachments.clone();
            }
            Declaration::BackgroundClip(clips) => background.clips = clips.clone(),
            Declaration::BackgroundOrigin(origins) => background.origins = origins.clone(),
            Declaration::BackgroundPosition(positions) => {
                set_position_lists(&mut background, positions)
            }
            Declaration::BackgroundRepeat(repeats) => {
                set_repeat_lists(&mut background, repeats, context.writing)
            }
            Declaration::BackgroundPositionKeyword(keyword) => {
                apply_position_keyword(&mut background, *keyword, parent, true, true)
            }
            Declaration::BackgroundPositionX(value) => apply_position_axis(
                &mut background,
                value,
                false,
                false,
                parent_context,
                context,
                parent,
            ),
            Declaration::BackgroundPositionY(value) => apply_position_axis(
                &mut background,
                value,
                true,
                false,
                parent_context,
                context,
                parent,
            ),
            Declaration::BackgroundPositionInline(value) => apply_position_axis(
                &mut background,
                value,
                false,
                true,
                parent_context,
                context,
                parent,
            ),
            Declaration::BackgroundPositionBlock(value) => apply_position_axis(
                &mut background,
                value,
                true,
                true,
                parent_context,
                context,
                parent,
            ),
            Declaration::BackgroundRepeatKeyword(keyword) => {
                apply_repeat_keyword(&mut background, *keyword, parent, true, true)
            }
            Declaration::BackgroundRepeatX(value) => {
                apply_repeat_axis(&mut background, value, false, parent)
            }
            Declaration::BackgroundRepeatY(value) => {
                apply_repeat_axis(&mut background, value, true, parent)
            }
            Declaration::BackgroundRepeatInline(value) => apply_logical_repeat_axis(
                &mut background,
                value,
                true,
                parent_context,
                context,
                parent,
            ),
            Declaration::BackgroundRepeatBlock(value) => apply_logical_repeat_axis(
                &mut background,
                value,
                false,
                parent_context,
                context,
                parent,
            ),
            Declaration::BackgroundPositionComputedX(values) => {
                background.position_x = values
                    .iter()
                    .cloned()
                    .map(crate::css::BackgroundPositionAxis::Computed)
                    .collect();
            }
            Declaration::BackgroundPositionComputedY(values) => {
                background.position_y = values
                    .iter()
                    .cloned()
                    .map(crate::css::BackgroundPositionAxis::Computed)
                    .collect();
            }
            Declaration::BackgroundSize(sizes) => background.sizes = sizes.clone(),
            Declaration::Layout(layout)
                if layout.value.eq_ignore_ascii_case("initial")
                    || layout.value.eq_ignore_ascii_case("unset")
                    || layout.value.eq_ignore_ascii_case("revert")
                    || layout.value.eq_ignore_ascii_case("inherit") =>
            {
                let defaults = BackgroundPaint::default();
                match layout.name.as_str() {
                    "background" => background = defaults,
                    "background-color" => background.color = Color32::TRANSPARENT,
                    "background-image" => background.images.clear(),
                    "background-attachment" => background.attachments.clear(),
                    "background-clip" => background.clips.clear(),
                    "background-origin" => background.origins.clear(),
                    "background-position" => {
                        background.position_x = defaults.position_x;
                        background.position_y = defaults.position_y;
                    }
                    "background-position-x" => background.position_x = defaults.position_x,
                    "background-position-y" => background.position_y = defaults.position_y,
                    "background-repeat" => {
                        background.repeat_x = defaults.repeat_x;
                        background.repeat_y = defaults.repeat_y;
                    }
                    "background-repeat-x" => background.repeat_x = defaults.repeat_x,
                    "background-repeat-y" => background.repeat_y = defaults.repeat_y,
                    "background-size" => background.sizes.clear(),
                    "background-tbd" => {
                        let color = background.color;
                        background = defaults;
                        background.color = color;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    background
}

fn apply_background_keyword(
    background: &mut BackgroundPaint,
    keyword: crate::css::AllKeyword,
    parent: Option<&BackgroundPaint>,
) {
    if keyword == crate::css::AllKeyword::Inherit {
        *background = parent.cloned().unwrap_or_default();
    } else {
        *background = BackgroundPaint::default();
    }
}

fn apply_background_tbd_keyword(
    background: &mut BackgroundPaint,
    keyword: crate::css::AllKeyword,
    parent: Option<&BackgroundPaint>,
) {
    let color = background.color;
    *background = if keyword == crate::css::AllKeyword::Inherit {
        parent.cloned().unwrap_or_default()
    } else {
        BackgroundPaint::default()
    };
    background.color = color;
}

fn set_background_layers(
    background: &mut BackgroundPaint,
    value: &crate::css::BackgroundShorthand,
    writing: crate::formatting::Writing,
) {
    background.images.clone_from(&value.images);
    background.attachments.clone_from(&value.attachments);
    background.clips.clone_from(&value.clips);
    background.origins.clone_from(&value.origins);
    set_position_lists(background, &value.positions);
    set_repeat_lists(background, &value.repeats, writing);
    background.sizes.clone_from(&value.sizes);
}

fn set_position_lists(
    background: &mut BackgroundPaint,
    positions: &[crate::css::BackgroundPosition],
) {
    background.position_x = positions
        .iter()
        .map(|position| position.x.clone())
        .collect();
    background.position_y = positions
        .iter()
        .map(|position| position.y.clone())
        .collect();
}

fn set_position_axis(
    background: &mut BackgroundPaint,
    values: &crate::css::BackgroundAxisValue<crate::css::BackgroundPositionAxis>,
    y: bool,
    parent: Option<&BackgroundPaint>,
) {
    let target = if y {
        &mut background.position_y
    } else {
        &mut background.position_x
    };
    match values {
        crate::css::BackgroundAxisValue::Values(values) => *target = values.clone(),
        crate::css::BackgroundAxisValue::Keyword(keyword) => {
            apply_position_keyword_axis(target, *keyword, parent, y)
        }
    }
}

fn apply_position_axis(
    background: &mut BackgroundPaint,
    values: &crate::css::BackgroundAxisValue<crate::css::BackgroundPositionAxis>,
    y: bool,
    logical: bool,
    parent_context: BackgroundContext,
    context: BackgroundContext,
    parent: Option<&BackgroundPaint>,
) {
    if !logical {
        match values {
            crate::css::BackgroundAxisValue::Values(values) => {
                let block = if y {
                    !context.writing.vertical()
                } else {
                    context.writing.vertical()
                };
                let values = values
                    .iter()
                    .cloned()
                    .map(|value| map_axis_position(value, block, context))
                    .collect();
                set_position_axis(
                    background,
                    &crate::css::BackgroundAxisValue::Values(values),
                    y,
                    parent,
                );
            }
            crate::css::BackgroundAxisValue::Keyword(_) => {
                set_position_axis(background, values, y, parent);
            }
        }
        return;
    }
    if let crate::css::BackgroundAxisValue::Keyword(crate::css::AllKeyword::Inherit) = values
        && let Some(parent) = parent
    {
        let parent_y = if y {
            !parent_context.writing.vertical()
        } else {
            parent_context.writing.vertical()
        };
        let parent_values = if parent_y {
            &parent.position_y
        } else {
            &parent.position_x
        };
        let values = parent_values
            .iter()
            .cloned()
            .map(|value| {
                map_logical_position(value, y, parent_context.writing, parent_context.direction)
            })
            .map(|value| map_logical_position(value, y, context.writing, context.direction))
            .collect();
        set_position_axis(
            background,
            &crate::css::BackgroundAxisValue::Values(values),
            if y {
                !context.writing.vertical()
            } else {
                context.writing.vertical()
            },
            None,
        );
        return;
    }
    let target_y = if y {
        !context.writing.vertical()
    } else {
        context.writing.vertical()
    };
    match values {
        crate::css::BackgroundAxisValue::Values(values) => {
            let values = values
                .iter()
                .cloned()
                .map(|value| map_logical_position(value, y, context.writing, context.direction))
                .collect();
            set_position_axis(
                background,
                &crate::css::BackgroundAxisValue::Values(values),
                target_y,
                parent,
            );
        }
        crate::css::BackgroundAxisValue::Keyword(_) => {
            let value = map_logical_position(
                crate::css::BackgroundPositionAxis::Start("0%".to_owned()),
                y,
                context.writing,
                context.direction,
            );
            if target_y {
                background.position_y = vec![value];
            } else {
                background.position_x = vec![value];
            }
        }
    }
}

fn apply_position_keyword(
    background: &mut BackgroundPaint,
    keyword: crate::css::AllKeyword,
    parent: Option<&BackgroundPaint>,
    x: bool,
    y: bool,
) {
    if x {
        apply_position_keyword_axis(&mut background.position_x, keyword, parent, false);
    }
    if y {
        apply_position_keyword_axis(&mut background.position_y, keyword, parent, true);
    }
}

fn apply_position_keyword_axis(
    target: &mut Vec<crate::css::BackgroundPositionAxis>,
    keyword: crate::css::AllKeyword,
    parent: Option<&BackgroundPaint>,
    y: bool,
) {
    if keyword == crate::css::AllKeyword::Inherit
        && let Some(parent) = parent
    {
        *target = if y {
            parent.position_y.clone()
        } else {
            parent.position_x.clone()
        };
        return;
    }
    *target = vec![crate::css::BackgroundPositionAxis::Start("0px".to_owned())];
}

fn map_logical_position(
    value: crate::css::BackgroundPositionAxis,
    block: bool,
    writing: crate::formatting::Writing,
    direction: taffy::Direction,
) -> crate::css::BackgroundPositionAxis {
    let reverse = if block {
        writing.vertical() && writing.reverse()
    } else {
        direction == taffy::Direction::Rtl
    };
    if !reverse {
        return value;
    }
    match value {
        crate::css::BackgroundPositionAxis::Start(value) => {
            crate::css::BackgroundPositionAxis::End(value)
        }
        crate::css::BackgroundPositionAxis::End(value) => {
            crate::css::BackgroundPositionAxis::Start(value)
        }
        value => value,
    }
}

fn map_axis_position(
    value: crate::css::BackgroundPositionAxis,
    block: bool,
    context: BackgroundContext,
) -> crate::css::BackgroundPositionAxis {
    match value {
        crate::css::BackgroundPositionAxis::AxisStart(offset) => map_logical_position(
            crate::css::BackgroundPositionAxis::Start(offset),
            block,
            context.writing,
            context.direction,
        ),
        crate::css::BackgroundPositionAxis::AxisEnd(offset) => map_logical_position(
            crate::css::BackgroundPositionAxis::End(offset),
            block,
            context.writing,
            context.direction,
        ),
        value => value,
    }
}

fn set_repeat_lists(
    background: &mut BackgroundPaint,
    repeats: &[crate::css::BackgroundRepeatPair],
    writing: crate::formatting::Writing,
) {
    let mut x = Vec::with_capacity(repeats.len());
    let mut y = Vec::with_capacity(repeats.len());
    for repeat in repeats {
        let (repeat_x, repeat_y) = repeat.physical(writing);
        x.push(repeat_x);
        y.push(repeat_y);
    }
    background.repeat_x = x;
    background.repeat_y = y;
}

fn apply_repeat_axis(
    background: &mut BackgroundPaint,
    values: &crate::css::BackgroundAxisValue<crate::css::BackgroundRepeat>,
    y: bool,
    parent: Option<&BackgroundPaint>,
) {
    let target = if y {
        &mut background.repeat_y
    } else {
        &mut background.repeat_x
    };
    match values {
        crate::css::BackgroundAxisValue::Values(values) => *target = values.clone(),
        crate::css::BackgroundAxisValue::Keyword(keyword) => {
            apply_repeat_keyword_axis(target, *keyword, parent, y)
        }
    }
}

fn apply_logical_repeat_axis(
    background: &mut BackgroundPaint,
    values: &crate::css::BackgroundAxisValue<crate::css::BackgroundRepeat>,
    inline: bool,
    parent_context: BackgroundContext,
    context: BackgroundContext,
    parent: Option<&BackgroundPaint>,
) {
    if let crate::css::BackgroundAxisValue::Keyword(crate::css::AllKeyword::Inherit) = values
        && let Some(parent) = parent
    {
        let parent_y = if inline {
            parent_context.writing.vertical()
        } else {
            !parent_context.writing.vertical()
        };
        let target_y = if inline {
            context.writing.vertical()
        } else {
            !context.writing.vertical()
        };
        let source = if parent_y {
            &parent.repeat_y
        } else {
            &parent.repeat_x
        };
        if target_y {
            background.repeat_y.clone_from(source);
        } else {
            background.repeat_x.clone_from(source);
        }
        return;
    }
    let y = if inline {
        context.writing.vertical()
    } else {
        !context.writing.vertical()
    };
    apply_repeat_axis(background, values, y, parent);
}

fn apply_repeat_keyword(
    background: &mut BackgroundPaint,
    keyword: crate::css::AllKeyword,
    parent: Option<&BackgroundPaint>,
    x: bool,
    y: bool,
) {
    if x {
        apply_repeat_keyword_axis(&mut background.repeat_x, keyword, parent, false);
    }
    if y {
        apply_repeat_keyword_axis(&mut background.repeat_y, keyword, parent, true);
    }
}

fn apply_repeat_keyword_axis(
    target: &mut Vec<crate::css::BackgroundRepeat>,
    keyword: crate::css::AllKeyword,
    parent: Option<&BackgroundPaint>,
    y: bool,
) {
    if keyword == crate::css::AllKeyword::Inherit
        && let Some(parent) = parent
    {
        *target = if y {
            parent.repeat_y.clone()
        } else {
            parent.repeat_x.clone()
        };
        return;
    }
    *target = vec![crate::css::BackgroundRepeat::Repeat];
}

pub(super) fn background_clip_path(
    geometry: &BoxDecorationGeometry<'_>,
    clip: crate::css::BackgroundClip,
    fallback_radius: egui::CornerRadius,
) -> Option<crate::box_decoration::PathClip> {
    let BoxDecorationGeometry {
        border,
        style,
        stroke,
        padding,
        margin,
        widths,
    } = *geometry;
    if let Some(border_shape) = &style.decorations.border_shape {
        let (paths, fill_rule) =
            border_shape.background_paths(border, clip, stroke, padding, margin, widths);
        return Some(crate::box_decoration::path_clip(paths, fill_rule));
    }

    let inset = match clip {
        crate::css::BackgroundClip::Border => egui::epaint::MarginF32::ZERO,
        crate::css::BackgroundClip::Padding => stroke,
        crate::css::BackgroundClip::Content => egui::epaint::MarginF32 {
            top: stroke.top + padding.top,
            right: stroke.right + padding.right,
            bottom: stroke.bottom + padding.bottom,
            left: stroke.left + padding.left,
        },
    };
    let mut radii = crate::box_decoration::resolved_radii(style, border);
    radii[0] -= vec2(inset.left, inset.top);
    radii[1] -= vec2(inset.right, inset.top);
    radii[2] -= vec2(inset.right, inset.bottom);
    radii[3] -= vec2(inset.left, inset.bottom);
    radii = radii.map(|radius| radius.max(Vec2::ZERO));

    let shapes = if radii.iter().all(|radius| *radius == Vec2::ZERO) {
        if fallback_radius == egui::CornerRadius::ZERO {
            return None;
        }
        radii = [
            fallback_radius.nw,
            fallback_radius.ne,
            fallback_radius.se,
            fallback_radius.sw,
        ]
        .map(|radius| vec2(f32::from(radius), f32::from(radius)));
        [crate::box_decoration::CornerShape::Round; 4]
    } else {
        style.decorations.corner_shapes
    };

    Some(crate::box_decoration::path_clip(
        vec![crate::box_decoration::rounded_path(
            border - inset,
            radii,
            shapes,
        )],
        crate::box_decoration::FillRule::Nonzero,
    ))
}

#[derive(Clone, Copy)]
pub(super) struct BoxDecorationGeometry<'a> {
    pub(super) border: Rect,
    pub(super) style: &'a BoxStyle,
    pub(super) stroke: egui::epaint::MarginF32,
    pub(super) padding: egui::epaint::MarginF32,
    pub(super) margin: egui::epaint::MarginF32,
    pub(super) widths: [f32; 4],
}

impl<'a> BoxDecorationGeometry<'a> {
    pub(super) fn new(
        border: Rect,
        style: &'a BoxStyle,
        stroke: egui::epaint::MarginF32,
        padding: egui::epaint::MarginF32,
        margin: egui::epaint::MarginF32,
    ) -> Self {
        let border_widths = style
            .layout
            .border
            .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
        Self {
            border,
            style,
            stroke,
            padding,
            margin,
            widths: [
                border_widths.top,
                border_widths.right,
                border_widths.bottom,
                border_widths.left,
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ImageResources<'a> {
    pub(super) context: &'a Context,
    pub(super) images: &'a crate::background::ImageAssets,
    pub(super) invalid_elements: &'a HashSet<String>,
}

pub(super) fn fragment_background_shapes(
    resources: &ImageResources<'_>,
    background: &BackgroundPaint,
    geometry: &BoxDecorationGeometry<'_>,
    corner_radius: egui::CornerRadius,
    scroll: Vec2,
    fragment_offset: Vec2,
    background_area: Option<Rect>,
) -> Vec<egui::Shape> {
    let context = resources.context;
    let images = resources.images;
    let invalid_elements = resources.invalid_elements;
    let border = geometry.border;
    let stroke = geometry.stroke;
    let padding = geometry.padding;
    let clip_count = background.clips.len().max(1);
    let last_background_layer = background.images.len().saturating_sub(1);
    let color_clip = background
        .clips
        .get(last_background_layer % clip_count)
        .copied()
        .unwrap_or(crate::css::BackgroundClip::Border);
    let shape_clip_for = |rect, clip| {
        let geometry = BoxDecorationGeometry {
            border: rect,
            ..*geometry
        };
        background_clip_path(&geometry, clip, corner_radius)
    };
    let color_surface = match color_clip {
        crate::css::BackgroundClip::Border => border,
        crate::css::BackgroundClip::Padding => border - stroke,
        crate::css::BackgroundClip::Content => border - stroke - padding,
    };
    let color_shape = box_surface_shape(
        color_surface,
        geometry,
        color_clip,
        background.color,
        corner_radius,
    );
    let mut shapes = vec![color_shape];
    let scale = context.pixels_per_point();
    let attachment_count = background.attachments.len().max(1);
    let origin_count = background.origins.len().max(1);
    let position_x_count = background.position_x.len().max(1);
    let position_y_count = background.position_y.len().max(1);
    let repeat_x_count = background.repeat_x.len().max(1);
    let repeat_y_count = background.repeat_y.len().max(1);
    let size_count = background.sizes.len().max(1);
    let arena = crate::math::Arena::default();
    let units = crate::layout_style::Units {
        math: &arena,
        font: geometry.style.format.em,
        root_font: 16.0,
        viewport: context.viewport_rect().size(),
        pixels_per_point: scale,
    };
    let default_position = crate::css::BackgroundPosition {
        x: crate::css::BackgroundPositionAxis::Start("0px".to_owned()),
        y: crate::css::BackgroundPositionAxis::Start("0px".to_owned()),
    };
    for layer_index in (0..background.images.len()).rev() {
        let layer_image = &background.images[layer_index];
        let layer_clip = background
            .clips
            .get(layer_index % clip_count)
            .copied()
            .unwrap_or(crate::css::BackgroundClip::Border);
        let attachment = background
            .attachments
            .get(layer_index % attachment_count)
            .copied()
            .unwrap_or(crate::css::BackgroundAttachment::Scroll);
        let origin = background
            .origins
            .get(layer_index % origin_count)
            .copied()
            .unwrap_or_default();
        let position = crate::css::BackgroundPosition {
            x: background
                .position_x
                .get(layer_index % position_x_count)
                .cloned()
                .unwrap_or_else(|| default_position.x.clone()),
            y: background
                .position_y
                .get(layer_index % position_y_count)
                .cloned()
                .unwrap_or_else(|| default_position.y.clone()),
        };
        let repeat = crate::css::BackgroundRepeatPair {
            x: background
                .repeat_x
                .get(layer_index % repeat_x_count)
                .copied()
                .unwrap_or(crate::css::BackgroundRepeat::Repeat),
            y: background
                .repeat_y
                .get(layer_index % repeat_y_count)
                .copied()
                .unwrap_or(crate::css::BackgroundRepeat::Repeat),
            logical: None,
        };
        let size = background
            .sizes
            .get(layer_index % size_count)
            .unwrap_or(&DEFAULT_BACKGROUND_SIZE);
        let background_surface = match layer_clip {
            crate::css::BackgroundClip::Border => border,
            crate::css::BackgroundClip::Padding => border - stroke,
            crate::css::BackgroundClip::Content => border - stroke - padding,
        };
        let sample_offset = match attachment {
            crate::css::BackgroundAttachment::Local => fragment_offset - scroll,
            crate::css::BackgroundAttachment::Scroll => fragment_offset,
            crate::css::BackgroundAttachment::Fixed => Vec2::ZERO,
        };
        let origin_border = background_area.unwrap_or(border);
        let origin_surface = match origin {
            crate::css::BackgroundOrigin::Border => origin_border,
            crate::css::BackgroundOrigin::Padding => origin_border - stroke,
            crate::css::BackgroundOrigin::Content => origin_border - stroke - padding,
        };
        let positioning_area = match attachment {
            crate::css::BackgroundAttachment::Fixed => context.viewport_rect(),
            crate::css::BackgroundAttachment::Local => origin_surface.translate(-scroll),
            crate::css::BackgroundAttachment::Scroll => origin_surface,
        };
        let clipped_area = background_surface.translate(sample_offset);
        let image_resolution = layer_image.resolution_for_scale(scale);
        let image = layer_image.for_scale(scale);
        if let crate::css::BackgroundImage::SolidColor(color) = image {
            shapes.push(box_surface_shape(
                background_surface,
                geometry,
                layer_clip,
                *color,
                corner_radius,
            ));
            continue;
        }

        let image_texture = match image {
            crate::css::BackgroundImage::Url(url) => {
                match (images.texture(context, url), images.size(url)) {
                    (Some(texture), Some(size)) => Some((
                        texture,
                        vec2(
                            size[0].max(1) as f32 / image_resolution,
                            size[1].max(1) as f32 / image_resolution,
                        ),
                    )),
                    _ => None,
                }
            }
            crate::css::BackgroundImage::CrossFade(_) => cross_fade_texture(
                context,
                images,
                layer_image,
                positioning_area.size(),
                scale / image_resolution,
                invalid_elements,
            )
            .map(|(texture, size)| (texture, size / image_resolution)),
            _ => None,
        };
        let element_source = match image {
            crate::css::BackgroundImage::Element(id) if !invalid_elements.contains(id) => {
                element_image(context, id)
            }
            _ => None,
        };
        let image_size = image_texture
            .as_ref()
            .map(|(_, size)| *size)
            .or_else(|| element_source.as_ref().map(|image| image.size))
            .or_else(|| {
                matches!(
                    image,
                    crate::css::BackgroundImage::LinearGradient(_)
                        | crate::css::BackgroundImage::RadialGradient(_)
                        | crate::css::BackgroundImage::ConicGradient(_)
                )
                .then_some(positioning_area.size())
            });
        let Some(natural_size) =
            image_size.filter(|size| size.is_finite() && size.x > 0.0 && size.y > 0.0)
        else {
            continue;
        };
        let image_size =
            resolve_background_size(size, natural_size, positioning_area.size(), repeat, units);
        if !image_size.is_finite() || image_size.x <= 0.0 || image_size.y <= 0.0 {
            continue;
        }
        let position_offset =
            resolve_background_position(&position, image_size, positioning_area.size(), units);
        let image_origin = positioning_area.min + position_offset;
        let x_tiles = background_tiles(
            clipped_area.left(),
            clipped_area.right(),
            positioning_area.left(),
            positioning_area.right(),
            image_size.x,
            image_origin.x,
            repeat.x,
        );
        let y_tiles = background_tiles(
            clipped_area.top(),
            clipped_area.bottom(),
            positioning_area.top(),
            positioning_area.bottom(),
            image_size.y,
            image_origin.y,
            repeat.y,
        );
        let shape_clip = shape_clip_for(border, layer_clip);
        let gradient = match image {
            crate::css::BackgroundImage::LinearGradient(gradient) => {
                Some(GradientPaint::Linear(gradient.clone()))
            }
            crate::css::BackgroundImage::RadialGradient(gradient) => {
                Some(GradientPaint::Radial(gradient.clone()))
            }
            crate::css::BackgroundImage::ConicGradient(gradient) => {
                Some(GradientPaint::Conic(gradient.clone()))
            }
            _ => None,
        };
        for (left, right, tile_left) in &x_tiles {
            for (top, bottom, tile_top) in &y_tiles {
                let tile = Rect::from_min_size(pos2(*tile_left, *tile_top), image_size);
                let visible_virtual =
                    Rect::from_min_max(pos2(*left, *top), pos2(*right, *bottom)).intersect(tile);
                if !visible_virtual.is_positive() {
                    continue;
                }
                let visible = visible_virtual.translate(-sample_offset);
                let uv = Rect::from_min_max(
                    pos2(
                        (visible_virtual.left() - tile.left()) / tile.width(),
                        (visible_virtual.top() - tile.top()) / tile.height(),
                    ),
                    pos2(
                        (visible_virtual.right() - tile.left()) / tile.width(),
                        (visible_virtual.bottom() - tile.top()) / tile.height(),
                    ),
                );
                if let Some(gradient) = &gradient {
                    shapes.push(gradient_paint_shape(
                        context,
                        visible,
                        tile,
                        sample_offset,
                        gradient.clone(),
                        shape_clip.as_ref(),
                    ));
                } else if let Some((texture, _)) = &image_texture {
                    shapes.push(clip_background_shape(
                        egui::Shape::image(texture.id(), visible, uv, Color32::WHITE),
                        shape_clip.as_ref(),
                    ));
                } else if let Some(element) = &element_source {
                    let source = Rect::from_min_max(
                        pos2(uv.left() * image_size.x, uv.top() * image_size.y),
                        pos2(uv.right() * image_size.x, uv.bottom() * image_size.y),
                    );
                    for shape in element_slice_shapes(element, source, visible) {
                        shapes.push(clip_background_shape(shape, shape_clip.as_ref()));
                    }
                }
            }
        }
    }
    shapes
}

static DEFAULT_BACKGROUND_SIZE: crate::css::BackgroundSize =
    crate::css::BackgroundSize::Explicit([
        crate::css::BackgroundSizeComponent::Auto,
        crate::css::BackgroundSizeComponent::Auto,
    ]);

fn resolve_background_size(
    size: &crate::css::BackgroundSize,
    natural: Vec2,
    area: Vec2,
    repeat: crate::css::BackgroundRepeatPair,
    units: crate::layout_style::Units<'_>,
) -> Vec2 {
    use crate::css::{BackgroundRepeat, BackgroundSize, BackgroundSizeComponent};
    let mut resolved = match size {
        BackgroundSize::Cover | BackgroundSize::Contain => {
            let scale = if matches!(size, BackgroundSize::Cover) {
                (area.x / natural.x).max(area.y / natural.y)
            } else {
                (area.x / natural.x).min(area.y / natural.y)
            };
            natural * scale
        }
        BackgroundSize::Explicit([width, height]) => {
            let width = match width {
                BackgroundSizeComponent::Auto => None,
                BackgroundSizeComponent::Value(value) => Some(
                    crate::box_decoration::length_percentage(value, units, false)
                        .map_or(0.0, |value| value.resolve(area.x)),
                ),
            };
            let height = match height {
                BackgroundSizeComponent::Auto => None,
                BackgroundSizeComponent::Value(value) => Some(
                    crate::box_decoration::length_percentage(value, units, false)
                        .map_or(0.0, |value| value.resolve(area.y)),
                ),
            };
            match (width, height) {
                (Some(width), Some(height)) => vec2(width, height),
                (Some(width), None) => vec2(width, width * natural.y / natural.x),
                (None, Some(height)) => vec2(height * natural.x / natural.y, height),
                (None, None) => natural,
            }
        }
    };
    if repeat.x == BackgroundRepeat::Round && resolved.x > 0.0 && area.x > 0.0 {
        let previous = resolved.x;
        let count = (area.x / previous).round().max(1.0);
        resolved.x = area.x / count;
        if matches!(
            size,
            BackgroundSize::Explicit([_, BackgroundSizeComponent::Auto])
        ) {
            resolved.y *= resolved.x / previous;
        }
    }
    if repeat.y == BackgroundRepeat::Round && resolved.y > 0.0 && area.y > 0.0 {
        let previous = resolved.y;
        let count = (area.y / previous).round().max(1.0);
        resolved.y = area.y / count;
        if matches!(
            size,
            BackgroundSize::Explicit([BackgroundSizeComponent::Auto, _])
        ) {
            resolved.x *= resolved.y / previous;
        }
    }
    resolved
}

fn resolve_background_position(
    position: &crate::css::BackgroundPosition,
    image_size: Vec2,
    area: Vec2,
    units: crate::layout_style::Units<'_>,
) -> Vec2 {
    use crate::css::BackgroundPositionAxis;
    let resolve = |value: &BackgroundPositionAxis, image: f32, extent: f32| {
        let free = extent - image;
        match value {
            BackgroundPositionAxis::FreeSpace(value) => {
                crate::box_decoration::length_percentage(value, units, true)
                    .map_or(0.0, |value| value.resolve(free))
            }
            BackgroundPositionAxis::Start(value) | BackgroundPositionAxis::AxisStart(value) => {
                crate::box_decoration::length_percentage(value, units, true)
                    .map_or(0.0, |value| value.resolve(free))
            }
            BackgroundPositionAxis::Center => free / 2.0,
            BackgroundPositionAxis::End(value) | BackgroundPositionAxis::AxisEnd(value) => {
                free - crate::box_decoration::length_percentage(value, units, true)
                    .map_or(0.0, |value| value.resolve(free))
            }
            BackgroundPositionAxis::Computed(value) => value.resolve(free),
        }
    };
    vec2(
        resolve(&position.x, image_size.x, area.x),
        resolve(&position.y, image_size.y, area.y),
    )
}

fn background_tiles(
    clip_start: f32,
    clip_end: f32,
    area_start: f32,
    area_end: f32,
    natural: f32,
    origin: f32,
    repeat: crate::css::BackgroundRepeat,
) -> Vec<(f32, f32, f32)> {
    use crate::css::BackgroundRepeat;
    if natural <= 0.0 || clip_end <= clip_start {
        return Vec::new();
    }
    let span = area_end - area_start;
    let (tile, first, end, gap, initial) = match repeat {
        BackgroundRepeat::NoRepeat => (natural, 0_isize, 1_isize, 0.0, origin),
        BackgroundRepeat::Repeat => {
            let first = ((clip_start - origin) / natural).floor() as isize;
            let end = ((clip_end - origin) / natural).ceil() as isize;
            (natural, first, end, 0.0, origin)
        }
        BackgroundRepeat::Round => {
            let count = (span / natural).round().max(1.0) as isize;
            (span / count as f32, 0, count, 0.0, area_start)
        }
        BackgroundRepeat::Space => {
            let count = (span / natural).floor() as isize;
            if count >= 2 {
                let gap = (span - count as f32 * natural) / (count - 1) as f32;
                (natural, 0, count, gap, area_start)
            } else {
                (natural, 0, 1, 0.0, origin)
            }
        }
    };
    (first..end)
        .filter_map(|index| {
            let tile_start = initial + index as f32 * (tile + gap);
            let visible_start = tile_start.max(clip_start);
            let visible_end = (tile_start + tile).min(clip_end);
            (visible_start < visible_end).then_some((visible_start, visible_end, tile_start))
        })
        .collect()
}

pub(super) fn resolved_element_references(
    stylesheet: &Stylesheet,
    target: &crate::css::SelectorTarget,
    state: u8,
    viewport: Vec2,
) -> HashSet<String> {
    let mut references = HashSet::new();
    let mut collect = |image: &crate::css::BackgroundImage| {
        let mut ids = Vec::new();
        image.collect_element_ids(&mut ids);
        references.extend(ids);
    };
    for image in &resolved_background(stylesheet, target, state, viewport).images {
        collect(image);
    }
    collect(&resolved_border_image_source(
        stylesheet, target, state, viewport,
    ));
    for pseudo in [
        crate::css::PseudoElementKind::FirstLetter,
        crate::css::PseudoElementKind::FirstLine,
    ] {
        collect(&resolved_pseudo_border_image_source(
            stylesheet, target, pseudo, state, viewport,
        ));
    }
    references
}

pub(super) fn resolved_element_references_from_rules(
    stylesheet: &Stylesheet,
    selector: &crate::css::SelectorTarget,
    pseudo_state: u8,
    _rules: &[(RuleState, Vec<Declaration>)],
    style: &BoxStyle,
    units: Units,
) -> HashSet<String> {
    let mut references = HashSet::new();
    let mut collect = |image: &crate::css::BackgroundImage| {
        let mut ids = Vec::new();
        image.collect_element_ids(&mut ids);
        references.extend(ids);
    };
    for image in &style.background_values.images {
        collect(image);
    }
    collect(&style.decorations.border_image.source);
    for pseudo in [
        crate::css::PseudoElementKind::FirstLetter,
        crate::css::PseudoElementKind::FirstLine,
    ] {
        if let Some((pseudo_style, _, _)) = pseudo_decoration_style(
            stylesheet,
            selector,
            pseudo,
            pseudo_state,
            style,
            units,
            style.current_color,
        ) {
            collect(&pseudo_style.decorations.border_image.source);
        }
    }
    references
}

pub(super) fn invalid_element_references(
    context: &Context,
    stylesheet: &Stylesheet,
    node_index: usize,
    frame: &Frame,
) -> HashSet<String> {
    if !stylesheet.rule_index.has_element_references {
        return HashSet::new();
    }
    let mut element_ids = HashSet::new();
    for rule in &stylesheet.rules {
        if let Some(id) = &rule.target.id {
            element_ids.insert(id.to_owned());
        }
        collect_rule_element_ids(rule, &mut element_ids);
    }
    for node in &frame.nodes {
        if let Some(id) = &node.element_id {
            element_ids.insert(id.clone());
        }
        element_ids.extend(node.element_references.iter().cloned());
    }

    let current_dependencies = element_descendant_dependencies(frame);
    for (id, targets) in &current_dependencies {
        element_ids.insert(id.clone());
        element_ids.extend(targets.iter().cloned());
    }

    let mut dependencies = HashMap::<String, HashSet<String>>::new();
    for id in &element_ids {
        let targets = if let Some(targets) = current_dependencies.get(id) {
            targets.clone()
        } else if let Some(image) = element_image(context, id) {
            image.dependencies.clone()
        } else {
            resolved_element_references(
                stylesheet,
                &crate::css::SelectorTarget {
                    id: Some(id.clone()),
                    ..Default::default()
                },
                RuleState::ENABLED.bits(),
                context.viewport_rect().size(),
            )
        };
        dependencies.insert(id.clone(), targets);
    }

    fn reaches(
        current: &str,
        target: &str,
        dependencies: &HashMap<String, HashSet<String>>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if current == target {
            return true;
        }
        if !visited.insert(current.to_owned()) {
            return false;
        }
        dependencies.get(current).is_some_and(|next| {
            next.iter()
                .any(|next| reaches(next, target, dependencies, visited))
        })
    }

    let mut source_ancestors = HashSet::new();
    let mut current = Some(node_index);
    while let Some(index) = current {
        let node = &frame.nodes[index];
        if node.element_first_occurrence
            && let Some(id) = &node.element_id
        {
            source_ancestors.insert(id.clone());
        }
        current = node.parent;
    }

    frame.nodes[node_index]
        .element_references
        .iter()
        .filter(|target| {
            source_ancestors
                .iter()
                .any(|ancestor| reaches(target, ancestor, &dependencies, &mut HashSet::new()))
        })
        .cloned()
        .collect()
}

pub(super) fn element_descendant_dependencies(frame: &Frame) -> HashMap<String, HashSet<String>> {
    fn visit(
        frame: &Frame,
        node_index: usize,
        ancestors: &[String],
        dependencies: &mut HashMap<String, HashSet<String>>,
    ) {
        let node = &frame.nodes[node_index];
        if node.absent {
            return;
        }
        let mut source_ids = ancestors.to_vec();
        if node.element_first_occurrence
            && let Some(id) = &node.element_id
        {
            for ancestor in ancestors {
                dependencies
                    .entry(ancestor.clone())
                    .or_default()
                    .insert(id.clone());
            }
            dependencies.entry(id.clone()).or_default();
            source_ids.push(id.clone());
        }
        for source_id in &source_ids {
            dependencies
                .entry(source_id.clone())
                .or_default()
                .extend(node.element_references.iter().cloned());
        }
        for child in &node.children {
            visit(frame, *child, &source_ids, dependencies);
        }
    }

    let mut dependencies = HashMap::new();
    for (index, node) in frame.nodes.iter().enumerate() {
        if node.parent.is_none() {
            visit(frame, index, &[], &mut dependencies);
        }
    }
    dependencies
}

pub(super) fn collect_rule_element_ids(rule: &crate::css::Rule, ids: &mut HashSet<String>) {
    fn collect(image: &crate::css::BackgroundImage, ids: &mut HashSet<String>) {
        let mut image_ids = Vec::new();
        image.collect_element_ids(&mut image_ids);
        ids.extend(image_ids);
    }

    for declaration in &rule.declarations {
        match declaration {
            Declaration::BackgroundShorthand(background)
            | Declaration::BackgroundTbdShorthand(background) => {
                for image in &background.images {
                    collect(image, ids);
                }
            }
            Declaration::BackgroundImage(images) => {
                for image in images {
                    collect(image, ids);
                }
            }
            Declaration::Layout(layout) if layout.name.starts_with("border-image") => {
                if let Some(image) =
                    crate::box_decoration::border_image_source(&layout.name, &layout.value)
                {
                    collect(&image, ids);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn resolved_border_image_source(
    stylesheet: &Stylesheet,
    selector: &crate::css::SelectorTarget,
    state: u8,
    viewport: Vec2,
) -> crate::css::BackgroundImage {
    let mut source = crate::css::BackgroundImage::None;
    let ordered = crate::css::cascade::ordered(
        matching_rules(stylesheet, selector, viewport)
            .filter(move |rule| rule.state.matches(state)),
    );
    for (_, declaration) in ordered {
        match declaration {
            Declaration::All(_) => source = crate::css::BackgroundImage::None,
            Declaration::Layout(layout) => {
                if let Some(value) =
                    crate::box_decoration::border_image_source(&layout.name, &layout.value)
                {
                    source = value;
                }
            }
            _ => {}
        }
    }
    source
}

pub(super) fn resolved_pseudo_border_image_source(
    stylesheet: &Stylesheet,
    selector: &crate::css::SelectorTarget,
    pseudo: crate::css::PseudoElementKind,
    state: u8,
    viewport: Vec2,
) -> crate::css::BackgroundImage {
    let mut source = crate::css::BackgroundImage::None;
    let rules = matching_pseudo_rules(stylesheet, selector, pseudo, state, viewport);
    let ordered = crate::css::cascade::ordered(rules);
    for (_, declaration) in ordered {
        match declaration {
            Declaration::All(_) => source = crate::css::BackgroundImage::None,
            Declaration::Layout(layout) => {
                if let Some(value) =
                    crate::box_decoration::border_image_source(&layout.name, &layout.value)
                {
                    source = value;
                }
            }
            _ => {}
        }
    }
    source
}

pub(super) fn cross_fade_texture(
    context: &Context,
    images: &crate::background::ImageAssets,
    image: &crate::css::BackgroundImage,
    fallback_size: Vec2,
    scale: f32,
    invalid_elements: &HashSet<String>,
) -> Option<(egui::TextureHandle, Vec2)> {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let logical_size = intrinsic_image_size(image, images, context, scale).unwrap_or(fallback_size);
    let logical_size = logical_size.max(Vec2::splat(1.0));
    let width = (logical_size.x * scale).ceil().clamp(1.0, 4096.0) as usize;
    let height = (logical_size.y * scale).ceil().clamp(1.0, 4096.0) as usize;
    let source_revision = image_source_revision(image, context, scale, invalid_elements);
    let key = egui::Id::unique((
        "ecss.cross-fade",
        format!("{image:?}:{scale}:{logical_size:?}:{source_revision}"),
    ));
    if let Some(texture) = context.data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return Some((texture, logical_size));
    }
    let area = Rect::from_min_size(Pos2::ZERO, logical_size);
    let resources = ImageResources {
        context,
        images,
        invalid_elements,
    };
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                sample_image(
                    &resources,
                    image,
                    scale,
                    (x as f32 + 0.5) / width as f32,
                    (y as f32 + 0.5) / height as f32,
                    area,
                )
            })
        })
        .collect();
    let pixels = egui::ColorImage::new([width, height], pixels);
    let texture = context.load_texture(
        key.value().to_string(),
        pixels.clone(),
        egui::TextureOptions::LINEAR,
    );
    crate::background::remember_texture_image(context, texture.id(), pixels);
    context.data_mut(|data| data.insert_temp(key, texture.clone()));
    Some((texture, logical_size))
}

pub(super) fn image_source_revision(
    image: &crate::css::BackgroundImage,
    context: &Context,
    scale: f32,
    invalid_elements: &HashSet<String>,
) -> u64 {
    use std::hash::{Hash, Hasher};

    fn visit(
        image: &crate::css::BackgroundImage,
        context: &Context,
        scale: f32,
        hasher: &mut impl Hasher,
    ) {
        match image.for_scale(scale) {
            crate::css::BackgroundImage::Element(id) => {
                id.hash(hasher);
                if let Some(image) = element_image(context, id) {
                    image.fingerprint.hash(hasher);
                }
            }
            crate::css::BackgroundImage::CrossFade(cross_fade) => {
                for item in &cross_fade.items {
                    item.weight.to_bits().hash(hasher);
                    visit(&item.image, context, scale, hasher);
                }
            }
            _ => {}
        }
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    visit(image, context, scale, &mut hasher);
    let mut invalid_elements = invalid_elements.iter().collect::<Vec<_>>();
    invalid_elements.sort();
    invalid_elements.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn intrinsic_image_size(
    image: &crate::css::BackgroundImage,
    images: &crate::background::ImageAssets,
    context: &Context,
    scale: f32,
) -> Option<Vec2> {
    let selected = image.for_scale(scale);
    let density = image.resolution_for_scale(scale);
    let size = match selected {
        crate::css::BackgroundImage::Url(url) => {
            let size = images.size(url)?;
            vec2(size[0] as f32, size[1] as f32)
        }
        crate::css::BackgroundImage::Element(id) => element_image(context, id)?.size,
        crate::css::BackgroundImage::CrossFade(cross_fade) => {
            let mut weighted_size = Vec2::ZERO;
            let mut total_weight = 0.0;
            for item in &cross_fade.items {
                if let Some(size) = intrinsic_image_size(&item.image, images, context, scale) {
                    weighted_size += size * item.weight;
                    total_weight += item.weight;
                }
            }
            (total_weight > 0.0).then(|| weighted_size / total_weight)?
        }
        crate::css::BackgroundImage::None
        | crate::css::BackgroundImage::SolidColor(_)
        | crate::css::BackgroundImage::LinearGradient(_)
        | crate::css::BackgroundImage::RadialGradient(_)
        | crate::css::BackgroundImage::ConicGradient(_)
        | crate::css::BackgroundImage::ImageSet(_) => return None,
    };
    Some(size / density.max(f32::MIN_POSITIVE))
}

pub(super) fn sample_image(
    resources: &ImageResources<'_>,
    image: &crate::css::BackgroundImage,
    scale: f32,
    u: f32,
    v: f32,
    area: Rect,
) -> Color32 {
    let ImageResources {
        context,
        images,
        invalid_elements,
    } = *resources;
    match image.for_scale(scale) {
        crate::css::BackgroundImage::None | crate::css::BackgroundImage::ImageSet(_) => {
            Color32::TRANSPARENT
        }
        crate::css::BackgroundImage::Element(id) if invalid_elements.contains(id) => {
            Color32::TRANSPARENT
        }
        crate::css::BackgroundImage::Element(id) => element_image(context, id)
            .map_or(Color32::TRANSPARENT, |image| {
                sample_element_image(&image, u, v)
            }),
        crate::css::BackgroundImage::SolidColor(color) => *color,
        crate::css::BackgroundImage::Url(url) => {
            let Some(image) = images.image(url) else {
                return Color32::TRANSPARENT;
            };
            if image.size[0] == 0 || image.size[1] == 0 {
                return Color32::TRANSPARENT;
            }
            let x = ((u * image.size[0] as f32).floor() as usize).min(image.size[0] - 1);
            let y = ((v * image.size[1] as f32).floor() as usize).min(image.size[1] - 1);
            image.pixels[y * image.size[0] + x]
        }
        crate::css::BackgroundImage::LinearGradient(gradient) => {
            let direction = vec2(
                gradient.angle.to_radians().sin(),
                -gradient.angle.to_radians().cos(),
            );
            let corners = [
                area.left_top(),
                area.right_top(),
                area.right_bottom(),
                area.left_bottom(),
            ];
            let min = corners
                .iter()
                .map(|point| point.to_vec2().dot(direction))
                .fold(f32::INFINITY, f32::min);
            let max = corners
                .iter()
                .map(|point| point.to_vec2().dot(direction))
                .fold(f32::NEG_INFINITY, f32::max);
            let point = pos2(
                area.left() + u * area.width(),
                area.top() + v * area.height(),
            );
            let position = ((point.to_vec2().dot(direction) - min) / (max - min).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            gradient_color(
                &gradient.stops,
                position,
                gradient.repeating,
                gradient.color_space,
            )
        }
        crate::css::BackgroundImage::RadialGradient(gradient) => {
            let point = pos2(
                area.left() + u * area.width(),
                area.top() + v * area.height(),
            );
            let (center, radii, corner_extent) = radial_gradient_parameters(area, gradient);
            gradient_color(
                &gradient.stops,
                radial_gradient_position(point, center, radii, corner_extent),
                gradient.repeating,
                gradient.color_space,
            )
        }
        crate::css::BackgroundImage::ConicGradient(gradient) => {
            let point = pos2(
                area.left() + u * area.width(),
                area.top() + v * area.height(),
            );
            let center = pos2(
                area.left() + gradient.center[0].resolve(area.width()),
                area.top() + gradient.center[1].resolve(area.height()),
            );
            gradient_color(
                &gradient.stops,
                conic_gradient_position(point, center, gradient.from_angle),
                gradient.repeating,
                gradient.color_space,
            )
        }
        crate::css::BackgroundImage::CrossFade(cross_fade) => {
            let total = cross_fade.items.iter().map(|item| item.weight).sum::<f32>();
            let normalization = total.max(1.0);
            let mut premultiplied = [0.0; 4];
            for item in &cross_fade.items {
                let color = sample_image(resources, &item.image, scale, u, v, area);
                let color = color.to_array();
                let weight = item.weight / normalization;
                for channel in 0..3 {
                    premultiplied[channel] += f32::from(color[channel]) / 255.0 * weight;
                }
                premultiplied[3] += f32::from(color[3]) / 255.0 * weight;
            }
            let alpha = premultiplied[3].clamp(0.0, 1.0);
            let channels: [u8; 3] = std::array::from_fn(|channel| {
                let value = if alpha > f32::EPSILON {
                    premultiplied[channel] / alpha
                } else {
                    0.0
                };
                (value.clamp(0.0, 1.0) * 255.0).round() as u8
            });
            Color32::from_rgba_premultiplied(
                channels[0],
                channels[1],
                channels[2],
                (alpha * 255.0).round() as u8,
            )
        }
    }
}

pub(super) fn sample_element_image(image: &ElementImage, u: f32, v: f32) -> Color32 {
    let point = pos2(u * image.size.x, v * image.size.y);
    let mut output = [0.0; 4];
    for mesh in &image.meshes {
        let (triangles, _) = mesh.indices.as_chunks::<3>();
        for triangle in triangles {
            let vertices = [
                &mesh.vertices[triangle[0] as usize],
                &mesh.vertices[triangle[1] as usize],
                &mesh.vertices[triangle[2] as usize],
            ];
            let Some(weights) =
                barycentric_weights(point, vertices[0].pos, vertices[1].pos, vertices[2].pos)
            else {
                continue;
            };
            let uv = pos2(
                vertices
                    .iter()
                    .zip(weights)
                    .map(|(vertex, weight)| vertex.uv.x * weight)
                    .sum(),
                vertices
                    .iter()
                    .zip(weights)
                    .map(|(vertex, weight)| vertex.uv.y * weight)
                    .sum(),
            );
            let texture = image
                .textures
                .get(&mesh.texture_id)
                .map_or(Color32::WHITE, |texture| sample_color_image(texture, uv));
            let texture = texture.to_array();
            let colors = vertices.map(|vertex| {
                let color = vertex.color.to_array();
                std::array::from_fn::<_, 4, _>(|channel| {
                    (u16::from(color[channel]) * u16::from(texture[channel]) / 255) as u8
                })
            });
            let source = std::array::from_fn::<_, 4, _>(|channel| {
                colors
                    .iter()
                    .zip(weights)
                    .map(|(color, weight)| f32::from(color[channel]) * weight / 255.0)
                    .sum::<f32>()
                    .clamp(0.0, 1.0)
            });
            let alpha = source[3];
            for channel in 0..3 {
                output[channel] = source[channel] + output[channel] * (1.0 - alpha);
            }
            output[3] = alpha + output[3] * (1.0 - alpha);
        }
    }
    let alpha = output[3];
    let channels = std::array::from_fn::<_, 3, _>(|channel| {
        let value = if alpha > f32::EPSILON {
            output[channel] / alpha
        } else {
            0.0
        };
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    });
    Color32::from_rgba_premultiplied(
        channels[0],
        channels[1],
        channels[2],
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn sample_color_image(image: &egui::ColorImage, uv: Pos2) -> Color32 {
    if image.size[0] == 0 || image.size[1] == 0 {
        return Color32::TRANSPARENT;
    }
    let x = (uv.x.clamp(0.0, 1.0) * image.size[0] as f32 - 0.5)
        .clamp(0.0, image.size[0].saturating_sub(1) as f32);
    let y = (uv.y.clamp(0.0, 1.0) * image.size[1] as f32 - 0.5)
        .clamp(0.0, image.size[1].saturating_sub(1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(image.size[0] - 1);
    let y1 = (y0 + 1).min(image.size[1] - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let colors = [
        image.pixels[y0 * image.size[0] + x0].to_array(),
        image.pixels[y0 * image.size[0] + x1].to_array(),
        image.pixels[y1 * image.size[0] + x0].to_array(),
        image.pixels[y1 * image.size[0] + x1].to_array(),
    ];
    let values = std::array::from_fn::<_, 4, _>(|channel| {
        let top = egui::emath::lerp(
            f32::from(colors[0][channel])..=f32::from(colors[1][channel]),
            tx,
        );
        let bottom = egui::emath::lerp(
            f32::from(colors[2][channel])..=f32::from(colors[3][channel]),
            tx,
        );
        egui::emath::lerp(top..=bottom, ty).round() as u8
    });
    Color32::from_rgba_premultiplied(values[0], values[1], values[2], values[3])
}

pub(super) fn barycentric_weights(point: Pos2, a: Pos2, b: Pos2, c: Pos2) -> Option<[f32; 3]> {
    let denominator = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if denominator.abs() <= f32::EPSILON {
        return None;
    }
    let first = ((b.y - c.y) * (point.x - c.x) + (c.x - b.x) * (point.y - c.y)) / denominator;
    let second = ((c.y - a.y) * (point.x - c.x) + (a.x - c.x) * (point.y - c.y)) / denominator;
    let third = 1.0 - first - second;
    (first >= -1.0e-5 && second >= -1.0e-5 && third >= -1.0e-5).then_some([first, second, third])
}
