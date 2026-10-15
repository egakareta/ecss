use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use egui::{Color32, Context, Id, Pos2, Rect, Response, Ui, Vec2, Widget, pos2, vec2};
use taffy::{Overflow, prelude::*};

use crate::{
    Stylesheet,
    css::{Declaration, RuleState},
    layout_style::{BorderStyle, BoxStyle, Units},
};

const ACTIVE: &str = "trcss.layout.active";

#[derive(Clone, Debug, PartialEq)]
struct Geometry {
    border: Rect,
    padding: egui::epaint::MarginF32,
    stroke: egui::epaint::MarginF32,
    gutter: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    extent: Vec2,
    fragments: Vec<crate::layout_tree::Fragment>,
    slices: Vec<crate::layout_tree::Slice>,
    rules: Vec<egui::Rect>,
    text_vertical: bool,
    collapsed: Option<[crate::formatting::CollapsedBorder; 4]>,
}

impl Geometry {
    fn content(&self) -> Rect {
        crate::nonnegative_rect(self.border - self.stroke - self.padding)
    }
    fn translated(&self, offset: Vec2) -> Self {
        Self {
            border: self.border.translate(offset),
            ..self.clone()
        }
    }
}

#[derive(Clone, Default)]
struct Cache {
    boxes: HashMap<Id, Geometry>,
    paint_order: Vec<Id>,
}

struct Node {
    id: Id,
    style: BoxStyle,
    children: Vec<usize>,
    parent: Option<usize>,
    measured: Vec2,
    used: Geometry,
    scroll: Vec2,
    ancestor_scroll: Vec2,
    sticky: Vec2,
    text: Option<TextMeasurement>,
    absent: bool,
    paint: Option<PaintRecord>,
    color: Color32,
}

struct PaintRecord {
    ui: Ui,
    response: Response,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    overlay: egui::layers::ShapeIdx,
    scrollbars: Vec<Response>,
    geometry: Geometry,
    clip: Rect,
}

struct TextMeasurement {
    job: egui::text::LayoutJob,
    extra: Vec2,
    minimum: f32,
}

struct Frame {
    nodes: Vec<Node>,
    stack: Vec<usize>,
    cache: Cache,
    origin: Pos2,
    available: Vec2,
    viewport: Rect,
    enabled: bool,
    math: Rc<crate::math::Arena>,
}

#[derive(Clone)]
struct Active(Rc<RefCell<Frame>>, u64);

thread_local! {
    static FRAMES: RefCell<HashMap<u64, Active>> = RefCell::default();
}
static NEXT_FRAME: AtomicU64 = AtomicU64::new(1);

struct Scope {
    context: Context,
    active: Active,
    root: bool,
}
impl Drop for Scope {
    fn drop(&mut self) {
        self.active.0.borrow_mut().stack.pop();
        if self.root {
            FRAMES.with(|frames| frames.borrow_mut().remove(&self.active.1));
            self.context.data_mut(|d| d.remove::<u64>(Id::new(ACTIVE)));
        }
    }
}

pub(crate) fn show(
    widget: impl Widget,
    ui: &mut Ui,
    stylesheet: &Stylesheet,
    class: &str,
) -> Response {
    let id = ui.next_auto_id().with("trcss.layout");
    let active = ui.ctx().data(|d| d.get_temp::<u64>(Id::new(ACTIVE)));
    let active = active.and_then(|key| FRAMES.with(|frames| frames.borrow().get(&key).cloned()));
    let root = active.is_none();
    let active = active.unwrap_or_else(|| {
        let available = ui.available_rect_before_wrap();
        let cache = ui
            .ctx()
            .data(|d| d.get_temp::<Cache>(id))
            .unwrap_or_default();
        let active = Active(
            Rc::new(RefCell::new(Frame {
                nodes: Vec::new(),
                stack: Vec::new(),
                cache,
                origin: available.min,
                available: available.size(),
                viewport: ui.ctx().viewport_rect(),
                enabled: ui.is_enabled(),
                math: Rc::default(),
            })),
            NEXT_FRAME.fetch_add(1, Ordering::Relaxed),
        );
        FRAMES.with(|frames| frames.borrow_mut().insert(active.1, active.clone()));
        ui.ctx()
            .data_mut(|d| d.insert_temp(Id::new(ACTIVE), active.1));
        active
    });
    let state = ui
        .ctx()
        .data(|d| d.get_temp::<u8>(id.with("state")))
        .unwrap_or(0);
    let parent_style = {
        let frame = active.0.borrow();
        frame
            .stack
            .last()
            .map(|parent| frame.nodes[*parent].style.clone())
            .unwrap_or_default()
    };
    let mut style = BoxStyle::default();
    style.layout.direction = parent_style.layout.direction;
    style.hidden = parent_style.hidden;
    style.collapsed = parent_style.collapsed;
    style.format.writing = parent_style.format.writing;
    style.format.text_orientation = parent_style.format.text_orientation.clone();
    style.format.line_height = parent_style.format.line_height;
    style.format.white_space = parent_style.format.white_space.clone();
    style.format.text_align = parent_style.format.text_align.clone();
    style.format.text_overflow = parent_style.format.text_overflow.clone();
    style.format.collapse = parent_style.format.collapse;
    style.format.spacing = parent_style.format.spacing;
    style.format.hide_empty = parent_style.format.hide_empty;
    style.format.caption_bottom = parent_style.format.caption_bottom;
    style.format.widows = parent_style.format.widows;
    style.format.orphans = parent_style.format.orphans;
    let math = active.0.borrow().math.clone();
    let units = Units {
        math: &math,
        font: ui.style().text_styles[&egui::TextStyle::Body].size,
        root_font: 16.0,
        viewport: ui.ctx().viewport_rect().size(),
    };
    style.format.em = units.font;

    for rule in matching_rules(stylesheet, class).filter(|r| {
        r.state == RuleState::Base
            || r.state == RuleState::Hovered && state & 1 != 0
            || r.state == RuleState::Active && state & 2 != 0
    }) {
        for declaration in &rule.declarations {
            if let Declaration::Layout(declaration) = declaration
                && matches!(declaration.name.as_str(), "direction" | "writing-mode")
            {
                declaration
                    .apply_with_parent(&mut style, &parent_style, units)
                    .expect("validated direction");
            }
        }
    }
    for rule in matching_rules(stylesheet, class) {
        if rule.state == RuleState::Base
            || rule.state == RuleState::Hovered && state & 1 != 0
            || rule.state == RuleState::Active && state & 2 != 0
        {
            for declaration in &rule.declarations {
                if let Declaration::Layout(declaration) = declaration {
                    declaration
                        .apply_with_parent(&mut style, &parent_style, units)
                        .expect("validated CSS layout declaration");
                }
            }
        }
    }
    style.finish();
    let scroll = ui
        .ctx()
        .data(|d| d.get_temp::<Vec2>(id.with("scroll")))
        .unwrap_or_default();
    let (index, geometry, absent) = {
        let mut frame = active.0.borrow_mut();
        let index = frame.nodes.len();
        let parent = frame.stack.last().copied();
        let absent =
            style.layout.display == Display::None || parent.is_some_and(|p| frame.nodes[p].absent);
        let ancestor_scroll = if style.fixed {
            Vec2::ZERO
        } else {
            parent.map_or(Vec2::ZERO, |p| {
                frame.nodes[p].ancestor_scroll + frame.nodes[p].scroll - frame.nodes[p].sticky
            })
        };
        let mut geometry = frame
            .cache
            .boxes
            .get(&id)
            .map(|g| g.translated(frame.origin.to_vec2() - ancestor_scroll))
            .unwrap_or_else(|| guess(&style, ui.available_rect_before_wrap()));
        let sticky = sticky_offset(&style, &geometry, parent, &frame);
        geometry = geometry.translated(sticky);
        if let Some(parent) = parent {
            frame.nodes[parent].children.push(index);
        }
        frame.nodes.push(Node {
            id,
            style: style.clone(),
            children: Vec::new(),
            parent,
            measured: Vec2::ZERO,
            used: geometry.clone(),
            scroll,
            ancestor_scroll,
            sticky,
            text: None,
            absent,
            paint: None,
            color: Color32::BLACK,
        });
        frame.stack.push(index);
        (index, geometry, absent)
    };
    let scope = Scope {
        context: ui.ctx().clone(),
        active: active.clone(),
        root,
    };
    let text_vertical = geometry.text_vertical && style.format.writing.vertical();
    let mut widget_rect = geometry.content().translate(-scroll);
    if text_vertical {
        widget_rect = Rect::from_min_size(
            widget_rect.min,
            vec2(widget_rect.height(), widget_rect.width()),
        );
    }
    let builder = egui::UiBuilder::new()
        .id(id)
        .max_rect(widget_rect)
        .layout(egui::Layout::top_down(egui::Align::Min));
    let mut content_ui =
        if parent_style.hidden && !style.hidden && !absent && active.0.borrow().enabled {
            ui.skip_ahead_auto_ids(1);
            let mut host = Ui::new(
                ui.ctx().clone(),
                id.with("visible"),
                egui::UiBuilder::new()
                    .max_rect(geometry.content().translate(-scroll))
                    .style(ui.style().clone())
                    .layer_id(ui.layer_id()),
            );
            host.set_clip_rect(ui.clip_rect());
            host.new_child(builder)
        } else {
            ui.new_child(builder)
        };
    let inherited_visuals = ui.visuals().widgets.clone();
    content_ui.style_mut().visuals.widgets = ui.ctx().global_style().visuals.widgets.clone();
    let widgets = &mut content_ui.style_mut().visuals.widgets;
    widgets.noninteractive.fg_stroke = inherited_visuals.noninteractive.fg_stroke;
    widgets.inactive.fg_stroke = inherited_visuals.inactive.fg_stroke;
    widgets.hovered.fg_stroke = inherited_visuals.hovered.fg_stroke;
    widgets.active.fg_stroke = inherited_visuals.active.fg_stroke;
    widgets.open.fg_stroke = inherited_visuals.open.fg_stroke;
    content_ui.spacing_mut().item_spacing = Vec2::ZERO;
    for rule in matching_rules(stylesheet, class) {
        crate::apply_declarations(&mut content_ui, rule.state, &rule.declarations);
    }
    if state != 0 {
        let effective_state = if state & 2 != 0 {
            RuleState::Active
        } else {
            RuleState::Hovered
        };
        for rule in matching_rules(stylesheet, class) {
            if rule.state == RuleState::Base
                || rule.state == RuleState::Hovered && state & 1 != 0
                || rule.state == RuleState::Active && state & 2 != 0
            {
                crate::apply_declarations(&mut content_ui, effective_state, &rule.declarations);
            }
        }
    }
    if style.padding_set {
        content_ui.spacing_mut().button_padding = Vec2::ZERO;
    }
    if let Some(request) = stylesheet.font_request_for_class(class)
        && let Some(family) = stylesheet.fonts.read().ok().and_then(|fonts| {
            fonts.family(ui.ctx(), &request.families, request.weight, request.italic)
        })
    {
        for font in content_ui.style_mut().text_styles.values_mut() {
            font.family = family.clone();
        }
    }
    if absent || style.hidden {
        content_ui.set_invisible();
    }
    let mut clip = content_ui.clip_rect();
    if !geometry.slices.is_empty() {
        clip = clip.union(geometry.border);
    }

    let padding_box = geometry.border - geometry.stroke;
    let clip_box = match style.overflow_clip_box {
        crate::layout_style::OverflowClipBox::Content => geometry.content(),
        crate::layout_style::OverflowClipBox::Padding => padding_box,
        crate::layout_style::OverflowClipBox::Border => geometry.border,
    };
    match style.layout.overflow.x {
        Overflow::Visible => {}
        Overflow::Clip => {
            clip.min.x = clip
                .min
                .x
                .max(clip_box.min.x - style.overflow_clip_margin[3]);
            clip.max.x = clip
                .max
                .x
                .min(clip_box.max.x + style.overflow_clip_margin[1]);
        }
        _ => {
            clip.min.x = clip.min.x.max(padding_box.min.x);
            clip.max.x = clip.max.x.min(padding_box.max.x);
        }
    }
    match style.layout.overflow.y {
        Overflow::Visible => {}
        Overflow::Clip => {
            clip.min.y = clip
                .min
                .y
                .max(clip_box.min.y - style.overflow_clip_margin[0]);
            clip.max.y = clip
                .max
                .y
                .min(clip_box.max.y + style.overflow_clip_margin[2]);
        }
        _ => {
            clip.min.y = clip.min.y.max(padding_box.min.y);
            clip.max.y = clip.max.y.min(padding_box.max.y);
        }
    }
    content_ui.set_clip_rect(clip);
    let painter = content_ui.painter().with_clip_rect(ui.clip_rect());
    let background = painter.add(egui::Shape::Noop);
    let layer = content_ui.layer_id();
    let start = shape_index(ui.ctx(), layer);
    let response = widget.ui(&mut content_ui);
    let end = shape_index(ui.ctx(), layer);
    let mut measured = content_ui
        .min_rect()
        .size()
        .max(response.rect.max - (geometry.content().min - scroll));
    if !measured.is_finite() {
        measured = Vec2::ZERO;
    }
    {
        let mut frame = active.0.borrow_mut();
        frame.nodes[index].measured = measured;
        frame.nodes[index].color = content_ui
            .visuals()
            .widgets
            .state(response.widget_state())
            .fg_stroke
            .color;
        if frame.nodes[index].children.is_empty() {
            frame.nodes[index].text =
                measure_text(ui.ctx(), layer, start, end, measured, &style.format);
        }
    }
    let content_size = active.0.borrow().nodes[index]
        .text
        .as_ref()
        .map(|text| reflow_text(ui.ctx(), layer, start, end, &text.job))
        .unwrap_or(measured);
    if !geometry.fragments.is_empty() {
        if let Some(text) = active.0.borrow().nodes[index].text.as_ref() {
            replace_fragmented_text(
                ui.ctx(),
                layer,
                start,
                end,
                &geometry,
                &text.job,
                &style.format,
            );
        }
    } else if text_vertical {
        rotate_text(ui.ctx(), layer, start, end, &geometry, &style.format);
    }
    if style.format.display == crate::formatting::DisplayKind::Cell
        && active.0.borrow().nodes[index].children.is_empty()
    {
        let block_size = if style.format.writing.vertical() {
            geometry.content().width()
        } else {
            geometry.content().height()
        };
        let free = (block_size - content_size.y).max(0.0);
        let shift = match style.format.vertical_align.as_str() {
            "middle" => free * 0.5,
            "bottom" => free,
            _ => 0.0,
        };
        let offset = if style.format.writing.vertical() {
            vec2(
                if style.format.writing.reverse() {
                    -shift
                } else {
                    shift
                },
                0.0,
            )
        } else {
            vec2(0.0, shift)
        };
        ui.ctx().graphics_mut(|layers| {
            let list = layers.entry(layer);
            for index in start.0..end.0 {
                list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                    entry
                        .shape
                        .transform(egui::emath::TSTransform::from_translation(offset))
                });
            }
        });
    }
    drop(scope);
    let final_geometry = if root {
        let frame = active.0.borrow();
        let cache = solve(&frame, ui.ctx());
        let changed = frame.nodes.iter().any(|node| {
            cache.boxes.get(&node.id).is_some_and(|g| {
                let actual = g
                    .translated(frame.origin.to_vec2() - node.ancestor_scroll + node.sticky)
                    .content();
                g.slices != node.used.slices
                    || g.fragments != node.used.fragments
                    || g.text_vertical != node.used.text_vertical
                    || !close_rect(actual, node.used.content())
            })
        });
        let geometry = cache
            .boxes
            .get(&id)
            .unwrap()
            .translated(frame.origin.to_vec2() + frame.nodes[index].sticky);
        ui.ctx().data_mut(|d| d.insert_temp(id, cache));
        if changed {
            ui.ctx().request_discard("CSS selector layout changed");
        }
        geometry
    } else if active.0.borrow().cache.boxes.contains_key(&id) {
        geometry
    } else {
        let mut geometry = geometry;
        if style.layout.size.width.is_auto() {
            geometry.border.max.x = geometry.border.min.x
                + measured.x
                + geometry.padding.sum().x
                + geometry.stroke.sum().x;
        }
        if style.layout.size.height.is_auto() {
            geometry.border.max.y = geometry.border.min.y
                + measured.y
                + geometry.padding.sum().y
                + geometry.stroke.sum().y;
        }
        geometry
    };
    let border = final_geometry.border;
    let visual = if final_geometry.slices.is_empty() {
        border
    } else {
        final_geometry
            .slices
            .iter()
            .fold(Rect::NOTHING, |rect, slice| {
                rect.union(slice.source.translate(border.min.to_vec2() + slice.offset))
            })
    };
    let frame_stroke = if active.0.borrow().nodes[index].children.is_empty() {
        crate::take_widget_frame_stroke(ui.ctx(), layer, start, end, response.rect)
    } else {
        None
    };
    let mut response = if absent {
        response.with_new_rect(Rect::from_min_size(border.min, Vec2::ZERO))
    } else {
        response
            .union(content_ui.interact(visual, response.id, response.sense))
            .with_new_rect(visual)
    };
    response.interact_rect = if absent || style.hidden {
        Rect::NOTHING
    } else {
        visual.intersect(ui.clip_rect())
    };
    let current_state =
        u8::from(response.hovered()) | (u8::from(response.is_pointer_button_down_on()) << 1);
    ui.ctx()
        .data_mut(|d| d.insert_temp(id.with("state"), current_state));
    if state != current_state {
        ui.ctx()
            .request_discard("CSS selector interaction state changed");
    }
    let overlay = shape_index(ui.ctx(), layer);
    let mut scrollbars = Vec::new();
    let empty_cell = style.format.display == crate::formatting::DisplayKind::Cell
        && style.format.hide_empty
        && !style.format.collapse
        && start == end
        && active.0.borrow().nodes[index].children.is_empty();
    if !absent && !style.hidden && !style.contents && !empty_cell {
        let visuals = content_ui
            .style()
            .visuals
            .widgets
            .state(response.widget_state());
        let has_background = matching_rules(stylesheet, class).any(|r| {
            (r.state == RuleState::Base
                || r.state == RuleState::Hovered && current_state & 1 != 0
                || r.state == RuleState::Active && current_state & 2 != 0)
                && r.declarations
                    .iter()
                    .any(|d| matches!(d, Declaration::Background(_)))
        });
        let corner_radius = if frame_stroke.is_some()
            || matching_rules(stylesheet, class).any(|r| {
                (r.state == RuleState::Base
                    || r.state == RuleState::Hovered && current_state & 1 != 0
                    || r.state == RuleState::Active && current_state & 2 != 0)
                    && r.declarations
                        .iter()
                        .any(|d| matches!(d, Declaration::BorderRadius(_)))
            }) {
            visuals.corner_radius
        } else {
            egui::CornerRadius::ZERO
        };
        let fill = if has_background {
            visuals.weak_bg_fill
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if style.border_set || final_geometry.collapsed.is_some() {
            egui::Stroke::NONE
        } else {
            frame_stroke.unwrap_or(egui::Stroke::NONE)
        };
        let mut surfaces = Vec::new();
        if style.format.display == crate::formatting::DisplayKind::Inline
            && !final_geometry.fragments.is_empty()
        {
            for fragment in &final_geometry.fragments {
                let rect = fragment.rect.translate(border.min.to_vec2());
                if let Some(last) = surfaces.last_mut()
                    && same_line(*last, rect, style.format.writing.vertical())
                {
                    *last = last.union(rect);
                } else {
                    surfaces.push(rect);
                }
            }
        } else {
            surfaces.push(border);
        }
        let mut shapes = Vec::new();
        let surface_count = surfaces.len();
        for (surface_index, surface) in surfaces.into_iter().enumerate() {
            shapes.push(egui::Shape::Rect(egui::epaint::RectShape::new(
                surface,
                corner_radius,
                fill,
                stroke,
                egui::StrokeKind::Inside,
            )));
            if let Some(edges) = final_geometry.collapsed {
                let mut paint_style = style.clone();
                let widths = edges.map(|edge| edge.width);
                let surface = surface
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
                paint_style.border_colors = edges.map(|edge| Some(edge.color));
                paint_style.border_styles = edges.map(|edge| {
                    if edge.draw {
                        edge.style
                    } else {
                        crate::layout_style::BorderStyle::None
                    }
                });
                shapes.extend(border_shapes(
                    surface,
                    &paint_style,
                    visuals.fg_stroke.color,
                ));
            } else {
                let mut fragment_style = style.clone();
                if style.format.display == crate::formatting::DisplayKind::Inline
                    && !style.format.clone_decoration
                {
                    let (start, end) = if style.format.writing.vertical() {
                        (0, 2)
                    } else if style.layout.direction == taffy::Direction::Rtl {
                        (1, 3)
                    } else {
                        (3, 1)
                    };
                    if surface_index > 0 {
                        fragment_style.border_styles[start] = BorderStyle::None;
                    }
                    if surface_index + 1 < surface_count {
                        fragment_style.border_styles[end] = BorderStyle::None;
                    }
                }
                shapes.extend(border_shapes(
                    surface,
                    &fragment_style,
                    visuals.fg_stroke.color,
                ));
            }
        }
        let rule_style = style.format.column_rule;
        if !matches!(rule_style.style, BorderStyle::None | BorderStyle::Hidden) {
            for rule in &final_geometry.rules {
                let rect = rule.translate(border.min.to_vec2());
                let mut stroke = BoxStyle::default();
                let width = rule_style.width;
                let side = if rect.width() < rect.height() { 3 } else { 0 };
                stroke.border_styles[side] = rule_style.style;
                stroke.border_colors[side] = rule_style.color;
                let rect = if side == 3 {
                    stroke.layout.border.left = length(width);
                    rect.expand2(vec2(width * 0.5, 0.0))
                } else {
                    stroke.layout.border.top = length(width);
                    rect.expand2(vec2(0.0, width * 0.5))
                };
                shapes.extend(border_shapes(rect, &stroke, visuals.fg_stroke.color));
            }
        }

        painter.set(background, egui::Shape::Vec(shapes));
        scrollbars = scroll_box(&mut content_ui, id, &final_geometry, &style, scroll);
    }
    if root && !absent && style.layout.position != Position::Absolute {
        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let sticky = active.0.borrow().nodes[index].sticky;
        ui.advance_cursor_after_rect(crate::nonnegative_rect(
            border.translate(-sticky) + final_geometry.margin,
        ));
        ui.spacing_mut().item_spacing = spacing;
    }
    let paint_end = shape_index(ui.ctx(), layer);
    active.0.borrow_mut().nodes[index].paint = Some(PaintRecord {
        ui: content_ui,
        response: response.clone(),
        start: background,
        end: paint_end,
        overlay,
        scrollbars,
        geometry: final_geometry.clone(),
        clip: ui.clip_rect(),
    });
    if root {
        reorder_paint(&active.0.borrow(), ui.ctx());
    }
    response
}

fn same_line(a: Rect, b: Rect, vertical: bool) -> bool {
    if vertical {
        (a.left() - b.left()).abs() < 0.01 && (a.width() - b.width()).abs() < 0.01
    } else {
        (a.top() - b.top()).abs() < 0.01 && (a.height() - b.height()).abs() < 0.01
    }
}

fn reorder_paint(frame: &Frame, context: &Context) {
    type Part = (i32, i32, usize);
    let count = frame.nodes.len();
    let mut contexts: Vec<Vec<Part>> = vec![Vec::new(); count];
    let mut groups = contexts.clone();
    let mut keys = contexts.clone();
    let mut ranks = vec![0; count];
    fn rank(frame: &Frame, node: usize, next: &mut usize, ranks: &mut [usize]) {
        ranks[node] = *next;
        *next += 1;
        let mut children = frame.nodes[node].children.clone();
        if matches!(
            frame.nodes[node].style.layout.display,
            Display::Flex | Display::Grid
        ) {
            children.sort_by_key(|i| frame.nodes[*i].style.order);
        }
        for child in children {
            rank(frame, child, next, ranks);
        }
    }
    rank(frame, 0, &mut 0, &mut ranks);
    for (i, node) in frame.nodes.iter().enumerate() {
        let parent_context = node.parent.map_or_else(Vec::new, |p| contexts[p].clone());
        let parent_group = node.parent.map_or_else(Vec::new, |p| groups[p].clone());
        let item = node.parent.is_some_and(|p| {
            matches!(
                frame.nodes[p].style.layout.display,
                Display::Flex | Display::Grid
            )
        });
        let real = i == 0
            || node.style.z_index.is_some() && (node.style.positioned || item)
            || node.style.fixed
            || node.style.layout.position == Position::Sticky;
        let pseudo = node.style.positioned
            || node.style.layout.float.is_floated()
            || node.style.inline
            || item;
        let mut prefix = if real {
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
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        let entries: Vec<_> = list
            .all_entries()
            .skip(root.start.0)
            .take(root.end.0 - root.start.0)
            .cloned()
            .collect();
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
        let mut grouped = vec![Vec::new(); count];
        let mut overlays = vec![Vec::new(); count];
        for (index, (entry, owner)) in entries.into_iter().zip(owners).enumerate() {
            if frame.nodes[owner]
                .paint
                .as_ref()
                .is_some_and(|paint| index + root.start.0 >= paint.overlay.0)
            {
                overlays[owner].push(entry);
            } else {
                grouped[owner].push(entry);
            }
        }
        for index in root.start.0..root.end.0 {
            list.reset_shape(egui::layers::ShapeIdx(index));
        }
        for index in &order {
            let paint = frame.nodes[*index].paint.as_ref().unwrap();
            for (entry_index, entry) in grouped[*index].drain(..).enumerate() {
                if paint.geometry.slices.is_empty() {
                    list.add(entry.clip_rect, entry.shape);
                } else {
                    for slice in &paint.geometry.slices {
                        let source = slice.source.translate(paint.geometry.border.min.to_vec2());
                        let clip = entry
                            .clip_rect
                            .intersect(source)
                            .translate(slice.offset)
                            .intersect(paint.clip);
                        if clip.is_positive() {
                            let mut shape = if entry_index == 0
                                && frame.nodes[*index].style.format.clone_decoration
                            {
                                clone_decoration(
                                    &entry.shape,
                                    source,
                                    &frame.nodes[*index].style,
                                    frame.nodes[*index].color,
                                )
                            } else {
                                entry.shape.clone()
                            };
                            shape.transform(egui::emath::TSTransform::from_translation(
                                slice.offset,
                            ));
                            list.add(clip, shape);
                        }
                    }
                }
            }
        }
        for index in &order {
            for entry in overlays[*index].drain(..) {
                list.add(entry.clip_rect, entry.shape);
            }
        }
    });
    for index in &order {
        let node = &frame.nodes[*index];
        if !node.absent
            && !node.style.hidden
            && !node.style.contents
            && let Some(paint) = &node.paint
        {
            paint.ui.interact_opt(
                paint.response.rect,
                paint.response.id,
                paint.response.sense,
                egui::InteractOptions { move_to_top: true },
            );
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

fn clone_decoration(
    shape: &egui::Shape,
    rect: Rect,
    style: &BoxStyle,
    color: Color32,
) -> egui::Shape {
    if let egui::Shape::Vec(shapes) = shape
        && let Some(egui::Shape::Rect(background)) = shapes.first()
    {
        let mut background = background.clone();
        background.rect = rect;
        let mut shapes = vec![egui::Shape::Rect(background)];
        shapes.extend(border_shapes(rect, style, color));
        egui::Shape::Vec(shapes)
    } else {
        shape.clone()
    }
}

fn sticky_offset(
    style: &BoxStyle,
    geometry: &Geometry,
    parent: Option<usize>,
    frame: &Frame,
) -> Vec2 {
    if style.layout.position != Position::Sticky {
        return Vec2::ZERO;
    }
    let mut scroll_parent = parent;
    let viewport = loop {
        let Some(p) = scroll_parent else {
            break frame.viewport;
        };
        let node = &frame.nodes[p];
        if node.style.layout.overflow.x.is_scroll_container()
            || node.style.layout.overflow.y.is_scroll_container()
        {
            break node.used.border - node.used.stroke;
        }
        scroll_parent = node.parent;
    };
    let containing = parent.map(|p| frame.nodes[p].used.content());
    let mut origin = geometry.border.min;
    let insets = style.layout.inset;
    for axis in 0..2 {
        let (start, end) = if axis == 0 {
            (insets.left, insets.right)
        } else {
            (insets.top, insets.bottom)
        };
        let basis = containing.map_or(viewport.size()[axis], |rect| rect.size()[axis]);
        let start = resolve(start.into_raw(), basis);
        let end = resolve(end.into_raw(), basis);
        if start.is_none() && end.is_none() {
            continue;
        }
        let minimum = start.map_or(f32::NEG_INFINITY, |inset| viewport.min[axis] + inset);
        let maximum = end
            .map_or(f32::INFINITY, |inset| {
                viewport.max[axis] - inset - geometry.border.size()[axis]
            })
            .max(minimum);
        origin[axis] = origin[axis].clamp(minimum, maximum);
        if let Some(containing) = containing {
            origin[axis] = origin[axis].clamp(
                containing.min[axis],
                (containing.max[axis] - geometry.border.size()[axis]).max(containing.min[axis]),
            );
        }
    }
    origin - geometry.border.min
}

fn scroll_box(
    ui: &mut Ui,
    id: Id,
    geometry: &Geometry,
    style: &BoxStyle,
    mut offset: Vec2,
) -> Vec<Response> {
    let mut responses = Vec::new();
    let border_box = geometry.border - geometry.stroke;
    let viewport = border_box - geometry.gutter;
    let extent = geometry.extent.max(viewport.size());
    let maximum = (extent - viewport.size()).max(Vec2::ZERO);
    let previous = offset;
    let hovered = ui.rect_contains_pointer(border_box);
    let axes = [
        style.layout.overflow.x == Overflow::Scroll,
        style.layout.overflow.y == Overflow::Scroll,
    ];
    for axis in 0..2 {
        if !axes[axis] {
            offset[axis] = 0.0;
            continue;
        }
        if hovered {
            ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta[axis];
                let next = (offset[axis] - delta).clamp(0.0, maximum[axis]);
                if next != offset[axis] {
                    input.smooth_scroll_delta[axis] = 0.0;
                }
                offset[axis] = next;
            });
        }
        offset[axis] = offset[axis].clamp(0.0, maximum[axis]);
        if maximum[axis] <= 0.0 {
            continue;
        }
        let track = if axis == 0 {
            Rect::from_min_max(
                pos2(border_box.left(), border_box.bottom() - 6.0),
                border_box.right_bottom(),
            )
        } else {
            Rect::from_min_max(
                pos2(border_box.right() - 6.0, border_box.top()),
                border_box.right_bottom(),
            )
        };
        let track_length = track.size()[axis];
        let thumb_length = (track_length * viewport.size()[axis] / extent[axis])
            .max(12.0)
            .min(track_length);
        let travel = track_length - thumb_length;
        let mut thumb = track;
        thumb.min[axis] += if maximum[axis] > 0.0 {
            offset[axis] / maximum[axis] * travel
        } else {
            0.0
        };
        thumb.max[axis] = thumb.min[axis] + thumb_length;
        let response = ui.interact(
            track,
            id.with(("scrollbar", axis)),
            egui::Sense::click_and_drag(),
        );
        let drag_frame = ui.ctx().cumulative_frame_nr();
        let drag_id = id.with(("scrollbar-drag", axis));
        let already_dragged =
            ui.ctx().data(|data| data.get_temp::<u64>(drag_id)) == Some(drag_frame);
        if response.dragged() && travel > 0.0 && !already_dragged {
            offset[axis] = (offset[axis] + response.drag_delta()[axis] / travel * maximum[axis])
                .clamp(0.0, maximum[axis]);
            ui.ctx()
                .data_mut(|data| data.insert_temp(drag_id, drag_frame));
        } else if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            offset[axis] = ((pointer[axis] - track.min[axis] - thumb_length * 0.5) / travel
                * maximum[axis])
                .clamp(0.0, maximum[axis]);
        }
        ui.painter().rect_filled(
            thumb,
            3.0,
            Color32::from_gray(if response.hovered() { 100 } else { 150 }),
        );
        responses.push(response);
    }
    if offset != previous {
        ui.ctx()
            .data_mut(|d| d.insert_temp(id.with("scroll"), offset));
        ui.ctx().request_discard("CSS scroll offset changed");
    }
    responses
}

fn matching_rules<'a>(
    stylesheet: &'a Stylesheet,
    class: &'a str,
) -> impl Iterator<Item = &'a crate::css::Rule> {
    let mut rules: Vec<_> = stylesheet
        .rules
        .iter()
        .filter(|rule| rule.class.is_none() || rule.class.as_deref() == Some(class))
        .collect();
    rules.sort_by_key(|rule| {
        usize::from(rule.class.is_some()) + usize::from(rule.state != RuleState::Base)
    });
    rules.into_iter()
}

fn shape_index(ctx: &Context, layer: egui::LayerId) -> egui::layers::ShapeIdx {
    ctx.graphics(|layers| {
        layers
            .get(layer)
            .map_or(egui::layers::ShapeIdx(0), |list| list.next_idx())
    })
}

fn close_rect(a: Rect, b: Rect) -> bool {
    (a.min - b.min).abs().max_elem() < 0.05 && (a.max - b.max).abs().max_elem() < 0.05
}

fn resolve(value: taffy::CompactLength, basis: f32) -> Option<f32> {
    match value.tag() {
        taffy::CompactLength::CALC_TAG => Some(crate::math::resolve(value.calc_value(), basis)),
        taffy::CompactLength::LENGTH_TAG => Some(value.value()),
        taffy::CompactLength::PERCENT_TAG if basis.is_finite() => Some(value.value() * basis),
        _ => None,
    }
}

fn guess(style: &BoxStyle, available: Rect) -> Geometry {
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
        slices: Vec::new(),
        rules: Vec::new(),
        text_vertical: false,
        collapsed: None,
    }
}

fn margin(rect: taffy::Rect<f32>) -> egui::epaint::MarginF32 {
    egui::epaint::MarginF32 {
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        left: rect.left,
    }
}

const SCROLLBAR_GUTTER_WIDTH: f32 = 6.0;

fn scrollbar_gutter(style: &BoxStyle) -> egui::epaint::MarginF32 {
    if style.scrollbar_gutter == crate::layout_style::ScrollbarGutter::Auto {
        return egui::epaint::MarginF32::ZERO;
    }
    let vertical = matches!(style.layout.overflow.y, Overflow::Hidden | Overflow::Scroll);
    let horizontal = matches!(style.layout.overflow.x, Overflow::Hidden | Overflow::Scroll);
    let both_edges =
        style.scrollbar_gutter == crate::layout_style::ScrollbarGutter::StableBothEdges;
    egui::epaint::MarginF32 {
        top: if horizontal && both_edges {
            SCROLLBAR_GUTTER_WIDTH
        } else {
            0.0
        },
        right: if vertical {
            SCROLLBAR_GUTTER_WIDTH
        } else {
            0.0
        },
        bottom: if horizontal {
            SCROLLBAR_GUTTER_WIDTH
        } else {
            0.0
        },
        left: if vertical && both_edges {
            SCROLLBAR_GUTTER_WIDTH
        } else {
            0.0
        },
    }
}

fn trim_child_margins(frame: &Frame, child: usize, style: &mut Style) {
    let mut parent = frame.nodes[child].parent;
    while let Some(index) = parent {
        if !frame.nodes[index].style.contents {
            break;
        }
        parent = frame.nodes[index].parent;
    }
    let Some(parent) = parent else {
        return;
    };
    let container = &frame.nodes[parent].style;
    let flex = container.layout.display == Display::Flex;
    if !matches!(
        container.layout.display,
        Display::Block | Display::FlowRoot | Display::Flex
    ) || container.format.display != crate::formatting::DisplayKind::Normal
        || !container.margin_trim.iter().any(|trim| *trim)
    {
        return;
    }

    fn append_flow_children(
        frame: &Frame,
        parent: usize,
        output: &mut Vec<usize>,
        include_floats: bool,
    ) {
        for child in &frame.nodes[parent].children {
            let node = &frame.nodes[*child];
            if node.absent
                || node.style.layout.display == Display::None
                || node.style.layout.position.is_out_of_flow()
                || !include_floats && node.style.layout.float.is_floated()
            {
                continue;
            }
            if node.style.contents {
                append_flow_children(frame, *child, output, include_floats);
            } else {
                output.push(*child);
            }
        }
    }

    let mut children = Vec::new();
    append_flow_children(frame, parent, &mut children, flex);
    let mut trim = [false; 4];
    if flex {
        children.sort_by_key(|index| frame.nodes[*index].style.order);
        let row = matches!(
            container.layout.flex_direction,
            FlexDirection::Row | FlexDirection::RowReverse
        );
        let reverse = matches!(
            container.layout.flex_direction,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        );
        let main_start = if row {
            if reverse { 3 } else { 2 }
        } else if reverse {
            1
        } else {
            0
        };
        let main_end = match main_start {
            0 => 1,
            1 => 0,
            2 => 3,
            _ => 2,
        };
        let first = children.first() == Some(&child);
        let last = children.last() == Some(&child);
        trim[main_start] = first && container.margin_trim[main_start];
        trim[main_end] = last && container.margin_trim[main_end];
        let single_line = container.layout.flex_wrap == FlexWrap::NoWrap;
        if single_line {
            let cross_start = if row { 0 } else { 2 };
            let cross_end = if row { 1 } else { 3 };
            trim[cross_start] = container.margin_trim[cross_start];
            trim[cross_end] = container.margin_trim[cross_end];
        }
    } else {
        let first = children.first() == Some(&child);
        let last = children.last() == Some(&child);
        for (axis, selected) in container.margin_trim.iter().copied().enumerate() {
            let at_edge = match axis {
                0 => first,
                1 => last,
                _ => true,
            };
            trim[axis] = selected && at_edge;
        }
    }
    for (axis, selected) in trim.into_iter().enumerate() {
        if !selected {
            continue;
        }
        let name = ["block-start", "block-end", "inline-start", "inline-end"][axis];
        let edge = crate::layout_style::logical_side(
            name,
            container.layout.direction,
            container.format.writing,
        )
        .unwrap();
        let zero = taffy::LengthPercentageAuto::length(0.0);
        match edge {
            0 => style.margin.top = zero,
            1 => style.margin.right = zero,
            2 => style.margin.bottom = zero,
            _ => style.margin.left = zero,
        }
    }
}

fn collapse_flex_item(frame: &Frame, child: usize, layout: Layout, style: &mut Style) {
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

fn solve(frame: &Frame, context: &Context) -> Cache {
    let mut tree = crate::layout_tree::Tree::new(crate::math::resolve);
    let mut ids = Vec::new();
    for (index, node) in frame.nodes.iter().enumerate() {
        let mut style = node.style.layout.clone();
        trim_child_margins(frame, index, &mut style);
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
                draw: true,
                priority: match node.style.format.display {
                    crate::formatting::DisplayKind::Table => 0,
                    crate::formatting::DisplayKind::RowGroup
                    | crate::formatting::DisplayKind::Header
                    | crate::formatting::DisplayKind::Footer => 3,
                    crate::formatting::DisplayKind::Row => 4,
                    _ => 5,
                },
            });

        tree.nodes[usize::from(id)].atomic = node.paint.as_ref().is_some_and(|paint| {
            paint.response.sense.senses_click() || paint.response.sense.senses_drag()
        }) || node.style.inline
            && node.style.format.display != crate::formatting::DisplayKind::Inline;
        if let Some(text) = &node.text {
            tree.nodes[usize::from(id)].runs = text_runs(context, &text.job, &node.style.format);
        }
        ids.push(id);
    }
    let mut children = vec![Vec::new(); frame.nodes.len()];
    let mut root_children = Vec::new();
    let mut fixed = Vec::new();
    let mut layout_parents = vec![None; frame.nodes.len()];
    for (i, node) in frame.nodes.iter().enumerate() {
        if node.style.contents {
            continue;
        }
        if node.style.fixed {
            fixed.push(i);
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
            children[p].push(i);
        } else {
            root_children.push(i);
        }
    }
    for (i, children) in children.iter_mut().enumerate() {
        if matches!(
            frame.nodes[i].style.layout.display,
            Display::Flex | Display::Grid
        ) {
            children.sort_by_key(|i| frame.nodes[*i].style.order);
        }
        tree.set_children(
            ids[i],
            &children.iter().map(|i| ids[*i]).collect::<Vec<_>>(),
        )
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
    let root = tree
        .new_with_children(
            holder,
            &root_children.iter().map(|i| ids[*i]).collect::<Vec<_>>(),
        )
        .unwrap();
    let measure =
        |input: taffy::LayoutInput, _: NodeId, index: Option<&mut usize>, style: &Style| {
            let node = index.map(|index| &frame.nodes[*index]);
            let measured = node.map_or(Vec2::ZERO, |node| {
                if node.style.format.writing.vertical() && node.text.is_none() {
                    vec2(node.measured.y, node.measured.x)
                } else {
                    node.measured
                }
            });
            taffy::compute_leaf_layout(input, style, crate::math::resolve, |known, available| {
                let measured = if let Some(text) = node.and_then(|node| node.text.as_ref()) {
                    let mut job = text.job.clone();
                    job.wrap.max_width = known
                        .width
                        .map(|w| (w - text.extra.x).max(0.0))
                        .unwrap_or_else(|| match available.width {
                            AvailableSpace::Definite(width) => (width - text.extra.x).max(0.0),
                            AvailableSpace::MinContent => text.minimum,
                            AvailableSpace::MaxContent => f32::INFINITY,
                        });
                    context.fonts_mut(|fonts| fonts.layout_job(job).size()) + text.extra
                } else {
                    measured
                };
                Size {
                    width: known.width.unwrap_or(measured.x),
                    height: known.height.unwrap_or(measured.y),
                }
            })
        };
    tree.compute_layout_with_measure(
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
            .new_with_children(
                Style {
                    display: Display::Block,
                    size: Size {
                        width: length(frame.viewport.width()),
                        height: length(frame.viewport.height()),
                    },
                    ..Style::default()
                },
                &fixed.iter().map(|i| ids[*i]).collect::<Vec<_>>(),
            )
            .unwrap();
        tree.compute_layout_with_measure(
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
        tree.compute_layout_with_measure(
            root,
            Size {
                width: AvailableSpace::Definite(frame.available.x),
                height: AvailableSpace::Definite(frame.available.y),
            },
            measure,
        )
        .unwrap();
        if let Some(viewport) = fixed_root {
            tree.compute_layout_with_measure(
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
    let mut cache = Cache::default();
    for (i, node) in frame.nodes.iter().enumerate() {
        let layout = tree.absolute_layout(ids[i]);
        let mut ancestor = Some(i);
        let mut fixed = false;
        while let Some(p) = ancestor {
            if frame.nodes[p].style.fixed {
                fixed = true;
                break;
            }
            ancestor = frame.nodes[p].parent;
        }
        let offset = if fixed {
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
                    })
                    .collect(),
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
                collapsed: tree.nodes[usize::from(ids[i])].collapsed.map(|e| {
                    let mode = tree.nodes[usize::from(ids[i])].layout_axes;
                    if !mode.vertical() {
                        e
                    } else if mode.reverse() {
                        [e[3], e[0], e[1], e[2]]
                    } else {
                        [e[3], e[2], e[1], e[0]]
                    }
                }),
            },
        );
    }
    cache
}

fn measure_text(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    measured: Vec2,
    format: &crate::formatting::Formatting,
) -> Option<TextMeasurement> {
    fn collect(shape: &egui::Shape, text: &mut Vec<std::sync::Arc<egui::Galley>>) {
        match shape {
            egui::Shape::Text(shape) if shape.angle == 0.0 => text.push(shape.galley.clone()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let galley = context.graphics(|layers| {
        let list = layers.get(layer)?;
        let mut text = Vec::new();
        for clipped in list.all_entries().skip(start.0).take(end.0 - start.0) {
            collect(&clipped.shape, &mut text);
        }
        let [galley] = text.as_slice() else {
            return None;
        };
        Some(galley.clone())
    })?;
    let source = prepare_text_job(&galley.job, format);
    let mut minimum = 0.0_f32;
    let mut start = 0;
    for (end, _) in unicode_linebreak::linebreaks(&source.text) {
        let content = &source.text[start..end];
        let content = content.trim_end_matches(|c: char| c.is_ascii_whitespace());
        let end_content = start + content.len();
        let mut job = source.clone();
        job.text = content.to_owned();
        job.sections = source
            .sections
            .iter()
            .filter_map(|section| {
                let from = section.byte_range.start.max(egui::text::ByteIndex(start));
                let to = section
                    .byte_range
                    .end
                    .min(egui::text::ByteIndex(end_content));
                (from < to).then(|| egui::text::LayoutSection {
                    byte_range: from - start..to - start,
                    ..section.clone()
                })
            })
            .collect();
        job.wrap.max_width = f32::INFINITY;
        minimum = minimum.max(context.fonts_mut(|fonts| fonts.layout_job(job).size().x));
        start = end;
    }
    Some(TextMeasurement {
        job: source,
        extra: (measured - galley.size()).max(Vec2::ZERO),
        minimum,
    })
}

fn prepare_text_job(
    job: &egui::text::LayoutJob,
    format: &crate::formatting::Formatting,
) -> egui::text::LayoutJob {
    let preserve = matches!(
        format.white_space.as_str(),
        "pre" | "pre-wrap" | "break-spaces"
    );
    let newlines = preserve || format.white_space == "pre-line";
    let mut result = job.clone();
    result.text.clear();
    result.sections.clear();
    let mut space = false;
    let mut carriage = false;
    for (index, mut c) in job.text.char_indices() {
        if c == '\n' && carriage {
            carriage = false;
            continue;
        }
        carriage = c == '\r';
        if carriage {
            c = '\n';
        }
        if !preserve && matches!(c, ' ' | '\t' | '\n' | '\u{c}') {
            if c == '\n' && newlines {
                space = false;
            } else {
                if space {
                    continue;
                }
                space = true;
                c = ' ';
            }
        } else {
            space = false;
        }
        let mut text_format = job.format_at_byte(egui::text::ByteIndex(index)).clone();
        if let Some(height) = format.line_height {
            text_format.line_height = Some(height);
        }
        result.append(c.encode_utf8(&mut [0; 4]), 0.0, text_format);
    }
    if matches!(format.white_space.as_str(), "pre" | "nowrap") {
        result.wrap.max_width = f32::INFINITY;
    }
    result
}

fn reflow_text(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    job: &egui::text::LayoutJob,
) -> Vec2 {
    let galley = context.fonts_mut(|fonts| fonts.layout_job(job.clone()));
    fn replace(shape: &mut egui::Shape, galley: &std::sync::Arc<egui::Galley>) {
        match shape {
            egui::Shape::Text(text) => text.galley = galley.clone(),
            egui::Shape::Vec(shapes) => shapes.iter_mut().for_each(|shape| replace(shape, galley)),
            _ => {}
        }
    }
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        for index in start.0..end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                replace(&mut entry.shape, &galley)
            });
        }
    });
    galley.size()
}

fn border_shapes(rect: Rect, style: &BoxStyle, current: Color32) -> Vec<egui::Shape> {
    let mut shapes = Vec::new();
    let widths = style
        .layout
        .border
        .map(|v| resolve(v.into_raw(), 0.0).unwrap_or(0.0));
    let inner = rect - margin(widths);
    let corners = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let inside = [
        inner.left_top(),
        inner.right_top(),
        inner.right_bottom(),
        inner.left_bottom(),
    ];
    for i in 0..4 {
        let width = [widths.top, widths.right, widths.bottom, widths.left][i];
        if width <= 0.0 {
            continue;
        }
        let color = style.border_colors[i].unwrap_or(current);
        let next = (i + 1) % 4;
        let polygon = |shapes: &mut Vec<egui::Shape>, a: f32, b: f32, color| {
            let p = |index: usize, t| corners[index].lerp(inside[index], t);
            shapes.push(egui::Shape::convex_polygon(
                vec![p(i, a), p(next, a), p(next, b), p(i, b)],
                color,
                egui::Stroke::NONE,
            ));
        };
        match style.border_styles[i] {
            BorderStyle::None | BorderStyle::Hidden => {}
            BorderStyle::Double if width >= 3.0 => {
                polygon(&mut shapes, 0.0, 1.0 / 3.0, color);
                polygon(&mut shapes, 2.0 / 3.0, 1.0, color);
            }
            BorderStyle::Dashed | BorderStyle::Dotted => {
                let a = corners[i].lerp(inside[i], 0.5);
                let b = corners[next].lerp(inside[next], 0.5);
                let length = a.distance(b);
                let step = if style.border_styles[i] == BorderStyle::Dotted {
                    width * 2.0
                } else {
                    width * 6.0
                };
                let direction = (b - a).normalized();
                let mut offset = width * 0.5;
                while offset < length {
                    if style.border_styles[i] == BorderStyle::Dotted {
                        shapes.push(egui::Shape::circle_filled(
                            a + direction * offset,
                            width * 0.5,
                            color,
                        ));
                    } else {
                        shapes.push(egui::Shape::line_segment(
                            [
                                a + direction * offset,
                                a + direction * (offset + width * 3.0).min(length),
                            ],
                            egui::Stroke::new(width, color),
                        ));
                    }
                    offset += step;
                }
            }
            BorderStyle::Inset | BorderStyle::Outset | BorderStyle::Groove | BorderStyle::Ridge => {
                let dark = color.gamma_multiply(0.55);
                let first = if (i == 0 || i == 3)
                    == matches!(
                        style.border_styles[i],
                        BorderStyle::Inset | BorderStyle::Groove
                    ) {
                    dark
                } else {
                    color
                };
                let second = if first == dark { color } else { dark };
                if matches!(
                    style.border_styles[i],
                    BorderStyle::Groove | BorderStyle::Ridge
                ) {
                    polygon(&mut shapes, 0.0, 0.5, first);
                    polygon(&mut shapes, 0.5, 1.0, second);
                } else {
                    polygon(&mut shapes, 0.0, 1.0, first);
                }
            }
            _ => polygon(&mut shapes, 0.0, 1.0, color),
        }
    }
    shapes
}

fn slice_job(job: &egui::text::LayoutJob, range: egui::text::ByteRange) -> egui::text::LayoutJob {
    let mut result = job.clone();
    result.text = job.text[range.start.0..range.end.0].to_owned();
    result.sections = job
        .sections
        .iter()
        .filter_map(|section| {
            let start = section.byte_range.start.max(range.start);
            let end = section.byte_range.end.min(range.end);
            (start < end).then(|| egui::text::LayoutSection {
                byte_range: (start.0 - range.start.0).into()..(end.0 - range.start.0).into(),
                ..section.clone()
            })
        })
        .collect();
    result.wrap.max_width = f32::INFINITY;
    result
}
fn text_runs(
    context: &Context,
    job: &egui::text::LayoutJob,
    format: &crate::formatting::Formatting,
) -> Vec<crate::layout_tree::TextRun> {
    let mut start = 0;
    let mut runs = Vec::new();
    let preserve = matches!(
        format.white_space.as_str(),
        "pre" | "pre-wrap" | "break-spaces"
    );
    for (end, _) in unicode_linebreak::linebreaks(&job.text) {
        let original = &job.text[start..end];
        let content = if preserve {
            original.trim_end_matches(['\r', '\n'])
        } else {
            original.trim_end_matches(|c: char| c.is_ascii_whitespace())
        };
        if let Some(section) = job
            .sections
            .iter()
            .find(|section| section.byte_range.end.0 > start)
        {
            let range = egui::text::ByteIndex(start)..egui::text::ByteIndex(start + content.len());
            let size =
                context.fonts_mut(|fonts| fonts.layout_job(slice_job(job, range.clone())).size());
            let (space, height) = context.fonts_mut(|fonts| {
                (
                    fonts.glyph_width(&section.format.font_id, ' '),
                    fonts.row_height(&section.format.font_id),
                )
            });
            runs.push(crate::layout_tree::TextRun {
                range,
                width: size.x,
                height: format.line_height.unwrap_or(size.y.max(height)),
                gap: if !preserve && content.len() < original.len() {
                    space
                } else {
                    0.0
                },
                marker_width: format.text_overflow[1].marker().map_or(0.0, |marker| {
                    let mut marker_job = egui::text::LayoutJob::default();
                    marker_job.append(marker, 0.0, section.format.clone());
                    context.fonts_mut(|fonts| fonts.layout_job(marker_job).size().x)
                }),
                newline: original.contains('\n'),
            });
        }
        start = end;
    }
    runs
}
fn replace_fragmented_text(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    geometry: &Geometry,
    job: &egui::text::LayoutJob,
    format: &crate::formatting::Formatting,
) {
    let original = context.graphics(|layers| {
        layers
            .get(layer)?
            .all_entries()
            .skip(start.0)
            .take(end.0 - start.0)
            .find_map(|entry| {
                if let egui::Shape::Text(text) = &entry.shape {
                    Some((text.clone(), entry.clip_rect))
                } else {
                    None
                }
            })
    });
    let Some((template, clip)) = original else {
        return;
    };
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        for index in start.0..end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |shape| {
                if matches!(shape.shape, egui::Shape::Text(_)) {
                    shape.shape = egui::Shape::Noop;
                }
            });
        }
    });
    let painter = egui::Painter::new(context.clone(), layer, clip);
    for fragment in &geometry.fragments {
        if let Some(range) = &fragment.range {
            let mut text = template.clone();
            let mut layout = if let Some(marker) = &fragment.marker {
                let content_width = (fragment.text_rect.width() - fragment.marker_width).max(0.0);
                let mut fitting = slice_job(job, range.clone());
                fitting.wrap = egui::text::TextWrapping::truncate_at_width(content_width);
                fitting.wrap.overflow_character = None;
                let visible_chars = context.fonts_mut(|fonts| {
                    fonts
                        .layout_job(fitting)
                        .rows
                        .first()
                        .map_or(0, |row| row.row.glyphs.len())
                });
                let source = &job.text[range.start.0..range.end.0];
                let visible_bytes = source
                    .char_indices()
                    .nth(visible_chars)
                    .map_or(source.len(), |(offset, _)| offset);
                let visible_end = egui::text::ByteIndex(range.start.0 + visible_bytes);
                let mut layout = slice_job(job, range.start..visible_end);
                if let Some(section) = layout.sections.last().or_else(|| {
                    job.sections
                        .iter()
                        .find(|section| section.byte_range.end > range.start)
                }) {
                    layout.append(marker, 0.0, section.format.clone());
                }
                layout
            } else {
                slice_job(job, range.clone())
            };
            if fragment.marker.is_some() {
                layout.wrap = egui::text::TextWrapping::no_max_width();
            }
            text.galley = context.fonts_mut(|fonts| fonts.layout_job(layout));
            text.pos = geometry.border.min + fragment.text_rect.min.to_vec2();
            if format.writing.vertical() {
                vertical_text(&mut text, fragment.text_rect.size(), format);
            }
            painter.add(egui::Shape::Text(text));
        }
    }
}

fn rotate_text(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    geometry: &Geometry,
    format: &crate::formatting::Formatting,
) {
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        for index in start.0..end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                if let egui::Shape::Text(text) = &mut entry.shape {
                    text.pos = geometry.content().min;
                    vertical_text(text, geometry.content().size(), format);
                }
            });
        }
    });
}

fn vertical_text(
    text: &mut egui::epaint::TextShape,
    size: Vec2,
    format: &crate::formatting::Formatting,
) {
    use crate::formatting::Writing;
    let galley = std::sync::Arc::make_mut(&mut text.galley);
    let mut bounds = Rect::NOTHING;
    for placed in &mut galley.rows {
        let origin = placed.pos;
        let row = std::sync::Arc::make_mut(&mut placed.row);
        let original = row.visuals.mesh.vertices.clone();
        for vertex in &mut row.visuals.mesh.vertices {
            let p = vertex.pos;
            vertex.pos = match format.writing {
                Writing::SidewaysLr => pos2(origin.y + p.y, size.y - origin.x - p.x),
                writing if writing.reverse() => pos2(size.x - origin.y - p.y, origin.x + p.x),
                _ => pos2(origin.y + row.size.y - p.y, origin.x + p.x),
            };
        }
        if matches!(format.writing, Writing::VerticalRl | Writing::VerticalLr)
            && format.text_orientation != "sideways"
        {
            for glyph in &row.glyphs {
                if glyph.uv_rect.is_nothing()
                    || format.text_orientation == "mixed" && !upright_character(glyph.chr)
                {
                    continue;
                }
                let column = if format.writing.reverse() {
                    size.x - origin.y - row.size.y
                } else {
                    origin.y
                };
                let cell = glyph.logical_rect();
                let target = pos2(
                    column + (row.size.y - glyph.advance_width) * 0.5,
                    origin.x + glyph.pos.x,
                );
                let start = glyph.first_vertex as usize;
                for (vertex, original) in row
                    .visuals
                    .mesh
                    .vertices
                    .iter_mut()
                    .zip(&original)
                    .skip(start)
                    .take(4)
                {
                    vertex.pos = original.pos + (target - cell.min);
                }
            }
        }
        row.visuals.mesh_bounds = row
            .visuals
            .mesh
            .vertices
            .iter()
            .fold(Rect::NOTHING, |bounds, vertex| {
                bounds.union(Rect::from_min_max(vertex.pos, vertex.pos))
            });
        bounds = bounds.union(row.visuals.mesh_bounds);
        row.size = vec2(row.size.y, row.size.x);
        placed.pos = Pos2::ZERO;
    }
    galley.rect = Rect::from_min_size(Pos2::ZERO, vec2(galley.rect.height(), galley.rect.width()));
    galley.mesh_bounds = bounds;
    text.angle = 0.0;
}

fn upright_character(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x11ff | 0x2e80..=0xa4cf | 0xac00..=0xd7ff |
        0xf900..=0xfaff | 0xfe10..=0xfe1f | 0xfe30..=0xfe4f |
        0xff01..=0xff60 | 0xffe0..=0xffe6 | 0x1b000..=0x1b2ff |
        0x1f000..=0x1faff | 0x20000..=0x3ffff)
}
