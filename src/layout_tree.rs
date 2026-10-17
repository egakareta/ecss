use std::{cell::RefCell, convert::Infallible};

use taffy::{
    Cache, CacheTree, DetailedGridInfo, DetailedLayoutInfo, Layout, LayoutBlockContainer,
    LayoutContainingBlock, LayoutFlexboxContainer, LayoutGridContainer, LayoutInput, LayoutOutput,
    LayoutPartialTree, NodeId, RunMode, Style, TraversePartialTree, TraverseTree,
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_oof_layout, compute_root_layout,
    prelude::{AvailableSpace, Display, Size},
};

type Measure<'a> = dyn FnMut(LayoutInput, NodeId, Option<&mut usize>, &Style) -> LayoutOutput + 'a;

pub(crate) struct Tree<'a> {
    pub nodes: Vec<Node>,
    measure: Option<RefCell<Box<Measure<'a>>>>,
    resolve: Box<dyn Fn(*const (), f32) -> f32 + 'a>,
    pub axes: Vec<crate::formatting::Writing>,
}

pub(crate) struct Node {
    pub style: Style,
    pub context: Option<usize>,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub layout: Layout,
    pub details: DetailedLayoutInfo,
    pub format: crate::formatting::Formatting,
    pub flow_name: Option<String>,
    pub inline_context: bool,
    pub runs: Vec<TextRun>,
    pub fragments: Vec<Fragment>,
    pub layout_parent: Option<NodeId>,
    pub table: Option<crate::format_layout::Table>,
    pub column_flow: Option<NodeId>,
    pub slices: Vec<Slice>,
    pub rules: Vec<egui::Rect>,
    pub proxy: Option<(NodeId, usize)>,
    pub subgrid_proxies: bool,
    pub column_regions: Option<Vec<NodeId>>,
    pub atomic: bool,
    pub fragmentable_text: bool,
    pub flow_source: bool,
    pub flow_element: bool,
    pub flow_region: bool,
    pub flow_region_auto_height: bool,
    pub flow_region_auto_target: Option<f32>,
    pub flow_region_content: bool,
    pub flow_remainder: bool,
    pub flow_overset: bool,
    pub flow_clip: Option<(egui::Rect, usize)>,
    pub edges: [crate::formatting::CollapsedBorder; 4],
    pub collapsed: Option<[crate::formatting::CollapsedBorder; 4]>,
    pub collapsed_segments: Vec<crate::formatting::CollapsedBorderSegment>,
    pub parent_style: Style,
    pub visibility_collapsed: bool,
    pub intrinsic_min: Size<Option<crate::layout_style::IntrinsicSizing>>,
    pub intrinsic_max: Size<Option<crate::layout_style::IntrinsicSizing>>,
    pub intrinsic_adjustment: Size<f32>,
    pub layout_axes: crate::formatting::Writing,
    axes_prepared: bool,
    cache: Cache,
    hoisted: Vec<NodeId>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Fragment {
    pub range: Option<egui::text::ByteRange>,
    pub marker: Option<String>,
    pub marker_width: f32,
    pub rect: egui::Rect,
    pub text_rect: egui::Rect,
    pub clip: Option<egui::Rect>,
}
#[derive(Clone)]
pub(crate) struct TextRun {
    pub range: egui::text::ByteRange,
    pub width: f32,
    pub height: f32,
    pub gap: f32,
    pub marker_width: f32,
    pub newline: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slice {
    pub source: egui::Rect,
    pub offset: egui::Vec2,
}

impl<'a> Tree<'a> {
    pub fn new(resolve: impl Fn(*const (), f32) -> f32 + 'a) -> Self {
        Self {
            nodes: Vec::new(),
            measure: None,
            resolve: Box::new(resolve),
            axes: Vec::new(),
        }
    }

    pub fn new_leaf_with_context(
        &mut self,
        style: Style,
        context: usize,
    ) -> Result<NodeId, Infallible> {
        let id = self.add(style);
        self.nodes[usize::from(id)].context = Some(context);
        Ok(id)
    }

    fn add(&mut self, style: Style) -> NodeId {
        let id = NodeId::from(self.nodes.len());
        self.nodes.push(Node {
            parent_style: style.clone(),
            visibility_collapsed: false,
            intrinsic_min: Size {
                width: None,
                height: None,
            },
            intrinsic_max: Size {
                width: None,
                height: None,
            },
            intrinsic_adjustment: Size::ZERO,
            layout_axes: Default::default(),
            axes_prepared: false,
            style,
            context: None,
            children: Vec::new(),
            parent: None,
            layout: Layout::with_order(0),
            details: DetailedLayoutInfo::None,
            format: Default::default(),
            flow_name: None,
            inline_context: false,
            runs: Vec::new(),
            fragments: Vec::new(),
            layout_parent: None,
            table: None,
            column_flow: None,
            slices: Vec::new(),
            rules: Vec::new(),
            proxy: None,
            subgrid_proxies: false,
            column_regions: None,
            atomic: false,
            fragmentable_text: false,
            flow_source: false,
            flow_element: false,
            flow_region: false,
            flow_region_auto_height: false,
            flow_region_auto_target: None,
            flow_region_content: false,
            flow_remainder: false,
            flow_overset: false,
            flow_clip: None,
            edges: Default::default(),
            collapsed: None,
            collapsed_segments: Vec::new(),
            cache: Cache::new(),
            hoisted: Vec::new(),
        });
        id
    }

    pub fn new_with_children(
        &mut self,
        style: Style,
        children: &[NodeId],
    ) -> Result<NodeId, Infallible> {
        self.new_with_children_owned(style, children.to_vec())
    }
    pub(crate) fn new_with_children_owned(
        &mut self,
        style: Style,
        children: Vec<NodeId>,
    ) -> Result<NodeId, Infallible> {
        let id = self.add(style);
        self.set_children_owned(id, children)?;
        Ok(id)
    }
    pub(crate) fn new_format_node(
        &mut self,
        style: Style,
        format: crate::formatting::Formatting,
        children: &[NodeId],
    ) -> NodeId {
        let id = self.new_with_children(style, children).unwrap();
        let node = &mut self.nodes[usize::from(id)];
        node.format = format;
        node.axes_prepared = true;
        id
    }

    pub fn set_children(&mut self, id: NodeId, children: &[NodeId]) -> Result<(), Infallible> {
        self.set_children_owned(id, children.to_vec())
    }
    pub(crate) fn set_children_owned(
        &mut self,
        id: NodeId,
        children: Vec<NodeId>,
    ) -> Result<(), Infallible> {
        for child in &children {
            self.nodes[usize::from(*child)].parent = Some(id);
        }
        self.nodes[usize::from(id)].children = children;
        Ok(())
    }

    pub(crate) fn compute_layout_with_measure_and_regions<F>(
        &mut self,
        root: NodeId,
        available: Size<AvailableSpace>,
        measure: F,
    ) -> Result<(), Infallible>
    where
        F: FnMut(LayoutInput, NodeId, Option<&mut usize>, &Style) -> LayoutOutput + 'a,
    {
        for node in &mut self.nodes {
            node.cache.clear();
        }
        self.measure = Some(RefCell::new(Box::new(measure)));
        self.prepare_axes(root);
        self.prepare_inline(root);
        self.resolve_intrinsic_sizes(root, available);
        for index in 0..self.nodes.len() {
            self.nodes[index].cache.clear();
        }
        compute_root_layout(self, root, available);
        for _ in 0..8 {
            if !self.layout_named_regions(root) {
                break;
            }
            for node in &mut self.nodes {
                node.cache.clear();
            }
            compute_root_layout(self, root, available);
        }
        self.measure = None;
        Ok(())
    }

    fn resolve_intrinsic_sizes(&mut self, id: NodeId, available: Size<AvailableSpace>) {
        let index = usize::from(id);
        for child_index in 0..self.nodes[index].children.len() {
            let child = self.nodes[index].children[child_index];
            self.resolve_intrinsic_sizes(child, available);
        }
        let min = self.nodes[index].intrinsic_min;
        let max = self.nodes[index].intrinsic_max;
        if min.width.is_none()
            && min.height.is_none()
            && max.width.is_none()
            && max.height.is_none()
        {
            return;
        }
        let parent_size = available.into_options();
        for axis in 0..2 {
            let sizing = if axis == 0 {
                (min.width, max.width)
            } else {
                (min.height, max.height)
            };
            let compute = |tree: &mut Self, keyword: crate::layout_style::IntrinsicSizing| {
                let constraint = match keyword {
                    crate::layout_style::IntrinsicSizing::MinContent => AvailableSpace::MinContent,
                    crate::layout_style::IntrinsicSizing::MaxContent => AvailableSpace::MaxContent,
                };
                let mut available_space = Size {
                    width: AvailableSpace::MaxContent,
                    height: AvailableSpace::MaxContent,
                };
                if axis == 0 {
                    available_space.width = constraint;
                } else {
                    available_space.height = constraint;
                }
                tree.compute_child_layout(
                    id,
                    LayoutInput {
                        run_mode: RunMode::ComputeSize,
                        sizing_mode: taffy::SizingMode::ContentSize,
                        axis: if axis == 0 {
                            taffy::RequestedAxis::Horizontal
                        } else {
                            taffy::RequestedAxis::Vertical
                        },
                        known_dimensions: Size::NONE,
                        known_dimensions_are_definite: Size {
                            width: false,
                            height: false,
                        },
                        parent_size,
                        available_space,
                        vertical_margins_are_collapsible: taffy::Line::FALSE,
                    },
                )
                .size
            };
            let measured_min = sizing.0.map(|keyword| compute(self, keyword));
            let measured_max = sizing.1.map(|keyword| compute(self, keyword));
            let writing_vertical = self.nodes[index].format.writing.vertical();
            let tree_axis = if writing_vertical { 1 - axis } else { axis };
            let adjustment = if axis == 0 {
                self.nodes[index].intrinsic_adjustment.width
            } else {
                self.nodes[index].intrinsic_adjustment.height
            };
            let constraint = |size: Size<f32>| {
                let value = if axis == 0 { size.width } else { size.height };
                taffy::LengthPercentageAuto::length((value - adjustment).max(0.0))
            };
            let style = &mut self.nodes[index].style;
            match tree_axis {
                0 => {
                    if let Some(size) = measured_min {
                        style.min_size.width = constraint(size);
                    }
                    if let Some(size) = measured_max {
                        style.max_size.width = constraint(size);
                    }
                }
                _ => {
                    if let Some(size) = measured_min {
                        style.min_size.height = constraint(size);
                    }
                    if let Some(size) = measured_max {
                        style.max_size.height = constraint(size);
                    }
                }
            }
        }
        self.nodes[index].intrinsic_min = Size {
            width: None,
            height: None,
        };
        self.nodes[index].intrinsic_max = Size {
            width: None,
            height: None,
        };
    }

    pub(crate) fn compute(
        &mut self,
        id: NodeId,
        input: LayoutInput,
        block: Option<&mut taffy::BlockContext<'_>>,
    ) -> LayoutOutput {
        if input.run_mode == RunMode::PerformHiddenLayout {
            return compute_hidden_layout(self, id);
        }
        let mut block = block;
        compute_cached_layout(self, id, input, |tree, id, input| {
            tree.prepare_axes(id);
            let mode = tree.nodes[usize::from(id)].format.writing;
            let parent_mode = tree.axes.last().copied().unwrap_or_default();
            let transpose = mode.vertical() != parent_mode.vertical();
            let input = if transpose { input.transpose() } else { input };
            tree.axes.push(mode);
            tree.refresh_children(id);
            let display = tree.nodes[usize::from(id)].style.display;
            let children = !tree.nodes[usize::from(id)].children.is_empty();
            let mut output = if display != Display::None
                && let Some(output) = tree.compute_format(id, input, block.as_deref_mut())
            {
                output
            } else {
                match (display, children) {
                    (Display::None, _) => compute_hidden_layout(tree, id),
                    (Display::Block, true) => compute_block_layout(tree, id, input, block),
                    (Display::FlowRoot, true) => compute_block_layout(tree, id, input, None),
                    (Display::Flex, true) => compute_flexbox_layout(tree, id, input),
                    (Display::Grid, true) => {
                        let output = compute_grid_layout(tree, id, input);
                        if input.run_mode == RunMode::PerformLayout {
                            tree.layout_chained_flows(id, input);
                        }
                        output
                    }
                    (_, false) => {
                        let node = &tree.nodes[usize::from(id)];
                        let mut context = node.context;
                        tree.measure.as_ref().unwrap().borrow_mut()(
                            input,
                            id,
                            context.as_mut(),
                            &node.style,
                        )
                    }
                }
            };
            let subgrid = tree.nodes[usize::from(id)].format.subgrid;
            if input.run_mode == RunMode::ComputeSize {
                if subgrid[0]
                    && input.known_dimensions.width.is_none()
                    && !matches!(input.available_space.width, AvailableSpace::Definite(_))
                {
                    output.size.width = 0.0;
                }
                if subgrid[1]
                    && input.known_dimensions.height.is_none()
                    && !matches!(input.available_space.height, AvailableSpace::Definite(_))
                {
                    output.size.height = 0.0;
                }
            }
            if input.run_mode == RunMode::PerformLayout {
                compute_oof_layout(tree, id, &mut output);
            }
            tree.axes.pop();
            if transpose {
                output.transpose()
            } else {
                output
            }
        })
    }

    fn grid_area_rect(&self, id: NodeId, name: &str) -> Option<egui::Rect> {
        let node = &self.nodes[usize::from(id)];
        let area = node
            .style
            .grid_template_areas
            .as_ref()?
            .areas
            .iter()
            .find(|area| area.name == name)?;
        let DetailedLayoutInfo::Grid(info) = &node.details else {
            return None;
        };
        let line = |value: u16| value.to_string().parse().ok();
        let row: taffy::Line<taffy::GridPlacement<String>> = taffy::Line {
            start: line(area.row_start)?,
            end: line(area.row_end)?,
        };
        let column: taffy::Line<taffy::GridPlacement<String>> = taffy::Line {
            start: line(area.column_start)?,
            end: line(area.column_end)?,
        };
        let padding_box = taffy::Rect {
            left: node.layout.border.left + node.layout.padding.left,
            right: node.layout.size.width - node.layout.border.right - node.layout.padding.right,
            top: node.layout.border.top + node.layout.padding.top,
            bottom: node.layout.size.height
                - node.layout.border.bottom
                - node.layout.padding.bottom,
        };
        let rect = info.resolve_absolute_grid_area(row, column, node.style.direction, padding_box);
        Some(egui::Rect::from_min_max(
            egui::pos2(rect.left, rect.top),
            egui::pos2(rect.right, rect.bottom),
        ))
    }

    fn layout_flow_text(
        &mut self,
        parent: NodeId,
        child: NodeId,
        regions: &[egui::Rect],
        clip_regions: &[bool],
        region_used_bottom: &mut [f32],
        last_region_break: bool,
        region_index: &mut usize,
        cursor: &mut f32,
    ) -> bool {
        let index = usize::from(child);
        let node = &self.nodes[index];
        if node.runs.is_empty()
            || !node.fragmentable_text
            || node.format.writing.vertical()
            || !node.children.is_empty()
            || !node.slices.is_empty()
            || matches!(
                node.format.inside,
                crate::formatting::Break::Avoid | crate::formatting::Break::AvoidRegion
            )
            || node.style.overflow.x.is_scroll_container()
            || node.style.overflow.y.is_scroll_container()
        {
            return false;
        }

        let runs = node.runs.clone();
        let style = node.style.clone();
        let format = node.format.clone();
        let initial = node.layout;
        let margin = initial.margin;
        let box_width = initial.size.width;
        let nowrap = matches!(format.white_space.as_str(), "nowrap" | "pre");
        let indent = |width: f32| {
            if format.text_indent.1 {
                format.text_indent.0 * width
            } else {
                format.text_indent.0
            }
        };
        let mut fragments = Vec::new();
        let mut run_index = 0;
        let mut started = false;

        while run_index < runs.len() {
            let region = regions[*region_index];
            let width = if style.size.width.is_auto() {
                (region.width() - margin.left - margin.right).max(0.0)
            } else {
                box_width.min((region.width() - margin.left - margin.right).max(0.0))
            };
            let content_width = (width
                - initial.border.left
                - initial.border.right
                - initial.padding.left
                - initial.padding.right)
                .max(0.0);
            let line_start = run_index;
            let mut line_width = 0.0;
            let mut line_height: f32 = 0.0;
            while run_index < runs.len() {
                let run = &runs[run_index];
                let next_width = if run_index == line_start {
                    run.width
                } else {
                    line_width + runs[run_index - 1].gap + run.width
                };
                if !nowrap && run_index > line_start && next_width > content_width + 0.001 {
                    break;
                }
                line_width = next_width;
                line_height = line_height.max(run.height);
                run_index += 1;
                if run.newline {
                    break;
                }
            }

            if run_index == line_start {
                run_index += 1;
                line_width = runs[line_start].width;
                line_height = runs[line_start].height;
            }

            let top_margin = if started { 0.0 } else { margin.top };
            let line_y = *cursor + top_margin;
            if line_y + line_height > region.bottom() + 0.01 && *region_index + 1 < regions.len() {
                *region_index += 1;
                *cursor = regions[*region_index].top();
                run_index = line_start;
                continue;
            }
            if line_y + line_height > region.bottom() + 0.01
                && *region_index + 1 == regions.len()
                && last_region_break
            {
                break;
            }

            let last_line = run_index >= runs.len() || runs[run_index - 1].newline;
            let line_indent = if format.text_indent_hanging {
                if started { indent(content_width) } else { 0.0 }
            } else if !started || format.text_indent_each_line {
                indent(content_width)
            } else {
                0.0
            };
            let line_align = if last_line && format.text_align_last != "auto" {
                format.text_align_last.as_str()
            } else {
                format.text_align.as_str()
            };
            let free = (content_width - line_width - line_indent).max(0.0);
            let rtl = style.direction == taffy::Direction::Rtl;
            let align = match (line_align, rtl) {
                ("center", _) => free * 0.5,
                ("right" | "end", false) => free,
                ("left" | "end", true) => -free,
                _ => 0.0,
            };
            let align = if line_align == "justify" && !last_line {
                0.0
            } else {
                align
            };
            let justify = matches!(line_align, "justify" | "justify-all")
                && (!last_line || line_align == "justify-all");
            let box_left = region.left() + margin.left;
            let content_left = box_left + initial.border.left + initial.padding.left;
            let text_x = if rtl {
                content_left + content_width - line_width - line_indent + align
            } else {
                content_left + line_indent + align
            };
            let first = &runs[line_start];
            let last = &runs[run_index - 1];
            let rect = egui::Rect::from_min_size(
                egui::pos2(box_left, line_y),
                egui::vec2(width, line_height),
            );
            fragments.push(Fragment {
                range: Some(first.range.start..last.range.end),
                marker: None,
                marker_width: 0.0,
                rect,
                text_rect: egui::Rect::from_min_size(
                    egui::pos2(text_x, line_y + initial.border.top + initial.padding.top),
                    egui::vec2(
                        if justify {
                            content_width - line_indent
                        } else {
                            line_width
                        },
                        line_height,
                    ),
                ),
                clip: clip_regions
                    .get(*region_index)
                    .copied()
                    .unwrap_or(false)
                    .then_some(region),
            });
            region_used_bottom[*region_index] =
                region_used_bottom[*region_index].max(line_y + line_height);
            *cursor = line_y + line_height;
            started = true;
        }

        if fragments.is_empty() && !last_region_break {
            return false;
        }
        if !fragments.is_empty() {
            *cursor += margin.bottom;
            region_used_bottom[*region_index] = region_used_bottom[*region_index].max(*cursor);
        }
        let bounds = fragments
            .iter()
            .fold(egui::Rect::NOTHING, |bounds, fragment| {
                bounds.union(fragment.rect)
            });
        let node = &mut self.nodes[index];
        if bounds.is_finite() {
            node.layout.location = taffy::Point {
                x: bounds.min.x,
                y: bounds.min.y,
            };
            node.layout.size = Size {
                width: bounds.width(),
                height: bounds.height(),
            };
        } else {
            node.layout.location = taffy::Point::ZERO;
            node.layout.size = Size::ZERO;
        }
        for fragment in &mut fragments {
            fragment.rect = fragment.rect.translate(-bounds.min.to_vec2());
            fragment.text_rect = fragment.text_rect.translate(-bounds.min.to_vec2());
            fragment.clip = fragment
                .clip
                .map(|clip| clip.translate(-bounds.min.to_vec2()));
        }
        node.fragments = fragments;
        node.layout_parent = Some(parent);
        node.layout_axes = self.axes.last().copied().unwrap_or_default();
        node.flow_remainder = run_index < runs.len();
        true
    }

    fn suppress_flow_subtree(&mut self, id: NodeId) {
        let index = usize::from(id);
        let children = self.nodes[index].children.clone();
        self.nodes[index].flow_overset = true;
        for child in children {
            self.suppress_flow_subtree(child);
        }
    }

    fn set_flow_clip_subtree(&mut self, id: NodeId, clip: (egui::Rect, usize)) {
        let index = usize::from(id);
        let children = self.nodes[index].children.clone();
        self.nodes[index].flow_clip = Some(clip);
        for child in children {
            self.set_flow_clip_subtree(child, clip);
        }
    }

    fn layout_named_regions(&mut self, root: NodeId) -> bool {
        let mut auto_region_resized = false;
        let mut flow_names = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            if belongs_to_visible_layout_root(&self.nodes, NodeId::from(index), root)
                && node.flow_element
                && let Some(name) = node.flow_name.as_ref()
                && !flow_names.contains(name)
            {
                flow_names.push(name.clone());
            }
        }

        for flow in flow_names {
            let sources: Vec<_> = self
                .nodes
                .iter()
                .enumerate()
                .filter(|(index, node)| {
                    belongs_to_visible_layout_root(&self.nodes, NodeId::from(*index), root)
                        && node.flow_element
                        && node.flow_name.as_deref() == Some(&flow)
                })
                .map(|(index, _)| NodeId::from(index))
                .collect();
            let regions: Vec<_> = self
                .nodes
                .iter()
                .enumerate()
                .filter(|(index, node)| {
                    belongs_to_visible_layout_root(&self.nodes, NodeId::from(*index), root)
                        && node.flow_region
                        && node.format.flow_from.as_deref() == Some(&flow)
                        && !node.format.writing.vertical()
                        && !node.flow_region_content
                        && !sources.contains(&NodeId::from(*index))
                })
                .map(|(index, _)| NodeId::from(index))
                .collect();
            if sources.is_empty() || regions.is_empty() {
                continue;
            }

            let mut active_regions = Vec::with_capacity(regions.len());
            let mut region_rects = Vec::with_capacity(regions.len());
            let mut region_clips = Vec::with_capacity(regions.len());
            let mut auto_region_heights = Vec::with_capacity(regions.len());
            let mut region_used_bottom = Vec::with_capacity(regions.len());
            for region in &regions {
                let node = &self.nodes[usize::from(*region)];
                let layout = self.absolute_layout(*region);
                let left = layout.location.x + layout.border.left + layout.padding.left;
                let top = layout.location.y + layout.border.top + layout.padding.top;
                let right = layout.location.x + layout.size.width
                    - layout.border.right
                    - layout.padding.right;
                let bottom = layout.location.y + layout.size.height
                    - layout.border.bottom
                    - layout.padding.bottom;
                let mut rect = egui::Rect::from_min_max(
                    egui::pos2(left, top),
                    egui::pos2(right.max(left), bottom.max(top)),
                );
                let auto_height = node.flow_region_auto_height;
                if auto_height && node.style.size.height.is_auto() {
                    rect.max.y = f32::MAX / 4.0;
                }
                active_regions.push(*region);
                auto_region_heights.push(auto_height);
                region_used_bottom.push(rect.top());
                region_rects.push(rect);
                region_clips.push(
                    node.style.overflow.x != taffy::Overflow::Visible
                        || node.style.overflow.y != taffy::Overflow::Visible,
                );
            }
            if region_rects.is_empty() {
                continue;
            }
            let last_region_break = self.nodes[usize::from(*active_regions.last().unwrap())]
                .format
                .region_fragment
                == crate::formatting::RegionFragment::Break;
            let mut region_index = 0;
            let mut cursor = region_rects[0].top();
            let mut exhausted = false;

            for source in sources {
                if exhausted {
                    self.suppress_flow_subtree(source);
                    continue;
                }
                let source_index = usize::from(source);
                let before = self.nodes[source_index].format.before;
                if forced_region_break(before)
                    && cursor > region_rects[region_index].top() + 0.01
                    && region_index + 1 < region_rects.len()
                {
                    region_index += 1;
                    cursor = region_rects[region_index].top();
                } else if forced_region_break(before) && last_region_break {
                    exhausted = true;
                    self.suppress_flow_subtree(source);
                    continue;
                }

                loop {
                    let auto_height = auto_region_heights[region_index]
                        && self.nodes[usize::from(active_regions[region_index])]
                            .style
                            .size
                            .height
                            .is_auto();
                    let region = region_rects[region_index];
                    let margin = self.nodes[source_index].layout.margin;
                    let width = (region.width() - margin.left - margin.right).max(0.0);
                    let known_width = self.nodes[source_index]
                        .style
                        .size
                        .width
                        .is_auto()
                        .then_some(width);
                    CacheTree::cache_clear(self, source);
                    let output = self.compute_child_layout(
                        source,
                        LayoutInput {
                            run_mode: RunMode::PerformLayout,
                            sizing_mode: taffy::SizingMode::InherentSize,
                            axis: taffy::RequestedAxis::Both,
                            known_dimensions: Size {
                                width: known_width,
                                height: None,
                            },
                            known_dimensions_are_definite: Size {
                                width: true,
                                height: true,
                            },
                            parent_size: Size {
                                width: Some(region.width()),
                                height: (!auto_region_heights[region_index])
                                    .then_some(region.height()),
                            },
                            available_space: Size {
                                width: AvailableSpace::Definite(width),
                                height: AvailableSpace::MaxContent,
                            },
                            vertical_margins_are_collapsible: taffy::Line::FALSE,
                        },
                    );
                    let margin = self.nodes[source_index].layout.margin;
                    let needed = margin.top + output.size.height + margin.bottom;
                    let avoid_region = matches!(
                        self.nodes[source_index].format.inside,
                        crate::formatting::Break::Avoid | crate::formatting::Break::AvoidRegion
                    );
                    let fits = auto_height || cursor + needed <= region.bottom() + 0.01;
                    let fits_later = region_rects.iter().enumerate().skip(region_index + 1).any(
                        |(index, region)| {
                            let later_is_auto = auto_region_heights[index]
                                && self.nodes[usize::from(active_regions[index])]
                                    .style
                                    .size
                                    .height
                                    .is_auto();
                            later_is_auto || needed <= region.height() + 0.01
                        },
                    );
                    if !fits
                        && region_index + 1 < region_rects.len()
                        && (avoid_region || cursor > region.top() + 0.01 || fits_later)
                    {
                        region_index += 1;
                        cursor = region_rects[region_index].top();
                        continue;
                    }

                    if self.layout_flow_text(
                        root,
                        source,
                        &region_rects,
                        &region_clips,
                        &mut region_used_bottom,
                        last_region_break,
                        &mut region_index,
                        &mut cursor,
                    ) {
                        exhausted |= self.nodes[source_index].flow_remainder;
                        break;
                    }

                    if !fits && last_region_break && region_index + 1 == region_rects.len() {
                        self.suppress_flow_subtree(source);
                        exhausted = true;
                        break;
                    }

                    let flow_clip = region_clips[region_index]
                        .then_some(region)
                        .and_then(|rect| {
                            self.nodes[usize::from(active_regions[region_index])]
                                .context
                                .map(|context| (rect, context))
                        });
                    {
                        let node = &mut self.nodes[source_index];
                        node.layout.location = taffy::Point {
                            x: region.left() + margin.left,
                            y: cursor + margin.top,
                        };
                        node.layout.size = output.size;
                        node.layout_parent = Some(root);
                    }
                    if let Some(flow_clip) = flow_clip {
                        self.set_flow_clip_subtree(source, flow_clip);
                    }
                    cursor += needed;
                    region_used_bottom[region_index] = region_used_bottom[region_index].max(cursor);
                    break;
                }

                if forced_region_break(self.nodes[source_index].format.after)
                    && cursor > region_rects[region_index].top() + 0.01
                    && region_index + 1 < region_rects.len()
                {
                    region_index += 1;
                    cursor = region_rects[region_index].top();
                } else if forced_region_break(self.nodes[source_index].format.after)
                    && last_region_break
                    && cursor > region_rects[region_index].top() + 0.01
                {
                    exhausted = true;
                }
            }

            for index in 0..active_regions.len() {
                if !auto_region_heights[index] {
                    continue;
                }
                let region_index = usize::from(active_regions[index]);
                let node = &mut self.nodes[region_index];
                let layout = node.layout;
                let content_height =
                    (region_used_bottom[index] - region_rects[index].top()).max(0.0);
                let edge_height = layout.border.top
                    + layout.border.bottom
                    + layout.padding.top
                    + layout.padding.bottom;
                let requested_height = if node.style.box_sizing == taffy::BoxSizing::ContentBox {
                    content_height
                } else {
                    content_height + edge_height
                };
                let requested_height =
                    requested_height.max(node.flow_region_auto_target.unwrap_or(0.0));
                if requested_height.is_finite()
                    && requested_height > node.flow_region_auto_target.unwrap_or(0.0) + 0.01
                {
                    node.style.size.height = taffy::Dimension::length(requested_height);
                    node.flow_region_auto_target = Some(requested_height);
                    auto_region_resized = true;
                }
            }
        }
        auto_region_resized
    }

    fn layout_chained_flows(&mut self, id: NodeId, input: LayoutInput) {
        let index = usize::from(id);
        let style = self.nodes[index].style.clone();
        let areas = style
            .grid_template_areas
            .as_ref()
            .map(|areas| areas.areas.clone())
            .unwrap_or_default();
        let chains: Vec<_> = self.nodes[index]
            .format
            .chains
            .iter()
            .filter_map(|chain| {
                let mut chain: Vec<_> = chain
                    .iter()
                    .filter(|name| areas.iter().any(|area| area.name == **name))
                    .cloned()
                    .collect();
                if chain.is_empty()
                    || chain
                        .iter()
                        .take(chain.len() - 1)
                        .any(|name| !grid_area_size_independent(&style, &areas, name))
                {
                    None
                } else {
                    Some(std::mem::take(&mut chain))
                }
            })
            .collect();
        let children = self.nodes[index].children.clone();
        let mut flows = Vec::new();
        for child in &children {
            if let Some(flow) = self.nodes[usize::from(*child)].format.flow.as_ref()
                && !flows.contains(flow)
            {
                flows.push(flow.clone());
            }
        }
        for flow in flows {
            let chain = chains
                .iter()
                .find(|chain| chain.first() == Some(&flow))
                .cloned()
                .unwrap_or_else(|| vec![flow.clone()]);
            let mut items: Vec<_> = children
                .iter()
                .copied()
                .filter(|child| {
                    let node = &self.nodes[usize::from(*child)];
                    node.format.flow.as_deref() == Some(&flow)
                        && node.style.display != Display::None
                        && !node.style.position.is_out_of_flow()
                })
                .collect();
            if items.is_empty() {
                continue;
            }
            items.sort_by_key(|child| usize::from(*child));
            let regions: Vec<_> = chain
                .iter()
                .filter_map(|name| self.grid_area_rect(id, name))
                .collect();
            if regions.is_empty() {
                continue;
            }
            if items.len() == 1 {
                let child = items[0];
                let child_index = usize::from(child);
                if regions.len() > 1 {
                    let mut region_index = 0;
                    let mut cursor = regions[0].top();
                    if self.layout_flow_text(
                        id,
                        child,
                        &regions,
                        &[],
                        &mut vec![regions[0].top(); regions.len()],
                        false,
                        &mut region_index,
                        &mut cursor,
                    ) {
                        continue;
                    }
                }
                let initial = self.nodes[child_index].layout;
                let initial_margin = initial.margin;
                if initial_margin.top + initial.size.height + initial_margin.bottom
                    > regions[0].height() + 0.01
                    && regions.len() > 1
                {
                    let mut region_index = 1;
                    loop {
                        let region = regions[region_index];
                        let mut pending = vec![child];
                        while let Some(descendant) = pending.pop() {
                            let descendant_index = usize::from(descendant);
                            self.nodes[descendant_index].layout_parent = None;
                            let descendants = self.nodes[descendant_index].children.clone();
                            CacheTree::cache_clear(self, descendant);
                            pending.extend(descendants);
                        }
                        let margin = self.nodes[child_index].layout.margin;
                        let width = (region.width() - margin.left - margin.right).max(0.0);
                        let known_width = self.nodes[child_index]
                            .style
                            .size
                            .width
                            .is_auto()
                            .then_some(width);
                        let output = self.compute_child_layout(
                            child,
                            LayoutInput {
                                run_mode: RunMode::PerformLayout,
                                known_dimensions: Size {
                                    width: known_width,
                                    height: None,
                                },
                                parent_size: Size {
                                    width: Some(region.width()),
                                    height: Some(region.height()),
                                },
                                available_space: Size {
                                    width: AvailableSpace::Definite(width),
                                    height: AvailableSpace::MaxContent,
                                },
                                vertical_margins_are_collapsible: taffy::Line::FALSE,
                                ..input
                            },
                        );
                        let margin = self.nodes[child_index].layout.margin;
                        let needed = margin.top + output.size.height + margin.bottom;
                        if needed <= region.height() + 0.01 || region_index + 1 == regions.len() {
                            let node = &mut self.nodes[child_index];
                            node.layout.location = taffy::Point {
                                x: region.left() + margin.left,
                                y: region.top() + margin.top,
                            };
                            node.layout.size = output.size;
                            node.layout_parent = Some(id);
                            node.layout_axes = self.axes.last().copied().unwrap_or_default();
                            break;
                        }
                        region_index += 1;
                    }
                }
                continue;
            }

            let mut region_index = 0;
            let mut cursor = regions[0].top();
            for child in items {
                let child_index = usize::from(child);
                if forced_chain_break(self.nodes[child_index].format.before)
                    && cursor > regions[region_index].top() + 0.01
                    && region_index + 1 < regions.len()
                {
                    region_index += 1;
                    cursor = regions[region_index].top();
                }
                if self.layout_flow_text(
                    id,
                    child,
                    &regions,
                    &[],
                    &mut vec![regions[0].top(); regions.len()],
                    false,
                    &mut region_index,
                    &mut cursor,
                ) {
                    if forced_chain_break(self.nodes[child_index].format.after)
                        && region_index + 1 < regions.len()
                    {
                        region_index += 1;
                        cursor = regions[region_index].top();
                    }
                    continue;
                }
                loop {
                    let region = regions[region_index];
                    let margin = self.nodes[child_index].layout.margin;
                    let width = (region.width() - margin.left - margin.right).max(0.0);
                    let known_width = self.nodes[child_index]
                        .style
                        .size
                        .width
                        .is_auto()
                        .then_some(width);
                    let mut pending = vec![child];
                    while let Some(descendant) = pending.pop() {
                        let descendant_index = usize::from(descendant);
                        self.nodes[descendant_index].layout_parent = None;
                        let descendants = self.nodes[descendant_index].children.clone();
                        CacheTree::cache_clear(self, descendant);
                        pending.extend(descendants);
                    }
                    let output = self.compute_child_layout(
                        child,
                        LayoutInput {
                            run_mode: RunMode::PerformLayout,
                            known_dimensions: Size {
                                width: known_width,
                                height: None,
                            },
                            parent_size: Size {
                                width: Some(region.width()),
                                height: Some(region.height()),
                            },
                            available_space: Size {
                                width: AvailableSpace::Definite(width),
                                height: AvailableSpace::MaxContent,
                            },
                            vertical_margins_are_collapsible: taffy::Line::FALSE,
                            ..input
                        },
                    );
                    let needed = margin.top + output.size.height + margin.bottom;
                    if cursor + needed > region.bottom() + 0.01 && region_index + 1 < regions.len()
                    {
                        region_index += 1;
                        cursor = regions[region_index].top();
                        continue;
                    }
                    let node = &mut self.nodes[child_index];
                    node.layout.location = taffy::Point {
                        x: region.left() + margin.left,
                        y: cursor + margin.top,
                    };
                    node.layout.size = output.size;
                    node.layout_parent = Some(id);
                    node.layout_axes = self.axes.last().copied().unwrap_or_default();
                    cursor += needed;
                    if forced_chain_break(self.nodes[child_index].format.after)
                        && region_index + 1 < regions.len()
                    {
                        region_index += 1;
                        cursor = regions[region_index].top();
                    }
                    break;
                }
            }
        }
    }

    pub fn absolute_layout(&self, id: NodeId) -> Layout {
        self.absolute_layout_cached(id, &mut vec![None; self.nodes.len()])
    }

    pub fn absolute_layouts(&self) -> Vec<Option<Layout>> {
        let mut layouts = vec![None; self.nodes.len()];
        for index in 0..self.nodes.len() {
            self.absolute_layout_cached(NodeId::from(index), &mut layouts);
        }
        layouts
    }

    fn absolute_layout_cached(&self, id: NodeId, layouts: &mut [Option<Layout>]) -> Layout {
        let index = usize::from(id);
        if let Some(layout) = layouts[index] {
            return layout;
        }
        let node = &self.nodes[usize::from(id)];
        let mut layout = node.layout;
        let mut parent = node.layout_parent.or(node.parent);
        if node.style.position == taffy::Position::Absolute {
            while let Some(p) = parent {
                if self.nodes[usize::from(p)].style.position.is_positioned() {
                    break;
                }
                parent = self.nodes[usize::from(p)].parent;
            }
        }
        let parent = parent.map(|parent| self.absolute_layout_cached(parent, layouts));
        let width = parent.map_or(layout.size.height, |parent| parent.size.width);
        if node.layout_axes.vertical() {
            let mode = node.layout_axes;
            let location = mode.rect(
                egui::Rect::from_min_size(
                    egui::pos2(layout.location.x, layout.location.y),
                    egui::vec2(layout.size.width, layout.size.height),
                ),
                width,
            );
            layout.location = taffy::Point {
                x: location.min.x,
                y: location.min.y,
            };
            layout.size = Size {
                width: location.width(),
                height: location.height(),
            };
            layout.padding = physical_edges(layout.padding, mode);
            layout.border = physical_edges(layout.border, mode);
            layout.margin = physical_edges(layout.margin, mode);
            layout.scrollable_overflow_rect = layout.scrollable_overflow_rect.transpose();
        }
        if let Some(parent) = parent {
            layout.location.x += parent.location.x;
            layout.location.y += parent.location.y;
        }
        layouts[index] = Some(layout);
        layout
    }

    fn prepare_axes(&mut self, id: NodeId) {
        let index = usize::from(id);
        if !self.nodes[index].axes_prepared {
            let mode = self.nodes[index].format.writing;
            if mode.vertical() {
                self.nodes[index].style = transpose_style(&self.nodes[index].style, mode, false);
                let e = self.nodes[index].edges.clone();
                self.nodes[index].edges = if mode.reverse() {
                    [e[1].clone(), e[2].clone(), e[3].clone(), e[0].clone()]
                } else {
                    [e[3].clone(), e[2].clone(), e[1].clone(), e[0].clone()]
                };
            }
            self.nodes[index].axes_prepared = true;
        }
        for child_index in 0..self.nodes[index].children.len() {
            let child = self.nodes[index].children[child_index];
            self.prepare_axes(child);
        }
    }
    pub(crate) fn refresh_children(&mut self, id: NodeId) {
        let mode = self.nodes[usize::from(id)].format.writing;
        let index = usize::from(id);
        for child_index in 0..self.nodes[index].children.len() {
            let child = self.nodes[index].children[child_index];
            self.prepare_axes(child);
            let node = &mut self.nodes[usize::from(child)];
            let physical = if node.format.writing.vertical() {
                transpose_style(&node.style, node.format.writing, true)
            } else {
                node.style.clone()
            };
            node.parent_style = if mode.vertical() {
                transpose_style(&physical, mode, false)
            } else {
                physical
            };
        }
    }
    pub(crate) fn place(&mut self, id: NodeId, layout: Layout) {
        self.set_unrounded_layout(id, &layout);
    }
    pub(crate) fn relative_layout(&self, id: NodeId, ancestor: NodeId) -> Layout {
        let mut layout = self.absolute_layout(id);
        let parent = self.absolute_layout(ancestor);
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                layout.location.x - parent.location.x,
                layout.location.y - parent.location.y,
            ),
            egui::vec2(layout.size.width, layout.size.height),
        );
        let mode = self.nodes[usize::from(ancestor)].format.writing;
        let rect = if mode.vertical() {
            mode.inverse_rect(rect, parent.size.width)
        } else {
            rect
        };
        layout.location = taffy::Point {
            x: rect.min.x,
            y: rect.min.y,
        };
        layout.size = Size {
            width: rect.width(),
            height: rect.height(),
        };
        layout
    }
}

fn grid_area_size_independent(
    style: &Style,
    areas: &[taffy::GridTemplateArea<String>],
    name: &str,
) -> bool {
    let Some(area) = areas.iter().find(|area| area.name == name) else {
        return false;
    };
    (area.column_start.saturating_sub(1)..area.column_end.saturating_sub(1))
        .all(|column| grid_track_is_size_independent(style, 0, usize::from(column)))
        && (area.row_start.saturating_sub(1)..area.row_end.saturating_sub(1))
            .all(|row| grid_track_is_size_independent(style, 1, usize::from(row)))
}

fn grid_track_is_size_independent(style: &Style, axis: usize, index: usize) -> bool {
    let tracks = if axis == 0 {
        &style.grid_template_columns
    } else {
        &style.grid_template_rows
    };
    let mut position = 0;
    for component in tracks {
        match component {
            taffy::GridTemplateComponent::Single(track) => {
                if position == index {
                    return track_is_size_independent(*track);
                }
                position += 1;
            }
            taffy::GridTemplateComponent::Repeat(repeat) => {
                let repeat_len = repeat.tracks.len();
                if repeat_len == 0 {
                    return false;
                }
                let count = match repeat.count {
                    taffy::RepetitionCount::Count(count) => usize::from(count),
                    taffy::RepetitionCount::AutoFit | taffy::RepetitionCount::AutoFill => {
                        if index >= position && index < position + repeat_len {
                            return track_is_size_independent(repeat.tracks[index - position]);
                        }
                        if index >= position + repeat_len {
                            return false;
                        }
                        continue;
                    }
                };
                let end = position + repeat_len * count;
                if index >= position && index < end {
                    return track_is_size_independent(
                        repeat.tracks[(index - position) % repeat_len],
                    );
                }
                position = end;
            }
        }
    }
    false
}

fn track_is_size_independent(track: taffy::TrackSizingFunction) -> bool {
    use taffy::style::{
        ExpandedMaxTrackSizingFunction as Max, ExpandedMinTrackSizingFunction as Min,
    };

    matches!(
        (
            track.min_sizing_function().expand(),
            track.max_sizing_function().expand()
        ),
        (
            Min::Length(_) | Min::Calc(_),
            Max::Length(_) | Max::Fr(_) | Max::Calc(_)
        )
    )
}

fn forced_chain_break(value: crate::formatting::Break) -> bool {
    matches!(
        value,
        crate::formatting::Break::Column
            | crate::formatting::Break::Page
            | crate::formatting::Break::Region
            | crate::formatting::Break::Always
    )
}

fn forced_region_break(value: crate::formatting::Break) -> bool {
    matches!(
        value,
        crate::formatting::Break::Region | crate::formatting::Break::Always
    )
}

fn belongs_to_visible_layout_root(nodes: &[Node], id: NodeId, root: NodeId) -> bool {
    let mut current = Some(id);
    while let Some(node) = current {
        if nodes[usize::from(node)].style.display == Display::None {
            return false;
        }
        if node == root {
            return true;
        }
        current = nodes[usize::from(node)].parent;
    }
    false
}

impl TraversePartialTree for Tree<'_> {
    type ChildIter<'a>
        = std::iter::Copied<std::slice::Iter<'a, NodeId>>
    where
        Self: 'a;
    fn child_ids(&self, id: NodeId) -> Self::ChildIter<'_> {
        self.nodes[usize::from(id)].children.iter().copied()
    }
    fn child_count(&self, id: NodeId) -> usize {
        self.nodes[usize::from(id)].children.len()
    }
    fn get_child_id(&self, id: NodeId, index: usize) -> NodeId {
        self.nodes[usize::from(id)].children[index]
    }
}
impl TraverseTree for Tree<'_> {}

impl LayoutPartialTree for Tree<'_> {
    type CustomIdent = String;
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_core_container_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].style
    }
    fn set_unrounded_layout(&mut self, id: NodeId, layout: &Layout) {
        self.nodes[usize::from(id)].layout = *layout;
        self.nodes[usize::from(id)].layout_axes = self.axes.last().copied().unwrap_or_default();
    }
    fn resolve_calc_value(&self, value: *const (), basis: f32) -> f32 {
        (self.resolve)(value, basis)
    }
    fn compute_child_layout(&mut self, id: NodeId, input: LayoutInput) -> LayoutOutput {
        self.compute(id, input, None)
    }
}
impl CacheTree for Tree<'_> {
    fn cache_get(&mut self, id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.nodes[usize::from(id)].cache.get(input)
    }
    fn cache_store(&mut self, id: NodeId, input: &LayoutInput, output: LayoutOutput) {
        self.nodes[usize::from(id)].cache.store(input, output);
    }
    fn cache_clear(&mut self, id: NodeId) {
        self.nodes[usize::from(id)].cache.clear();
    }
}
impl LayoutContainingBlock for Tree<'_> {
    type OofItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_oof_item_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].parent_style
    }
    fn clear_hoisted_children(&mut self, id: NodeId) {
        self.nodes[usize::from(id)].hoisted.clear();
    }
    fn add_hoisted_children(&mut self, id: NodeId, children: &[NodeId]) {
        self.nodes[usize::from(id)]
            .hoisted
            .extend_from_slice(children);
    }
    fn get_detailed_layout_info(&self, id: NodeId) -> &DetailedLayoutInfo {
        &self.nodes[usize::from(id)].details
    }
}
impl LayoutFlexboxContainer for Tree<'_> {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_flexbox_container_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].style
    }
    fn get_flexbox_child_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].parent_style
    }
}
impl LayoutGridContainer for Tree<'_> {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_grid_container_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].style
    }
    fn get_grid_child_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].parent_style
    }
    fn set_detailed_grid_info(&mut self, id: NodeId, info: DetailedGridInfo) {
        self.nodes[usize::from(id)].details = DetailedLayoutInfo::Grid(Box::new(info));
    }
}
impl LayoutBlockContainer for Tree<'_> {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;
    fn get_block_container_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].style
    }
    fn get_block_child_style(&self, id: NodeId) -> &Style {
        &self.nodes[usize::from(id)].parent_style
    }
    fn compute_block_child_layout(
        &mut self,
        id: NodeId,
        input: LayoutInput,
        block: Option<&mut taffy::BlockContext<'_>>,
    ) -> LayoutOutput {
        self.compute(id, input, block)
    }
}

fn physical_edges<T: Copy>(
    rect: taffy::Rect<T>,
    mode: crate::formatting::Writing,
) -> taffy::Rect<T> {
    if !mode.vertical() {
        return rect;
    }
    taffy::Rect {
        left: if mode.reverse() {
            rect.bottom
        } else {
            rect.top
        },
        right: if mode.reverse() {
            rect.top
        } else {
            rect.bottom
        },
        top: rect.left,
        bottom: rect.right,
    }
}
fn logical_edges<T: Copy>(
    rect: taffy::Rect<T>,
    mode: crate::formatting::Writing,
) -> taffy::Rect<T> {
    taffy::Rect {
        left: rect.top,
        right: rect.bottom,
        top: if mode.reverse() {
            rect.right
        } else {
            rect.left
        },
        bottom: if mode.reverse() {
            rect.left
        } else {
            rect.right
        },
    }
}
fn transpose_style(style: &Style, mode: crate::formatting::Writing, inverse: bool) -> Style {
    let mut s = style.clone();
    s.size = s.size.transpose();
    s.min_size = s.min_size.transpose();
    s.max_size = s.max_size.transpose();
    s.padding = if inverse {
        physical_edges(s.padding, mode)
    } else {
        logical_edges(s.padding, mode)
    };
    s.margin = if inverse {
        physical_edges(s.margin, mode)
    } else {
        logical_edges(s.margin, mode)
    };
    s.border = if inverse {
        physical_edges(s.border, mode)
    } else {
        logical_edges(s.border, mode)
    };
    s.inset = if inverse {
        physical_edges(s.inset, mode)
    } else {
        logical_edges(s.inset, mode)
    };
    s.overflow = s.overflow.transpose();
    s.aspect_ratio = s.aspect_ratio.map(|ratio| 1.0 / ratio);
    s
}
