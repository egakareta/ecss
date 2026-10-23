use super::*;

pub(super) fn resolve(value: taffy::CompactLength, basis: f32) -> Option<f32> {
    if value.is_plain_calc() {
        return Some(crate::math::resolve(value.calc_value(), basis));
    }
    match value.tag() {
        taffy::CompactLength::LENGTH_TAG => Some(value.value()),
        taffy::CompactLength::PERCENT_TAG if basis.is_finite() => Some(value.value() * basis),
        _ => None,
    }
}

pub(super) fn refresh_transition_box(style: &BoxStyle, geometry: &mut Geometry, containing: Rect) {
    if style.format.writing.vertical()
        || style.inline
        || geometry.collapsed.is_some()
        || !geometry.fragments.is_empty()
        || !geometry.slices.is_empty()
    {
        return;
    }
    let content = geometry.content().size();
    let previous_margin = geometry.margin;
    let updated = guess(style, containing);
    geometry.padding = updated.padding;
    geometry.stroke = updated.stroke;
    geometry.margin = updated.margin;
    let extras = geometry.padding.sum() + geometry.stroke.sum();
    let size = |dimension: Dimension,
                min: LengthPercentageAuto,
                max: LengthPercentageAuto,
                basis,
                intrinsic,
                extras| {
        let adjustment = if style.layout.box_sizing == BoxSizing::ContentBox {
            extras
        } else {
            0.0
        };
        let value = resolve(dimension.into_raw(), basis).unwrap_or(intrinsic + extras - adjustment);
        let min = resolve(min.into_raw(), basis).unwrap_or(0.0);
        let max = resolve(max.into_raw(), basis)
            .unwrap_or(f32::INFINITY)
            .max(min);
        (value.clamp(min, max) + adjustment).max(extras)
    };
    let size = vec2(
        size(
            style.layout.size.width,
            style.layout.min_size.width,
            style.layout.max_size.width,
            containing.width(),
            content.x,
            extras.x,
        ),
        size(
            style.layout.size.height,
            style.layout.min_size.height,
            style.layout.max_size.height,
            containing.height(),
            content.y,
            extras.y,
        ),
    );
    let offset = vec2(
        geometry.margin.left - previous_margin.left,
        geometry.margin.top - previous_margin.top,
    );
    geometry.border = Rect::from_min_size(geometry.border.min + offset, size);
}

pub(super) fn guess(style: &BoxStyle, available: Rect) -> Geometry {
    let s = &style.layout;
    let width = available.width();
    let padding = margin(
        s.padding
            .map(|v| resolve(v.into_raw(), width).unwrap_or(0.0)),
    );
    let stroke = margin(
        s.border
            .map(|v| resolve(v.into_raw(), width).unwrap_or(0.0)),
    );
    let outer = margin(
        s.margin
            .map(|v| resolve(v.into_raw(), width).unwrap_or(0.0)),
    );
    let extras = if s.box_sizing == BoxSizing::ContentBox {
        padding.sum() + stroke.sum()
    } else {
        Vec2::ZERO
    };
    let size = vec2(
        resolve(s.size.width.into_raw(), width)
            .map_or(available.width() - outer.sum().x, |w| w + extras.x),
        resolve(s.size.height.into_raw(), available.height())
            .map_or(available.height() - outer.sum().y, |h| h + extras.y),
    )
    .max(padding.sum() + stroke.sum());
    Geometry {
        border: Rect::from_min_size(available.min + vec2(outer.left, outer.top), size),
        padding,
        stroke,
        gutter: egui::epaint::MarginF32::ZERO,
        margin: outer,
        extent: Vec2::ZERO,
        fragments: Vec::new(),
        flow_source: false,
        flow_region_content: false,
        slices: Vec::new(),
        rules: Vec::new(),
        text_vertical: false,
        collapsed: None,
        collapsed_segments: Vec::new(),
    }
}

pub(super) fn margin(rect: taffy::Rect<f32>) -> egui::epaint::MarginF32 {
    egui::epaint::MarginF32 {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
    }
}

pub(super) fn scrollbar_gutter(style: &BoxStyle) -> egui::epaint::MarginF32 {
    if style.scrollbar_gutter == crate::layout_style::ScrollbarGutter::Auto {
        return egui::epaint::MarginF32::ZERO;
    }
    let width = style.scrollbar_width.thickness();
    if width == 0.0 {
        return egui::epaint::MarginF32::ZERO;
    }
    let vertical = matches!(style.layout.overflow.y, Overflow::Hidden | Overflow::Scroll);
    let horizontal = matches!(style.layout.overflow.x, Overflow::Hidden | Overflow::Scroll);
    let both_edges =
        style.scrollbar_gutter == crate::layout_style::ScrollbarGutter::StableBothEdges;
    egui::epaint::MarginF32 {
        top: if horizontal && both_edges { width } else { 0.0 },
        right: if vertical { width } else { 0.0 },
        bottom: if horizontal { width } else { 0.0 },
        left: if vertical && both_edges { width } else { 0.0 },
    }
}

pub(super) fn trimmed_child_margins(frame: &Frame) -> Option<Vec<[bool; 4]>> {
    fn append_flow_children(frame: &Frame, parent: usize, output: &mut Vec<usize>) {
        for child in &frame.nodes[parent].children {
            let node = &frame.nodes[*child];
            if node.absent
                || node.style.layout.display == Display::None
                || node.style.layout.position.is_out_of_flow()
                || node.style.layout.float.is_floated()
            {
                continue;
            }
            if node.style.contents {
                append_flow_children(frame, *child, output);
            } else {
                output.push(*child);
            }
        }
    }

    fn block_level(style: &BoxStyle) -> bool {
        !style.inline
            && matches!(
                style.format.display,
                crate::formatting::DisplayKind::Normal
                    | crate::formatting::DisplayKind::Table
                    | crate::formatting::DisplayKind::Caption
            )
    }

    fn block_container(style: &BoxStyle) -> bool {
        !style.contents
            && matches!(style.layout.display, Display::Block | Display::FlowRoot)
            && style.format.display == crate::formatting::DisplayKind::Normal
    }

    fn physical_edge<T: Copy>(rect: &taffy::Rect<T>, edge: usize) -> T {
        match edge {
            0 => rect.top,
            1 => rect.right,
            2 => rect.bottom,
            _ => rect.left,
        }
    }

    fn zero_or_auto(value: Dimension) -> bool {
        value.is_auto() || resolve(value.into_raw(), 1.0).is_some_and(|value| value == 0.0)
    }

    fn collapses_through(frame: &Frame, index: usize, block_edge: usize) -> bool {
        let container = &frame.nodes[index].style;
        if container.layout.display != Display::Block
            || container.format.display != crate::formatting::DisplayKind::Normal
            || container.layout.position.is_out_of_flow()
            || container.layout.float.is_floated()
            || !matches!(
                container.layout.overflow.x,
                Overflow::Visible | Overflow::Clip
            )
            || !matches!(
                container.layout.overflow.y,
                Overflow::Visible | Overflow::Clip
            )
        {
            return false;
        }

        let logical_edge = if block_edge == 0 {
            "block-start"
        } else {
            "block-end"
        };
        let Some(edge) = crate::layout_style::logical_side(
            logical_edge,
            container.layout.direction,
            container.format.writing,
            &container.format.text_orientation,
        ) else {
            return false;
        };
        let border = physical_edge(&container.layout.border, edge);
        let padding = physical_edge(&container.layout.padding, edge);
        if resolve(border.into_raw(), 0.0).unwrap_or(0.0) != 0.0
            || resolve(padding.into_raw(), 1.0).unwrap_or(0.0) != 0.0
        {
            return false;
        }

        let (size, min_size) = if container.format.writing.vertical() {
            (container.layout.size.width, container.layout.min_size.width)
        } else {
            (
                container.layout.size.height,
                container.layout.min_size.height,
            )
        };
        zero_or_auto(size) && zero_or_auto(min_size.into())
    }

    let containers = frame
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            block_container(&node.style) && node.style.margin_trim[..2].iter().any(|trim| *trim)
        })
        .collect::<Vec<_>>();
    if containers.is_empty() {
        return None;
    }

    let mut trim = vec![[false; 4]; frame.nodes.len()];
    for (index, node) in containers {
        let mut children = Vec::new();
        append_flow_children(frame, index, &mut children);
        let boundary_children = [children.first().copied(), children.last().copied()];
        for edge in 0..2 {
            if !node.style.margin_trim[edge] {
                continue;
            }
            let Some(mut current) = boundary_children[edge] else {
                continue;
            };
            if !block_level(&frame.nodes[current].style) {
                continue;
            }
            let name = ["block-start", "block-end"][edge];
            let Some(physical) = crate::layout_style::logical_side(
                name,
                node.style.layout.direction,
                node.style.format.writing,
                &node.style.format.text_orientation,
            ) else {
                continue;
            };
            loop {
                trim[current][physical] = true;
                if !collapses_through(frame, current, edge) {
                    break;
                }
                let mut descendants = Vec::new();
                append_flow_children(frame, current, &mut descendants);
                let next = if edge == 0 {
                    descendants.first()
                } else {
                    descendants.last()
                };
                let Some(next) = next
                    .copied()
                    .filter(|next| block_level(&frame.nodes[*next].style))
                else {
                    break;
                };
                current = next;
            }
        }
    }
    Some(trim)
}

pub(super) fn collapse_flex_item(frame: &Frame, child: usize, layout: Layout, style: &mut Style) {
    let node = &frame.nodes[child];
    if !node.style.collapsed {
        return;
    }
    let Some(parent) = node.parent.map(|index| &frame.nodes[index]) else {
        return;
    };
    if parent.style.layout.display != Display::Flex {
        return;
    }
    let row = matches!(
        parent.style.layout.flex_direction,
        FlexDirection::Row | FlexDirection::RowReverse
    );
    let main_x = row != parent.style.format.writing.vertical();
    let content_cross = if main_x {
        (layout.size.height
            - layout.padding.top
            - layout.padding.bottom
            - layout.border.top
            - layout.border.bottom)
            .max(0.0)
    } else {
        (layout.size.width
            - layout.padding.left
            - layout.padding.right
            - layout.border.left
            - layout.border.right)
            .max(0.0)
    };
    let cross_border = if style.box_sizing == BoxSizing::ContentBox {
        content_cross
    } else if main_x {
        layout.size.height
    } else {
        layout.size.width
    };
    let zero = taffy::LengthPercentage::length(0.0);
    let zero_auto = taffy::LengthPercentageAuto::length(0.0);
    let cross = taffy::Dimension::length(cross_border);
    let cross_limit = taffy::LengthPercentageAuto::length(cross_border);
    style.flex_basis = zero_auto.into();
    style.flex_grow = 0.0;
    style.flex_shrink = 0.0;
    if main_x {
        style.size.width = taffy::Dimension::length(0.0);
        style.min_size.width = zero_auto;
        style.max_size.width = zero_auto;
        style.size.height = cross;
        style.min_size.height = cross_limit;
        style.max_size.height = cross_limit;
        style.margin.left = zero_auto;
        style.margin.right = zero_auto;
        style.padding.left = zero;
        style.padding.right = zero;
        style.border.left = zero;
        style.border.right = zero;
    } else {
        style.size.height = taffy::Dimension::length(0.0);
        style.min_size.height = zero_auto;
        style.max_size.height = zero_auto;
        style.size.width = cross;
        style.min_size.width = cross_limit;
        style.max_size.width = cross_limit;
        style.margin.top = zero_auto;
        style.margin.bottom = zero_auto;
        style.padding.top = zero;
        style.padding.bottom = zero;
        style.border.top = zero;
        style.border.bottom = zero;
    }
}

pub(super) fn solve(frame: &mut Frame, context: &Context) -> Cache {
    let mut cache = std::mem::take(&mut frame.cache);
    frame.cache.paint_order = cache.paint_order.clone();
    cache.boxes.clear();
    let mut measurements = frame
        .nodes
        .iter()
        .map(|node| {
            let measured = if node.style.format.writing.vertical() && node.text.is_none() {
                vec2(node.measured.y, node.measured.x)
            } else {
                node.measured
            };
            let text = node
                .text
                .as_ref()
                .map(|text| (text.job.clone(), text.extra, text.minimum));
            (measured, text)
        })
        .collect::<Vec<_>>();
    let mut tree = crate::layout_tree::Tree::new(crate::math::resolve);
    let mut ids = Vec::new();
    let trimmed_margins = trimmed_child_margins(frame);
    for index in 0..frame.nodes.len() {
        let node = &frame.nodes[index];
        let mut style = node.style.layout.clone();
        if let Some(trimmed_margins) = &trimmed_margins {
            let zero = taffy::LengthPercentageAuto::length(0.0);
            for (edge, selected) in trimmed_margins[index].iter().copied().enumerate() {
                if !selected {
                    continue;
                }
                match edge {
                    0 => style.margin.top = zero,
                    1 => style.margin.right = zero,
                    2 => style.margin.bottom = zero,
                    _ => style.margin.left = zero,
                }
            }
        }
        if node.absent {
            style.display = Display::None;
        }
        let gutter = scrollbar_gutter(&node.style);
        if style.box_sizing == BoxSizing::ContentBox {
            style.size.width = frame
                .math
                .add_pixels_dimension(style.size.width, -(gutter.left + gutter.right));
            style.min_size.width = frame.math.add_pixels_length_percentage_auto(
                style.min_size.width,
                -(gutter.left + gutter.right),
            );
            style.max_size.width = frame.math.add_pixels_length_percentage_auto(
                style.max_size.width,
                -(gutter.left + gutter.right),
            );
            style.size.height = frame
                .math
                .add_pixels_dimension(style.size.height, -(gutter.top + gutter.bottom));
            style.min_size.height = frame.math.add_pixels_length_percentage_auto(
                style.min_size.height,
                -(gutter.top + gutter.bottom),
            );
            style.max_size.height = frame.math.add_pixels_length_percentage_auto(
                style.max_size.height,
                -(gutter.top + gutter.bottom),
            );
        }
        style.padding.left = frame
            .math
            .add_pixels(style.padding.left.into_raw(), gutter.left);
        style.padding.right = frame
            .math
            .add_pixels(style.padding.right.into_raw(), gutter.right);
        style.padding.top = frame
            .math
            .add_pixels(style.padding.top.into_raw(), gutter.top);
        style.padding.bottom = frame
            .math
            .add_pixels(style.padding.bottom.into_raw(), gutter.bottom);
        // Unspecified leaf display preserves egui's intrinsic widget sizing.
        if (node.parent.is_none() || node.style.inline)
            && !node.style.display_set
            && node.children.is_empty()
            && style.size.width.is_auto()
        {
            style.size.width = if node.style.format.writing.vertical() && node.text.is_some() {
                Dimension::fit_content()
            } else {
                length(node.measured.x)
            };
        }
        if node.style.inline && style.size.width.is_auto() && node.children.is_empty() {
            style.size.width = length(node.measured.x);
        }
        if node.style.format.display == crate::formatting::DisplayKind::Inline {
            style.size = Size::auto();
            style.min_size = Size::auto();
            style.max_size = Size::auto();
        }
        let intrinsic_basis = node
            .parent
            .map(|parent| frame.nodes[parent].used.content().width())
            .unwrap_or(frame.available.x);
        let intrinsic_padding = style.padding.map(|value| {
            crate::format_layout::raw(value.into_raw(), intrinsic_basis).unwrap_or(0.0)
        });
        let intrinsic_border = style.border.map(|value| {
            crate::format_layout::raw(value.into_raw(), intrinsic_basis).unwrap_or(0.0)
        });
        let intrinsic_adjustment = if style.box_sizing == BoxSizing::ContentBox {
            Size {
                width: intrinsic_padding.left
                    + intrinsic_padding.right
                    + intrinsic_border.left
                    + intrinsic_border.right,
                height: intrinsic_padding.top
                    + intrinsic_padding.bottom
                    + intrinsic_border.top
                    + intrinsic_border.bottom,
            }
        } else {
            Size::ZERO
        };
        let id = tree.new_leaf_with_context(style, index).unwrap();
        tree.nodes[usize::from(id)].format = node.style.format.clone();
        if node.style.format.display != crate::formatting::DisplayKind::Inline {
            tree.nodes[usize::from(id)].intrinsic_min = node.style.intrinsic_min;
            tree.nodes[usize::from(id)].intrinsic_max = node.style.intrinsic_max;
        }
        tree.nodes[usize::from(id)].intrinsic_adjustment = intrinsic_adjustment;
        tree.nodes[usize::from(id)].visibility_collapsed = node.style.collapsed;
        tree.nodes[usize::from(id)].format.inline_level = node.style.inline;
        tree.nodes[usize::from(id)].edges =
            std::array::from_fn(|side| crate::formatting::CollapsedBorder {
                width: if matches!(
                    node.style.border_styles[side],
                    crate::layout_style::BorderStyle::None
                        | crate::layout_style::BorderStyle::Hidden
                ) {
                    0.0
                } else {
                    node.style.border_widths[side]
                },
                color: node.style.border_colors[side].unwrap_or(node.color),
                style: node.style.border_styles[side],
                source_side: side,
                stripes: node.style.border_stripes[side].as_ref().map(|stripes| {
                    Arc::from(
                        stripes
                            .iter()
                            .map(|stripe| crate::formatting::CollapsedBorderStripe {
                                color: stripe.color.unwrap_or(node.color),
                                thickness: match stripe.thickness {
                                    crate::box_decoration::ClipPart::Length(value) => {
                                        crate::formatting::CollapsedStripeThickness::Length(
                                            value.resolve(node.style.border_widths[side]),
                                        )
                                    }
                                    crate::box_decoration::ClipPart::Flex(value) => {
                                        crate::formatting::CollapsedStripeThickness::Flex(value)
                                    }
                                },
                            })
                            .collect::<Vec<_>>(),
                    )
                }),
                draw: true,
                priority: match node.style.format.display {
                    crate::formatting::DisplayKind::Table => 0,
                    crate::formatting::DisplayKind::RowGroup
                    | crate::formatting::DisplayKind::Header
                    | crate::formatting::DisplayKind::Footer => 3,
                    crate::formatting::DisplayKind::ColumnGroup => 1,
                    crate::formatting::DisplayKind::Column => 2,
                    crate::formatting::DisplayKind::Row => 4,
                    _ => 5,
                },
            });

        let interactive = node.paint.as_ref().is_some_and(|paint| {
            paint.response.sense.senses_click() || paint.response.sense.senses_drag()
        });
        tree.nodes[usize::from(id)].atomic = interactive
            || node.style.inline
                && node.style.format.display != crate::formatting::DisplayKind::Inline;
        tree.nodes[usize::from(id)].fragmentable_text = node.text.is_some()
            && node.paint.as_ref().is_some_and(|paint| {
                context.graphics(|layers| {
                    let Some(list) = layers.get(paint.ui.layer_id()) else {
                        return false;
                    };
                    let mut text_shapes = 0;
                    let only_text = list
                        .all_entries()
                        .skip(paint.start.0)
                        .take(paint.end.0 - paint.start.0)
                        .all(|entry| collect_fragmentable_text(&entry.shape, &mut text_shapes));
                    only_text
                        && (text_shapes == 1
                            || node.style.format.flow_into_content && text_shapes > 0)
                })
            });
        tree.nodes[usize::from(id)].flow_region = node.style.format.flow_from.is_some()
            && !node.style.contents
            && tree.nodes[usize::from(id)].style.display != Display::None
            && matches!(
                tree.nodes[usize::from(id)].style.display,
                Display::Block | Display::FlowRoot
            )
            && node.style.format.display != crate::formatting::DisplayKind::Inline
            && !node.style.format.writing.vertical();
        tree.nodes[usize::from(id)].flow_region_auto_height = tree.nodes[usize::from(id)]
            .flow_region
            && tree.nodes[usize::from(id)].style.size.height.is_auto();
        let flow_name = if let Some(name) = node.style.format.flow_into.as_ref() {
            (!node.style.format.flow_into_content
                || node.children.is_empty() && node.text.is_some())
            .then(|| name.clone())
        } else {
            node.parent.and_then(|parent| {
                let parent_format = &frame.nodes[parent].style.format;
                (parent_format.flow_into_content)
                    .then(|| parent_format.flow_into.clone())
                    .flatten()
            })
        };
        tree.nodes[usize::from(id)].flow_name = flow_name.clone();
        tree.nodes[usize::from(id)].flow_element = flow_name.is_some()
            && tree.nodes[usize::from(id)].style.display != Display::None
            && !node.style.layout.position.is_out_of_flow()
            && !node.style.format.writing.vertical();
        tree.nodes[usize::from(id)].flow_source = tree.nodes[usize::from(id)].flow_element
            && node.children.is_empty()
            && tree.nodes[usize::from(id)].fragmentable_text;
        let mut ancestor = node.parent;
        while let Some(parent) = ancestor {
            if tree.nodes[usize::from(ids[parent])].flow_region {
                tree.nodes[usize::from(id)].flow_region_content =
                    !tree.nodes[usize::from(id)].flow_element;
                break;
            }
            ancestor = frame.nodes[parent].parent;
        }
        if let Some((runs, minimum)) = node
            .text
            .as_ref()
            .map(|text| text_runs(context, &text.job, &node.style.format))
        {
            frame.nodes[index].text.as_mut().unwrap().minimum = minimum;
            if let Some((_, _, snapshot_minimum)) = &mut measurements[index].1 {
                *snapshot_minimum = minimum;
            }
            tree.nodes[usize::from(id)].runs = runs;
        }
        frame.nodes[index].flow_source = tree.nodes[usize::from(id)].flow_element;
        ids.push(id);
    }
    let mut children = vec![Vec::new(); frame.nodes.len()];
    let mut root_children = Vec::new();
    let mut fixed = Vec::new();
    let mut layout_parents = vec![None; frame.nodes.len()];
    for (i, node) in frame.nodes.iter().enumerate() {
        if node.style.contents
            || tree.nodes[usize::from(ids[i])].flow_element
            || tree.nodes[usize::from(ids[i])].flow_region_content
        {
            if tree.nodes[usize::from(ids[i])].flow_element {
                let mut parent = node.parent;
                while let Some(index) = parent {
                    if frame.nodes[index].style.contents {
                        parent = frame.nodes[index].parent;
                    } else {
                        tree.nodes[usize::from(ids[i])].parent = Some(ids[index]);
                        break;
                    }
                }
            }
            continue;
        }
        if node.style.fixed {
            if let Some(containing) = fixed_containing_block(frame, node.parent) {
                children[containing].push(ids[i]);
            } else {
                fixed.push(i);
            }
            continue;
        }
        let mut parent = node.parent;
        while let Some(p) = parent {
            if frame.nodes[p].style.contents {
                parent = frame.nodes[p].parent;
            } else {
                break;
            }
        }
        let mut containing = parent;
        if node.style.layout.position == Position::Absolute {
            while let Some(p) = containing {
                if frame.nodes[p].style.positioned {
                    break;
                }
                containing = frame.nodes[p].parent;
            }
        }
        layout_parents[i] = containing;
        if let Some(p) = parent {
            children[p].push(ids[i]);
        } else {
            root_children.push(ids[i]);
        }
    }
    for (i, children) in children.iter_mut().enumerate() {
        if matches!(
            frame.nodes[i].style.layout.display,
            Display::Flex | Display::Grid
        ) {
            children.sort_by_key(|id| frame.nodes[usize::from(*id)].style.order);
        }
        tree.set_children_owned(ids[i], std::mem::take(children))
            .unwrap();
    }
    let holder = Style {
        display: Display::Block,
        size: Size {
            width: length(frame.available.x),
            height: length(frame.available.y),
        },
        ..Style::default()
    };
    let root = tree.new_with_children_owned(holder, root_children).unwrap();
    let measure =
        |input: taffy::LayoutInput, _: NodeId, index: Option<&mut usize>, style: &Style| {
            let measurement = index.map(|index| &measurements[*index]);
            let measured = measurement.map_or(Vec2::ZERO, |measurement| measurement.0);
            taffy::compute_leaf_layout(input, style, crate::math::resolve, |known, available| {
                let measured = if let Some((job, extra, minimum)) =
                    measurement.and_then(|measurement| measurement.1.as_ref())
                {
                    let mut job = job.clone();
                    job.wrap.max_width = known
                        .width
                        .map(|w| (w - extra.x).max(0.0))
                        .unwrap_or_else(|| match available.width {
                            AvailableSpace::Definite(width) => (width - extra.x).max(0.0),
                            AvailableSpace::MinContent => *minimum,
                            AvailableSpace::MaxContent => f32::INFINITY,
                        });
                    context.fonts_mut(|fonts| fonts.layout_job(job).size()) + *extra
                } else {
                    measured
                };
                Size {
                    width: known.width.unwrap_or(measured.x),
                    height: known.height.unwrap_or(measured.y),
                }
            })
        };
    tree.compute_layout_with_measure_and_regions(
        root,
        Size {
            width: AvailableSpace::Definite(frame.available.x),
            height: AvailableSpace::Definite(frame.available.y),
        },
        measure,
    )
    .unwrap();
    let fixed_root = if !fixed.is_empty() {
        let viewport = tree
            .new_with_children_owned(
                Style {
                    display: Display::Block,
                    size: Size {
                        width: length(frame.viewport.width()),
                        height: length(frame.viewport.height()),
                    },
                    ..Style::default()
                },
                fixed.iter().map(|i| ids[*i]).collect(),
            )
            .unwrap();
        tree.compute_layout_with_measure_and_regions(
            viewport,
            Size {
                width: AvailableSpace::Definite(frame.viewport.width()),
                height: AvailableSpace::Definite(frame.viewport.height()),
            },
            measure,
        )
        .unwrap();
        Some(viewport)
    } else {
        None
    };
    let mut collapsed_flex = false;
    for (index, node) in frame.nodes.iter().enumerate() {
        if !node.style.collapsed
            || !node
                .parent
                .is_some_and(|parent| frame.nodes[parent].style.layout.display == Display::Flex)
        {
            continue;
        }
        let id = ids[index];
        let layout = tree.nodes[usize::from(id)].layout;
        let mut style = tree.nodes[usize::from(id)].style.clone();
        collapse_flex_item(frame, index, layout, &mut style);
        tree.nodes[usize::from(id)].style = style;
        collapsed_flex = true;
    }
    if collapsed_flex {
        for index in 0..tree.nodes.len() {
            taffy::CacheTree::cache_clear(&mut tree, NodeId::from(index));
        }
        tree.compute_layout_with_measure_and_regions(
            root,
            Size {
                width: AvailableSpace::Definite(frame.available.x),
                height: AvailableSpace::Definite(frame.available.y),
            },
            measure,
        )
        .unwrap();
        if let Some(viewport) = fixed_root {
            tree.compute_layout_with_measure_and_regions(
                viewport,
                Size {
                    width: AvailableSpace::Definite(frame.viewport.width()),
                    height: AvailableSpace::Definite(frame.viewport.height()),
                },
                measure,
            )
            .unwrap();
        }
    }
    for (index, id) in ids.iter().enumerate() {
        frame.nodes[index].flow_clip =
            tree.nodes[usize::from(*id)]
                .flow_clip
                .map(|(clip, region_index)| {
                    let region = &frame.nodes[region_index];
                    let offset = if viewport_fixed(frame, region_index) {
                        frame.viewport.min.to_vec2()
                    } else {
                        frame.origin.to_vec2() - region.ancestor_scroll
                    };
                    clip.translate(offset + region.sticky)
                });
    }
    let absolute_layouts = tree.absolute_layouts();
    for (i, node) in frame.nodes.iter().enumerate() {
        let layout = absolute_layouts[usize::from(ids[i])].expect("layout was resolved");
        let offset = if viewport_fixed(frame, i) {
            frame.viewport.min - frame.origin
        } else {
            Vec2::ZERO
        };
        let origin = pos2(layout.location.x, layout.location.y) + offset;
        cache.boxes.insert(
            node.id,
            Geometry {
                border: egui::Rect::from_min_size(
                    origin,
                    vec2(layout.size.width, layout.size.height),
                ),
                padding: margin(layout.padding),
                stroke: margin(layout.border),
                gutter: scrollbar_gutter(&node.style),
                margin: margin(layout.margin),
                extent: vec2(
                    layout.scrollable_overflow_rect.right,
                    layout.scrollable_overflow_rect.bottom,
                ),
                fragments: tree.nodes[usize::from(ids[i])]
                    .fragments
                    .iter()
                    .map(|fragment| crate::layout_tree::Fragment {
                        range: fragment.range.clone(),
                        marker: fragment.marker.clone(),
                        marker_width: fragment.marker_width,
                        rect: tree.nodes[usize::from(ids[i])]
                            .layout_axes
                            .rect(fragment.rect, layout.size.width),
                        text_rect: tree.nodes[usize::from(ids[i])]
                            .layout_axes
                            .rect(fragment.text_rect, layout.size.width),
                        clip: fragment.clip.map(|clip| {
                            tree.nodes[usize::from(ids[i])]
                                .layout_axes
                                .rect(clip, layout.size.width)
                        }),
                    })
                    .collect(),
                flow_source: tree.nodes[usize::from(ids[i])].flow_source,
                flow_region_content: tree.nodes[usize::from(ids[i])].flow_region_content
                    || tree.nodes[usize::from(ids[i])].flow_overset,
                slices: tree.nodes[usize::from(ids[i])]
                    .slices
                    .iter()
                    .map(|slice| crate::layout_tree::Slice {
                        source: tree.nodes[usize::from(ids[i])]
                            .layout_axes
                            .rect(slice.source, layout.size.width),
                        offset: tree.nodes[usize::from(ids[i])]
                            .layout_axes
                            .vector(slice.offset),
                    })
                    .collect(),
                rules: tree.nodes[usize::from(ids[i])]
                    .rules
                    .iter()
                    .map(|rect| node.style.format.writing.rect(*rect, layout.size.width))
                    .collect(),
                text_vertical: node.text.is_some() && node.style.format.writing.vertical(),
                collapsed: tree.nodes[usize::from(ids[i])].collapsed.clone().map(|e| {
                    let mode = tree.nodes[usize::from(ids[i])].layout_axes;
                    if !mode.vertical() {
                        e
                    } else if mode.reverse() {
                        [e[3].clone(), e[0].clone(), e[1].clone(), e[2].clone()]
                    } else {
                        [e[3].clone(), e[2].clone(), e[1].clone(), e[0].clone()]
                    }
                }),
                collapsed_segments: tree.nodes[usize::from(ids[i])]
                    .collapsed_segments
                    .iter()
                    .map(|segment| {
                        let mode = node.style.format.writing;
                        let edges = if !mode.vertical() {
                            segment.edges.clone()
                        } else if mode.reverse() {
                            [
                                segment.edges[3].clone(),
                                segment.edges[0].clone(),
                                segment.edges[1].clone(),
                                segment.edges[2].clone(),
                            ]
                        } else {
                            [
                                segment.edges[3].clone(),
                                segment.edges[2].clone(),
                                segment.edges[1].clone(),
                                segment.edges[0].clone(),
                            ]
                        };
                        let rect = mode
                            .rect(segment.rect, layout.size.width)
                            .translate(origin.to_vec2());
                        crate::formatting::CollapsedBorderSegment { rect, edges }
                    })
                    .collect(),
            },
        );
    }
    cache
}
