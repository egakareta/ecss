use crate::{
    formatting::DisplayKind,
    layout_tree::{Fragment, Tree},
};

#[derive(Clone)]
pub(crate) struct Table {
    rows: Vec<(Option<NodeId>, Vec<NodeId>)>,
    groups: Vec<(NodeId, std::ops::Range<usize>)>,
    captions: Vec<NodeId>,
    columns: Vec<Dimension>,
    column_nodes: Vec<NodeId>,
    column_groups: Vec<(NodeId, std::ops::Range<usize>)>,
    collapsed_columns: Vec<bool>,
}
use egui::{Rect, Vec2, pos2, vec2};
use taffy::{
    Layout, LayoutInput, LayoutOutput, LayoutPartialTree, RunMode, compute_leaf_layout, prelude::*,
};

pub(crate) fn raw(value: taffy::CompactLength, basis: f32) -> Option<f32> {
    match value.tag() {
        taffy::CompactLength::LENGTH_TAG => Some(value.value()),
        taffy::CompactLength::PERCENT_TAG => Some(value.value() * basis),
        taffy::CompactLength::CALC_TAG => Some(crate::math::resolve(value.calc_value(), basis)),
        _ => None,
    }
}

impl Tree<'_> {
    pub(crate) fn prepare_inline(&mut self, root: NodeId) {
        let children = self.nodes[usize::from(root)].children.clone();
        for child in &children {
            self.prepare_inline(*child);
        }
        let node = &self.nodes[usize::from(root)];
        if !matches!(node.style.display, Display::Block | Display::FlowRoot)
            || !matches!(
                node.format.display,
                DisplayKind::Normal | DisplayKind::Cell | DisplayKind::Caption
            )
            || node.inline_context
        {
            return;
        }
        let inline = |id: &NodeId| {
            let n = &self.nodes[usize::from(*id)];
            n.format.inline_level
                || n.style.position.is_out_of_flow()
                || n.style.display == Display::None
        };
        if !children
            .iter()
            .any(|id| self.nodes[usize::from(*id)].format.inline_level)
        {
            return;
        }
        if children.iter().all(inline) {
            self.nodes[usize::from(root)].inline_context = true;
            return;
        }
        let mut groups = Vec::new();
        let mut pending = Vec::new();
        for child in children {
            if inline(&child) {
                pending.push(child);
            } else {
                if !pending.is_empty() {
                    groups.push(std::mem::take(&mut pending));
                }
                groups.push(vec![child]);
            }
        }
        if !pending.is_empty() {
            groups.push(pending);
        }
        let mut output = Vec::new();
        for group in groups {
            if group.len() == 1 && !self.nodes[usize::from(group[0])].format.inline_level {
                output.push(group[0]);
                continue;
            }
            let wrapper = self
                .new_with_children(
                    Style {
                        display: Display::Block,
                        ..Style::default()
                    },
                    &group,
                )
                .unwrap();
            self.nodes[usize::from(wrapper)].inline_context = true;
            self.nodes[usize::from(wrapper)].format = self.nodes[usize::from(root)].format.clone();
            output.push(wrapper);
        }
        self.set_children(root, &output).unwrap();
    }

    pub(crate) fn compute_format(
        &mut self,
        id: NodeId,
        input: LayoutInput,
        block: Option<&mut taffy::BlockContext<'_>>,
    ) -> Option<LayoutOutput> {
        if let Some((source, axis)) = self.nodes[usize::from(id)].proxy {
            let style = self.nodes[usize::from(id)].style.clone();
            return Some(compute_leaf_layout(
                input,
                &style,
                crate::math::resolve,
                |known, available| {
                    let output = self.compute_child_layout(
                        source,
                        LayoutInput {
                            run_mode: RunMode::ComputeSize,
                            known_dimensions: Size::NONE,
                            available_space: available,
                            ..input
                        },
                    );
                    Size {
                        width: known.width.unwrap_or(if axis == 0 {
                            output.size.width
                        } else {
                            0.0
                        }),
                        height: known.height.unwrap_or(if axis == 1 {
                            output.size.height
                        } else {
                            0.0
                        }),
                    }
                },
            ));
        }
        if self.nodes[usize::from(id)].style.display == Display::Grid
            && self.nodes[usize::from(id)].children.iter().any(|child| {
                self.nodes[usize::from(*child)]
                    .format
                    .subgrid
                    .iter()
                    .any(|value| *value)
            })
        {
            return Some(self.compute_subgrid(id, input));
        }
        if self.nodes[usize::from(id)].format.display == DisplayKind::Table {
            return Some(self.compute_table(id, input));
        }
        if self.nodes[usize::from(id)].format.column_count.is_some()
            || self.nodes[usize::from(id)].format.column_width.is_some()
        {
            return Some(self.compute_columns(id, input));
        }
        if self.nodes[usize::from(id)].inline_context {
            return Some(self.compute_inline(id, input, block));
        }
        None
    }

    fn compute_subgrid(&mut self, id: NodeId, input: LayoutInput) -> LayoutOutput {
        use taffy::{CacheTree, DetailedLayoutInfo, compute_grid_layout, style_helpers::*};
        let index = usize::from(id);
        let children = self.nodes[index].children.clone();
        for child in children {
            for axis in 0..2 {
                let node = &self.nodes[usize::from(child)];
                if !node.format.subgrid[axis] {
                    continue;
                }
                let current = if axis == 0 {
                    &node.style.grid_template_columns
                } else {
                    &node.style.grid_template_rows
                };
                if !current.is_empty() {
                    continue;
                }
                let parent = if axis == 0 {
                    &self.nodes[index].style.grid_template_columns
                } else {
                    &self.nodes[index].style.grid_template_rows
                };
                let count = parent
                    .iter()
                    .map(|track| match track {
                        GridTemplateComponent::Single(_) => 1,
                        GridTemplateComponent::Repeat(repeat) => match repeat.count {
                            taffy::RepetitionCount::Count(n) => n as usize * repeat.tracks.len(),
                            _ => repeat.tracks.len(),
                        },
                    })
                    .sum::<usize>()
                    .max(1);
                let placement = if axis == 0 {
                    &node.style.grid_column
                } else {
                    &node.style.grid_row
                };
                let line_number = |line: &GridPlacement<String>| match line {
                    GridPlacement::Line(n) => {
                        let n = n.as_i16();
                        Some(if n < 0 {
                            count as i32 + 2 + n as i32
                        } else {
                            n as i32
                        })
                    }
                    _ => None,
                };
                let span = match (&placement.start, &placement.end) {
                    (GridPlacement::Span(n), _) | (_, GridPlacement::Span(n)) => *n as usize,
                    _ => line_number(&placement.start)
                        .zip(line_number(&placement.end))
                        .map_or(count, |(a, b)| (b - a).unsigned_abs() as usize),
                }
                .max(1);
                let node = &mut self.nodes[usize::from(child)];
                if axis == 0 {
                    node.style.grid_template_columns = vec![auto(); span];
                } else {
                    node.style.grid_template_rows = vec![auto(); span];
                }
            }
        }
        self.refresh_children(id);
        let mut output = compute_grid_layout(self, id, input);
        if input.run_mode != RunMode::PerformLayout {
            return output;
        }
        for _ in 0..8 {
            let DetailedLayoutInfo::Grid(info) = &self.nodes[index].details else {
                break;
            };
            let info = info.as_ref().clone();
            let children: Vec<_> = self.nodes[index]
                .children
                .iter()
                .copied()
                .filter(|child| {
                    let n = &self.nodes[usize::from(*child)];
                    n.style.display != Display::None && !n.style.position.is_out_of_flow()
                })
                .collect();
            let mut changed = false;
            let mut proxies = Vec::new();
            for (item, child) in children.iter().enumerate() {
                let Some(placement) = info.items.get(item) else {
                    continue;
                };
                let child_index = usize::from(*child);
                let subgrid = self.nodes[child_index].format.subgrid;
                if !subgrid.iter().any(|value| *value) {
                    continue;
                }
                for (axis, enabled) in subgrid.into_iter().enumerate() {
                    if !enabled {
                        continue;
                    }
                    let tracks = if axis == 0 { &info.columns } else { &info.rows };
                    let start = if axis == 0 {
                        placement.column_start
                    } else {
                        placement.row_start
                    } as usize
                        - 1;
                    let end = if axis == 0 {
                        placement.column_end
                    } else {
                        placement.row_end
                    } as usize
                        - 1;
                    let Some(parent_tracks) = tracks.positions.get(start..end) else {
                        continue;
                    };
                    if parent_tracks.is_empty() {
                        continue;
                    }
                    let parent_gap = tracks
                        .positions
                        .get(start + 1)
                        .map_or(0.0, |next| next.start - parent_tracks[0].end);
                    let child_gap = if self.nodes[child_index].format.normal_gaps[axis] {
                        parent_gap
                    } else {
                        let gap = if axis == 0 {
                            self.nodes[child_index].style.gap.width
                        } else {
                            self.nodes[child_index].style.gap.height
                        };
                        raw(gap.into_raw(), output.size.width).unwrap_or(0.0)
                    };
                    let style = &self.nodes[child_index].style;
                    let (first, last) = if axis == 0 {
                        (style.padding.left, style.padding.right)
                    } else {
                        (style.padding.top, style.padding.bottom)
                    };
                    let (border_first, border_last) = if axis == 0 {
                        (style.border.left, style.border.right)
                    } else {
                        (style.border.top, style.border.bottom)
                    };
                    let first = raw(first.into_raw(), output.size.width).unwrap_or(0.0)
                        + raw(border_first.into_raw(), output.size.width).unwrap_or(0.0);
                    let last = raw(last.into_raw(), output.size.width).unwrap_or(0.0)
                        + raw(border_last.into_raw(), output.size.width).unwrap_or(0.0);
                    let definitions: Vec<GridTemplateComponent<String>> = parent_tracks
                        .iter()
                        .enumerate()
                        .map(|(i, track)| {
                            length(
                                (track.end - track.start
                                    + if i > 0 {
                                        (parent_gap - child_gap) * 0.5
                                    } else {
                                        -first
                                    }
                                    + if i + 1 < parent_tracks.len() {
                                        (parent_gap - child_gap) * 0.5
                                    } else {
                                        -last
                                    })
                                .max(0.0),
                            )
                        })
                        .collect();
                    let parent_names = if axis == 0 {
                        &self.nodes[index].style.grid_template_column_names
                    } else {
                        &self.nodes[index].style.grid_template_row_names
                    };
                    let mut names: Vec<Vec<String>> = (start..=end)
                        .map(|line| {
                            line.checked_sub(tracks.negative_implicit_tracks as usize)
                                .and_then(|line| parent_names.get(line))
                                .cloned()
                                .unwrap_or_default()
                        })
                        .collect();
                    for (line, own) in names
                        .iter_mut()
                        .zip(&self.nodes[child_index].format.subgrid_names[axis])
                    {
                        line.extend(own.iter().cloned());
                    }
                    let child = &mut self.nodes[child_index];
                    let old = if axis == 0 {
                        &child.style.grid_template_columns
                    } else {
                        &child.style.grid_template_rows
                    };
                    if *old != definitions {
                        changed = true;
                    }
                    if axis == 0 {
                        child.style.grid_template_columns = definitions;
                        child.style.grid_template_column_names = names;
                        child.style.gap.width = length(child_gap);
                    } else {
                        child.style.grid_template_rows = definitions;
                        child.style.grid_template_row_names = names;
                        child.style.gap.height = length(child_gap);
                    }
                    if !self.nodes[index].subgrid_proxies {
                        let nested = self.nodes[child_index].children.clone();
                        let child_details = match &self.nodes[child_index].details {
                            DetailedLayoutInfo::Grid(grid) => Some(grid.as_ref().clone()),
                            _ => None,
                        };
                        for (nested_index, source) in nested.into_iter().enumerate() {
                            let Some(area) = child_details
                                .as_ref()
                                .and_then(|grid| grid.items.get(nested_index))
                            else {
                                continue;
                            };
                            let mut style = Style {
                                display: Display::Block,
                                ..Style::default()
                            };
                            let column_start = if axis == 0 {
                                placement.column_start + area.column_start - 1
                            } else {
                                placement.column_start
                            };
                            let column_end = if axis == 0 {
                                placement.column_start + area.column_end - 1
                            } else {
                                placement.column_end
                            };
                            let row_start = if axis == 1 {
                                placement.row_start + area.row_start - 1
                            } else {
                                placement.row_start
                            };
                            let row_end = if axis == 1 {
                                placement.row_start + area.row_end - 1
                            } else {
                                placement.row_end
                            };
                            style.grid_column = taffy::Line {
                                start: line(column_start as i16),
                                end: line(column_end as i16),
                            };
                            style.grid_row = taffy::Line {
                                start: line(row_start as i16),
                                end: line(row_end as i16),
                            };
                            let proxy = self.new_with_children(style, &[]).unwrap();
                            self.nodes[usize::from(proxy)].proxy = Some((source, axis));
                            self.nodes[usize::from(proxy)].format.writing =
                                self.nodes[index].format.writing;
                            proxies.push(proxy);
                        }
                    }
                }
            }
            if !self.nodes[index].subgrid_proxies {
                for (item, child) in children.iter().enumerate() {
                    if let Some(area) = info.items.get(item) {
                        let style = &mut self.nodes[usize::from(*child)].style;
                        style.grid_column = taffy::Line {
                            start: line(area.column_start as i16),
                            end: line(area.column_end as i16),
                        };
                        style.grid_row = taffy::Line {
                            start: line(area.row_start as i16),
                            end: line(area.row_end as i16),
                        };
                    }
                }
                let mut all = self.nodes[index].children.clone();
                all.extend(proxies.iter().copied());
                self.set_children(id, &all).unwrap();
                self.nodes[index].subgrid_proxies = true;
                changed |= !proxies.is_empty();
            }
            if !changed {
                break;
            }
            let mut stack = self.nodes[index].children.clone();
            while let Some(child) = stack.pop() {
                self.cache_clear(child);
                stack.extend(self.nodes[usize::from(child)].children.clone());
            }
            self.refresh_children(id);
            output = compute_grid_layout(self, id, input);
        }
        output
    }

    fn compute_columns(&mut self, id: NodeId, input: LayoutInput) -> LayoutOutput {
        let index = usize::from(id);
        let style = self.nodes[index].style.clone();
        let format = self.nodes[index].format.clone();
        let basis = input.parent_size.width.unwrap_or(0.0);
        let padding = style
            .padding
            .map(|v| raw(v.into_raw(), basis).unwrap_or(0.0));
        let border = style
            .border
            .map(|v| raw(v.into_raw(), basis).unwrap_or(0.0));
        let inset = vec2(padding.left + border.left, padding.top + border.top);
        compute_leaf_layout(input, &style, crate::math::resolve, |known, available| {
            let width = known.width.unwrap_or(match available.width {
                AvailableSpace::Definite(v) => v,
                _ => format.column_width.unwrap_or(0.0) * format.column_count.unwrap_or(1) as f32,
            });
            let gap = if format.normal_gaps[0] {
                format.em
            } else {
                raw(style.gap.width.into_raw(), width).unwrap_or(0.0)
            };
            let max_count = format
                .column_width
                .filter(|v| *v > 0.0)
                .map(|column| ((width + gap) / (column + gap)).floor().max(1.0) as usize);
            let count = format
                .column_count
                .map_or(max_count.unwrap_or(1), |n| {
                    max_count.map_or(n, |max| n.min(max))
                })
                .max(1);
            let column_width = ((width - gap * (count - 1) as f32) / count as f32).max(0.0);
            if self.nodes[index].column_regions.is_some()
                || self.nodes[index]
                    .children
                    .iter()
                    .any(|child| self.nodes[usize::from(*child)].format.column_span)
            {
                return self.column_spanners(id, input, width, inset);
            }
            let flow = if let Some(flow) = self.nodes[index].column_flow {
                flow
            } else {
                let children = self.nodes[index].children.clone();
                let flow = self
                    .new_with_children(
                        Style {
                            display: Display::FlowRoot,
                            ..Style::default()
                        },
                        &children,
                    )
                    .unwrap();
                self.nodes[usize::from(flow)].inline_context = self.nodes[index].inline_context;
                self.nodes[usize::from(flow)].format = self.nodes[index].format.clone();
                self.nodes[usize::from(flow)].format.column_count = None;
                self.nodes[usize::from(flow)].format.column_width = None;
                self.nodes[index].column_flow = Some(flow);
                flow
            };
            let mut stack = self.nodes[usize::from(flow)].children.clone();
            while let Some(node) = stack.pop() {
                self.nodes[usize::from(node)].layout_parent = None;
                taffy::CacheTree::cache_clear(self, node);
                stack.extend(self.nodes[usize::from(node)].children.clone());
            }
            taffy::CacheTree::cache_clear(self, flow);
            let flow_input = LayoutInput {
                run_mode: RunMode::PerformLayout,
                known_dimensions: Size {
                    width: Some(column_width),
                    height: None,
                },
                parent_size: Size {
                    width: Some(column_width),
                    height: known.height,
                },
                available_space: Size {
                    width: AvailableSpace::Definite(column_width),
                    height: AvailableSpace::MaxContent,
                },
                vertical_margins_are_collapsible: taffy::Line::FALSE,
                ..input
            };
            let output = self.compute_child_layout(flow, flow_input);
            self.nodes[usize::from(flow)].layout = Layout {
                size: output.size,
                ..Layout::with_order(0)
            };
            let total = output.size.height;
            let mut candidates = vec![0.0, total];
            let mut forced = Vec::new();
            let mut avoid = Vec::new();
            let mut nodes = Vec::new();
            let mut stack = self.nodes[usize::from(flow)].children.clone();
            while let Some(node_id) = stack.pop() {
                let node = &self.nodes[usize::from(node_id)];
                if node.style.position.is_out_of_flow() || node.style.display == Display::None {
                    continue;
                }
                let layout = self.relative_layout(node_id, flow);
                let top = layout.location.y;
                let bottom = top + layout.size.height;
                nodes.push((node_id, layout));
                candidates.push(top);
                candidates.push(bottom);
                if matches!(
                    node.format.before,
                    crate::formatting::Break::Column | crate::formatting::Break::Always
                ) {
                    forced.push(top);
                }
                if matches!(
                    node.format.after,
                    crate::formatting::Break::Column | crate::formatting::Break::Always
                ) {
                    forced.push(bottom);
                }
                if node.format.inside == crate::formatting::Break::Avoid
                    || node.atomic
                    || node.style.overflow.x.is_scroll_container()
                    || node.style.overflow.y.is_scroll_container()
                {
                    avoid.push((top, bottom));
                    continue;
                }
                if let Some(run) = node.runs.first().filter(|run| run.height > 0.0) {
                    let lines = (layout.size.height / run.height).round() as usize;
                    if lines < node.format.orphans + node.format.widows {
                        avoid.push((top, bottom));
                    } else {
                        avoid.push((top, top + node.format.orphans as f32 * run.height));
                        avoid.push((bottom - node.format.widows as f32 * run.height, bottom));
                    }
                    for line in node.format.orphans..=lines.saturating_sub(node.format.widows) {
                        if line > 0 {
                            candidates.push(top + line as f32 * run.height);
                        }
                    }
                }
                stack.extend(node.children.iter().copied());
            }
            candidates.retain(|point| {
                !avoid
                    .iter()
                    .any(|(a, b)| *point > *a + 0.01 && *point < *b - 0.01)
            });
            candidates.sort_by(f32::total_cmp);
            candidates.dedup_by(|a, b| (*a - *b).abs() < 0.01);
            forced.sort_by(f32::total_cmp);
            let unbreakable = avoid.iter().map(|(a, b)| b - a).fold(0.0, f32::max);
            let mut height = known
                .height
                .unwrap_or((total / count as f32).max(unbreakable));
            if format.column_balance {
                height = height
                    .min((total / count as f32).max(unbreakable))
                    .max(0.001);
            }
            if height <= 0.0 {
                height = total.max(0.001);
            }
            let plan = |height: f32| {
                let mut starts = vec![0.0];
                let mut cursor = 0.0;
                while cursor < total - 0.01 {
                    let target = cursor + height;
                    let forced = forced
                        .iter()
                        .copied()
                        .find(|point| *point > cursor + 0.01 && *point <= target + 0.01);
                    let next = forced
                        .unwrap_or_else(|| {
                            candidates
                                .iter()
                                .copied()
                                .rfind(|point| *point > cursor + 0.01 && *point <= target + 0.01)
                                .unwrap_or_else(|| {
                                    avoid
                                        .iter()
                                        .find(|(a, b)| *a <= cursor + 0.01 && *b > target)
                                        .map_or(target.min(total), |(_, b)| *b)
                                })
                        })
                        .min(total);
                    if next <= cursor {
                        break;
                    }
                    starts.push(next);
                    cursor = next;
                }
                starts
            };
            if format.column_balance || known.height.is_none() {
                let mut low = height;
                let mut high = known.height.unwrap_or(total).max(low);
                if plan(low).len() > count + 1 {
                    for _ in 0..24 {
                        let middle = (low + high) * 0.5;
                        if plan(middle).len() > count + 1 {
                            low = middle;
                        } else {
                            high = middle;
                        }
                    }
                    height = high;
                }
            }
            let starts = plan(height);
            if input.run_mode == RunMode::PerformLayout {
                self.nodes[index].rules.clear();
                let rtl = style.direction == taffy::Direction::Rtl;
                for column in 0..starts.len().saturating_sub(1) {
                    if column > 0 {
                        self.nodes[index].rules.push(Rect::from_min_size(
                            pos2(
                                inset.x + column as f32 * (column_width + gap) - gap * 0.5,
                                inset.y,
                            ),
                            vec2(0.0, height),
                        ));
                    }
                }
                for (node_id, layout) in nodes {
                    let source = Rect::from_min_size(
                        pos2(layout.location.x, layout.location.y),
                        vec2(layout.size.width, layout.size.height),
                    );
                    let first = starts
                        .windows(2)
                        .position(|range| source.min.y < range[1] - 0.001)
                        .unwrap_or(0);
                    let column_x = |column: usize| {
                        if rtl {
                            (count as f32 - column as f32 - 1.0) * (column_width + gap)
                        } else {
                            column as f32 * (column_width + gap)
                        }
                    };
                    let primary = pos2(
                        inset.x + source.min.x + column_x(first),
                        inset.y + source.min.y - starts[first],
                    );
                    let node = &mut self.nodes[usize::from(node_id)];
                    node.layout_axes = self.axes.last().copied().unwrap_or_default();
                    node.layout.location = taffy::Point {
                        x: primary.x,
                        y: primary.y,
                    };
                    node.layout_parent = Some(id);
                    node.slices.clear();
                    for (column, range) in starts.windows(2).enumerate() {
                        let top = source.min.y.max(range[0]);
                        let bottom = source.max.y.min(range[1]);
                        if bottom <= top {
                            continue;
                        }
                        let local = Rect::from_min_max(
                            pos2(0.0, top - source.min.y),
                            pos2(source.width(), bottom - source.min.y),
                        );
                        let destination = pos2(
                            inset.x + source.min.x + column_x(column),
                            inset.y + top - range[0],
                        );
                        node.slices.push(crate::layout_tree::Slice {
                            source: local,
                            offset: destination - primary - local.min.to_vec2(),
                        });
                    }
                }
            }
            Size {
                width,
                height: known.height.unwrap_or(height),
            }
        })
    }

    fn column_spanners(
        &mut self,
        id: NodeId,
        input: LayoutInput,
        width: f32,
        inset: Vec2,
    ) -> Size<f32> {
        let index = usize::from(id);
        if self.nodes[index].column_regions.is_none() {
            let children = self.nodes[index].children.clone();
            let format = self.nodes[index].format.clone();
            let style = self.nodes[index].style.clone();
            let mut regions = Vec::new();
            let mut pending = Vec::new();
            for child in children {
                if self.nodes[usize::from(child)].format.column_span {
                    if !pending.is_empty() {
                        let mut format = format.clone();
                        format.column_balance = true;
                        let mut style = style.clone();
                        style.size = Size::auto();
                        style.padding = taffy::Rect::zero();
                        style.border = taffy::Rect::zero();
                        style.margin = taffy::Rect::zero();
                        regions.push(self.new_format_node(
                            style,
                            format,
                            &std::mem::take(&mut pending),
                        ));
                    }
                    regions.push(child);
                } else {
                    pending.push(child);
                }
            }
            if !pending.is_empty() {
                let mut style = style;
                style.size = Size::auto();
                style.padding = taffy::Rect::zero();
                style.border = taffy::Rect::zero();
                style.margin = taffy::Rect::zero();
                regions.push(self.new_format_node(style, format, &pending));
            }
            self.set_children(id, &regions).unwrap();
            self.nodes[index].column_regions = Some(regions);
        }
        let regions = self.nodes[index].column_regions.clone().unwrap();
        let mut y = inset.y;
        self.nodes[index].rules.clear();
        for region in regions {
            self.prepare_inline(region);
            let output = self.compute_child_layout(
                region,
                LayoutInput {
                    known_dimensions: Size {
                        width: Some(width),
                        height: None,
                    },
                    parent_size: Size {
                        width: Some(width),
                        height: None,
                    },
                    available_space: Size {
                        width: AvailableSpace::Definite(width),
                        height: AvailableSpace::MaxContent,
                    },
                    vertical_margins_are_collapsible: taffy::Line::FALSE,
                    ..input
                },
            );
            if input.run_mode == RunMode::PerformLayout {
                self.place(
                    region,
                    Layout {
                        location: taffy::Point { x: inset.x, y },
                        size: output.size,
                        ..Layout::with_order(0)
                    },
                );
                let rules = self.nodes[usize::from(region)].rules.clone();
                for rule in rules {
                    self.nodes[index]
                        .rules
                        .push(rule.translate(vec2(inset.x, y)));
                }
            }
            y += output.size.height;
        }
        Size {
            width,
            height: y - inset.y,
        }
    }

    fn table_model(&mut self, id: NodeId) -> Table {
        let mut table = Table {
            rows: Vec::new(),
            groups: Vec::new(),
            captions: Vec::new(),
            columns: Vec::new(),
            column_nodes: Vec::new(),
            column_groups: Vec::new(),
            collapsed_columns: Vec::new(),
        };
        let mut children = self.nodes[usize::from(id)].children.clone();
        children.sort_by_key(|id| match self.nodes[usize::from(*id)].format.display {
            DisplayKind::Header => 0,
            DisplayKind::Footer => 2,
            _ => 1,
        });
        let mut pending = Vec::new();
        for child in children {
            let node = &self.nodes[usize::from(child)];
            match node.format.display {
                DisplayKind::Row => {
                    if !pending.is_empty() {
                        table.rows.push((None, std::mem::take(&mut pending)));
                    }
                    table.rows.push((Some(child), node.children.clone()));
                }
                DisplayKind::RowGroup | DisplayKind::Header | DisplayKind::Footer => {
                    if !pending.is_empty() {
                        table.rows.push((None, std::mem::take(&mut pending)));
                    }
                    let start = table.rows.len();
                    for row in node.children.clone() {
                        let node = &self.nodes[usize::from(row)];
                        if node.format.display == DisplayKind::Row {
                            table.rows.push((Some(row), node.children.clone()));
                        } else {
                            table.rows.push((None, vec![row]));
                        }
                    }
                    table.groups.push((child, start..table.rows.len()));
                }
                DisplayKind::Caption => table.captions.push(child),
                DisplayKind::Column => {
                    table.columns.push(node.style.size.width);
                    table.column_nodes.push(child);
                    table.collapsed_columns.push(node.visibility_collapsed);
                }
                DisplayKind::ColumnGroup => {
                    let start = table.column_nodes.len();
                    for column_id in &node.children {
                        let column = &self.nodes[usize::from(*column_id)];
                        table.columns.push(column.style.size.width);
                        table.column_nodes.push(*column_id);
                        table
                            .collapsed_columns
                            .push(node.visibility_collapsed || column.visibility_collapsed);
                    }
                    if start < table.column_nodes.len() {
                        table
                            .column_groups
                            .push((child, start..table.column_nodes.len()));
                    }
                }
                _ => pending.push(child),
            }
        }
        if !pending.is_empty() {
            table.rows.push((None, pending));
        }
        table
    }

    fn compute_table(&mut self, id: NodeId, input: LayoutInput) -> LayoutOutput {
        use taffy::{compute_grid_layout, style_helpers::*};
        let index = usize::from(id);
        let model = if let Some(model) = &self.nodes[index].table {
            model.clone()
        } else {
            let model = self.table_model(id);
            self.nodes[index].table = Some(model.clone());
            model
        };
        self.nodes[index].collapsed_segments.clear();
        let missing_borders = if self.nodes[index].format.collapse {
            self.collapse_table(id, &model)
        } else {
            Vec::new()
        };
        let original = self.nodes[index].style.clone();
        let format = self.nodes[index].format.clone();
        let columns = model
            .rows
            .iter()
            .map(|(_, cells)| cells.len())
            .max()
            .unwrap_or(1)
            .max(model.columns.len())
            .max(1);
        let mut style = original.clone();
        style.display = Display::Grid;
        style.grid_template_rows.clear();
        style.gap = if format.collapse {
            Size::zero()
        } else {
            Size {
                width: length(format.spacing.width),
                height: length(format.spacing.height),
            }
        };
        let basis = input.parent_size.width.unwrap_or(0.0);
        if !format.collapse {
            style.padding = taffy::Rect {
                left: length(
                    raw(style.padding.left.into_raw(), basis).unwrap_or(0.0) + format.spacing.width,
                ),
                right: length(
                    raw(style.padding.right.into_raw(), basis).unwrap_or(0.0)
                        + format.spacing.width,
                ),
                top: length(
                    raw(style.padding.top.into_raw(), basis).unwrap_or(0.0) + format.spacing.height,
                ),
                bottom: length(
                    raw(style.padding.bottom.into_raw(), basis).unwrap_or(0.0)
                        + format.spacing.height,
                ),
            };
        }
        let table_width = input
            .known_dimensions
            .width
            .or_else(|| raw(style.size.width.into_raw(), basis));
        style.grid_template_columns = (0..columns)
            .map(|column| {
                let collapsed = model
                    .collapsed_columns
                    .get(column)
                    .copied()
                    .unwrap_or(false);
                let declared = model.columns.get(column).copied().or_else(|| {
                    model
                        .rows
                        .first()
                        .and_then(|(_, cells)| cells.get(column))
                        .map(|cell| self.nodes[usize::from(*cell)].style.size.width)
                });
                if format.table_fixed
                    && let Some(width) = table_width
                {
                    let specified: Vec<_> = (0..columns)
                        .map(|c| {
                            model
                                .columns
                                .get(c)
                                .copied()
                                .or_else(|| {
                                    model
                                        .rows
                                        .first()
                                        .and_then(|(_, cells)| cells.get(c))
                                        .map(|cell| self.nodes[usize::from(*cell)].style.size.width)
                                })
                                .and_then(|d| raw(d.into_raw(), width))
                                .map(|value| {
                                    if model.collapsed_columns.get(c).copied().unwrap_or(false) {
                                        0.0
                                    } else {
                                        value
                                    }
                                })
                        })
                        .collect();
                    let extra = style
                        .padding
                        .map(|v| raw(v.into_raw(), width).unwrap_or(0.0))
                        .sum_axes()
                        .width
                        + style
                            .border
                            .map(|v| raw(v.into_raw(), width).unwrap_or(0.0))
                            .sum_axes()
                            .width
                        + if format.collapse {
                            0.0
                        } else {
                            format.spacing.width * (columns - 1) as f32
                        };
                    let remaining =
                        (width - extra - specified.iter().flatten().sum::<f32>()).max(0.0);
                    return length(specified[column].unwrap_or(
                        remaining / specified.iter().filter(|v| v.is_none()).count().max(1) as f32,
                    ));
                }
                if collapsed {
                    return minmax(length(0.0), length(0.0));
                }
                if let Some(size) =
                    declared.and_then(|d| raw(d.into_raw(), table_width.unwrap_or(basis)))
                {
                    minmax(length(size), auto())
                } else {
                    auto()
                }
            })
            .collect();
        let top = model
            .captions
            .iter()
            .filter(|id| !self.nodes[usize::from(**id)].format.caption_bottom)
            .count();
        let mut all = Vec::new();
        for (row, (row_id, cells)) in model.rows.iter().enumerate() {
            let collapsed = row_id
                .is_some_and(|row| self.nodes[usize::from(row)].visibility_collapsed)
                || model.groups.iter().any(|(group, rows)| {
                    rows.contains(&row) && self.nodes[usize::from(*group)].visibility_collapsed
                })
                || row_id.is_none()
                    && !cells.is_empty()
                    && cells
                        .iter()
                        .all(|cell| self.nodes[usize::from(*cell)].visibility_collapsed);
            for (column, cell) in cells.iter().enumerate() {
                let node = &mut self.nodes[usize::from(*cell)];
                node.style.grid_row = line(row as i16 + top as i16 + 1);
                node.style.grid_column = line(column as i16 + 1);
                node.style.margin = taffy::Rect::zero();
                if collapsed {
                    node.style.size.height = length(0.0);
                    node.style.min_size.height = length(0.0);
                    node.style.max_size.height = length(0.0);
                }
                if model
                    .collapsed_columns
                    .get(column)
                    .copied()
                    .unwrap_or(false)
                {
                    node.style.size.width = length(0.0);
                    node.style.min_size.width = length(0.0);
                    node.style.max_size.width = length(0.0);
                }
                node.style.align_content = match node.format.vertical_align.as_str() {
                    "middle" => taffy::AlignContent::CENTER,
                    "bottom" => taffy::AlignContent::END,
                    _ => taffy::AlignContent::START,
                };
                all.push(*cell);
            }
            let height = row_id.and_then(|row| {
                raw(
                    self.nodes[usize::from(row)].style.size.height.into_raw(),
                    input.parent_size.height.unwrap_or(0.0),
                )
            });
            style.grid_template_rows.push(if collapsed {
                minmax(length(0.0), length(0.0))
            } else {
                height.map_or_else(auto, |v| minmax(length(v), auto()))
            });
        }
        if top > 0 {
            style
                .grid_template_rows
                .splice(0..0, std::iter::repeat_n(auto(), top));
        }
        let mut first = 0;
        let mut last = top + model.rows.len();
        for caption in &model.captions {
            let node = &mut self.nodes[usize::from(*caption)];
            let row = if node.format.caption_bottom {
                let row = last;
                last += 1;
                style.grid_template_rows.push(auto());
                row
            } else {
                let row = first;
                first += 1;
                row
            };
            node.style.grid_row = line(row as i16 + 1);
            node.style.grid_column = taffy::Line {
                start: line(1),
                end: line(columns as i16 + 1),
            };
            all.push(*caption);
        }
        self.nodes[index].style = style;
        self.set_children(id, &all).unwrap();
        self.refresh_children(id);
        let output = compute_grid_layout(self, id, input);
        self.nodes[index].style = original.clone();
        if input.run_mode == RunMode::PerformLayout {
            let grid_bounds = model
                .rows
                .iter()
                .flat_map(|(_, cells)| cells)
                .map(|cell| {
                    let layout = self.nodes[usize::from(*cell)].layout;
                    Rect::from_min_size(
                        pos2(layout.location.x, layout.location.y),
                        vec2(layout.size.width, layout.size.height),
                    )
                })
                .fold(Rect::NOTHING, Rect::union);
            let border = original
                .border
                .map(|value| raw(value.into_raw(), output.size.width).unwrap_or(0.0));
            let padding = original
                .padding
                .map(|value| raw(value.into_raw(), output.size.width).unwrap_or(0.0));
            let grid_bounds = if grid_bounds.is_finite() && !format.writing.vertical() {
                Rect::from_min_max(
                    pos2(
                        grid_bounds.left().min(border.left + padding.left),
                        grid_bounds.top(),
                    ),
                    pos2(
                        grid_bounds
                            .right()
                            .max(output.size.width - border.right - padding.right),
                        grid_bounds.bottom(),
                    ),
                )
            } else {
                grid_bounds
            };
            let mut bounds = Vec::new();
            let mut collapsed_segments = Vec::new();
            for (row_index, (row_id, cells)) in model.rows.iter().enumerate() {
                let rect = cells.iter().fold(Rect::NOTHING, |rect, cell| {
                    let l = self.nodes[usize::from(*cell)].layout;
                    rect.union(Rect::from_min_size(
                        pos2(l.location.x, l.location.y),
                        vec2(l.size.width, l.size.height),
                    ))
                });
                let rect = if rect.is_finite() && grid_bounds.is_finite() {
                    Rect::from_min_max(
                        pos2(grid_bounds.left(), rect.top()),
                        pos2(grid_bounds.right(), rect.bottom()),
                    )
                } else {
                    rect
                };
                if let Some((_, edges)) = missing_borders
                    .iter()
                    .find(|(missing_row, _)| *missing_row == row_index)
                    && rect.is_finite()
                {
                    collapsed_segments.push(crate::formatting::CollapsedBorderSegment {
                        rect,
                        edges: edges.clone(),
                    });
                }
                bounds.push(rect);
                if let Some(row) = row_id
                    && rect.is_finite()
                {
                    self.place(*row, rectangle_layout(rect));
                    self.nodes[usize::from(*row)].layout_parent = Some(id);
                }
            }
            for (group, rows) in &model.groups {
                let rect = bounds[rows.clone()]
                    .iter()
                    .copied()
                    .fold(Rect::NOTHING, Rect::union);
                if rect.is_finite() {
                    self.place(*group, rectangle_layout(rect));
                    self.nodes[usize::from(*group)].layout_parent = Some(id);
                }
            }
            self.nodes[index].collapsed_segments = collapsed_segments;
        }
        output
    }

    fn collapse_table(
        &mut self,
        id: NodeId,
        table: &Table,
    ) -> Vec<(usize, [crate::formatting::CollapsedBorder; 4])> {
        use crate::{formatting::CollapsedBorder as Edge, layout_style::BorderStyle};
        fn winner(a: Edge, b: Edge) -> Edge {
            let rank = |edge: &Edge| match edge.style {
                BorderStyle::Hidden => 10,
                BorderStyle::Double => 9,
                BorderStyle::Solid => 8,
                BorderStyle::Dashed => 7,
                BorderStyle::Dotted => 6,
                BorderStyle::Ridge => 5,
                BorderStyle::Outset => 4,
                BorderStyle::Groove => 3,
                BorderStyle::Inset => 2,
                BorderStyle::None => 0,
            };
            let score = |edge: &Edge| {
                (
                    edge.style == BorderStyle::Hidden,
                    edge.style != BorderStyle::None,
                    edge.width,
                    rank(edge),
                    edge.priority,
                )
            };
            if score(&a) >= score(&b) { a } else { b }
        }
        let mut edges: Vec<Vec<[Edge; 4]>> = table
            .rows
            .iter()
            .map(|(_, cells)| {
                cells
                    .iter()
                    .map(|cell| self.nodes[usize::from(*cell)].edges.clone())
                    .collect()
            })
            .collect();
        let rtl = self.nodes[usize::from(id)].style.direction == taffy::Direction::Rtl;
        let columns = table
            .rows
            .iter()
            .map(|(_, cells)| cells.len())
            .max()
            .unwrap_or_default()
            .max(table.column_nodes.len());
        for (row, (row_id, _)) in table.rows.iter().enumerate() {
            for (column, cell_edges) in edges[row].iter_mut().enumerate() {
                for (side, current) in cell_edges.iter_mut().enumerate() {
                    let mut edge = current.clone();
                    let outer = match side {
                        0 => row == 0,
                        1 => {
                            if rtl {
                                column == 0
                            } else {
                                column + 1 == columns
                            }
                        }
                        2 => row + 1 == table.rows.len(),
                        _ => {
                            if rtl {
                                column + 1 == columns
                            } else {
                                column == 0
                            }
                        }
                    };
                    if (side == 0 || side == 2 || outer)
                        && let Some(row) = row_id
                    {
                        edge = winner(edge, self.nodes[usize::from(*row)].edges[side].clone());
                    }
                    for (group, range) in &table.groups {
                        if range.contains(&row)
                            && (side == 0 && row == range.start
                                || side == 2 && row + 1 == range.end
                                || (side == 1 || side == 3) && outer)
                        {
                            edge =
                                winner(edge, self.nodes[usize::from(*group)].edges[side].clone());
                        }
                    }
                    if outer {
                        edge = winner(edge, self.nodes[usize::from(id)].edges[side].clone());
                    }
                    if side == 1 || side == 3 {
                        if let Some(column) = table.column_nodes.get(column) {
                            edge =
                                winner(edge, self.nodes[usize::from(*column)].edges[side].clone());
                        }
                        for (group, range) in &table.column_groups {
                            if range.contains(&column)
                                && if rtl {
                                    side == 3 && column + 1 == range.end
                                        || side == 1 && column == range.start
                                } else {
                                    side == 3 && column == range.start
                                        || side == 1 && column + 1 == range.end
                                }
                            {
                                edge = winner(
                                    edge,
                                    self.nodes[usize::from(*group)].edges[side].clone(),
                                );
                            }
                        }
                    }
                    *current = edge;
                }
            }
        }
        for row in 0..edges.len() {
            for column in 0..edges[row].len() {
                if column + 1 < edges[row].len() {
                    let (first_side, second_side) = if rtl { (3, 1) } else { (1, 3) };
                    let mut edge = winner(
                        edges[row][column][first_side].clone(),
                        edges[row][column + 1][second_side].clone(),
                    );
                    edge.draw = true;
                    edges[row][column][first_side] = edge.clone();
                    edge.draw = false;
                    edges[row][column + 1][second_side] = edge;
                }
                if row + 1 < edges.len() && column < edges[row + 1].len() {
                    let mut edge = winner(
                        edges[row][column][2].clone(),
                        edges[row + 1][column][0].clone(),
                    );
                    edge.draw = true;
                    edges[row][column][2] = edge.clone();
                    edge.draw = false;
                    edges[row + 1][column][0] = edge;
                }
            }
        }
        let mut row_edges = vec![std::array::from_fn(|_| Edge::default()); table.rows.len()];
        for (row_index, (row_id, cells)) in table.rows.iter().enumerate() {
            if cells.len() >= columns {
                continue;
            }
            let side = if rtl { 3 } else { 1 };
            let column = columns - 1;
            let mut edge = Edge::default();
            if let Some(row) = row_id {
                edge = winner(edge, self.nodes[usize::from(*row)].edges[side].clone());
            }
            for (group, range) in &table.groups {
                if range.contains(&row_index) {
                    edge = winner(edge, self.nodes[usize::from(*group)].edges[side].clone());
                }
            }
            if let Some(column_node) = table.column_nodes.get(column) {
                edge = winner(
                    edge,
                    self.nodes[usize::from(*column_node)].edges[side].clone(),
                );
            }
            for (group, range) in &table.column_groups {
                if range.contains(&column)
                    && if rtl {
                        side == 3 && column + 1 == range.end
                    } else {
                        side == 1 && column + 1 == range.end
                    }
                {
                    edge = winner(edge, self.nodes[usize::from(*group)].edges[side].clone());
                }
            }
            edge = winner(edge, self.nodes[usize::from(id)].edges[side].clone());
            if matches!(edge.style, BorderStyle::None | BorderStyle::Hidden) {
                edge.width = 0.0;
                edge.draw = false;
            } else {
                edge.draw = true;
            }
            row_edges[row_index][side] = edge;
        }
        let mut outside = [0.0_f32; 4];
        for (row, (_, cells)) in table.rows.iter().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let mut border = edges[row][column].clone();
                for side in 0..4 {
                    if matches!(border[side].style, BorderStyle::None | BorderStyle::Hidden) {
                        border[side].width = 0.0;
                        border[side].draw = false;
                    }
                    let outer = match side {
                        0 => row == 0,
                        1 => {
                            if rtl {
                                column == 0
                            } else {
                                column + 1 == columns
                            }
                        }
                        2 => row + 1 == table.rows.len(),
                        _ => {
                            if rtl {
                                column + 1 == columns
                            } else {
                                column == 0
                            }
                        }
                    };
                    if outer {
                        border[side].draw = true;
                        outside[side] = outside[side].max(border[side].width);
                    }
                }
                let node = &mut self.nodes[usize::from(*cell)];
                node.style.border = taffy::Rect {
                    top: length(border[0].width * 0.5),
                    right: length(border[1].width * 0.5),
                    bottom: length(border[2].width * 0.5),
                    left: length(border[3].width * 0.5),
                };
                node.collapsed = Some(border);
            }
        }
        for edges in &row_edges {
            for side in 0..4 {
                outside[side] = outside[side].max(edges[side].width);
            }
        }
        self.nodes[usize::from(id)].style.border = taffy::Rect {
            top: length(outside[0] * 0.5),
            right: length(outside[1] * 0.5),
            bottom: length(outside[2] * 0.5),
            left: length(outside[3] * 0.5),
        };
        self.nodes[usize::from(id)].collapsed = Some(std::array::from_fn(|_| Edge::default()));
        for ((row, _), edges) in table.rows.iter().zip(&row_edges) {
            if let Some(row) = row {
                self.nodes[usize::from(*row)].collapsed = Some(edges.clone());
            }
        }
        for (group, _) in &table.groups {
            self.nodes[usize::from(*group)].collapsed =
                Some(std::array::from_fn(|_| Edge::default()));
        }
        table
            .rows
            .iter()
            .enumerate()
            .filter(|(_, (row_id, cells))| row_id.is_none() && cells.len() < columns)
            .map(|(row, _)| (row, row_edges[row].clone()))
            .collect()
    }

    fn compute_inline(
        &mut self,
        id: NodeId,
        input: LayoutInput,
        block: Option<&mut taffy::BlockContext<'_>>,
    ) -> LayoutOutput {
        let style = self.nodes[usize::from(id)].style.clone();
        let basis = input.parent_size.width.unwrap_or(0.0);
        let padding = style
            .padding
            .map(|v| raw(v.into_raw(), basis).unwrap_or(0.0));
        let border = style
            .border
            .map(|v| raw(v.into_raw(), basis).unwrap_or(0.0));
        let inset = vec2(padding.left + border.left, padding.top + border.top);
        let block = block.as_deref();
        compute_leaf_layout(input, &style, crate::math::resolve, |known, available| {
            let width = known.width.unwrap_or(match available.width {
                AvailableSpace::Definite(v) => v,
                AvailableSpace::MinContent => 0.0,
                AvailableSpace::MaxContent => f32::INFINITY,
            });
            self.inline_content(id, input, width, inset, block)
        })
    }

    fn inline_content(
        &mut self,
        id: NodeId,
        input: LayoutInput,
        width: f32,
        inset: Vec2,
        block: Option<&taffy::BlockContext<'_>>,
    ) -> Size<f32> {
        struct Token {
            node: NodeId,
            range: Option<egui::text::ByteRange>,
            width: f32,
            height: f32,
            gap: f32,
            marker_width: f32,
            newline: bool,
            offset: Vec2,
            size: Vec2,
            text_offset: Vec2,
            text_size: Vec2,
            marker: Option<String>,
        }
        let mut tokens = Vec::new();
        let mut stack: Vec<_> = self.nodes[usize::from(id)]
            .children
            .iter()
            .rev()
            .copied()
            .collect();
        let mut wrappers = Vec::new();
        while let Some(child) = stack.pop() {
            let node = &self.nodes[usize::from(child)];
            if node.style.display == Display::None {
                self.compute_child_layout(child, LayoutInput::HIDDEN);
                continue;
            }
            if node.style.position.is_out_of_flow() {
                continue;
            }
            if node.format.display == DisplayKind::Inline && !node.children.is_empty() {
                wrappers.push(child);
                stack.extend(node.children.iter().rev().copied());
                continue;
            }
            if node.format.display == DisplayKind::Inline && !node.runs.is_empty() {
                let padding = node
                    .parent_style
                    .padding
                    .map(|v| raw(v.into_raw(), width).unwrap_or(0.0));
                let border = node
                    .parent_style
                    .border
                    .map(|v| raw(v.into_raw(), width).unwrap_or(0.0));
                let margin = node
                    .parent_style
                    .margin
                    .map(|v| raw(v.into_raw(), width).unwrap_or(0.0));
                let top = padding.top + border.top;
                let bottom = padding.bottom + border.bottom;
                for (index, run) in node.runs.iter().enumerate() {
                    let first = index == 0;
                    let last = index + 1 == node.runs.len();
                    let left = if first {
                        padding.left + border.left
                    } else {
                        0.0
                    };
                    let right = if last {
                        padding.right + border.right
                    } else {
                        0.0
                    };
                    let before = if first { margin.left } else { 0.0 };
                    let after = if last { margin.right } else { 0.0 };
                    tokens.push(Token {
                        node: child,
                        range: Some(run.range.clone()),
                        width: run.width + left + right + before + after,
                        height: run.height,
                        gap: run.gap,
                        marker_width: run.marker_width,
                        newline: run.newline,
                        offset: vec2(before, -top),
                        size: vec2(run.width + left + right, run.height + top + bottom),
                        text_offset: vec2(left, top),
                        text_size: vec2(run.width, run.height),
                        marker: None,
                    });
                }
            } else {
                let output = self.compute_child_layout(
                    child,
                    LayoutInput {
                        known_dimensions: Size::NONE,
                        parent_size: Size {
                            width: Some(width),
                            height: input.parent_size.height,
                        },
                        available_space: Size::MAX_CONTENT,
                        vertical_margins_are_collapsible: taffy::Line::FALSE,
                        ..input
                    },
                );
                let margin = self.nodes[usize::from(child)]
                    .parent_style
                    .margin
                    .map(|value| raw(value.into_raw(), width).unwrap_or(0.0));
                tokens.push(Token {
                    node: child,
                    range: None,
                    width: output.size.width + margin.left + margin.right,
                    height: output.size.height + margin.top + margin.bottom,
                    gap: 0.0,
                    marker_width: 0.0,
                    newline: false,
                    offset: vec2(margin.left, margin.top),
                    size: vec2(output.size.width, output.size.height),
                    text_offset: Vec2::ZERO,
                    text_size: vec2(output.size.width, output.size.height),
                    marker: None,
                });
            }
            self.nodes[usize::from(child)].fragments.clear();
        }
        let format = self.nodes[usize::from(id)].format.clone();
        let nowrap = matches!(format.white_space.as_str(), "nowrap" | "pre");
        struct Line {
            items: Vec<(usize, f32)>,
            x: f32,
            y: f32,
            width: f32,
        }
        let make_line = |y: f32| {
            block.map_or(
                Line {
                    items: Vec::new(),
                    x: 0.0,
                    y,
                    width,
                },
                |block| {
                    let slot = block.find_content_slot(y, taffy::Clear::None, None);
                    Line {
                        items: Vec::new(),
                        x: slot.x,
                        y: slot.y.max(y),
                        width: width.min(slot.width),
                    }
                },
            )
        };
        fn line_height(line: &Line, tokens: &[Token], base: f32) -> f32 {
            line.items
                .iter()
                .map(|(index, _)| tokens[*index].height)
                .fold(base, f32::max)
        }
        let default_line_height = format.line_height.unwrap_or(0.0);
        let mut lines = Vec::new();
        let mut line = make_line(inset.y);
        let mut line_width = 0.0;
        let mut gap = 0.0_f32;
        let mut maximum = 0.0_f32;
        for (index, token) in tokens.iter().enumerate() {
            if token.range.as_ref().is_some_and(|range| range.is_empty())
                && !token.newline
                && token.width == 0.0
            {
                if !line.items.is_empty() {
                    gap = gap.max(token.gap);
                }
                continue;
            }
            if !nowrap
                && !line.items.is_empty()
                && line_width + gap + token.width > line.width + 0.001
            {
                maximum = maximum.max(line_width);
                let next_y = line.y + line_height(&line, &tokens, default_line_height);
                lines.push(line);
                line = make_line(next_y);
                line_width = 0.0;
                gap = 0.0;
            }
            line.items.push((index, line_width + gap));
            line_width += gap + token.width;
            gap = token.gap;
            if token.newline
                && matches!(
                    format.white_space.as_str(),
                    "pre" | "pre-wrap" | "pre-line" | "break-spaces"
                )
            {
                maximum = maximum.max(line_width);
                let next_y = line.y + line_height(&line, &tokens, default_line_height);
                lines.push(line);
                line = make_line(next_y);
                line_width = 0.0;
                gap = 0.0;
            }
        }
        maximum = maximum.max(line_width);
        lines.push(line);
        let rtl = self.nodes[usize::from(id)].style.direction == taffy::Direction::Rtl;
        let last_line = lines.len().saturating_sub(1);
        let mut content_height = 0.0_f32;
        for (line_index, mut line) in lines.into_iter().enumerate() {
            let overflow = self.nodes[usize::from(id)].style.overflow;
            let overflow_inline = if format.writing.vertical() {
                overflow.y != taffy::Overflow::Visible
            } else {
                overflow.x != taffy::Overflow::Visible
            };
            if overflow_inline
                && let Some(marker) = format.text_overflow[1].marker()
                && line.width.is_finite()
                && line
                    .items
                    .last()
                    .is_some_and(|(index, x)| x + tokens[*index].width > line.width + 0.001)
            {
                while let Some((index, x)) = line.items.last().copied() {
                    let token = &mut tokens[index];
                    if token.range.is_some() {
                        let extra = token.width - token.text_size.x;
                        let available = line.width - x - extra;
                        if available >= token.marker_width {
                            let width_delta = available - token.text_size.x;
                            token.text_size.x = available;
                            token.size.x += width_delta;
                            token.width += width_delta;
                            token.marker = Some(marker.to_owned());
                            break;
                        }
                        if line.items.len() == 1 {
                            let width_delta = available.max(0.0) - token.text_size.x;
                            token.text_size.x = available.max(0.0);
                            token.size.x += width_delta;
                            token.width += width_delta;
                            break;
                        }
                    }
                    line.items.pop();
                }
            }
            let height = line_height(&line, &tokens, default_line_height);
            content_height = content_height.max(line.y + height - inset.y);
            let line_end = line.items.last().map_or(0.0, |(i, x)| x + tokens[*i].width);
            let used_width = if line.width.is_finite() {
                line.width
            } else {
                line_end
            };
            let free = (used_width - line_end).max(0.0);
            let align = match (format.text_align.as_str(), rtl) {
                ("center", true) => -free * 0.5,
                ("center", false) => free * 0.5,
                ("right" | "end", false) => free,
                ("left" | "end", true) => -free,
                _ => 0.0,
            };
            let spaces = line
                .items
                .iter()
                .take(line.items.len().saturating_sub(1))
                .filter(|(index, _)| tokens[*index].gap > 0.0)
                .count();
            let justify = if format.text_align == "justify" && line_index != last_line && spaces > 0
            {
                free / spaces as f32
            } else {
                0.0
            };
            let mut added = 0.0;
            for (index, x) in line.items {
                let token = &tokens[index];
                let node = &mut self.nodes[usize::from(token.node)];
                let x = x + added;
                let x = if rtl { used_width - x - token.width } else { x };
                let offset = match node.format.vertical_align.as_str() {
                    "top" | "text-top" => 0.0,
                    "middle" => (height - token.height) * 0.5,
                    _ => height - token.height,
                };
                node.fragments.push(Fragment {
                    range: token.range.clone(),
                    marker: token.marker.clone(),
                    marker_width: token.marker_width,
                    rect: Rect::from_min_size(
                        pos2(
                            inset.x + line.x + x + align + token.offset.x,
                            line.y + offset + token.offset.y,
                        ),
                        token.size,
                    ),
                    text_rect: Rect::from_min_size(
                        pos2(
                            inset.x + line.x + x + align + token.offset.x + token.text_offset.x,
                            line.y + offset + token.offset.y + token.text_offset.y,
                        ),
                        token.text_size,
                    ),
                    clip: None,
                });
                node.layout_parent = Some(id);
                if token.gap > 0.0 {
                    added += justify;
                }
            }
        }
        if input.run_mode == RunMode::PerformLayout {
            let mut nodes: Vec<_> = tokens.iter().map(|token| token.node).collect();
            nodes.dedup();
            for node_id in nodes {
                let node = &mut self.nodes[usize::from(node_id)];
                let bounds = node
                    .fragments
                    .iter()
                    .fold(Rect::NOTHING, |rect, fragment| rect.union(fragment.rect));
                if bounds.is_finite() {
                    node.layout_axes = self.axes.last().copied().unwrap_or_default();
                    let padding = node
                        .parent_style
                        .padding
                        .map(|v| raw(v.into_raw(), width).unwrap_or(0.0));
                    let border = node
                        .parent_style
                        .border
                        .map(|v| raw(v.into_raw(), width).unwrap_or(0.0));
                    let margin = node
                        .parent_style
                        .margin
                        .map(|v| raw(v.into_raw(), width).unwrap_or(0.0));
                    node.layout = Layout {
                        padding,
                        border,
                        margin,
                        location: taffy::Point {
                            x: bounds.min.x,
                            y: bounds.min.y,
                        },
                        size: Size {
                            width: bounds.width(),
                            height: bounds.height(),
                        },
                        ..Layout::with_order(0)
                    };
                    for fragment in &mut node.fragments {
                        fragment.rect = fragment.rect.translate(-bounds.min.to_vec2());
                        fragment.text_rect = fragment.text_rect.translate(-bounds.min.to_vec2());
                    }
                }
            }
            for wrapper in wrappers.into_iter().rev() {
                let children = self.nodes[usize::from(wrapper)].children.clone();
                let fragments: Vec<_> = children
                    .iter()
                    .flat_map(|child| {
                        let node = &self.nodes[usize::from(*child)];
                        node.fragments.iter().map(|fragment| Fragment {
                            range: None,
                            marker: None,
                            marker_width: 0.0,
                            rect: fragment
                                .rect
                                .translate(vec2(node.layout.location.x, node.layout.location.y)),
                            text_rect: fragment
                                .text_rect
                                .translate(vec2(node.layout.location.x, node.layout.location.y)),
                            clip: fragment.clip.map(|clip| {
                                clip.translate(vec2(node.layout.location.x, node.layout.location.y))
                            }),
                        })
                    })
                    .collect();
                let bounds = children
                    .iter()
                    .map(|child| self.nodes[usize::from(*child)].layout)
                    .fold(Rect::NOTHING, |bounds, l| {
                        bounds.union(Rect::from_min_size(
                            pos2(l.location.x, l.location.y),
                            vec2(l.size.width, l.size.height),
                        ))
                    });
                let node = &mut self.nodes[usize::from(wrapper)];
                if bounds.is_finite() {
                    node.layout_axes = self.axes.last().copied().unwrap_or_default();
                    node.layout = Layout {
                        location: taffy::Point {
                            x: bounds.min.x,
                            y: bounds.min.y,
                        },
                        size: Size {
                            width: bounds.width(),
                            height: bounds.height(),
                        },
                        ..Layout::with_order(0)
                    };
                    node.layout_parent = Some(id);
                    node.fragments = fragments
                        .into_iter()
                        .map(|mut fragment| {
                            fragment.rect = fragment.rect.translate(-bounds.min.to_vec2());
                            fragment.text_rect =
                                fragment.text_rect.translate(-bounds.min.to_vec2());
                            fragment
                        })
                        .collect();
                }
            }
        }
        Size {
            width: maximum,
            height: content_height,
        }
    }
}

fn rectangle_layout(rect: Rect) -> Layout {
    Layout {
        location: taffy::Point {
            x: rect.min.x,
            y: rect.min.y,
        },
        size: Size {
            width: rect.width(),
            height: rect.height(),
        },
        ..Layout::with_order(0)
    }
}
