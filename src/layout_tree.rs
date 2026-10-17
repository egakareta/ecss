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

    pub fn compute_layout_with_measure<F>(
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
                    (Display::Grid, true) => compute_grid_layout(tree, id, input),
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

    pub fn absolute_layout(&self, id: NodeId) -> Layout {
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
        let parent = parent.map(|parent| self.absolute_layout(parent));
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
