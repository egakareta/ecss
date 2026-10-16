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

const ACTIVE: &str = "ecss.layout.active";
const LABEL_SELECTION_PRESS: &str = "ecss.text_selection.press";

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
    let id = ui.next_auto_id().with("ecss.layout");
    ui.skip_ahead_auto_ids(1);
    let active = ui.ctx().data(|d| d.get_temp::<u64>(Id::new(ACTIVE)));
    let active = active.and_then(|key| FRAMES.with(|frames| frames.borrow().get(&key).cloned()));
    let root = active.is_none();
    if root {
        clear_previous_label_selection(ui.ctx());
    }
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
    style.format.font_kerning = parent_style.format.font_kerning;
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
    let base_font_size = if root {
        ui.style().text_styles[&egui::TextStyle::Body].size
    } else {
        parent_style.format.em
    };
    style.format.em = base_font_size;
    style.format.font_size_set = parent_style.format.font_size_set;
    let mut units = Units {
        math: &math,
        font: base_font_size,
        root_font: 16.0,
        viewport: ui.ctx().viewport_rect().size(),
    };
    for rule in matching_rules(stylesheet, class).filter(|r| {
        r.state == RuleState::Base
            || r.state == RuleState::Hovered && state & 1 != 0
            || r.state == RuleState::Active && state & 2 != 0
    }) {
        for declaration in &rule.declarations {
            if let Declaration::Layout(declaration) = declaration
                && declaration.name == "font-size"
            {
                declaration
                    .apply_with_parent(&mut style, &parent_style, units)
                    .expect("validated font-size declaration");
            }
        }
    }
    units.font = style.format.em;

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
                    if declaration.name == "font-size" {
                        continue;
                    }
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
    let disable_kerning = style.format.font_kerning == crate::formatting::FontKerning::None;
    for font in content_ui.style_mut().text_styles.values_mut() {
        font.family =
            crate::fonts::FontAssets::family_for_kerning(ui.ctx(), &font.family, disable_kerning);
    }
    if matches!(style.format.white_space.as_str(), "nowrap" | "pre") {
        content_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    }
    if style.format.font_size_set {
        for font in content_ui.style_mut().text_styles.values_mut() {
            font.size = style.format.em;
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
        .map(|text| {
            reflow_text(
                ui.ctx(),
                layer,
                start,
                end,
                &text.job,
                content_ui.visuals().selection.bg_fill,
            )
        })
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
                content_ui.visuals(),
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
        let frame_nr = ui.ctx().cumulative_frame_nr();
        // Scroll offsets are stored after descendants run, so defer geometry correction
        // to the next frame.
        let scroll_changed_last_frame = frame_nr.checked_sub(1).is_some_and(|previous_frame| {
            ui.ctx().data(|data| {
                frame.nodes.iter().any(|node| {
                    data.get_temp::<u64>(node.id.with("scroll-updated")) == Some(previous_frame)
                })
            })
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, cache));
        if changed {
            if scroll_changed_last_frame {
                ui.ctx().request_repaint();
            } else {
                ui.ctx().request_discard("CSS selector layout changed");
            }
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
        let interaction = without_id_clash_warning(ui.ctx(), || {
            content_ui.interact(visual, response.id, response.sense)
        });
        response.union(interaction).with_new_rect(visual)
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
    let interaction_layout_changed = [(1, RuleState::Hovered), (2, RuleState::Active)]
        .into_iter()
        .any(|(mask, rule_state)| {
            state & mask != current_state & mask
                && matching_rules(stylesheet, class).any(|rule| {
                    rule.state == rule_state
                        && rule
                            .declarations
                            .iter()
                            .any(|declaration| matches!(declaration, Declaration::Layout(_)))
                })
        });
    if interaction_layout_changed {
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
        let background_style = resolved_background(stylesheet, class, current_state);
        let corner_radius = if frame_stroke.is_some()
            || matching_rules(stylesheet, class).any(|r| {
                (r.state == RuleState::Base
                    || r.state == RuleState::Hovered && current_state & 1 != 0
                    || r.state == RuleState::Active && current_state & 2 != 0)
                    && r.declarations.iter().any(|declaration| {
                        matches!(declaration, Declaration::BorderRadius(_))
                            || matches!(declaration, Declaration::Layout(layout) if layout.name == "border-radius")
                    })
            }) {
            visuals.corner_radius
        } else {
            egui::CornerRadius::ZERO
        };
        let fill = background_style.color;
        let last_background_layer = background_style.images.len().saturating_sub(1);
        let color_clip = if background_style.clips.is_empty() {
            crate::css::BackgroundClip::Border
        } else {
            let clip_index = if last_background_layer < background_style.clips.len() {
                last_background_layer
            } else {
                last_background_layer % background_style.clips.len()
            };
            background_style.clips[clip_index]
        };
        let shape_clip_for = |clip| {
            style.decorations.border_shape.as_ref().map(|border_shape| {
                let border_widths = style
                    .layout
                    .border
                    .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
                let (paths, fill_rule) = border_shape.background_paths(
                    border,
                    clip,
                    final_geometry.stroke,
                    final_geometry.padding,
                    final_geometry.margin,
                    [
                        border_widths.top,
                        border_widths.right,
                        border_widths.bottom,
                        border_widths.left,
                    ],
                );
                crate::box_decoration::path_clip(paths, fill_rule)
            })
        };
        let frame_stroke = if style.border_set || final_geometry.collapsed.is_some() {
            egui::Stroke::NONE
        } else {
            frame_stroke.unwrap_or(egui::Stroke::NONE)
        };
        let mut surfaces = Vec::new();
        let inline_fragmented = style.format.display == crate::formatting::DisplayKind::Inline
            && !final_geometry.fragments.is_empty();
        let (inline_vertical, inline_forward) = if inline_fragmented {
            let frame = active.0.borrow();
            frame.nodes[index].parent.map_or(
                (
                    style.format.writing.vertical(),
                    style.layout.direction != taffy::Direction::Rtl,
                ),
                |parent| {
                    let parent_style = &frame.nodes[parent].style;
                    (
                        parent_style.format.writing.vertical(),
                        parent_style.layout.direction != taffy::Direction::Rtl,
                    )
                },
            )
        } else {
            (false, true)
        };
        if inline_fragmented {
            for fragment in &final_geometry.fragments {
                let rect = fragment.rect.translate(border.min.to_vec2());
                if let Some(last) = surfaces.last_mut()
                    && same_line(*last, rect, inline_vertical)
                {
                    *last = last.union(rect);
                } else {
                    surfaces.push(rect);
                }
            }
        } else {
            surfaces.push(border);
        }
        let suppress_outer_shadow = final_geometry.collapsed.is_some()
            && matches!(
                style.format.display,
                crate::formatting::DisplayKind::Row
                    | crate::formatting::DisplayKind::Cell
                    | crate::formatting::DisplayKind::RowGroup
                    | crate::formatting::DisplayKind::Header
                    | crate::formatting::DisplayKind::Footer
                    | crate::formatting::DisplayKind::Column
                    | crate::formatting::DisplayKind::ColumnGroup
            );
        let surface_count = surfaces.len();
        let mut shapes = Vec::new();
        if !suppress_outer_shadow && final_geometry.slices.is_empty() {
            for (surface_index, surface) in surfaces.iter().copied().enumerate() {
                let shadow = crate::box_decoration::shadow_shapes(
                    if inline_fragmented { surface } else { border },
                    &style,
                    visuals.fg_stroke.color,
                    crate::box_decoration::ShadowPosition::Outset,
                    final_geometry.stroke,
                    final_geometry.padding,
                    final_geometry.margin,
                );
                if inline_fragmented && !style.format.clone_decoration && surface_count > 1 {
                    let clip = fragment_shadow_clip(
                        surface,
                        ui.clip_rect(),
                        inline_vertical,
                        inline_forward,
                        surface_index,
                        surface_count,
                        crate::box_decoration::ShadowPosition::Outset,
                        0.0,
                        final_geometry.stroke,
                    );
                    shapes.extend(clip_fragment_shadows(ui.ctx(), shadow, clip));
                } else {
                    shapes.extend(shadow);
                }
            }
        }
        for (surface_index, surface) in surfaces.into_iter().enumerate() {
            let color_surface = match color_clip {
                crate::css::BackgroundClip::Border => surface,
                crate::css::BackgroundClip::Padding => surface - final_geometry.stroke,
                crate::css::BackgroundClip::Content => {
                    surface - final_geometry.stroke - final_geometry.padding
                }
            };
            shapes.push(box_surface_shape(
                color_surface,
                border,
                &style,
                final_geometry.stroke,
                final_geometry.padding,
                final_geometry.margin,
                color_clip,
                fill,
                corner_radius,
            ));
            let clip_count = background_style.clips.len().max(1);
            let attachment_count = background_style.attachments.len().max(1);
            for layer_index in (0..background_style.images.len()).rev() {
                let layer_image = &background_style.images[layer_index];
                let layer_clip = background_style
                    .clips
                    .get(layer_index % clip_count)
                    .copied()
                    .unwrap_or(crate::css::BackgroundClip::Border);
                let attachment = background_style
                    .attachments
                    .get(layer_index % attachment_count)
                    .copied()
                    .unwrap_or(crate::css::BackgroundAttachment::Scroll);
                let background_surface = match layer_clip {
                    crate::css::BackgroundClip::Border => surface,
                    crate::css::BackgroundClip::Padding => surface - final_geometry.stroke,
                    crate::css::BackgroundClip::Content => {
                        surface - final_geometry.stroke - final_geometry.padding
                    }
                };
                let shape_clip = shape_clip_for(layer_clip);
                let image_resolution =
                    layer_image.resolution_for_scale(ui.ctx().pixels_per_point());
                let background_image = layer_image.for_scale(ui.ctx().pixels_per_point());
                if let crate::css::BackgroundImage::SolidColor(color) = background_image {
                    shapes.push(clip_background_shape(
                        box_surface_shape(
                            background_surface,
                            border,
                            &style,
                            final_geometry.stroke,
                            final_geometry.padding,
                            final_geometry.margin,
                            layer_clip,
                            *color,
                            corner_radius,
                        ),
                        shape_clip.as_ref(),
                    ));
                }
                if let crate::css::BackgroundImage::RadialGradient(gradient) = background_image {
                    let paint_area = match attachment {
                        crate::css::BackgroundAttachment::Scroll => background_surface,
                        crate::css::BackgroundAttachment::Local => {
                            background_surface.translate(-scroll)
                        }
                        crate::css::BackgroundAttachment::Fixed => ui.ctx().viewport_rect(),
                    };
                    shapes.push(clip_background_shape(
                        egui::Shape::Mesh(
                            radial_gradient_mesh(background_surface, paint_area, gradient).into(),
                        ),
                        shape_clip.as_ref(),
                    ));
                }
                if let crate::css::BackgroundImage::LinearGradient(gradient) = background_image {
                    let paint_area = match attachment {
                        crate::css::BackgroundAttachment::Scroll => background_surface,
                        crate::css::BackgroundAttachment::Local => {
                            background_surface.translate(-scroll)
                        }
                        crate::css::BackgroundAttachment::Fixed => ui.ctx().viewport_rect(),
                    };
                    shapes.push(clip_background_shape(
                        egui::Shape::Mesh(
                            linear_gradient_mesh(background_surface, paint_area, gradient).into(),
                        ),
                        shape_clip.as_ref(),
                    ));
                }
                if let crate::css::BackgroundImage::ConicGradient(gradient) = background_image {
                    let paint_area = match attachment {
                        crate::css::BackgroundAttachment::Scroll => background_surface,
                        crate::css::BackgroundAttachment::Local => {
                            background_surface.translate(-scroll)
                        }
                        crate::css::BackgroundAttachment::Fixed => ui.ctx().viewport_rect(),
                    };
                    shapes.push(clip_background_shape(
                        egui::Shape::Mesh(
                            conic_gradient_mesh(background_surface, paint_area, gradient).into(),
                        ),
                        shape_clip.as_ref(),
                    ));
                }
                let image_texture = match background_image {
                    crate::css::BackgroundImage::Url(url) => {
                        match (
                            stylesheet.background_images.texture(ui.ctx(), url),
                            stylesheet.background_images.size(url),
                        ) {
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
                        ui.ctx(),
                        &stylesheet.background_images,
                        background_image,
                        background_surface.size(),
                        ui.ctx().pixels_per_point() / image_resolution,
                    )
                    .map(|(texture, size)| (texture, size / image_resolution)),
                    _ => None,
                };
                if let Some((texture, image_size)) = image_texture {
                    let viewport = ui.ctx().viewport_rect();
                    let origin = match attachment {
                        crate::css::BackgroundAttachment::Scroll => background_surface.min,
                        crate::css::BackgroundAttachment::Local => background_surface.min - scroll,
                        crate::css::BackgroundAttachment::Fixed => viewport.min,
                    };
                    let min_x =
                        ((background_surface.left() - origin.x) / image_size.x).floor() as i32;
                    let max_x =
                        ((background_surface.right() - origin.x) / image_size.x).ceil() as i32;
                    let min_y =
                        ((background_surface.top() - origin.y) / image_size.y).floor() as i32;
                    let max_y =
                        ((background_surface.bottom() - origin.y) / image_size.y).ceil() as i32;
                    for y in min_y..max_y {
                        for x in min_x..max_x {
                            let tile = Rect::from_min_size(
                                origin + vec2(x as f32 * image_size.x, y as f32 * image_size.y),
                                image_size,
                            );
                            let visible = tile.intersect(background_surface);
                            if visible.is_positive() {
                                let uv = Rect::from_min_max(
                                    pos2(
                                        (visible.left() - tile.left()) / tile.width(),
                                        (visible.top() - tile.top()) / tile.height(),
                                    ),
                                    pos2(
                                        (visible.right() - tile.left()) / tile.width(),
                                        (visible.bottom() - tile.top()) / tile.height(),
                                    ),
                                );
                                shapes.push(clip_background_shape(
                                    egui::Shape::image(texture.id(), visible, uv, Color32::WHITE),
                                    shape_clip.as_ref(),
                                ));
                            }
                        }
                    }
                }
            }
            let inset_shadow = crate::box_decoration::shadow_shapes(
                surface,
                &style,
                visuals.fg_stroke.color,
                crate::box_decoration::ShadowPosition::Inset,
                final_geometry.stroke,
                final_geometry.padding,
                final_geometry.margin,
            );
            if inline_fragmented && !style.format.clone_decoration && surface_count > 1 {
                let clip = fragment_shadow_clip(
                    surface,
                    ui.clip_rect(),
                    inline_vertical,
                    inline_forward,
                    surface_index,
                    surface_count,
                    crate::box_decoration::ShadowPosition::Inset,
                    crate::box_decoration::inset_shadow_extent(surface, &style),
                    final_geometry.stroke,
                );
                shapes.extend(clip_fragment_shadows(ui.ctx(), inset_shadow, clip));
            } else {
                shapes.extend(inset_shadow);
            }
            if frame_stroke.width > 0.0 {
                shapes.push(egui::Shape::Rect(egui::epaint::RectShape::new(
                    surface,
                    corner_radius,
                    Color32::TRANSPARENT,
                    frame_stroke,
                    egui::StrokeKind::Inside,
                )));
            }
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
                if let Some(border_image) = border_image_shapes(
                    surface,
                    &fragment_style,
                    &stylesheet.background_images,
                    ui.ctx(),
                ) {
                    if let Some(border_shape) = &fragment_style.decorations.border_shape {
                        let border_widths = fragment_style
                            .layout
                            .border
                            .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
                        let (paths, fill_rule) = border_shape.background_paths(
                            surface,
                            crate::css::BackgroundClip::Border,
                            final_geometry.stroke,
                            final_geometry.padding,
                            final_geometry.margin,
                            [
                                border_widths.top,
                                border_widths.right,
                                border_widths.bottom,
                                border_widths.left,
                            ],
                        );
                        let clip = crate::box_decoration::path_clip(paths, fill_rule);
                        for shape in border_image {
                            shapes.extend(clip_shape_to_paths(ui.ctx(), shape, &clip));
                        }
                    } else {
                        shapes.extend(border_image);
                    }
                } else if let Some(border_shape) = border_shape_shapes(
                    surface,
                    &fragment_style,
                    final_geometry.stroke,
                    final_geometry.padding,
                    final_geometry.margin,
                    visuals.fg_stroke.color,
                ) {
                    shapes.extend(border_shape);
                } else {
                    shapes.extend(border_shapes(
                        surface,
                        &fragment_style,
                        visuals.fg_stroke.color,
                    ));
                }
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

fn without_id_clash_warning<R>(context: &Context, f: impl FnOnce() -> R) -> R {
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

fn clear_previous_label_selection(context: &Context) {
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
    let last_press = context.data(|data| data.get_temp::<u64>(Id::new(LABEL_SELECTION_PRESS)));
    // egui can rerun this input frame for additional layout passes.
    if last_press == Some(press_time) {
        return;
    }
    context.data_mut(|data| data.insert_temp(Id::new(LABEL_SELECTION_PRESS), press_time));
    context
        .plugin::<egui::text_selection::LabelSelectionState>()
        .lock()
        .clear_selection();
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
    let border_clips = frame
        .nodes
        .iter()
        .map(border_content_clip)
        .collect::<Vec<_>>();
    let mut ancestor_clips = vec![Vec::new(); count];
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
        let decoration = index + root.start.0 == paint.start.0;
        let mut clips = ancestor_clips[owner]
            .iter()
            .filter_map(|ancestor| border_clips[*ancestor].as_ref())
            .collect::<Vec<_>>();
        if !decoration {
            clips.extend(border_clips[owner].as_ref());
        }
        let is_overlay = index + root.start.0 >= paint.overlay.0;
        if is_overlay {
            overlays[owner].extend(clip_painted_entry(context, entry, &clips));
        } else if paint.geometry.slices.is_empty() {
            grouped[owner].extend(clip_painted_entry(context, entry, &clips));
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
                                Some(paint.clip)
                            } else {
                                fragment_shadow_clip(
                                    destination,
                                    paint.clip,
                                    !node.style.format.writing.vertical(),
                                    !node.style.format.writing.reverse(),
                                    slice_index,
                                    slices.len(),
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
                                    grouped[owner].extend(clip_painted_entry(
                                        context,
                                        egui::epaint::ClippedShape {
                                            clip_rect: shadow_clip,
                                            shape: shadow,
                                        },
                                        &clips,
                                    ));
                                }
                            }
                        }
                    }
                }
                let clip = entry
                    .clip_rect
                    .intersect(source)
                    .translate(slice.offset)
                    .intersect(paint.clip);
                if !clip.is_positive() {
                    continue;
                }
                let mut shape = if decoration && node.style.format.clone_decoration {
                    clone_decoration(
                        &entry.shape,
                        source,
                        &node.style,
                        node.color,
                        paint.geometry.stroke,
                        paint.geometry.padding,
                        paint.geometry.margin,
                    )
                } else {
                    entry.shape.clone()
                };
                shape.transform(egui::emath::TSTransform::from_translation(slice.offset));
                grouped[owner].extend(clip_painted_entry(
                    context,
                    egui::epaint::ClippedShape {
                        clip_rect: clip,
                        shape,
                    },
                    &clips,
                ));
            }
        }
    }
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        for index in root.start.0..root.end.0 {
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

fn clone_decoration(
    shape: &egui::Shape,
    rect: Rect,
    style: &BoxStyle,
    color: Color32,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
) -> egui::Shape {
    if let egui::Shape::Vec(shapes) = shape
        && let Some(egui::Shape::Rect(background)) = shapes.first()
    {
        let mut background = background.clone();
        background.rect = rect;
        let mut shapes = vec![egui::Shape::Rect(background)];
        shapes.extend(crate::box_decoration::shadow_shapes(
            rect,
            style,
            color,
            crate::box_decoration::ShadowPosition::Inset,
            stroke,
            padding,
            margin,
        ));
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
        let frame_nr = ui.ctx().cumulative_frame_nr();
        ui.ctx().data_mut(|data| {
            data.insert_temp(id.with("scroll"), offset);
            data.insert_temp(id.with("scroll-updated"), frame_nr);
        });
        ui.ctx().request_repaint();
    }
    responses
}

fn matching_rules<'a>(
    stylesheet: &'a Stylesheet,
    class: &'a str,
) -> impl Iterator<Item = &'a crate::css::Rule> {
    let rules = &stylesheet.rules;
    rules
        .iter()
        .filter(|rule| rule.class.is_none() && rule.state == RuleState::Base)
        .chain(rules.iter().filter(move |rule| {
            (rule.class.is_some()
                && rule.class.as_deref() == Some(class)
                && rule.state == RuleState::Base)
                || (rule.class.is_none() && rule.state != RuleState::Base)
        }))
        .chain(rules.iter().filter(move |rule| {
            rule.class.is_some()
                && rule.class.as_deref() == Some(class)
                && rule.state != RuleState::Base
        }))
}

struct BackgroundPaint {
    color: Color32,
    images: Vec<crate::css::BackgroundImage>,
    attachments: Vec<crate::css::BackgroundAttachment>,
    clips: Vec<crate::css::BackgroundClip>,
}

fn resolved_background(stylesheet: &Stylesheet, class: &str, state: u8) -> BackgroundPaint {
    let mut background = BackgroundPaint {
        color: Color32::TRANSPARENT,
        images: vec![crate::css::BackgroundImage::None],
        attachments: vec![crate::css::BackgroundAttachment::Scroll],
        clips: vec![crate::css::BackgroundClip::Border],
    };
    for rule in matching_rules(stylesheet, class) {
        if rule.state != RuleState::Base
            && !(rule.state == RuleState::Hovered && state & 1 != 0)
            && !(rule.state == RuleState::Active && state & 2 != 0)
        {
            continue;
        }
        for declaration in &rule.declarations {
            match declaration {
                Declaration::Background(color) => background.color = *color,
                Declaration::BackgroundShorthand(value) => {
                    background.color = value.color;
                    background.images = value.images.clone();
                    background.attachments = value.attachments.clone();
                    background.clips = value.clips.clone();
                }
                Declaration::BackgroundImage(images) => background.images = images.clone(),
                Declaration::BackgroundAttachment(attachments) => {
                    background.attachments = attachments.clone();
                }
                Declaration::BackgroundClip(clips) => background.clips = clips.clone(),
                _ => {}
            }
        }
    }
    background
}

fn cross_fade_texture(
    context: &Context,
    images: &crate::background::ImageAssets,
    image: &crate::css::BackgroundImage,
    fallback_size: Vec2,
    scale: f32,
) -> Option<(egui::TextureHandle, Vec2)> {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let logical_size = intrinsic_image_size(image, images, scale).unwrap_or(fallback_size);
    let logical_size = logical_size.max(Vec2::splat(1.0));
    let width = (logical_size.x * scale).ceil().clamp(1.0, 4096.0) as usize;
    let height = (logical_size.y * scale).ceil().clamp(1.0, 4096.0) as usize;
    let key = egui::Id::new((
        "ecss.cross-fade",
        format!("{image:?}:{scale}:{logical_size:?}"),
    ));
    if let Some(texture) = context.data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return Some((texture, logical_size));
    }
    let area = Rect::from_min_size(Pos2::ZERO, logical_size);
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                sample_image(
                    images,
                    image,
                    scale,
                    (x as f32 + 0.5) / width as f32,
                    (y as f32 + 0.5) / height as f32,
                    area,
                )
            })
        })
        .collect();
    let texture = context.load_texture(
        key.value().to_string(),
        egui::ColorImage::new([width, height], pixels),
        egui::TextureOptions::LINEAR,
    );
    context.data_mut(|data| data.insert_temp(key, texture.clone()));
    Some((texture, logical_size))
}

fn intrinsic_image_size(
    image: &crate::css::BackgroundImage,
    images: &crate::background::ImageAssets,
    scale: f32,
) -> Option<Vec2> {
    let selected = image.for_scale(scale);
    let density = image.resolution_for_scale(scale);
    let size = match selected {
        crate::css::BackgroundImage::Url(url) => {
            let size = images.size(url)?;
            vec2(size[0] as f32, size[1] as f32)
        }
        crate::css::BackgroundImage::CrossFade(cross_fade) => {
            let mut weighted_size = Vec2::ZERO;
            let mut total_weight = 0.0;
            for item in &cross_fade.items {
                if let Some(size) = intrinsic_image_size(&item.image, images, scale) {
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

fn sample_image(
    images: &crate::background::ImageAssets,
    image: &crate::css::BackgroundImage,
    scale: f32,
    u: f32,
    v: f32,
    area: Rect,
) -> Color32 {
    match image.for_scale(scale) {
        crate::css::BackgroundImage::None | crate::css::BackgroundImage::ImageSet(_) => {
            Color32::TRANSPARENT
        }
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
                let color = sample_image(images, &item.image, scale, u, v, area);
                let color = color.to_array();
                let alpha = f32::from(color[3]) / 255.0;
                let weight = item.weight / normalization;
                for channel in 0..3 {
                    premultiplied[channel] += f32::from(color[channel]) / 255.0 * alpha * weight;
                }
                premultiplied[3] += alpha * weight;
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
            Color32::from_rgba_unmultiplied(
                channels[0],
                channels[1],
                channels[2],
                (alpha * 255.0).round() as u8,
            )
        }
    }
}

fn linear_gradient_mesh(
    clip: Rect,
    paint_area: Rect,
    gradient: &crate::css::LinearGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    let steps = 32;
    let radians = gradient.angle.to_radians();
    let direction = vec2(radians.sin(), -radians.cos());
    let corners = [
        paint_area.left_top(),
        paint_area.right_top(),
        paint_area.right_bottom(),
        paint_area.left_bottom(),
    ];
    let min = corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::INFINITY, f32::min);
    let max = corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::NEG_INFINITY, f32::max);
    let mut mesh = Mesh::default();
    for y in 0..=steps {
        for x in 0..=steps {
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), x as f32 / steps as f32),
                egui::emath::lerp(clip.top()..=clip.bottom(), y as f32 / steps as f32),
            );
            let t = ((point.to_vec2().dot(direction) - min) / (max - min).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: gradient_color(&gradient.stops, t, gradient.repeating, gradient.color_space),
            });
        }
    }
    for y in 0..steps {
        for x in 0..steps {
            let top_left = y * (steps + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + steps + 1;
            let bottom_right = bottom_left + 1;
            mesh.add_triangle(top_left as u32, top_right as u32, bottom_right as u32);
            mesh.add_triangle(top_left as u32, bottom_right as u32, bottom_left as u32);
        }
    }
    mesh
}

fn linear_gradient_slice_mesh(
    clip: Rect,
    source: Rect,
    image_size: Vec2,
    gradient: &crate::css::LinearGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    let steps = 32;
    let radians = gradient.angle.to_radians();
    let direction = vec2(radians.sin(), -radians.cos());
    let image = Rect::from_min_size(Pos2::ZERO, image_size);
    let corners = [
        image.left_top(),
        image.right_top(),
        image.right_bottom(),
        image.left_bottom(),
    ];
    let min = corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::INFINITY, f32::min);
    let max = corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::NEG_INFINITY, f32::max);
    let mut mesh = Mesh::default();
    for y in 0..=steps {
        for x in 0..=steps {
            let u = x as f32 / steps as f32;
            let v = y as f32 / steps as f32;
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), u),
                egui::emath::lerp(clip.top()..=clip.bottom(), v),
            );
            let source_point = pos2(
                egui::emath::lerp(source.left()..=source.right(), u),
                egui::emath::lerp(source.top()..=source.bottom(), v),
            );
            let t = ((source_point.to_vec2().dot(direction) - min) / (max - min).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: gradient_color(&gradient.stops, t, gradient.repeating, gradient.color_space),
            });
        }
    }
    for y in 0..steps {
        for x in 0..steps {
            let top_left = y * (steps + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + steps + 1;
            let bottom_right = bottom_left + 1;
            mesh.add_triangle(top_left as u32, top_right as u32, bottom_right as u32);
            mesh.add_triangle(top_left as u32, bottom_right as u32, bottom_left as u32);
        }
    }
    mesh
}

fn radial_gradient_mesh(
    clip: Rect,
    paint_area: Rect,
    gradient: &crate::css::RadialGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 48;
    let (center, radii, corner_extent) = radial_gradient_parameters(paint_area, gradient);
    let mut mesh = Mesh::default();
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), x as f32 / STEPS as f32),
                egui::emath::lerp(clip.top()..=clip.bottom(), y as f32 / STEPS as f32),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: gradient_color(
                    &gradient.stops,
                    radial_gradient_position(point, center, radii, corner_extent),
                    gradient.repeating,
                    gradient.color_space,
                ),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

fn radial_gradient_slice_mesh(
    clip: Rect,
    source: Rect,
    image_size: Vec2,
    gradient: &crate::css::RadialGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 32;
    let image = Rect::from_min_size(Pos2::ZERO, image_size);
    let (center, radii, corner_extent) = radial_gradient_parameters(image, gradient);
    let mut mesh = Mesh::default();
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let u = x as f32 / STEPS as f32;
            let v = y as f32 / STEPS as f32;
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), u),
                egui::emath::lerp(clip.top()..=clip.bottom(), v),
            );
            let source_point = pos2(
                egui::emath::lerp(source.left()..=source.right(), u),
                egui::emath::lerp(source.top()..=source.bottom(), v),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: gradient_color(
                    &gradient.stops,
                    radial_gradient_position(source_point, center, radii, corner_extent),
                    gradient.repeating,
                    gradient.color_space,
                ),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

fn radial_gradient_parameters(
    area: Rect,
    gradient: &crate::css::RadialGradient,
) -> (Pos2, Vec2, bool) {
    let center = pos2(
        area.left() + gradient.center[0].resolve(area.width()),
        area.top() + gradient.center[1].resolve(area.height()),
    );
    match gradient.size {
        crate::css::RadialSize::Explicit(values) => {
            let radius = |value: crate::css::RadialRadius, axis: usize| match value {
                crate::css::RadialRadius::Length(value) => value,
                crate::css::RadialRadius::Percentage(value) => {
                    if gradient.shape == crate::css::RadialShape::Circle {
                        value * (area.width().powi(2) + area.height().powi(2)).sqrt()
                            / std::f32::consts::SQRT_2
                    } else if axis == 0 {
                        value * area.width()
                    } else {
                        value * area.height()
                    }
                }
            };
            let x = radius(values[0], 0);
            let y = if gradient.shape == crate::css::RadialShape::Circle {
                x
            } else {
                radius(values[1], 1)
            };
            (center, vec2(x, y), false)
        }
        crate::css::RadialSize::Extent(extent) => {
            let horizontal = [center.x - area.left(), area.right() - center.x];
            let vertical = [center.y - area.top(), area.bottom() - center.y];
            let corner_extent = matches!(
                extent,
                crate::css::RadialExtent::ClosestCorner | crate::css::RadialExtent::FarthestCorner
            );
            if gradient.shape == crate::css::RadialShape::Circle {
                let radius = match extent {
                    crate::css::RadialExtent::ClosestSide => horizontal
                        .into_iter()
                        .chain(vertical)
                        .fold(f32::INFINITY, f32::min),
                    crate::css::RadialExtent::FarthestSide => {
                        horizontal.into_iter().chain(vertical).fold(0.0, f32::max)
                    }
                    crate::css::RadialExtent::ClosestCorner
                    | crate::css::RadialExtent::FarthestCorner => {
                        let distances = [
                            vec2(horizontal[0], vertical[0]).length(),
                            vec2(horizontal[1], vertical[0]).length(),
                            vec2(horizontal[1], vertical[1]).length(),
                            vec2(horizontal[0], vertical[1]).length(),
                        ];
                        if extent == crate::css::RadialExtent::ClosestCorner {
                            distances.into_iter().fold(f32::INFINITY, f32::min)
                        } else {
                            distances.into_iter().fold(0.0, f32::max)
                        }
                    }
                };
                (center, Vec2::splat(radius), false)
            } else if corner_extent {
                let distances = [
                    (horizontal[0], vertical[0]),
                    (horizontal[1], vertical[0]),
                    (horizontal[1], vertical[1]),
                    (horizontal[0], vertical[1]),
                ];
                let selected = if extent == crate::css::RadialExtent::ClosestCorner {
                    distances.into_iter().min_by(|first, second| {
                        (first.0 * first.0 + first.1 * first.1)
                            .total_cmp(&(second.0 * second.0 + second.1 * second.1))
                    })
                } else {
                    distances.into_iter().max_by(|first, second| {
                        (first.0 * first.0 + first.1 * first.1)
                            .total_cmp(&(second.0 * second.0 + second.1 * second.1))
                    })
                }
                .unwrap_or((0.0, 0.0));
                (center, vec2(selected.0, selected.1), true)
            } else {
                let choose = |values: [f32; 2]| {
                    if extent == crate::css::RadialExtent::ClosestSide {
                        values.into_iter().fold(f32::INFINITY, f32::min)
                    } else {
                        values.into_iter().fold(0.0, f32::max)
                    }
                };
                (center, vec2(choose(horizontal), choose(vertical)), false)
            }
        }
    }
}

fn radial_gradient_position(point: Pos2, center: Pos2, radii: Vec2, corner_extent: bool) -> f32 {
    if point == center {
        return 0.0;
    }
    let distance = vec2(
        (point.x - center.x) / radii.x.max(f32::EPSILON),
        (point.y - center.y) / radii.y.max(f32::EPSILON),
    )
    .length();
    if corner_extent {
        distance / std::f32::consts::SQRT_2
    } else {
        distance
    }
}

fn conic_gradient_mesh(
    clip: Rect,
    paint_area: Rect,
    gradient: &crate::css::ConicGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 64;
    let center = pos2(
        paint_area.left() + gradient.center[0].resolve(paint_area.width()),
        paint_area.top() + gradient.center[1].resolve(paint_area.height()),
    );
    let mut mesh = Mesh::default();
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), x as f32 / STEPS as f32),
                egui::emath::lerp(clip.top()..=clip.bottom(), y as f32 / STEPS as f32),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: gradient_color(
                    &gradient.stops,
                    conic_gradient_position(point, center, gradient.from_angle),
                    gradient.repeating,
                    gradient.color_space,
                ),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

fn conic_gradient_slice_mesh(
    clip: Rect,
    source: Rect,
    image_size: Vec2,
    gradient: &crate::css::ConicGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 32;
    let center = pos2(
        gradient.center[0].resolve(image_size.x),
        gradient.center[1].resolve(image_size.y),
    );
    let mut mesh = Mesh::default();
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let u = x as f32 / STEPS as f32;
            let v = y as f32 / STEPS as f32;
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), u),
                egui::emath::lerp(clip.top()..=clip.bottom(), v),
            );
            let source_point = pos2(
                egui::emath::lerp(source.left()..=source.right(), u),
                egui::emath::lerp(source.top()..=source.bottom(), v),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: gradient_color(
                    &gradient.stops,
                    conic_gradient_position(source_point, center, gradient.from_angle),
                    gradient.repeating,
                    gradient.color_space,
                ),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

fn conic_gradient_position(point: Pos2, center: Pos2, from_angle: f32) -> f32 {
    if point == center {
        return 0.0;
    }
    let angle = (point.y - center.y).atan2(point.x - center.x).to_degrees() + 90.0 - from_angle;
    angle.rem_euclid(360.0) / 360.0
}

fn add_grid_triangles(mesh: &mut egui::epaint::Mesh, steps: usize) {
    for y in 0..steps {
        for x in 0..steps {
            let top_left = y * (steps + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + steps + 1;
            let bottom_right = bottom_left + 1;
            mesh.add_triangle(top_left as u32, top_right as u32, bottom_right as u32);
            mesh.add_triangle(top_left as u32, bottom_right as u32, bottom_left as u32);
        }
    }
}

fn gradient_color(
    stops: &[crate::css::GradientStop],
    position: f32,
    repeating: bool,
    color_space: crate::css::GradientColorSpace,
) -> Color32 {
    let mut positions = stops.iter().map(|stop| stop.position).collect::<Vec<_>>();
    if positions[0].is_none() {
        positions[0] = Some(0.0);
    }
    let last = positions.len() - 1;
    if positions[last].is_none() {
        positions[last] = Some(1.0);
    }
    let mut previous_fixed = 0;
    for index in 1..positions.len() {
        if let Some(value) = positions[index] {
            let previous = positions[previous_fixed].unwrap_or(0.0);
            let value = value.max(previous);
            positions[index] = Some(value);
            let gap = index - previous_fixed;
            for missing in 1..gap {
                let amount = missing as f32 / gap as f32;
                positions[previous_fixed + missing] =
                    Some(egui::emath::lerp(previous..=value, amount));
            }
            previous_fixed = index;
        }
    }

    let stops = stops
        .iter()
        .zip(positions)
        .map(|(stop, position)| (position.unwrap_or(0.0), stop.color, stop.hint_after))
        .collect::<Vec<_>>();
    let position = if repeating {
        let start = stops[0].0;
        let end = stops[stops.len() - 1].0;
        let period = end - start;
        if period <= f32::EPSILON {
            return stops[stops.len() - 1].1;
        }
        (position - start).rem_euclid(period) + start
    } else {
        position
    };
    let after = stops
        .iter()
        .position(|(stop_position, _, _)| *stop_position >= position)
        .unwrap_or(stops.len() - 1);
    if after == 0 {
        return stops[0].1;
    }
    let (start, start_color, hint) = stops[after - 1];
    let (end, end_color, _) = stops[after];
    let mut amount = ((position - start) / (end - start).max(f32::EPSILON)).clamp(0.0, 1.0);
    if let Some(hint) = hint {
        let hint_amount = ((hint - start) / (end - start).max(f32::EPSILON)).clamp(0.0, 1.0);
        amount = if hint_amount <= f32::EPSILON {
            if amount <= 0.0 { 0.0 } else { 1.0 }
        } else if hint_amount >= 1.0 - f32::EPSILON {
            if amount >= 1.0 { 1.0 } else { 0.0 }
        } else {
            amount.powf(0.5_f32.ln() / hint_amount.ln())
        };
    }
    interpolate_gradient_colors(start_color, end_color, amount, color_space)
}

fn interpolate_gradient_colors(
    start: Color32,
    end: Color32,
    amount: f32,
    color_space: crate::css::GradientColorSpace,
) -> Color32 {
    let start_rgba = start.to_array();
    let end_rgba = end.to_array();
    let start_alpha = f32::from(start_rgba[3]) / 255.0;
    let end_alpha = f32::from(end_rgba[3]) / 255.0;
    let alpha = egui::emath::lerp(start_alpha..=end_alpha, amount);
    let start_color = gradient_color_components(start_rgba, color_space);
    let end_color = gradient_color_components(end_rgba, color_space);
    let mut color = std::array::from_fn(|index| {
        egui::emath::lerp(
            (start_color[index] * start_alpha)..=(end_color[index] * end_alpha),
            amount,
        )
    });
    if alpha > f32::EPSILON {
        color.iter_mut().for_each(|component| *component /= alpha);
    } else {
        color = [0.0; 3];
    }
    let channels = gradient_color_channels(color, alpha, color_space);
    Color32::from_rgba_unmultiplied(channels[0], channels[1], channels[2], channels[3])
}

fn gradient_color_components(
    rgba: [u8; 4],
    color_space: crate::css::GradientColorSpace,
) -> [f32; 3] {
    let srgb = [
        f32::from(rgba[0]) / 255.0,
        f32::from(rgba[1]) / 255.0,
        f32::from(rgba[2]) / 255.0,
    ];
    match color_space {
        crate::css::GradientColorSpace::Srgb => srgb,
        crate::css::GradientColorSpace::SrgbLinear => srgb.map(srgb_to_linear),
        crate::css::GradientColorSpace::Oklab => {
            let [red, green, blue] = srgb.map(srgb_to_linear);
            let lightness = (0.41222147 * red + 0.53633255 * green + 0.051445995 * blue).cbrt();
            let m = (0.2119035 * red + 0.6806995 * green + 0.10739696 * blue).cbrt();
            let s = (0.08830246 * red + 0.28171885 * green + 0.6299787 * blue).cbrt();
            [
                0.21045426 * lightness + 0.7936178 * m - 0.004072047 * s,
                1.9779985 * lightness - 2.4285922 * m + 0.4505937 * s,
                0.025904037 * lightness + 0.78277177 * m - 0.80867577 * s,
            ]
        }
    }
}

fn gradient_color_channels(
    components: [f32; 3],
    alpha: f32,
    color_space: crate::css::GradientColorSpace,
) -> [u8; 4] {
    let srgb = match color_space {
        crate::css::GradientColorSpace::Srgb => components,
        crate::css::GradientColorSpace::SrgbLinear => components.map(linear_to_srgb),
        crate::css::GradientColorSpace::Oklab => {
            let [lightness, a, b] = components;
            let l = (lightness + 0.39633778 * a + 0.21580376 * b).powi(3);
            let m = (lightness - 0.105561346 * a - 0.06385417 * b).powi(3);
            let s = (lightness - 0.08948418 * a - 1.2914855 * b).powi(3);
            [
                linear_to_srgb(4.076742 * l - 3.3077116 * m + 0.23096994 * s),
                linear_to_srgb(-1.268438 * l + 2.6097574 * m - 0.3413194 * s),
                linear_to_srgb(-0.0041960863 * l - 0.7034186 * m + 1.7076147 * s),
            ]
        }
    };
    [
        (srgb[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.max(0.0).powf(1.0 / 2.4) - 0.055
    }
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
    let source = prepare_text_job(context, &galley.job, format);
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
    context: &Context,
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
        text_format.font_id.family = crate::fonts::FontAssets::family_for_kerning(
            context,
            &text_format.font_id.family,
            format.font_kerning == crate::formatting::FontKerning::None,
        );
        if format.font_size_set {
            text_format.font_id.size = format.em;
        }
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
    selection_color: Color32,
) -> Vec2 {
    fn find_galley(shape: &egui::Shape) -> Option<&std::sync::Arc<egui::Galley>> {
        match shape {
            egui::Shape::Text(text) => Some(&text.galley),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(find_galley),
            _ => None,
        }
    }

    let current = context.graphics(|layers| {
        let list = layers.get(layer)?;
        list.all_entries()
            .skip(start.0)
            .take(end.0 - start.0)
            .find_map(|entry| find_galley(&entry.shape).cloned())
    });
    if let Some(current) = current.as_ref().filter(|galley| galley.job.as_ref() == job) {
        return current.size();
    }

    let galley = context.fonts_mut(|fonts| fonts.layout_job(job.clone()));
    if current
        .as_ref()
        .is_some_and(|current| galley_has_selection(current, selection_color))
    {
        // Replacing a selected galley here would discard egui's selection vertices.
        return galley.size();
    }
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

fn galley_has_selection(galley: &egui::Galley, selection_color: Color32) -> bool {
    galley.rows.iter().any(|row| {
        row.row
            .visuals
            .mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == selection_color)
    })
}

fn clip_background_shape(
    shape: egui::Shape,
    clip: Option<&crate::box_decoration::PathClip>,
) -> egui::Shape {
    let Some(clip) = clip else {
        return shape;
    };
    match shape {
        egui::Shape::Mesh(mesh) => egui::Shape::Mesh(
            crate::box_decoration::clip_mesh_to_paths(mesh.as_ref().clone(), clip).into(),
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

fn clip_shape_to_paths(
    context: &Context,
    shape: egui::Shape,
    clip: &crate::box_decoration::PathClip,
) -> Vec<egui::Shape> {
    context
        .tessellate(
            vec![egui::epaint::ClippedShape {
                clip_rect: clip.bounds(),
                shape,
            }],
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

fn fragment_shadow_clip(
    surface: Rect,
    mut clip: Rect,
    vertical: bool,
    forward: bool,
    index: usize,
    count: usize,
    position: crate::box_decoration::ShadowPosition,
    inset_extent: f32,
    stroke: egui::epaint::MarginF32,
) -> Option<Rect> {
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

fn clip_fragment_shadows(
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

fn border_content_clip(node: &Node) -> Option<crate::box_decoration::PathClip> {
    let paint = node.paint.as_ref()?;
    let widths = node
        .style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
    let widths = [widths.top, widths.right, widths.bottom, widths.left];
    if let Some(border_shape) = &node.style.decorations.border_shape {
        let (paths, fill_rule) = border_shape.background_paths(
            paint.geometry.border,
            crate::css::BackgroundClip::Padding,
            paint.geometry.stroke,
            paint.geometry.padding,
            paint.geometry.margin,
            widths,
        );
        return Some(crate::box_decoration::path_clip(paths, fill_rule));
    }

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
    let mut radii = crate::box_decoration::resolved_radii(&node.style, border);
    radii[0] -= vec2(inset.left, inset.top);
    radii[1] -= vec2(inset.right, inset.top);
    radii[2] -= vec2(inset.right, inset.bottom);
    radii[3] -= vec2(inset.left, inset.bottom);
    radii = radii.map(|radius| radius.max(Vec2::ZERO));
    if outset.iter().any(|margin| *margin > 0.0) {
        radii = outset_radii(edge_rect, radii, outset);
    }
    if radii.iter().all(|radius| radius.max_elem() <= 0.0)
        && outset.iter().all(|margin| *margin <= 0.0)
    {
        return None;
    }
    let path =
        crate::box_decoration::rounded_path(clip_rect, radii, node.style.decorations.corner_shapes);
    Some(crate::box_decoration::path_clip(
        vec![path],
        crate::box_decoration::FillRule::Nonzero,
    ))
}

fn outset_radii(rect: Rect, radii: [Vec2; 4], outset: [f32; 4]) -> [Vec2; 4] {
    let coverage = radii.map(|radius| {
        (2.0 * (radius.x / rect.width().max(f32::EPSILON))
            .min(radius.y / rect.height().max(f32::EPSILON)))
        .max(0.0)
    });
    std::array::from_fn(|corner| {
        let (horizontal, vertical) = match corner {
            0 => (outset[3], outset[0]),
            1 => (outset[1], outset[0]),
            2 => (outset[1], outset[2]),
            _ => (outset[3], outset[2]),
        };
        let adjusted = |radius: f32, amount: f32| {
            if amount <= 0.0 || radius > amount || coverage[corner] > 1.0 {
                radius + amount
            } else {
                let ratio = radius / amount;
                radius + amount * (1.0 - (1.0 - ratio).powi(3) * (1.0 - coverage[corner].powi(3)))
            }
        };
        vec2(
            adjusted(radii[corner].x, horizontal),
            adjusted(radii[corner].y, vertical),
        )
    })
}

fn clip_painted_entry(
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
                    Some(egui::epaint::ClippedShape {
                        clip_rect,
                        shape: egui::Shape::Mesh(mesh.into()),
                    })
                }
                egui::epaint::Primitive::Callback(callback) => {
                    for clip in clips {
                        clip_rect = clip_rect.intersect(clip.bounds());
                    }
                    clip_rect
                        .is_positive()
                        .then_some(egui::epaint::ClippedShape {
                            clip_rect,
                            shape: egui::Shape::Callback(callback),
                        })
                }
            }
        })
        .collect()
}

fn box_surface_shape(
    rect: Rect,
    border: Rect,
    style: &BoxStyle,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    clip: crate::css::BackgroundClip,
    fill: Color32,
    fallback_radius: egui::CornerRadius,
) -> egui::Shape {
    if let Some(border_shape) = &style.decorations.border_shape {
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

fn border_image_shapes(
    border: Rect,
    style: &BoxStyle,
    images: &crate::background::ImageAssets,
    context: &Context,
) -> Option<Vec<egui::Shape>> {
    let image = &style.decorations.border_image;
    let source = image.source.for_scale(context.pixels_per_point());
    if let crate::css::BackgroundImage::SolidColor(color) = source {
        return Some(solid_color_border_image_shapes(
            border, style, image, *color,
        ));
    }
    let auto_slice_density = match source {
        crate::css::BackgroundImage::Url(_) => Some(
            image
                .source
                .resolution_for_scale(context.pixels_per_point()),
        ),
        crate::css::BackgroundImage::CrossFade(_)
            if intrinsic_image_size(source, images, context.pixels_per_point()).is_some() =>
        {
            Some(context.pixels_per_point())
        }
        _ => None,
    };
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
    let outset = std::array::from_fn::<_, 4, _>(|side| match image.outset[side] {
        crate::box_decoration::ImageOutset::LengthZero => 0.0,
        crate::box_decoration::ImageOutset::Length(value) => value,
        crate::box_decoration::ImageOutset::Number(value) => value * widths[side],
    });
    let area = border
        + egui::epaint::MarginF32 {
            top: outset[0],
            right: outset[1],
            bottom: outset[2],
            left: outset[3],
        };
    let (texture, image_width, image_height, gradient, radial_gradient, conic_gradient) =
        match source {
            crate::css::BackgroundImage::Url(url) => {
                let texture = images.texture(context, url)?;
                let size = images.size(url)?;
                (
                    Some(texture),
                    size[0] as f32,
                    size[1] as f32,
                    None,
                    None,
                    None,
                )
            }
            crate::css::BackgroundImage::LinearGradient(gradient) => (
                None,
                area.width(),
                area.height(),
                Some(gradient),
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
            ),
            crate::css::BackgroundImage::ConicGradient(gradient) => (
                None,
                area.width(),
                area.height(),
                None,
                None,
                Some(gradient),
            ),
            crate::css::BackgroundImage::CrossFade(_) => {
                let (texture, _) = cross_fade_texture(
                    context,
                    images,
                    source,
                    area.size(),
                    context.pixels_per_point(),
                )?;
                let size = texture.size();
                (
                    Some(texture),
                    size[0] as f32,
                    size[1] as f32,
                    None,
                    None,
                    None,
                )
            }
            crate::css::BackgroundImage::SolidColor(_) => unreachable!(),
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
        crate::box_decoration::ImageWidth::Length(value) => value.resolve(if side % 2 == 0 {
            area.height()
        } else {
            area.width()
        }),
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
                    if let Some(texture) = &texture {
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

fn solid_color_border_image_shapes(
    border: Rect,
    style: &BoxStyle,
    image: &crate::box_decoration::BorderImage,
    color: Color32,
) -> Vec<egui::Shape> {
    let widths = style
        .layout
        .border
        .map(|value| resolve(value.into_raw(), 0.0).unwrap_or(0.0));
    let widths = [widths.top, widths.right, widths.bottom, widths.left];
    let outset = std::array::from_fn::<_, 4, _>(|side| match image.outset[side] {
        crate::box_decoration::ImageOutset::LengthZero => 0.0,
        crate::box_decoration::ImageOutset::Length(value) => value,
        crate::box_decoration::ImageOutset::Number(value) => value * widths[side],
    });
    let area = border
        + egui::epaint::MarginF32 {
            top: outset[0],
            right: outset[1],
            bottom: outset[2],
            left: outset[3],
        };
    let width_for = |side: usize| match image.width[side] {
        crate::box_decoration::ImageWidth::NumberOne => widths[side],
        crate::box_decoration::ImageWidth::Number(value) => value * widths[side],
        crate::box_decoration::ImageWidth::Auto => widths[side],
        crate::box_decoration::ImageWidth::Length(value) => value.resolve(if side % 2 == 0 {
            area.height()
        } else {
            area.width()
        }),
    };
    let mut draw_widths = [width_for(0), width_for(1), width_for(2), width_for(3)];
    let factor = (area.width() / (draw_widths[1] + draw_widths[3]).max(0.001))
        .min(area.height() / (draw_widths[0] + draw_widths[2]).max(0.001))
        .min(1.0);
    if factor < 1.0 {
        draw_widths.iter_mut().for_each(|value| *value *= factor);
    }
    let x = [
        area.left(),
        area.left() + draw_widths[3],
        area.right() - draw_widths[1],
        area.right(),
    ];
    let y = [
        area.top(),
        area.top() + draw_widths[0],
        area.bottom() - draw_widths[2],
        area.bottom(),
    ];
    let mut shapes = Vec::new();
    for row in 0..3 {
        for column in 0..3 {
            if row == 1 && column == 1 && !image.fill {
                continue;
            }
            let rect = Rect::from_min_max(pos2(x[column], y[row]), pos2(x[column + 1], y[row + 1]));
            if rect.is_positive() {
                shapes.push(egui::Shape::rect_filled(
                    rect,
                    egui::CornerRadius::ZERO,
                    color,
                ));
            }
        }
    }
    shapes
}

fn image_tiles(
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

fn border_shapes(rect: Rect, style: &BoxStyle, current: Color32) -> Vec<egui::Shape> {
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
    let corners =
        crate::box_decoration::rounded_path(rect, outer_radii, style.decorations.corner_shapes);
    let inside =
        crate::box_decoration::rounded_path(inner, inner_radii, style.decorations.corner_shapes);
    let mut shapes = Vec::new();
    let mut side_distance = [0.0_f32; 4];
    let mut side_lengths = [0.0_f32; 4];
    for index in 0..corners.len() {
        let within = index % 9;
        let side = [[3, 0], [0, 1], [1, 2], [2, 3]][index / 9][usize::from(within >= 4)];
        side_lengths[side] += corners[index].distance(corners[(index + 1) % corners.len()]);
    }
    for index in 0..corners.len() {
        let next = (index + 1) % corners.len();
        let corner = index / 9;
        let within = index % 9;
        let side = [[3, 0], [0, 1], [1, 2], [2, 3]][corner][usize::from(within >= 4)];
        let width = [widths.top, widths.right, widths.bottom, widths.left][side];
        let distance = corners[index].distance(corners[next]);
        let segment_start = side_distance[side];
        let segment_end = segment_start + distance;
        side_distance[side] = segment_end;
        if width <= 0.0
            || distance <= 0.001
            || matches!(
                style.border_styles[side],
                BorderStyle::None | BorderStyle::Hidden
            )
        {
            continue;
        }
        let in_corner = within != 8;
        let mut visible = crate::box_decoration::limit_intervals(
            style.decorations.border_limit,
            side,
            side_lengths[side],
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
                    (start - segment_start) / distance,
                    (end - segment_start) / distance,
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

fn dash_intervals(length: f32, dash: f32, gap: f32) -> Vec<(f32, f32)> {
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

fn border_shape_shapes(
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

fn relevant_border_side(style: &BoxStyle) -> usize {
    ["block-start", "inline-start", "block-end", "inline-end"]
        .into_iter()
        .filter_map(|name| {
            crate::layout_style::logical_side(name, style.layout.direction, style.format.writing)
        })
        .find(|side| !matches!(style.border_styles[*side], BorderStyle::None))
        .or_else(|| {
            crate::layout_style::logical_side(
                "block-start",
                style.layout.direction,
                style.format.writing,
            )
        })
        .unwrap_or(0)
}

fn clockwise(mut points: Vec<Pos2>) -> Vec<Pos2> {
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

fn resample_path(points: &[Pos2], count: usize) -> Vec<Pos2> {
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

fn align_path_start(reference: &[Pos2], points: Vec<Pos2>) -> Vec<Pos2> {
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

fn border_stripe_bands(
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
                distributable * weight / flex
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
    selection_visuals: &egui::Visuals,
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
    let selection_color = selection_visuals.selection.bg_fill;
    let selection_by_range = template.galley.job.text == job.text;
    let selected_ranges = selection_by_range
        .then(|| galley_selection_ranges(&template.galley, selection_color))
        .unwrap_or_default();
    let selection_rects = galley_selection_rects(&template.galley, template.pos, selection_color);
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
    for rect in &selection_rects {
        painter.rect_filled(*rect, 0.0, selection_color);
    }
    for fragment in &geometry.fragments {
        if let Some(range) = &fragment.range {
            let mut text = template.clone();
            let mut visible_range = range.clone();
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
                visible_range.end = visible_end;
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
            if selection_by_range {
                let source_start = job.text[..visible_range.start.0].chars().count();
                let source_end = job.text[..visible_range.end.0].chars().count();
                for selected in &selected_ranges {
                    let start = selected.start.max(source_start);
                    let end = selected.end.min(source_end);
                    if start < end {
                        paint_galley_selection_range(
                            &mut text.galley,
                            start - source_start..end - source_start,
                            selection_visuals.selection.stroke.color,
                        );
                    }
                }
            } else {
                paint_galley_selection(
                    &mut text.galley,
                    text.pos,
                    &selection_rects,
                    selection_visuals.selection.stroke.color,
                );
            }
            if format.writing.vertical() {
                vertical_text(&mut text, fragment.text_rect.size(), format);
            }
            painter.add(egui::Shape::Text(text));
        }
    }
}

fn paint_galley_selection_range(
    galley: &mut std::sync::Arc<egui::Galley>,
    selected: std::ops::Range<usize>,
    selection_text_color: Color32,
) {
    let galley = std::sync::Arc::make_mut(galley);
    let mut char_offset = 0;
    for placed in &mut galley.rows {
        let row = std::sync::Arc::make_mut(&mut placed.row);
        for (index, glyph) in row.glyphs.iter().enumerate() {
            if !selected.contains(&(char_offset + index)) {
                continue;
            }
            let start = glyph.first_vertex as usize;
            let end = row
                .glyphs
                .get(index + 1)
                .map_or(row.visuals.glyph_vertex_range.end, |next| {
                    next.first_vertex as usize
                });
            for vertex in &mut row.visuals.mesh.vertices[start..end] {
                vertex.color = selection_text_color;
            }
        }
        char_offset += row.glyphs.len() + usize::from(placed.ends_with_newline);
    }
}

fn galley_selection_ranges(
    galley: &egui::Galley,
    selection_color: Color32,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut char_offset = 0;
    for placed in &galley.rows {
        let row = &placed.row;
        let selected = row
            .visuals
            .mesh
            .vertices
            .iter()
            .filter(|vertex| vertex.color == selection_color)
            .fold(Rect::NOTHING, |rect, vertex| {
                rect.union(Rect::from_min_size(vertex.pos, Vec2::ZERO))
            });
        if selected.is_positive() {
            let start = char_offset + row.char_at(selected.left()).0;
            let end = char_offset + row.char_at(selected.right()).0;
            if start < end {
                if let Some(previous) = ranges.last_mut()
                    && previous.end == start
                {
                    previous.end = end;
                } else {
                    ranges.push(start..end);
                }
            }
        }
        char_offset += row.glyphs.len() + usize::from(placed.ends_with_newline);
    }
    ranges
}

fn galley_selection_rects(
    galley: &egui::Galley,
    pos: egui::Pos2,
    selection_color: Color32,
) -> Vec<Rect> {
    galley
        .rows
        .iter()
        .filter_map(|placed| {
            let rect = placed
                .row
                .visuals
                .mesh
                .vertices
                .iter()
                .filter(|vertex| vertex.color == selection_color)
                .fold(Rect::NOTHING, |rect, vertex| {
                    rect.union(Rect::from_min_size(vertex.pos, Vec2::ZERO))
                });
            rect.is_positive()
                .then_some(rect.translate(pos.to_vec2() + placed.pos.to_vec2()))
        })
        .collect()
}

fn paint_galley_selection(
    galley: &mut std::sync::Arc<egui::Galley>,
    pos: egui::Pos2,
    selection_rects: &[Rect],
    selection_text_color: Color32,
) {
    if selection_rects.is_empty() {
        return;
    }
    let galley = std::sync::Arc::make_mut(galley);
    for placed in &mut galley.rows {
        let row = std::sync::Arc::make_mut(&mut placed.row);
        for (index, glyph) in row.glyphs.iter().enumerate() {
            let glyph_center = pos + placed.pos.to_vec2() + glyph.logical_rect().center().to_vec2();
            if !selection_rects
                .iter()
                .any(|rect| rect.contains(glyph_center))
            {
                continue;
            }
            let start = glyph.first_vertex as usize;
            let end = row
                .glyphs
                .get(index + 1)
                .map_or(row.visuals.glyph_vertex_range.end, |next| {
                    next.first_vertex as usize
                });
            for vertex in &mut row.visuals.mesh.vertices[start..end] {
                vertex.color = selection_text_color;
            }
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
