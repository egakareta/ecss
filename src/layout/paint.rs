use super::*;

pub(super) fn clip_background_shape(
    shape: egui::Shape,
    clip: Option<&crate::box_decoration::PathClip>,
) -> egui::Shape {
    let Some(clip) = clip else {
        return shape;
    };
    match shape {
        egui::Shape::Mesh(mesh) => egui::Shape::Mesh(
            crate::box_decoration::clip_mesh_to_paths(
                std::sync::Arc::try_unwrap(mesh).unwrap_or_else(|mesh| mesh.as_ref().clone()),
                clip,
            )
            .into(),
        ),
        egui::Shape::Vec(shapes) => egui::Shape::Vec(
            shapes
                .into_iter()
                .map(|shape| clip_background_shape(shape, Some(clip)))
                .collect(),
        ),
        shape => shape,
    }
}

pub(super) fn clip_shape_to_paths(
    context: &Context,
    shape: egui::Shape,
    clip: &crate::box_decoration::PathClip,
) -> Vec<egui::Shape> {
    context
        .tessellate(
            vec![egui::epaint::ClippedShape::new(clip.bounds(), shape)],
            context.pixels_per_point(),
        )
        .into_iter()
        .filter_map(|primitive| match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => {
                let mesh = crate::box_decoration::clip_mesh_to_paths(mesh, clip);
                (!mesh.indices.is_empty()).then_some(egui::Shape::Mesh(mesh.into()))
            }
            egui::epaint::Primitive::Callback(_) => None,
        })
        .collect()
}

pub(super) fn inline_slice_offsets(surfaces: &[Rect], vertical: bool, forward: bool) -> Vec<Vec2> {
    let axis = usize::from(vertical);
    let cross_axis = 1 - axis;
    let mut advance = 0.0;
    let cross_center = surfaces
        .first()
        .map_or(0.0, |surface| surface.center()[cross_axis]);
    surfaces
        .iter()
        .map(|surface| {
            let extent = surface.size()[axis];
            let target_start = if forward { advance } else { -advance - extent };
            let mut offset = Vec2::ZERO;
            offset[axis] = target_start - surface.min[axis];
            offset[cross_axis] = cross_center - surface.center()[cross_axis];
            advance += extent;
            offset
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(super) struct FragmentPosition {
    pub(super) vertical: bool,
    pub(super) forward: bool,
    pub(super) index: usize,
    pub(super) count: usize,
}

pub(super) fn fragment_shadow_clip(
    surface: Rect,
    mut clip: Rect,
    fragment: FragmentPosition,
    position: crate::box_decoration::ShadowPosition,
    inset_extent: f32,
    stroke: egui::epaint::MarginF32,
) -> Option<Rect> {
    let FragmentPosition {
        vertical,
        forward,
        index,
        count,
    } = fragment;
    let axis = usize::from(vertical);
    let (start_stroke, end_stroke) = if vertical {
        (stroke.top, stroke.bottom)
    } else {
        (stroke.left, stroke.right)
    };
    let start_edge = surface.min[axis]
        + if position == crate::box_decoration::ShadowPosition::Inset {
            start_stroke + inset_extent
        } else {
            0.0
        };
    let end_edge = surface.max[axis]
        - if position == crate::box_decoration::ShadowPosition::Inset {
            end_stroke + inset_extent
        } else {
            0.0
        };
    if index > 0 {
        if forward {
            clip.min[axis] = clip.min[axis].max(start_edge);
        } else {
            clip.max[axis] = clip.max[axis].min(end_edge);
        }
    }
    if index + 1 < count {
        if forward {
            clip.max[axis] = clip.max[axis].min(end_edge);
        } else {
            clip.min[axis] = clip.min[axis].max(start_edge);
        }
    }
    clip.is_positive().then_some(clip)
}

pub(super) fn clip_fragment_shadows(
    context: &Context,
    shapes: Vec<egui::Shape>,
    clip: Option<Rect>,
) -> Vec<egui::Shape> {
    let Some(clip) = clip else {
        return Vec::new();
    };
    let path = vec![
        clip.left_top(),
        clip.right_top(),
        clip.right_bottom(),
        clip.left_bottom(),
    ];
    let clip =
        crate::box_decoration::path_clip(vec![path], crate::box_decoration::FillRule::Nonzero);
    shapes
        .into_iter()
        .flat_map(|shape| clip_shape_to_paths(context, shape, &clip))
        .collect()
}

pub(super) fn border_content_clip(node: &Node) -> Option<crate::box_decoration::PathClip> {
    let paint = node.paint.as_ref()?;
    let widths = node
        .style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
    let widths = [widths.top, widths.right, widths.bottom, widths.left];
    let mut overflow_x = node.style.layout.overflow.x;
    let mut overflow_y = node.style.layout.overflow.y;
    if overflow_x == Overflow::Visible && !matches!(overflow_y, Overflow::Visible | Overflow::Clip)
    {
        overflow_x = Overflow::Scroll;
    }
    if overflow_y == Overflow::Visible && !matches!(overflow_x, Overflow::Visible | Overflow::Clip)
    {
        overflow_y = Overflow::Scroll;
    }
    if overflow_x == Overflow::Clip && !matches!(overflow_y, Overflow::Visible | Overflow::Clip) {
        overflow_x = Overflow::Hidden;
    }
    if overflow_y == Overflow::Clip && !matches!(overflow_x, Overflow::Visible | Overflow::Clip) {
        overflow_y = Overflow::Hidden;
    }
    if overflow_x == Overflow::Visible || overflow_y == Overflow::Visible {
        return None;
    }
    if let Some(border_shape) = &node.style.decorations.border_shape {
        let (clip, outset) = if node.style.layout.overflow.x == Overflow::Clip
            && node.style.layout.overflow.y == Overflow::Clip
        {
            let clip = match node.style.overflow_clip_box {
                crate::layout_style::OverflowClipBox::Content => {
                    crate::css::BackgroundClip::Content
                }
                crate::layout_style::OverflowClipBox::Padding => {
                    crate::css::BackgroundClip::Padding
                }
                crate::layout_style::OverflowClipBox::Border => crate::css::BackgroundClip::Border,
            };
            (clip, node.style.overflow_clip_margin[0])
        } else {
            (crate::css::BackgroundClip::Padding, 0.0)
        };
        let (paths, fill_rule) = border_shape.overflow_paths(
            clip,
            crate::box_decoration::OverflowGeometry {
                border: paint.geometry.border,
                stroke: paint.geometry.stroke,
                padding: paint.geometry.padding,
                margin: paint.geometry.margin,
                widths,
                outset,
            },
        );
        return Some(crate::box_decoration::path_clip(paths, fill_rule));
    }
    let border = paint.geometry.border;
    let (clip_rect, edge_rect, inset, outset) = if node.style.layout.overflow.x == Overflow::Clip
        && node.style.layout.overflow.y == Overflow::Clip
    {
        let (rect, inset) = match node.style.overflow_clip_box {
            crate::layout_style::OverflowClipBox::Content => (
                paint.geometry.content(),
                egui::epaint::MarginF32 {
                    top: paint.geometry.stroke.top + paint.geometry.padding.top,
                    right: paint.geometry.stroke.right + paint.geometry.padding.right,
                    bottom: paint.geometry.stroke.bottom + paint.geometry.padding.bottom,
                    left: paint.geometry.stroke.left + paint.geometry.padding.left,
                },
            ),
            crate::layout_style::OverflowClipBox::Padding => {
                (border - paint.geometry.stroke, paint.geometry.stroke)
            }
            crate::layout_style::OverflowClipBox::Border => (border, egui::epaint::MarginF32::ZERO),
        };
        let margins = node.style.overflow_clip_margin;
        (
            rect + egui::epaint::MarginF32 {
                top: margins[0],
                right: margins[1],
                bottom: margins[2],
                left: margins[3],
            },
            rect,
            inset,
            margins,
        )
    } else {
        let rect = border - paint.geometry.stroke;
        (rect, rect, paint.geometry.stroke, [0.0; 4])
    };
    let collapsed = paint.geometry.collapsed.is_some();
    let mut radii = if collapsed {
        [Vec2::ZERO; 4]
    } else {
        crate::box_decoration::resolved_radii(&node.style, border)
    };
    radii[0] -= vec2(inset.left, inset.top);
    radii[1] -= vec2(inset.right, inset.top);
    radii[2] -= vec2(inset.right, inset.bottom);
    radii[3] -= vec2(inset.left, inset.bottom);
    radii = radii.map(|radius| radius.max(Vec2::ZERO));
    if outset.iter().any(|margin| *margin > 0.0) {
        radii = crate::box_decoration::outset_radii(edge_rect, radii, outset);
    }
    if radii.iter().all(|radius| radius.max_elem() <= 0.0)
        && outset.iter().all(|margin| *margin <= 0.0)
    {
        return None;
    }
    let shapes = if collapsed {
        [crate::box_decoration::CornerShape::Round; 4]
    } else {
        node.style.decorations.corner_shapes
    };
    let path = crate::box_decoration::rounded_path(clip_rect, radii, shapes);
    Some(crate::box_decoration::path_clip(
        vec![path],
        crate::box_decoration::FillRule::Nonzero,
    ))
}

pub(super) fn append_painted_entry(
    context: &Context,
    entry: egui::epaint::ClippedShape,
    clips: &[&crate::box_decoration::PathClip],
    output: &mut Vec<egui::epaint::ClippedShape>,
) {
    if clips.is_empty() {
        output.push(entry);
    } else {
        output.extend(clip_painted_entry(context, entry, clips));
    }
}

pub(super) fn clip_painted_entry(
    context: &Context,
    entry: egui::epaint::ClippedShape,
    clips: &[&crate::box_decoration::PathClip],
) -> Vec<egui::epaint::ClippedShape> {
    if clips.is_empty() {
        return vec![entry];
    }
    context
        .tessellate(vec![entry], context.pixels_per_point())
        .into_iter()
        .filter_map(|primitive| {
            let mut clip_rect = primitive.clip_rect;
            match primitive.primitive {
                egui::epaint::Primitive::Mesh(mut mesh) => {
                    for clip in clips {
                        clip_rect = clip_rect.intersect(clip.bounds());
                        if !clip_rect.is_positive() {
                            return None;
                        }
                        mesh = crate::box_decoration::clip_mesh_to_paths(mesh, clip);
                        if mesh.indices.is_empty() {
                            return None;
                        }
                    }
                    Some(egui::epaint::ClippedShape::new(
                        clip_rect,
                        egui::Shape::Mesh(mesh.into()),
                    ))
                }
                egui::epaint::Primitive::Callback(callback) => {
                    for clip in clips {
                        clip_rect = clip_rect.intersect(clip.bounds());
                    }
                    clip_rect
                        .is_positive()
                        .then_some(egui::epaint::ClippedShape::new(
                            clip_rect,
                            egui::Shape::Callback(callback),
                        ))
                }
            }
        })
        .collect()
}

pub(super) fn box_surface_shape(
    rect: Rect,
    geometry: &BoxDecorationGeometry<'_>,
    clip: crate::css::BackgroundClip,
    fill: Color32,
    fallback_radius: egui::CornerRadius,
) -> egui::Shape {
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
        return if paths.len() == 1 && border_shape.background_is_convex(clip) {
            egui::Shape::convex_polygon(paths.into_iter().next().unwrap(), fill, egui::Stroke::NONE)
        } else {
            crate::box_decoration::filled_paths(paths, fill_rule, fill)
        };
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
    for radius in &mut radii {
        *radius = radius.max(Vec2::ZERO);
    }
    if radii.iter().all(|radius| *radius == Vec2::ZERO) {
        return egui::Shape::Rect(egui::epaint::RectShape::new(
            rect,
            fallback_radius,
            fill,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        ));
    }
    let points = crate::box_decoration::rounded_path(rect, radii, style.decorations.corner_shapes);
    if style
        .decorations
        .corner_shapes
        .iter()
        .any(|shape| shape.parameter_is_concave())
    {
        crate::box_decoration::filled_path(points, fill)
    } else {
        egui::Shape::convex_polygon(points, fill, egui::Stroke::NONE)
    }
}

pub(super) fn border_image_shapes(
    border: Rect,
    style: &BoxStyle,
    images: &crate::background::ImageAssets,
    context: &Context,
    invalid_elements: &HashSet<String>,
) -> Option<Vec<egui::Shape>> {
    if style.format.collapse
        && matches!(
            style.format.display,
            crate::formatting::DisplayKind::Row
                | crate::formatting::DisplayKind::Cell
                | crate::formatting::DisplayKind::RowGroup
                | crate::formatting::DisplayKind::Header
                | crate::formatting::DisplayKind::Footer
                | crate::formatting::DisplayKind::Column
                | crate::formatting::DisplayKind::ColumnGroup
        )
    {
        return None;
    }
    let image = &style.decorations.border_image;
    let source = image.source.for_scale(context.pixels_per_point());
    if let crate::css::BackgroundImage::Element(id) = source
        && invalid_elements.contains(id)
    {
        return None;
    }
    let auto_slice_density = match source {
        crate::css::BackgroundImage::Url(_) => Some(
            image
                .source
                .resolution_for_scale(context.pixels_per_point()),
        ),
        crate::css::BackgroundImage::Element(_) => Some(1.0),
        crate::css::BackgroundImage::CrossFade(_)
            if intrinsic_image_size(source, images, context, context.pixels_per_point())
                .is_some() =>
        {
            Some(context.pixels_per_point())
        }
        _ => None,
    };
    let source_element = match source {
        crate::css::BackgroundImage::Element(id) => element_image(context, id),
        _ => None,
    };
    let widths = style.border_widths;
    let outset = border_image_outset(style);
    let area = border + outset;
    let (
        texture,
        image_width,
        image_height,
        gradient,
        radial_gradient,
        conic_gradient,
        solid_color,
    ) = match source {
        crate::css::BackgroundImage::SolidColor(color) => (
            None,
            area.width(),
            area.height(),
            None,
            None,
            None,
            Some(*color),
        ),
        crate::css::BackgroundImage::Url(url) => {
            let (texture, size) = images.border_image_texture(context, url, area.size())?;
            (
                Some(texture),
                size[0] as f32,
                size[1] as f32,
                None,
                None,
                None,
                None,
            )
        }
        crate::css::BackgroundImage::Element(_) => {
            let element = source_element.as_ref()?;
            (None, element.size.x, element.size.y, None, None, None, None)
        }
        crate::css::BackgroundImage::LinearGradient(gradient) => (
            None,
            area.width(),
            area.height(),
            Some(gradient),
            None,
            None,
            None,
        ),
        crate::css::BackgroundImage::RadialGradient(gradient) => (
            None,
            area.width(),
            area.height(),
            None,
            Some(gradient),
            None,
            None,
        ),
        crate::css::BackgroundImage::ConicGradient(gradient) => (
            None,
            area.width(),
            area.height(),
            None,
            None,
            Some(gradient),
            None,
        ),
        crate::css::BackgroundImage::CrossFade(_) => {
            let (texture, _) = cross_fade_texture(
                context,
                images,
                source,
                area.size(),
                context.pixels_per_point(),
                invalid_elements,
            )?;
            let size = texture.size();
            (
                Some(texture),
                size[0] as f32,
                size[1] as f32,
                None,
                None,
                None,
                None,
            )
        }
        crate::css::BackgroundImage::None => return None,
        crate::css::BackgroundImage::ImageSet(_) => return None,
    };
    if image_width <= 0.0 || image_height <= 0.0 {
        return None;
    }
    let slice = std::array::from_fn::<_, 4, _>(|side| {
        let extent = if side % 2 == 0 {
            image_height
        } else {
            image_width
        };
        match image.slice[side] {
            crate::box_decoration::ImageSlice::Number(value) => value,
            crate::box_decoration::ImageSlice::Percentage(value) => value * extent,
            crate::box_decoration::ImageSlice::PercentageCalc(handle) => {
                crate::math::resolve(handle, extent)
            }
        }
        .min(extent)
    });
    let width_for = |side: usize| match image.width[side] {
        crate::box_decoration::ImageWidth::NumberOne => widths[side],
        crate::box_decoration::ImageWidth::Number(value) => value * widths[side],
        crate::box_decoration::ImageWidth::Auto => {
            let natural = [slice[0], slice[1], slice[2], slice[3]][side];
            auto_slice_density.map_or(widths[side], |density| {
                if natural > 0.0 {
                    natural / density.max(f32::MIN_POSITIVE)
                } else {
                    widths[side]
                }
            })
        }
        crate::box_decoration::ImageWidth::Length(value) => {
            value.resolve(if side.is_multiple_of(2) {
                area.height()
            } else {
                area.width()
            })
        }
    };
    let mut draw_widths = [width_for(0), width_for(1), width_for(2), width_for(3)];
    let factor = (area.width() / (draw_widths[1] + draw_widths[3]).max(0.001))
        .min(area.height() / (draw_widths[0] + draw_widths[2]).max(0.001))
        .min(1.0);
    if factor < 1.0 {
        draw_widths.iter_mut().for_each(|value| *value *= factor);
    }
    let x_src = [
        0.0,
        slice[3].min(image_width),
        (image_width - slice[1]).max(0.0),
        image_width,
    ];
    let y_src = [
        0.0,
        slice[0].min(image_height),
        (image_height - slice[2]).max(0.0),
        image_height,
    ];
    let x_dst = [
        area.left(),
        area.left() + draw_widths[3],
        area.right() - draw_widths[1],
        area.right(),
    ];
    let y_dst = [
        area.top(),
        area.top() + draw_widths[0],
        area.bottom() - draw_widths[2],
        area.bottom(),
    ];
    let axis_scale = |destination: f32, source: f32| {
        let scale = destination / source;
        (source > 0.0 && scale.is_finite() && scale > 0.0).then_some(scale)
    };
    let middle_x_scale = axis_scale(y_dst[1] - y_dst[0], y_src[1] - y_src[0])
        .or_else(|| axis_scale(y_dst[3] - y_dst[2], y_src[3] - y_src[2]))
        .unwrap_or(1.0);
    let middle_y_scale = axis_scale(x_dst[1] - x_dst[0], x_src[1] - x_src[0])
        .or_else(|| axis_scale(x_dst[3] - x_dst[2], x_src[3] - x_src[2]))
        .unwrap_or(1.0);
    let mut shapes = Vec::new();
    for row in 0..3 {
        for column in 0..3 {
            if row == 1 && column == 1 && !image.fill {
                continue;
            }
            let source = Rect::from_min_max(
                pos2(x_src[column], y_src[row]),
                pos2(x_src[column + 1], y_src[row + 1]),
            );
            let target = Rect::from_min_max(
                pos2(x_dst[column], y_dst[row]),
                pos2(x_dst[column + 1], y_dst[row + 1]),
            );
            if !source.is_positive() || !target.is_positive() {
                continue;
            }
            let tile_x = source.width()
                * if row == 1 {
                    middle_x_scale
                } else {
                    target.height() / source.height()
                };
            let tile_y = source.height()
                * if column == 1 {
                    middle_y_scale
                } else {
                    target.width() / source.width()
                };
            let repeat_x = image.repeat[0];
            let repeat_y = image.repeat[1];
            let tile_x_enabled = column == 1;
            let tile_y_enabled = row == 1;
            let tile_x_active =
                tile_x_enabled && repeat_x != crate::box_decoration::ImageRepeat::Stretch;
            let tile_y_active =
                tile_y_enabled && repeat_y != crate::box_decoration::ImageRepeat::Stretch;
            let x_tiles = image_tiles(
                target.left(),
                target.right(),
                tile_x,
                repeat_x,
                tile_x_enabled,
            );
            let y_tiles = image_tiles(
                target.top(),
                target.bottom(),
                tile_y,
                repeat_y,
                tile_y_enabled,
            );
            for (left, right, tile_left) in x_tiles {
                for (top, bottom, tile_top) in &y_tiles {
                    let visible = Rect::from_min_max(pos2(left, *top), pos2(right, *bottom));
                    let (u0, u1) = if tile_x_active {
                        (
                            source.left()
                                + ((visible.left() - tile_left) / tile_x).clamp(0.0, 1.0)
                                    * source.width(),
                            source.left()
                                + ((visible.right() - tile_left) / tile_x).clamp(0.0, 1.0)
                                    * source.width(),
                        )
                    } else {
                        (
                            source.left()
                                + (visible.left() - target.left()) / target.width()
                                    * source.width(),
                            source.left()
                                + (visible.right() - target.left()) / target.width()
                                    * source.width(),
                        )
                    };
                    let (v0, v1) = if tile_y_active {
                        (
                            source.top()
                                + ((visible.top() - tile_top) / tile_y).clamp(0.0, 1.0)
                                    * source.height(),
                            source.top()
                                + ((visible.bottom() - tile_top) / tile_y).clamp(0.0, 1.0)
                                    * source.height(),
                        )
                    } else {
                        (
                            source.top()
                                + (visible.top() - target.top()) / target.height()
                                    * source.height(),
                            source.top()
                                + (visible.bottom() - target.top()) / target.height()
                                    * source.height(),
                        )
                    };
                    if let Some(color) = solid_color {
                        shapes.push(egui::Shape::rect_filled(
                            visible,
                            egui::CornerRadius::ZERO,
                            color,
                        ));
                    } else if let Some(texture) = &texture {
                        let uv = Rect::from_min_max(
                            pos2(u0 / image_width, v0 / image_height),
                            pos2(u1 / image_width, v1 / image_height),
                        );
                        shapes.push(egui::Shape::image(
                            texture.id(),
                            visible,
                            uv,
                            Color32::WHITE,
                        ));
                    } else if let Some(element) = &source_element {
                        shapes.extend(element_slice_shapes(
                            element,
                            Rect::from_min_max(pos2(u0, v0), pos2(u1, v1)),
                            visible,
                        ));
                    } else if let Some(gradient) = gradient {
                        let source = Rect::from_min_max(pos2(u0, v0), pos2(u1, v1));
                        shapes.push(egui::Shape::Mesh(
                            linear_gradient_slice_mesh(
                                visible,
                                source,
                                vec2(image_width, image_height),
                                gradient,
                            )
                            .into(),
                        ));
                    } else if let Some(gradient) = radial_gradient {
                        let source = Rect::from_min_max(pos2(u0, v0), pos2(u1, v1));
                        shapes.push(egui::Shape::Mesh(
                            radial_gradient_slice_mesh(
                                visible,
                                source,
                                vec2(image_width, image_height),
                                gradient,
                            )
                            .into(),
                        ));
                    } else if let Some(gradient) = conic_gradient {
                        let source = Rect::from_min_max(pos2(u0, v0), pos2(u1, v1));
                        shapes.push(egui::Shape::Mesh(
                            conic_gradient_slice_mesh(
                                visible,
                                source,
                                vec2(image_width, image_height),
                                gradient,
                            )
                            .into(),
                        ));
                    }
                }
            }
        }
    }
    Some(shapes)
}

pub(super) fn border_image_outset(style: &BoxStyle) -> egui::epaint::MarginF32 {
    let widths = style.border_widths;
    let outset =
        std::array::from_fn::<_, 4, _>(|side| match style.decorations.border_image.outset[side] {
            crate::box_decoration::ImageOutset::LengthZero => 0.0,
            crate::box_decoration::ImageOutset::Length(value) => value,
            crate::box_decoration::ImageOutset::Number(value) => value * widths[side],
        });
    egui::epaint::MarginF32 {
        top: outset[0],
        right: outset[1],
        bottom: outset[2],
        left: outset[3],
    }
}

pub(super) fn clip_border_image_shapes(
    context: &Context,
    border: Rect,
    style: &BoxStyle,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    shapes: Vec<egui::Shape>,
) -> Vec<egui::Shape> {
    let Some(border_shape) = &style.decorations.border_shape else {
        return shapes;
    };
    let border_widths = style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
    let (paths, fill_rule) = border_shape.background_paths(
        border,
        crate::css::BackgroundClip::Border,
        stroke,
        padding,
        margin,
        [
            border_widths.top,
            border_widths.right,
            border_widths.bottom,
            border_widths.left,
        ],
    );
    let clip = crate::box_decoration::path_clip(paths, fill_rule);
    shapes
        .into_iter()
        .flat_map(|shape| clip_shape_to_paths(context, shape, &clip))
        .collect()
}

pub(super) fn image_tiles(
    start: f32,
    end: f32,
    natural: f32,
    repeat: crate::box_decoration::ImageRepeat,
    tiled: bool,
) -> Vec<(f32, f32, f32)> {
    if !tiled || repeat == crate::box_decoration::ImageRepeat::Stretch || natural <= 0.0 {
        return vec![(start, end, start)];
    }
    let span = end - start;
    let (tile, first_index, end_index, gap, initial) = match repeat {
        crate::box_decoration::ImageRepeat::Stretch => (span, 0, 1, 0.0, start),
        crate::box_decoration::ImageRepeat::Repeat => {
            let initial = start + (span - natural) * 0.5;
            let first = ((start - initial) / natural).floor() as isize;
            let end = ((end - initial) / natural).ceil() as isize;
            (natural, first, end, 0.0, initial)
        }
        crate::box_decoration::ImageRepeat::Round => {
            let count = (span / natural).round().max(1.0) as isize;
            (span / count as f32, 0, count, 0.0, start)
        }
        crate::box_decoration::ImageRepeat::Space => {
            let count = (span / natural).floor() as isize;
            if count <= 0 {
                return Vec::new();
            }
            let gap = (span - count as f32 * natural) / (count + 1) as f32;
            (natural, 0, count, gap, start + gap)
        }
    };
    (first_index..end_index)
        .filter_map(|index| {
            let tile_start = initial + index as f32 * (tile + gap);
            let left = tile_start.max(start);
            let right = (tile_start + tile).min(end);
            (left < end && right > left).then_some((left, right, tile_start))
        })
        .collect()
}

pub(super) fn collapsed_border_shapes(
    rect: Rect,
    style: &BoxStyle,
    edges: &[crate::formatting::CollapsedBorder; 4],
    current: Color32,
) -> Vec<egui::Shape> {
    let mut paint_style = style.clone();
    let widths: [f32; 4] = std::array::from_fn(|side| edges[side].width);
    let surface = rect
        + egui::epaint::MarginF32 {
            top: widths[0] * 0.5,
            right: widths[1] * 0.5,
            bottom: widths[2] * 0.5,
            left: widths[3] * 0.5,
        };
    paint_style.layout.border = taffy::Rect {
        top: length(widths[0]),
        right: length(widths[1]),
        bottom: length(widths[2]),
        left: length(widths[3]),
    };
    paint_style.border_colors = std::array::from_fn(|side| Some(edges[side].color));
    paint_style.border_stripes = std::array::from_fn(|side| {
        edges[side].stripes.as_ref().map(|stripes| {
            let mut stripes = stripes.iter().collect::<Vec<_>>();
            if edges[side].source_side != side {
                stripes.reverse();
            }
            stripes
                .into_iter()
                .map(|stripe| crate::box_decoration::BorderStripe {
                    color: Some(stripe.color),
                    thickness: match stripe.thickness {
                        crate::formatting::CollapsedStripeThickness::Length(value) => {
                            crate::box_decoration::ClipPart::Length(
                                crate::box_decoration::LengthPercentage::Length(value),
                            )
                        }
                        crate::formatting::CollapsedStripeThickness::Flex(value) => {
                            crate::box_decoration::ClipPart::Flex(value)
                        }
                    },
                })
                .collect()
        })
    });
    paint_style.border_styles = std::array::from_fn(|side| {
        if edges[side].draw {
            edges[side].style
        } else {
            BorderStyle::None
        }
    });
    border_shapes(surface, &paint_style, current)
}

pub(super) fn border_shapes(rect: Rect, style: &BoxStyle, current: Color32) -> Vec<egui::Shape> {
    let widths = style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
    let inner = rect - margin(widths);
    let outer_radii = crate::box_decoration::resolved_radii(style, rect);
    let inner_radii = [
        outer_radii[0] - vec2(widths.left, widths.top),
        outer_radii[1] - vec2(widths.right, widths.top),
        outer_radii[2] - vec2(widths.right, widths.bottom),
        outer_radii[3] - vec2(widths.left, widths.bottom),
    ]
    .map(|radius| radius.max(Vec2::ZERO));
    let corner_segments = crate::box_decoration::rounded_corner_segments(outer_radii);
    let corners = crate::box_decoration::rounded_path_with_segments(
        rect,
        outer_radii,
        style.decorations.corner_shapes,
        corner_segments,
    );
    let inside = crate::box_decoration::rounded_path_with_segments(
        inner,
        inner_radii,
        style.decorations.corner_shapes,
        corner_segments,
    );
    let mut shapes = Vec::new();
    let mut side_distance = [0.0_f32; 4];
    let mut side_lengths = [0.0_f32; 4];
    let border_widths = [widths.top, widths.right, widths.bottom, widths.left];
    let mut segments = Vec::new();
    let points_per_corner = corner_segments + 1;
    for index in 0..corners.len() {
        let corner = index / points_per_corner;
        let within = index % points_per_corner;
        let [first_side, second_side] = [[3, 0], [0, 1], [1, 2], [2, 3]][corner];
        if within == corner_segments {
            segments.push((index, second_side, 0.0, 1.0));
            continue;
        }
        let total_width = border_widths[first_side] + border_widths[second_side];
        let split = if total_width > 0.0 {
            border_widths[first_side] / total_width
        } else {
            0.5
        };
        let parameter_start = within as f32 / corner_segments as f32;
        let parameter_end = (within + 1) as f32 / corner_segments as f32;
        if split > parameter_start {
            let end = split.min(parameter_end);
            segments.push((
                index,
                first_side,
                0.0,
                (end - parameter_start) * corner_segments as f32,
            ));
        }
        if split < parameter_end {
            let start = split.max(parameter_start);
            segments.push((
                index,
                second_side,
                (start - parameter_start) * corner_segments as f32,
                1.0,
            ));
        }
    }
    for (index, side, from, to) in &segments {
        let distance = corners[*index].distance(corners[(*index + 1) % corners.len()]);
        side_lengths[*side] += distance * (*to - *from);
    }
    for (index, side, part_start, part_end) in segments {
        let next = (index + 1) % corners.len();
        let within = index % points_per_corner;
        let width = [widths.top, widths.right, widths.bottom, widths.left][side];
        let distance = corners[index].distance(corners[next]);
        let segment_distance = distance * (part_end - part_start);
        let segment_start = side_distance[side];
        let segment_end = segment_start + segment_distance;
        side_distance[side] = segment_end;
        if width <= 0.0
            || segment_distance <= 0.001
            || matches!(
                style.border_styles[side],
                BorderStyle::None | BorderStyle::Hidden
            )
        {
            continue;
        }
        let in_corner = within != corner_segments;
        let percentage_basis = if side % 2 == 0 {
            rect.width()
        } else {
            rect.height()
        };
        let mut visible = crate::box_decoration::limit_intervals(
            style.decorations.border_limit,
            side,
            side_lengths[side],
            percentage_basis,
            in_corner,
        );
        if let Some(clips) = &style.decorations.border_clips[side] {
            visible = crate::box_decoration::intersect_intervals(
                &visible,
                &crate::box_decoration::clip_intervals(clips, side_lengths[side]),
            );
        }
        let style_kind = style.border_styles[side];
        let dash = match style_kind {
            BorderStyle::Dashed => Some((width * 3.0, width * 3.0)),
            BorderStyle::Dotted => Some((width, width * 1.5)),
            _ => None,
        };
        if let Some((dash, gap)) = dash {
            visible = crate::box_decoration::intersect_intervals(
                &visible,
                &dash_intervals(side_lengths[side], dash, gap),
            );
        }
        visible = visible
            .into_iter()
            .filter_map(|(start, end)| {
                let start = start.max(segment_start);
                let end = end.min(segment_end);
                (end > start).then_some((
                    part_start
                        + (part_end - part_start) * ((start - segment_start) / segment_distance),
                    part_start
                        + (part_end - part_start) * ((end - segment_start) / segment_distance),
                ))
            })
            .collect();
        if visible.is_empty() {
            continue;
        }
        let color = style.border_colors[side].unwrap_or(current);
        let quad = |a: f32, b: f32, from: f32, to: f32| {
            let at = |inset: f32, along: f32| {
                corners[index]
                    .lerp(inside[index], inset)
                    .lerp(corners[next].lerp(inside[next], inset), along)
            };
            vec![at(a, from), at(a, to), at(b, to), at(b, from)]
        };
        let shade = |color: Color32, amount: f32| color.gamma_multiply(amount);
        let stripes = style.border_stripes[side].as_ref();
        let mut bands = stripes.map_or_else(
            || vec![(0.0, 1.0, color)],
            |stripes| border_stripe_bands(stripes, width, current),
        );
        if stripes.is_none() {
            match style_kind {
                BorderStyle::Double if width >= 3.0 => {
                    bands = vec![(0.0, 1.0 / 3.0, color), (2.0 / 3.0, 1.0, color)];
                }
                BorderStyle::Groove
                | BorderStyle::Ridge
                | BorderStyle::Inset
                | BorderStyle::Outset => {
                    let inset = matches!(style_kind, BorderStyle::Inset | BorderStyle::Groove);
                    let dark_outer = (side == 0 || side == 3) == inset;
                    let first = shade(color, if dark_outer { 0.55 } else { 1.0 });
                    let second = shade(color, if dark_outer { 1.0 } else { 0.55 });
                    bands = if matches!(style_kind, BorderStyle::Groove | BorderStyle::Ridge) {
                        vec![(0.0, 0.5, first), (0.5, 1.0, second)]
                    } else {
                        vec![(0.0, 1.0, first)]
                    };
                }
                _ => {}
            }
        }
        for (start, end, color) in bands {
            let (start, end) = if stripes.is_some() {
                ((start - 0.5 / width).max(0.0), (end + 0.5 / width).min(1.0))
            } else {
                (start, end)
            };
            for (from, to) in &visible {
                shapes.push(egui::Shape::convex_polygon(
                    quad(start, end, *from, *to),
                    color,
                    egui::Stroke::NONE,
                ));
            }
        }
    }
    shapes
}

pub(super) fn dash_intervals(length: f32, dash: f32, gap: f32) -> Vec<(f32, f32)> {
    let period = dash + gap;
    if period <= 0.0 {
        return vec![(0.0, length)];
    }
    let mut intervals = Vec::new();
    let mut start = 0.0;
    while start < length {
        let end = (start + dash).min(length);
        if end > start {
            intervals.push((start, end));
        }
        start += period;
    }
    intervals
}

pub(super) fn border_shape_shapes(
    rect: Rect,
    style: &BoxStyle,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    current: Color32,
) -> Option<Vec<egui::Shape>> {
    let border_shape = style.decorations.border_shape.as_ref()?;
    let side = relevant_border_side(style);
    let border_style = style.border_styles[side];
    if matches!(border_style, BorderStyle::None | BorderStyle::Hidden) {
        return Some(Vec::new());
    }
    let border_widths = style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
    let widths = [
        border_widths.top,
        border_widths.right,
        border_widths.bottom,
        border_widths.left,
    ];
    let width = widths[side];
    if width <= 0.0 {
        return Some(Vec::new());
    }
    let color = style.border_colors[side].unwrap_or(current);
    let outers = border_shape.outer_paths(rect, stroke, padding, margin, widths);
    let inners = border_shape.inner_paths(rect, stroke, padding, margin, widths);
    let mut shapes = Vec::new();
    for (path_index, outer) in outers.into_iter().map(clockwise).enumerate() {
        if outer.len() < 3 {
            continue;
        }
        let inner = inners
            .as_ref()
            .and_then(|paths| paths.get(path_index))
            .cloned()
            .map(clockwise)
            .map(|inner| align_path_start(&outer, inner));
        if let Some(inner) = inner {
            let count = outer.len().max(inner.len());
            let outer = resample_path(&outer, count);
            let inner = resample_path(&inner, count);
            let bands = style.border_stripes[side].as_ref().map_or_else(
                || vec![(0.0, 1.0, color)],
                |stripes| border_stripe_bands(stripes, width, current),
            );
            for index in 0..count {
                let next = (index + 1) % count;
                for (start, end, color) in &bands {
                    let outer_start = outer[index].lerp(inner[index], *start);
                    let outer_end = outer[next].lerp(inner[next], *start);
                    let inner_end = outer[next].lerp(inner[next], *end);
                    let inner_start = outer[index].lerp(inner[index], *end);
                    shapes.push(egui::Shape::convex_polygon(
                        vec![outer_start, outer_end, inner_end, inner_start],
                        *color,
                        egui::Stroke::NONE,
                    ));
                }
            }
        } else {
            let mut distance = 0.0;
            for index in 0..outer.len() {
                let next = (index + 1) % outer.len();
                let start = outer[index];
                let end = outer[next];
                let segment = end - start;
                let length = segment.length();
                if length <= 0.001 {
                    continue;
                }
                let normal = vec2(segment.y, -segment.x) / length;
                let visible = match border_style {
                    BorderStyle::Dashed => (distance % (width * 6.0)) < width * 3.0,
                    BorderStyle::Dotted => (distance % (width * 2.5)) < width,
                    _ => true,
                };
                if visible {
                    let bands = style.border_stripes[side].as_ref().map_or_else(
                        || vec![(0.0, 1.0, color)],
                        |stripes| border_stripe_bands(stripes, width, current),
                    );
                    for (start_inset, end_inset, color) in bands {
                        let half = width * 0.5;
                        let outer_offset = half * (1.0 - 2.0 * start_inset);
                        let inner_offset = half * (1.0 - 2.0 * end_inset);
                        shapes.push(egui::Shape::convex_polygon(
                            vec![
                                start + normal * outer_offset,
                                end + normal * outer_offset,
                                end + normal * inner_offset,
                                start + normal * inner_offset,
                            ],
                            color,
                            egui::Stroke::NONE,
                        ));
                    }
                }
                distance += length;
            }
        }
    }
    Some(shapes)
}

pub(super) fn relevant_border_side(style: &BoxStyle) -> usize {
    ["block-start", "inline-start", "block-end", "inline-end"]
        .into_iter()
        .filter_map(|name| {
            crate::layout_style::logical_side(
                name,
                style.layout.direction,
                style.format.writing,
                &style.format.text_orientation,
            )
        })
        .find(|side| !matches!(style.border_styles[*side], BorderStyle::None))
        .or_else(|| {
            crate::layout_style::logical_side(
                "block-start",
                style.layout.direction,
                style.format.writing,
                &style.format.text_orientation,
            )
        })
        .unwrap_or(0)
}

pub(super) fn clockwise(mut points: Vec<Pos2>) -> Vec<Pos2> {
    let area = (0..points.len())
        .map(|index| {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>();
    if area < 0.0 {
        points.reverse();
    }
    points
}

pub(super) fn resample_path(points: &[Pos2], count: usize) -> Vec<Pos2> {
    let lengths = (0..points.len())
        .map(|index| points[index].distance(points[(index + 1) % points.len()]))
        .collect::<Vec<_>>();
    let total = lengths.iter().sum::<f32>();
    if total <= 0.001 || count == 0 {
        return points.to_vec();
    }
    (0..count)
        .map(|sample| {
            let target = total * sample as f32 / count as f32;
            let mut consumed = 0.0;
            for (index, segment) in lengths.iter().copied().enumerate() {
                if target <= consumed + segment || index + 1 == lengths.len() {
                    let t = if segment <= 0.001 {
                        0.0
                    } else {
                        (target - consumed) / segment
                    };
                    return points[index]
                        .lerp(points[(index + 1) % points.len()], t.clamp(0.0, 1.0));
                }
                consumed += segment;
            }
            points[0]
        })
        .collect()
}

pub(super) fn align_path_start(reference: &[Pos2], points: Vec<Pos2>) -> Vec<Pos2> {
    if points.is_empty() || reference.is_empty() {
        return points;
    }
    let start = reference[0];
    let index = points
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            left.distance_sq(start).total_cmp(&right.distance_sq(start))
        })
        .map_or(0, |(index, _)| index);
    points[index..]
        .iter()
        .chain(points[..index].iter())
        .copied()
        .collect()
}

pub(super) fn border_stripe_bands(
    stripes: &[crate::box_decoration::BorderStripe],
    width: f32,
    current: Color32,
) -> Vec<(f32, f32, Color32)> {
    let mut fixed = 0.0;
    let mut flex = 0.0;
    for stripe in stripes {
        match stripe.thickness {
            crate::box_decoration::ClipPart::Length(value) => {
                fixed += value.resolve(width).max(0.0)
            }
            crate::box_decoration::ClipPart::Flex(weight) => flex += weight.max(0.0),
        }
    }
    let distributable = (width - fixed).max(0.0);
    let mut inset = 0.0;
    let mut bands = Vec::new();
    for stripe in stripes {
        let thickness = match stripe.thickness {
            crate::box_decoration::ClipPart::Length(value) => value.resolve(width).max(0.0),
            crate::box_decoration::ClipPart::Flex(weight) if flex > 0.0 => {
                distributable * weight / flex.max(1.0)
            }
            crate::box_decoration::ClipPart::Flex(_) => 0.0,
        };
        let end = (inset + thickness).min(width);
        if end > inset {
            bands.push((inset / width, end / width, stripe.color.unwrap_or(current)));
        }
        inset = end;
    }
    bands
        .into_iter()
        .rev()
        .map(|(start, end, color)| (1.0 - end, 1.0 - start, color))
        .collect()
}

pub(super) fn same_line(a: Rect, b: Rect, vertical: bool) -> bool {
    if vertical {
        (a.left() - b.left()).abs() < 0.01 && (a.width() - b.width()).abs() < 0.01
    } else {
        (a.top() - b.top()).abs() < 0.01 && (a.height() - b.height()).abs() < 0.01
    }
}

pub(super) fn without_id_clash_warning<R>(context: &Context, f: impl FnOnce() -> R) -> R {
    // Selector boxes intentionally re-register their child's ID over the CSS box rect.
    let warn_on_id_clash = context.options(|options| options.warn_on_id_clash);
    if warn_on_id_clash {
        context.options_mut(|options| options.warn_on_id_clash = false);
    }
    let result = f();
    if warn_on_id_clash {
        context.options_mut(|options| options.warn_on_id_clash = true);
    }
    result
}

pub(super) fn clear_previous_label_selection(context: &Context) {
    let press_time = context.input(|input| {
        input
            .events
            .iter()
            .any(|event| {
                matches!(
                    event,
                    egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers,
                        ..
                    } if !modifiers.shift
                )
            })
            .then_some(input.time.to_bits())
    });
    let Some(press_time) = press_time else {
        return;
    };
    let last_press = context.data(|data| data.get_temp::<u64>(Id::unique(LABEL_SELECTION_PRESS)));
    // egui can rerun this input frame for additional layout passes.
    if last_press == Some(press_time) {
        return;
    }
    context.data_mut(|data| data.insert_temp(Id::unique(LABEL_SELECTION_PRESS), press_time));
    context
        .plugin::<egui::text_selection::LabelSelectionState>()
        .lock()
        .clear_selection();
}

pub(super) fn reorder_paint(frame: &Frame, context: &Context) {
    type Part = (i32, i32, usize);
    type Path = smallvec::SmallVec<[Part; 4]>;
    let count = frame.nodes.len();
    let border_clips = frame
        .nodes
        .iter()
        .map(border_content_clip)
        .collect::<Vec<_>>();
    if frame.nodes.iter().all(|node| {
        !node.style.positioned
            && node.style.z_index.is_none()
            && !node.style.fixed
            && node.style.layout.position != Position::Sticky
            && !node.style.layout.float.is_floated()
            && !node.style.inline
            && !node.flow_source
            && node.paint.as_ref().is_none_or(|paint| {
                paint.overlay == paint.end
                    && paint.geometry.slices.is_empty()
                    && paint.geometry.collapsed.is_none()
            })
    }) && frame.nodes.iter().all(|node| node.style.order == 0)
        && border_clips.iter().all(Option::is_none)
    {
        return;
    }
    let mut contexts: Vec<Path> = vec![Path::new(); count];
    let mut groups = contexts.clone();
    let mut keys = contexts.clone();
    let mut ranks = vec![0; count];
    fn rank(frame: &Frame, node: usize, next: &mut usize, ranks: &mut [usize]) {
        ranks[node] = *next;
        *next += 1;
        if matches!(
            frame.nodes[node].style.layout.display,
            Display::Flex | Display::Grid
        ) {
            let mut children = frame.nodes[node].children.clone();
            children.sort_by_key(|i| frame.nodes[*i].style.order);
            for child in children {
                rank(frame, child, next, ranks);
            }
        } else {
            for child in frame.nodes[node].children.iter().copied() {
                rank(frame, child, next, ranks);
            }
        }
    }
    rank(frame, 0, &mut 0, &mut ranks);
    let mut named_flow_content = vec![false; count];
    for (index, node) in frame.nodes.iter().enumerate() {
        if !node.flow_source {
            continue;
        }
        let mut pending = vec![index];
        while let Some(index) = pending.pop() {
            if std::mem::replace(&mut named_flow_content[index], true) {
                continue;
            }
            pending.extend(frame.nodes[index].children.iter().copied());
        }
    }
    for (rank, in_named_flow) in ranks.iter_mut().zip(named_flow_content) {
        if in_named_flow {
            *rank += count;
        }
    }
    for (i, node) in frame.nodes.iter().enumerate() {
        let parent_context = node.parent.map_or_else(Path::new, |p| contexts[p].clone());
        let parent_group = node.parent.map_or_else(Path::new, |p| groups[p].clone());
        let item = node.parent.is_some_and(|p| {
            matches!(
                frame.nodes[p].style.layout.display,
                Display::Flex | Display::Grid
            )
        });
        let will_change_z_index =
            node.style.will_change("z-index") && (node.style.positioned || item);
        let real = i == 0
            || node.style.z_index.is_some() && (node.style.positioned || item)
            || node.style.will_change("position")
            || will_change_z_index
            || node.style.fixed
            || node.style.layout.position == Position::Sticky;
        let pseudo = node.style.positioned
            || node.style.layout.float.is_floated()
            || node.style.inline
            || item;
        let mut prefix: Path = if real {
            parent_context.clone()
        } else {
            parent_group
        };
        if real && i != 0 {
            let z = node.style.z_index.unwrap_or(0);
            prefix.push((
                if z < 0 {
                    0
                } else if z == 0 {
                    4
                } else {
                    5
                },
                z,
                ranks[i],
            ));
        } else if !real && pseudo {
            prefix.push((
                if node.style.positioned {
                    4
                } else if node.style.layout.float.is_floated() {
                    2
                } else if node.style.inline {
                    3
                } else {
                    1
                },
                0,
                ranks[i],
            ));
        }
        contexts[i] = if real { prefix.clone() } else { parent_context };
        groups[i] = prefix.clone();
        prefix.push((if real || pseudo { -1 } else { 1 }, 0, ranks[i]));
        keys[i] = prefix;
    }
    let mut order: Vec<_> = (0..count).collect();
    order.sort_by(|a, b| keys[*a].cmp(&keys[*b]));
    let Some(root) = frame.nodes[0].paint.as_ref() else {
        return;
    };
    let layer = root.ui.layer_id();
    let entries = context.graphics(|layers| {
        layers
            .get(layer)
            .map(|list| {
                list.all_entries()
                    .skip(root.start.0)
                    .take(root.end.0 - root.start.0)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let trailing_entries = context.graphics(|layers| {
        layers
            .get(layer)
            .map(|list| {
                list.all_entries()
                    .skip(root.end.0)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let mut owners = vec![0; entries.len()];
    for (i, node) in frame.nodes.iter().enumerate() {
        if let Some(paint) = &node.paint
            && paint.ui.layer_id() == layer
        {
            for owner in owners
                .iter_mut()
                .take(paint.end.0 - root.start.0)
                .skip(paint.start.0 - root.start.0)
            {
                *owner = i;
            }
        }
    }
    let mut ancestor_clips: Vec<smallvec::SmallVec<[usize; 4]>> =
        vec![smallvec::SmallVec::new(); count];
    for (index, node) in frame.nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            ancestor_clips[index] = ancestor_clips[parent].clone();
            if border_clips[parent].is_some() {
                ancestor_clips[index].push(parent);
            }
        }
    }
    let mut grouped = vec![Vec::new(); count];
    let mut overlays = vec![Vec::new(); count];
    for (index, (entry, owner)) in entries.into_iter().zip(owners).enumerate() {
        let node = &frame.nodes[owner];
        let Some(paint) = node.paint.as_ref() else {
            continue;
        };
        let paint_clip = node
            .flow_clip
            .map_or(paint.clip, |flow_clip| paint.clip.intersect(flow_clip));
        let mut entry = entry;
        entry.clip_rect = entry.clip_rect.intersect(paint_clip);
        let decoration = index + root.start.0 == paint.start.0;
        let mut clips = ancestor_clips[owner]
            .iter()
            .filter_map(|ancestor| border_clips[*ancestor].as_ref())
            .collect::<smallvec::SmallVec<[&crate::box_decoration::PathClip; 4]>>();
        if !decoration {
            clips.extend(border_clips[owner].as_ref());
        }
        let is_overlay = index + root.start.0 >= paint.overlay.0;
        if is_overlay {
            append_painted_entry(context, entry, &clips, &mut overlays[owner]);
        } else if paint.geometry.slices.is_empty() {
            append_painted_entry(context, entry, &clips, &mut grouped[owner]);
        } else {
            let slices = &paint.geometry.slices;
            for (slice_index, slice) in slices.iter().enumerate() {
                let source = slice.source.translate(paint.geometry.border.min.to_vec2());
                if decoration {
                    let collapsed_internal = paint.geometry.collapsed.is_some()
                        && matches!(
                            node.style.format.display,
                            crate::formatting::DisplayKind::Row
                                | crate::formatting::DisplayKind::Cell
                                | crate::formatting::DisplayKind::RowGroup
                                | crate::formatting::DisplayKind::Header
                                | crate::formatting::DisplayKind::Footer
                                | crate::formatting::DisplayKind::Column
                                | crate::formatting::DisplayKind::ColumnGroup
                        );
                    if !collapsed_internal {
                        let mut shadows = crate::box_decoration::shadow_shapes(
                            source,
                            &node.style,
                            node.color,
                            crate::box_decoration::ShadowPosition::Outset,
                            paint.geometry.stroke,
                            paint.geometry.padding,
                            paint.geometry.margin,
                        );
                        if !shadows.is_empty() {
                            let destination = source.translate(slice.offset);
                            let shadow_clip = if node.style.format.clone_decoration {
                                Some(paint_clip)
                            } else {
                                fragment_shadow_clip(
                                    destination,
                                    paint_clip,
                                    FragmentPosition {
                                        vertical: !node.style.format.writing.vertical(),
                                        forward: !node.style.format.writing.reverse(),
                                        index: slice_index,
                                        count: slices.len(),
                                    },
                                    crate::box_decoration::ShadowPosition::Outset,
                                    0.0,
                                    paint.geometry.stroke,
                                )
                            };
                            if let Some(shadow_clip) = shadow_clip {
                                let mut shadow = egui::Shape::Vec(std::mem::take(&mut shadows));
                                shadow.transform(egui::emath::TSTransform::from_translation(
                                    slice.offset,
                                ));
                                for shadow in
                                    clip_fragment_shadows(context, vec![shadow], Some(shadow_clip))
                                {
                                    append_painted_entry(
                                        context,
                                        egui::epaint::ClippedShape::new(shadow_clip, shadow),
                                        &clips,
                                        &mut grouped[owner],
                                    );
                                }
                            }
                        }
                    }
                }
                let source_clip = if decoration {
                    source + border_image_outset(&node.style)
                } else {
                    source
                };
                let clip = entry
                    .clip_rect
                    .intersect(source_clip)
                    .translate(slice.offset)
                    .intersect(paint_clip);
                if !clip.is_positive() {
                    continue;
                }
                let shape = if decoration && node.style.format.clone_decoration {
                    clone_decoration(
                        &entry.shape,
                        &BoxDecorationGeometry::new(
                            source,
                            &node.style,
                            paint.geometry.stroke,
                            paint.geometry.padding,
                            paint.geometry.margin,
                        ),
                        node.color,
                        paint
                            .fragment_border_images
                            .get(slice_index)
                            .and_then(Option::as_deref),
                        paint
                            .fragment_background_shapes
                            .get(slice_index)
                            .map_or(&[], Vec::as_slice),
                    )
                } else {
                    entry.shape.clone()
                };
                let mut clipped_shape = egui::epaint::ClippedShape::new(entry.clip_rect, shape);
                clipped_shape.transform_after_tessellation = entry.transform_after_tessellation;
                clipped_shape.transform(egui::emath::TSTransform::from_translation(slice.offset));
                clipped_shape.clip_rect = clip;
                append_painted_entry(context, clipped_shape, &clips, &mut grouped[owner]);
            }
        }
    }
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        let end = list.next_idx().0;
        for index in root.start.0..end {
            list.reset_shape(egui::layers::ShapeIdx(index));
        }
        for index in &order {
            for entry in grouped[*index].drain(..) {
                list.add(entry.clip_rect, entry.shape);
            }
        }
        for index in &order {
            for entry in overlays[*index].drain(..) {
                list.add(entry.clip_rect, entry.shape);
            }
        }
        for entry in trailing_entries {
            list.add(entry.clip_rect, entry.shape);
        }
    });
    for index in &order {
        let node = &frame.nodes[*index];
        if !node.absent
            && !node.style.hidden
            && !node.style.contents
            && let Some(paint) = &node.paint
        {
            without_id_clash_warning(context, || {
                paint.ui.interact_opt(
                    paint.response.rect,
                    paint.response.id,
                    paint.response.sense,
                    egui::InteractOptions { move_to_top: true },
                );
            });
        }
    }
    for index in &order {
        if let Some(paint) = &frame.nodes[*index].paint {
            for response in &paint.scrollbars {
                paint.ui.interact_opt(
                    response.rect,
                    response.id,
                    response.sense,
                    egui::InteractOptions { move_to_top: true },
                );
            }
        }
    }
    let ids: Vec<_> = order.iter().map(|i| frame.nodes[*i].id).collect();
    if ids != frame.cache.paint_order {
        context.request_discard("CSS paint order changed");
    }
    context.data_mut(|data| {
        let mut cache = data
            .get_temp::<Cache>(frame.nodes[0].id)
            .unwrap_or_default();
        cache.paint_order = ids;
        data.insert_temp(frame.nodes[0].id, cache);
    });
}
