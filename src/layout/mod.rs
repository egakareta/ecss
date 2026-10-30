use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
};

use egui::{Color32, Context, Id, Pos2, Rect, Response, Ui, Vec2, Widget, pos2, vec2};
use taffy::{Overflow, prelude::*};
use unicode_general_category::{GeneralCategory, get_general_category};

use crate::{
    Stylesheet,
    css::{Declaration, RuleState},
    layout_style::{BorderStyle, BoxStyle, Units},
};

mod background_paint;
mod elements;
mod engine;
mod gradient;
mod invalidation;
mod object;
mod opacity;
mod paint;
mod pseudo;
mod states;
mod text;

use background_paint::*;
use elements::*;
use engine::*;
use invalidation::{LayoutInvalidation, request_relayout};
use paint::*;
use pseudo::*;
pub(crate) use states::keyboard_modality;

use gradient::{
    GradientPaint, conic_gradient_position, conic_gradient_slice_mesh, gradient_color,
    gradient_paint_shape, linear_gradient_slice_mesh, radial_gradient_parameters,
    radial_gradient_position, radial_gradient_slice_mesh,
};
use text::{
    FragmentedTextInput, apply_first_letter_inline_spacing, collect_fragmentable_text,
    first_text_descendant, measure_text, paint_named_flow_texts, reflow_text,
    replace_fragmented_text, rotate_text, text_runs,
};

const ACTIVE: &str = "ecss.layout.active";
const LABEL_SELECTION_PRESS: &str = "ecss.text_selection.press";
const ELEMENT_IMAGES: &str = "ecss.element-images";
const ELEMENT_OCCURRENCES: &str = "ecss.element-occurrences";

#[derive(Clone, Debug, PartialEq)]
struct Geometry {
    border: Rect,
    padding: egui::epaint::MarginF32,
    stroke: egui::epaint::MarginF32,
    gutter: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    extent: Vec2,
    fragments: Vec<crate::layout_tree::Fragment>,
    flow_source: bool,
    flow_region_content: bool,
    slices: Vec<crate::layout_tree::Slice>,
    rules: Vec<egui::Rect>,
    text_vertical: bool,
    collapsed: Option<[crate::formatting::CollapsedBorder; 4]>,
    collapsed_segments: Vec<crate::formatting::CollapsedBorderSegment>,
}

impl Geometry {
    fn content(&self) -> Rect {
        crate::nonnegative_rect(self.border - self.stroke - self.padding)
    }
    fn translated(&self, offset: Vec2) -> Self {
        if offset == Vec2::ZERO {
            return self.clone();
        }
        let mut translated = self.clone();
        translated.border = self.border.translate(offset);
        for segment in &mut translated.collapsed_segments {
            segment.rect = segment.rect.translate(offset);
        }
        translated
    }
}

#[derive(Clone, Default)]
struct Cache {
    boxes: HashMap<Id, Geometry>,
    paint_order: Vec<Id>,
}

struct Node {
    id: Id,
    element_id: Option<String>,
    element_first_occurrence: bool,
    element_references: HashSet<String>,
    style: Rc<BoxStyle>,
    children: smallvec::SmallVec<[usize; 4]>,
    parent: Option<usize>,
    flow_source: bool,
    flow_clip: Option<Rect>,
    measured: Vec2,
    used: Geometry,
    scroll: Vec2,
    ancestor_scroll: Vec2,
    sticky: Vec2,
    text: Option<TextMeasurement>,
    image: Option<Vec2>,
    absent: bool,
    paint: Option<PaintRecord>,
    color: Color32,
    text_decorations: Vec<crate::text_decoration::Decoration>,
}

struct PaintRecord {
    ui: Ui,
    response: Response,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    overlay: egui::layers::ShapeIdx,
    outline: Option<egui::layers::ShapeIdx>,
    scrollbars: Vec<Response>,
    geometry: Geometry,
    clip: Rect,
    fragment_border_images: Vec<Option<Vec<egui::Shape>>>,
    fragment_background_shapes: Vec<Vec<egui::Shape>>,
}

struct TextMeasurement {
    job: egui::text::LayoutJob,
    extra: Vec2,
    minimum: f32,
}

#[derive(Clone, Copy)]
struct ShapeRange {
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
}

struct Frame {
    nodes: Vec<Node>,
    stack: Vec<usize>,
    cache: Cache,
    layout_animation_active: bool,
    layout_animation_can_defer: bool,
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
    static FRAME_BUFFERS: RefCell<Vec<(Vec<Node>, Vec<usize>)>> = RefCell::default();
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
            self.context
                .data_mut(|d| d.remove::<u64>(Id::unique(ACTIVE)));
        }
    }
}

fn survives_all(declaration: &Declaration) -> bool {
    match declaration {
        Declaration::CustomProperty { .. } => true,
        Declaration::Layout(declaration) => {
            matches!(declaration.name.as_str(), "direction" | "unicode-bidi")
        }
        _ => false,
    }
}

fn inherited_by_all(declaration: &Declaration, keyword: crate::css::AllKeyword) -> bool {
    match declaration {
        Declaration::All(_) | Declaration::CustomProperty { .. } | Declaration::Deferred { .. } => {
            false
        }
        Declaration::FontShorthand(_) => false,
        Declaration::Color(_)
        | Declaration::FontFamily(_)
        | Declaration::FontWeight(_)
        | Declaration::FontStyle(_) => matches!(
            keyword,
            crate::css::AllKeyword::Inherit
                | crate::css::AllKeyword::Unset
                | crate::css::AllKeyword::Revert
        ),
        Declaration::Layout(declaration) => {
            keyword == crate::css::AllKeyword::Inherit
                && (declaration.name == "border-radius"
                    || declaration.name == "overscroll-behavior"
                    || declaration.name.starts_with("overscroll-behavior-"))
        }
        _ => keyword == crate::css::AllKeyword::Inherit,
    }
}

fn ordinarily_inherited(declaration: &Declaration) -> bool {
    matches!(
        declaration,
        Declaration::Color(_)
            | Declaration::FontFamily(_)
            | Declaration::FontWeight(_)
            | Declaration::FontStyle(_)
            | Declaration::FontShorthand(_)
    )
}

fn is_css_wide_value(value: &str) -> bool {
    ["inherit", "initial", "unset", "revert"]
        .iter()
        .any(|keyword| value.eq_ignore_ascii_case(keyword))
}

fn is_background_property(name: &str) -> bool {
    name == "background" || name.starts_with("background-")
}

fn is_font_property(name: &str) -> bool {
    matches!(name, "font-family" | "font-weight" | "font-style")
}

fn should_inherit_wide_property(name: &str, value: &str) -> bool {
    (is_font_property(name) && is_css_wide_value(value) && !value.eq_ignore_ascii_case("initial"))
        || (is_background_property(name) && value.eq_ignore_ascii_case("inherit"))
}

fn matches_wide_property(declaration: &Declaration, name: &str) -> bool {
    if name == "background" {
        return match declaration {
            Declaration::Background(_)
            | Declaration::BackgroundShorthand(_)
            | Declaration::BackgroundImage(_)
            | Declaration::BackgroundAttachment(_)
            | Declaration::BackgroundClip(_)
            | Declaration::BackgroundOrigin(_)
            | Declaration::BackgroundPosition(_)
            | Declaration::BackgroundRepeat(_)
            | Declaration::BackgroundSize(_) => true,
            Declaration::Layout(layout) => {
                layout.name == "background" || is_background_property(&layout.name)
            }
            _ => false,
        };
    }
    match name {
        "background-color" => match declaration {
            Declaration::Background(_) | Declaration::BackgroundShorthand(_) => true,
            Declaration::Layout(layout) => layout.name == "background-color",
            _ => false,
        },
        "background-image" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundImage(_) => true,
            Declaration::Layout(layout) => layout.name == "background-image",
            _ => false,
        },
        "background-attachment" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundAttachment(_) => true,
            Declaration::Layout(layout) => layout.name == "background-attachment",
            _ => false,
        },
        "background-clip" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundClip(_) => true,
            Declaration::Layout(layout) => layout.name == "background-clip",
            _ => false,
        },
        "background-origin" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundOrigin(_) => true,
            Declaration::Layout(layout) => layout.name == "background-origin",
            _ => false,
        },
        "background-position" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundPosition(_) => true,
            Declaration::Layout(layout) => layout.name == "background-position",
            _ => false,
        },
        "background-repeat" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundRepeat(_) => true,
            Declaration::Layout(layout) => layout.name == "background-repeat",
            _ => false,
        },
        "background-size" => match declaration {
            Declaration::BackgroundShorthand(_) | Declaration::BackgroundSize(_) => true,
            Declaration::Layout(layout) => layout.name == "background-size",
            _ => false,
        },
        "font-family" => match declaration {
            Declaration::FontFamily(_) => true,
            Declaration::Layout(layout) => layout.name == "font-family",
            _ => false,
        },
        "font-weight" => match declaration {
            Declaration::FontWeight(_) => true,
            Declaration::Layout(layout) => layout.name == "font-weight",
            _ => false,
        },
        "font-style" => match declaration {
            Declaration::FontStyle(_) => true,
            Declaration::Layout(layout) => layout.name == "font-style",
            _ => false,
        },
        _ => false,
    }
}

fn inherited_background_property(
    name: &str,
    declarations: &[Declaration],
    color: Color32,
) -> Option<Declaration> {
    if !matches!(
        name,
        "background"
            | "background-color"
            | "background-image"
            | "background-attachment"
            | "background-clip"
            | "background-origin"
            | "background-position"
            | "background-repeat"
            | "background-size"
    ) {
        return None;
    }
    let background = background_from_declarations(declarations.iter());
    Some(match name {
        "background" => Declaration::BackgroundShorthand(crate::css::BackgroundShorthand {
            color,
            images: background.images.to_vec(),
            attachments: background.attachments.to_vec(),
            clips: background.clips.to_vec(),
            origins: background.origins.to_vec(),
            positions: background.positions.to_vec(),
            repeats: background.repeats.to_vec(),
            sizes: background.sizes.to_vec(),
        }),
        "background-color" => Declaration::Background(color),
        "background-image" => Declaration::BackgroundImage(background.images.to_vec()),
        "background-attachment" => {
            Declaration::BackgroundAttachment(background.attachments.to_vec())
        }
        "background-clip" => Declaration::BackgroundClip(background.clips.to_vec()),
        "background-origin" => Declaration::BackgroundOrigin(background.origins.to_vec()),
        "background-position" => Declaration::BackgroundPosition(background.positions.to_vec()),
        "background-repeat" => Declaration::BackgroundRepeat(background.repeats.to_vec()),
        "background-size" => Declaration::BackgroundSize(background.sizes.to_vec()),
        _ => return None,
    })
}

fn apply_all_rules(
    rules: Vec<(RuleState, Declaration)>,
    parent_declarations: &[Declaration],
    parent_background_color: Color32,
) -> (
    Vec<(RuleState, Vec<Declaration>)>,
    Option<crate::css::AllKeyword>,
) {
    let mut effective = Vec::new();
    let mut all_keyword = None;
    for (state, declaration) in rules {
        if let Declaration::Layout(layout) = &declaration
            && should_inherit_wide_property(&layout.name, &layout.value)
        {
            let name = layout.name.clone();
            effective.push((state, declaration));
            if let Some(inherited) =
                inherited_background_property(&name, parent_declarations, parent_background_color)
            {
                effective.push((state, inherited));
            } else {
                effective.extend(
                    parent_declarations
                        .iter()
                        .filter(|previous| matches_wide_property(previous, &name))
                        .cloned()
                        .map(|previous| (state, previous)),
                );
                if name == "background" {
                    effective.push((state, Declaration::Background(parent_background_color)));
                }
            }
        } else if let Declaration::All(keyword) = declaration {
            effective.retain(|(_, declaration)| survives_all(declaration));
            all_keyword = Some(keyword);
            effective.push((state, Declaration::All(keyword)));
            effective.extend(
                parent_declarations
                    .iter()
                    .filter(|declaration| inherited_by_all(declaration, keyword))
                    .cloned()
                    .map(|declaration| (state, declaration)),
            );
        } else {
            effective.push((state, declaration));
        }
    }

    let mut grouped: Vec<(RuleState, Vec<Declaration>)> = Vec::new();
    for (state, declaration) in effective {
        if let Some((previous_state, declarations)) = grouped.last_mut()
            && *previous_state == state
        {
            declarations.push(declaration);
        } else {
            grouped.push((state, vec![declaration]));
        }
    }
    (grouped, all_keyword)
}

pub(crate) fn show(
    widget: impl Widget,
    ui: &mut Ui,
    stylesheet: &Stylesheet,
    target: &crate::css::SelectorTarget,
) -> Response {
    let element_id = target.id.clone();
    let first_element_occurrence = element_id
        .as_deref()
        .is_none_or(|element_id| first_element_occurrence(ui.ctx(), element_id));
    let id = ui.next_auto_id().with("ecss.layout");
    ui.skip_ahead_auto_ids(1);
    let active = ui.ctx().data(|d| d.get_temp::<u64>(Id::unique(ACTIVE)));
    let active = active.and_then(|key| FRAMES.with(|frames| frames.borrow().get(&key).cloned()));
    let root = active.is_none();
    if root {
        clear_previous_label_selection(ui.ctx());
    }
    let active = active.unwrap_or_else(|| {
        let available = ui.available_rect_before_wrap();
        let (mut nodes, mut stack) =
            FRAME_BUFFERS.with(|buffers| buffers.borrow_mut().pop().unwrap_or_default());
        nodes.clear();
        stack.clear();
        let cache = ui
            .ctx()
            .data_mut(|data| data.remove_temp::<Cache>(id))
            .unwrap_or_default();
        let active = Active(
            Rc::new(RefCell::new(Frame {
                nodes,
                stack,
                cache,
                layout_animation_active: false,
                layout_animation_can_defer: true,
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
            .data_mut(|d| d.insert_temp(Id::unique(ACTIVE), active.1));
        active
    });
    let parent_style = {
        let frame = active.0.borrow();
        frame
            .stack
            .last()
            .map(|parent| frame.nodes[*parent].style.clone())
            .unwrap_or_else(|| Rc::new(BoxStyle::default()))
    };
    // egui disables invisible Uis; CSS visibility does not change enabled state.
    let ui_enabled = ui.is_enabled() || parent_style.hidden && active.0.borrow().enabled;
    let state = states::initial_state(ui, id, ui_enabled);
    let mut style = BoxStyle::default();
    style.current_color = if root {
        ui.visuals().widgets.noninteractive.fg_stroke.color
    } else {
        parent_style.current_color
    };
    style.scrollbar_color = parent_style
        .scrollbar_color
        .inherited(parent_style.current_color);
    style.layout.direction = parent_style.layout.direction;
    style.custom_properties = parent_style.custom_properties.clone();
    style.hidden = parent_style.hidden;
    style.object_position = parent_style.object_position.clone();
    style.collapsed = parent_style.collapsed;
    style.format.writing = parent_style.format.writing;
    style.format.font_family = parent_style.format.font_family.clone();
    style.format.font_weight = parent_style.format.font_weight;
    style.format.font_italic = parent_style.format.font_italic;
    style.format.font_kerning = parent_style.format.font_kerning;
    style.format.text_orientation = parent_style.format.text_orientation.clone();
    style.format.line_height = parent_style.format.line_height;
    style.format.line_height_multiplier = parent_style.format.line_height_multiplier;
    style.format.white_space = parent_style.format.white_space.clone();
    style.format.text_align = parent_style.format.text_align.clone();
    style.format.text_align_last = parent_style.format.text_align_last.clone();
    style.format.text_justify = parent_style.format.text_justify.clone();
    style.format.text_overflow = parent_style.format.text_overflow.clone();
    style.format.letter_spacing = parent_style.format.letter_spacing;
    style.format.word_spacing = parent_style.format.word_spacing;
    style.format.tab_size = parent_style.format.tab_size;
    style.format.tab_size_length = parent_style.format.tab_size_length;
    style.format.text_indent = parent_style.format.text_indent;
    style.format.text_indent_each_line = parent_style.format.text_indent_each_line;
    style.format.text_indent_hanging = parent_style.format.text_indent_hanging;
    style.format.text_transform = parent_style.format.text_transform.clone();
    style.format.line_break = parent_style.format.line_break.clone();
    style.format.overflow_wrap = parent_style.format.overflow_wrap.clone();
    style.format.word_break = parent_style.format.word_break.clone();
    style.format.hyphens = parent_style.format.hyphens.clone();
    style.format.hanging_punctuation = parent_style.format.hanging_punctuation.clone();
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
    let inherited_style = style.clone();
    let mut units = Units {
        math: &math,
        font: base_font_size,
        root_font: 16.0,
        viewport: ui.ctx().viewport_rect().size(),
        pixels_per_point: ui.ctx().pixels_per_point(),
    };
    let resolve_rules = |state: u8| {
        let rules: smallvec::SmallVec<[&crate::css::Rule; 8]> =
            applicable_rules(stylesheet, target, state, units.viewport).collect();
        let ordered = crate::css::cascade::ordered_with_importance(rules.iter().copied());
        let custom_declarations = ordered
            .iter()
            .map(|(_, declaration, _)| *declaration)
            .collect::<Vec<_>>();
        let custom_properties =
            crate::css::variables::compute(&parent_style.custom_properties, &custom_declarations);
        let resolved = ordered
            .into_iter()
            .filter_map(|(rule_state, declaration, important)| {
                crate::css::resolve_deferred(declaration, &custom_properties)
                    .map(|declaration| (rule_state, declaration, important))
            })
            .collect::<Vec<_>>();
        let important_declarations = resolved
            .iter()
            .filter(|(_, _, important)| *important)
            .map(|(_, declaration, _)| declaration.clone())
            .collect::<Vec<_>>();
        let resolved = resolved
            .into_iter()
            .map(|(rule_state, declaration, _)| (rule_state, declaration))
            .collect();
        let (resolved, all_keyword) = apply_all_rules(
            resolved,
            &parent_style.computed_declarations,
            parent_style.background_color,
        );
        (
            custom_properties,
            resolved,
            all_keyword,
            important_declarations,
        )
    };
    let (custom_properties, resolved_rules, all_keyword, important_declarations) =
        resolve_rules(state);
    style.custom_properties = custom_properties.clone();
    match all_keyword {
        Some(crate::css::AllKeyword::Inherit) => {
            style = (*parent_style).clone();
            style.custom_properties = custom_properties.clone();
        }
        Some(crate::css::AllKeyword::Unset | crate::css::AllKeyword::Revert) => {
            style = inherited_style;
            style.custom_properties = custom_properties.clone();
        }
        Some(crate::css::AllKeyword::Initial) => {
            let direction = style.layout.direction;
            style = BoxStyle::default();
            style.layout.direction = direction;
            style.custom_properties = custom_properties.clone();
        }
        None => {}
    }
    units.font = style.format.em;
    let non_pointer_state = state & !(RuleState::HOVERED.bits() | RuleState::ACTIVE.bits());
    let (_, base_visual_rules, _, _) = resolve_rules(non_pointer_state);
    let (_, hovered_visual_rules, _, _) =
        resolve_rules(non_pointer_state | RuleState::HOVERED.bits());
    let active_state = if state & (RuleState::FOCUS.bits() | RuleState::ACTIVE.bits()) != 0 {
        state
    } else {
        non_pointer_state | RuleState::HOVERED.bits() | RuleState::ACTIVE.bits()
    };
    let (_, active_visual_rules, _, _) = resolve_rules(active_state);
    let mut background_urls = Vec::new();
    for declaration in resolved_rules
        .iter()
        .flat_map(|(_, declarations)| declarations)
    {
        match declaration {
            Declaration::BackgroundShorthand(background) => {
                for image in &background.images {
                    image.collect_urls(&mut background_urls);
                }
            }
            Declaration::BackgroundImage(images) => {
                for image in images {
                    image.collect_urls(&mut background_urls);
                }
            }
            Declaration::Layout(declaration) if declaration.name.starts_with("border-image") => {
                background_urls
                    .extend(crate::box_decoration::border_image_urls(&declaration.value));
            }
            _ => {}
        }
    }
    stylesheet.start_background_image_load_for_urls(ui.ctx(), &background_urls);
    let mut animation_properties = Vec::new();
    for (_, declarations) in &resolved_rules {
        for declaration in declarations {
            if let Declaration::Animation(property) = declaration {
                animation_properties.push(property.clone());
            }
        }
    }
    let mut underlying_declarations = Vec::new();
    if !animation_properties.is_empty() {
        for (_, declarations) in &resolved_rules {
            underlying_declarations.extend(
                declarations
                    .iter()
                    .filter(|declaration| !matches!(declaration, Declaration::Animation(_)))
                    .cloned(),
            );
        }
    }
    for (_, declarations) in &resolved_rules {
        for declaration in declarations {
            if let Declaration::Layout(declaration) = declaration
                && declaration.name == "font-size"
            {
                declaration
                    .apply_with_parent(&mut style, &parent_style, units)
                    .expect("validated font-size declaration");
            } else if let Declaration::FontShorthand(shorthand) = declaration {
                crate::layout_style::apply_font_shorthand(
                    shorthand,
                    &mut style,
                    &parent_style,
                    units,
                    true,
                )
                .expect("validated font shorthand declaration");
            }
        }
    }
    units.font = style.format.em;

    for (_, declarations) in &resolved_rules {
        for declaration in declarations {
            if let Declaration::Layout(declaration) = declaration
                && matches!(
                    declaration.name.as_str(),
                    "direction" | "writing-mode" | "text-orientation"
                )
            {
                declaration
                    .apply_with_parent(&mut style, &parent_style, units)
                    .expect("validated direction");
            }
        }
    }
    for (_, declarations) in &resolved_rules {
        for declaration in declarations {
            match declaration {
                Declaration::Color(color) => style.current_color = *color,
                Declaration::Background(color) => {
                    style.background_color = *color;
                    style.background_set = true;
                    style.background_current_color = false;
                }
                Declaration::BackgroundShorthand(background) => {
                    style.background_color = background.color;
                    style.background_set = true;
                    style.background_current_color = false;
                }
                Declaration::Transition(property) => {
                    style.transitions.apply(property, &parent_style.transitions)
                }
                Declaration::FontShorthand(shorthand) => {
                    crate::layout_style::apply_font_shorthand(
                        shorthand,
                        &mut style,
                        &parent_style,
                        units,
                        false,
                    )
                    .expect("validated font shorthand declaration");
                }
                Declaration::FontFamily(families) => {
                    style.format.font_family = Some(families.clone());
                }
                Declaration::FontWeight(weight) => {
                    style.format.font_weight = *weight;
                }
                Declaration::FontStyle(italic) => {
                    style.format.font_italic = *italic;
                }
                Declaration::Layout(declaration) => {
                    if declaration.name == "font-size" {
                        continue;
                    }
                    declaration
                        .apply_with_parent(&mut style, &parent_style, units)
                        .expect("validated CSS layout declaration");
                }
                _ => {}
            }
        }
    }
    if style.background_current_color {
        style.background_color = style.current_color;
    }
    let keyframes = stylesheet
        .keyframes
        .iter()
        .filter(|keyframes| {
            keyframes
                .media
                .iter()
                .all(|query| query.matches(units.viewport))
        })
        .map(|keyframes| {
            (
                keyframes.name.clone(),
                keyframes
                    .frames
                    .iter()
                    .map(|frame| crate::css::Keyframe {
                        offset: frame.offset,
                        timing_function: frame.timing_function.clone(),
                        declarations: frame
                            .declarations
                            .iter()
                            .filter_map(|declaration| {
                                crate::css::resolve_deferred(declaration, &style.custom_properties)
                            })
                            .collect(),
                    })
                    .collect(),
            )
        })
        .collect::<HashMap<_, _>>();
    let animation = crate::animation::sample(
        ui.ctx(),
        id,
        ui.input(|input| input.time),
        &animation_properties,
        &keyframes,
        &underlying_declarations,
        &important_declarations,
    );
    {
        let mut frame = active.0.borrow_mut();
        frame.layout_animation_can_defer &= animation.can_defer_layout;
        frame.layout_animation_active |= animation.layout_active;
    }
    let animated_declarations = animation.declarations;
    for declaration in &animated_declarations {
        match declaration {
            Declaration::Color(color) => style.current_color = *color,
            Declaration::Background(color) => {
                style.background_color = *color;
                style.background_set = true;
                style.background_current_color = false;
            }
            Declaration::Layout(declaration) => {
                declaration
                    .apply_with_parent(&mut style, &parent_style, units)
                    .expect("validated animated CSS declaration");
                if declaration.name == "font-size" {
                    units.font = style.format.em;
                }
            }
            _ => {}
        }
    }
    if style.background_current_color {
        style.background_color = style.current_color;
    }
    let transition = if stylesheet.rule_index.has_transitions {
        crate::transition::sample(ui.ctx(), id, &mut style, &math)
    } else {
        crate::transition::Sample::default()
    };
    if style.background_current_color && !transition.background {
        style.background_color = style.current_color;
    }
    if transition.layout_sampled {
        let mut frame = active.0.borrow_mut();
        frame.layout_animation_active = true;
    }
    let font_request = style
        .format
        .font_family
        .clone()
        .map(|families| crate::fonts::FontRequest {
            families,
            weight: style.format.font_weight,
            italic: style.format.font_italic,
        });
    if let Some(request) = &font_request {
        stylesheet.start_font_load_for_request(request.clone(), ui.ctx());
    }
    let mut computed_declarations = if all_keyword.is_none() {
        parent_style
            .computed_declarations
            .iter()
            .filter(|declaration| ordinarily_inherited(declaration))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    computed_declarations.extend(
        resolved_rules
            .iter()
            .flat_map(|(_, declarations)| declarations.iter())
            .filter(|declaration| {
                !matches!(
                    declaration,
                    Declaration::All(_) | Declaration::CustomProperty { .. }
                )
            })
            .cloned(),
    );
    style.computed_declarations = std::sync::Arc::new(computed_declarations);
    crate::layout_style::resolve_flow(&mut style, &parent_style);
    style.finish();
    let style = Rc::new(style);
    let scroll = ui
        .ctx()
        .data(|d| d.get_temp::<Vec2>(id.with("scroll")))
        .unwrap_or_default();
    let (index, geometry, absent) = {
        let mut frame = active.0.borrow_mut();
        let index = frame.nodes.len();
        let parent = frame.stack.last().copied();
        let mut text_decorations = if style.layout.position.is_out_of_flow()
            || style.layout.float.is_floated()
            || style.inline && style.format.display != crate::formatting::DisplayKind::Inline
        {
            Vec::new()
        } else {
            parent.map_or_else(Vec::new, |parent| {
                frame.nodes[parent].text_decorations.clone()
            })
        };
        if !style.contents
            && let Some(decoration) = style
                .text_decoration
                .resolve(style.current_color, style.format.em)
        {
            text_decorations.push(decoration);
        }
        let absent =
            style.layout.display == Display::None || parent.is_some_and(|p| frame.nodes[p].absent);
        let viewport_fixed = style.fixed && fixed_containing_block(&frame, parent).is_none();
        let ancestor_scroll = if viewport_fixed {
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
        if transition.layout_sampled
            && parent.is_none_or(|parent| {
                !matches!(
                    frame.nodes[parent].style.layout.display,
                    Display::Flex | Display::Grid
                )
            })
        {
            let containing = parent.map_or_else(
                || ui.available_rect_before_wrap(),
                |parent| frame.nodes[parent].used.content(),
            );
            refresh_transition_box(&style, &mut geometry, containing);
        }
        let sticky = sticky_offset(&style, &geometry, parent, &frame);
        geometry = geometry.translated(sticky);
        if let Some(parent) = parent {
            frame.nodes[parent].children.push(index);
        }
        frame.nodes.push(Node {
            id,
            element_id: element_id.clone(),
            element_first_occurrence: first_element_occurrence,
            element_references: HashSet::new(),
            style: style.clone(),
            children: smallvec::SmallVec::new(),
            parent,
            flow_source: false,
            flow_clip: None,
            measured: Vec2::ZERO,
            used: geometry.clone(),
            scroll,
            ancestor_scroll,
            sticky,
            text: None,
            image: None,
            absent,
            paint: None,
            color: Color32::BLACK,
            text_decorations,
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
        .scope_id(id)
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
    for (visual_state, rules) in [
        (crate::VisualState::Base, &base_visual_rules),
        (crate::VisualState::Hovered, &hovered_visual_rules),
        (crate::VisualState::Active, &active_visual_rules),
    ] {
        for (_, declarations) in rules {
            crate::apply_declarations(&mut content_ui, visual_state, declarations);
        }
    }
    crate::apply_declarations(
        &mut content_ui,
        crate::VisualState::Base,
        &animated_declarations,
    );
    transition.apply_visuals(&mut content_ui, &style, geometry.border);
    if resolved_rules.iter().flat_map(|(_, declarations)| declarations).any(|declaration| matches!(declaration, Declaration::Layout(layout) if layout.name == "color")) {
        crate::apply_declarations(&mut content_ui, crate::VisualState::Base, &[Declaration::Color(style.current_color)]);
    }
    if style.padding_set {
        content_ui.spacing_mut().button_padding = Vec2::ZERO;
    }
    let default_text_styles = egui::Style::default().text_styles;
    for (text_style, font) in content_ui.style_mut().text_styles.iter_mut() {
        if let Some(default) = default_text_styles.get(text_style) {
            font.family = default.family.clone();
        }
    }
    if let Some(request) = font_request
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
    let image = active.0.borrow().nodes[index]
        .children
        .is_empty()
        .then(|| object::image_size(ui.ctx(), ShapeRange { layer, start, end }, response.rect))
        .flatten();
    active.0.borrow_mut().nodes[index].image = image;
    let mut measured = content_ui
        .min_rect()
        .size()
        .max(response.rect.max - (geometry.content().min - scroll));
    if !measured.is_finite() {
        measured = Vec2::ZERO;
    }
    {
        let mut frame = active.0.borrow_mut();
        frame.nodes[index].color = content_ui
            .visuals()
            .widgets
            .state(response.widget_state())
            .fg_stroke
            .color;
        if frame.nodes[index].children.is_empty() {
            let color = frame.nodes[index].color;
            let mut text = measure_text(
                ui.ctx(),
                ShapeRange { layer, start, end },
                measured,
                &style.format,
                geometry.content().width(),
                style.format.flow_into_content,
            );
            if let Some(text) = &mut text
                && let Some((pseudo_style, _, _)) = pseudo_decoration_style(
                    stylesheet,
                    target,
                    crate::css::PseudoElementKind::FirstLetter,
                    state,
                    &style,
                    units,
                    color,
                )
            {
                if let Some((trailing, inline_extent)) = apply_first_letter_inline_spacing(
                    ui.ctx(),
                    &mut text.job,
                    &pseudo_style,
                    geometry.content().width(),
                    style.format.writing,
                    style.layout.direction,
                ) {
                    text.extra.x += trailing;
                    let measured_text = ui
                        .ctx()
                        .fonts_mut(|fonts| fonts.layout_job(text.job.clone()).size())
                        + text.extra;
                    let expected_width = (measured.x + inline_extent).max(0.0);
                    frame.nodes[index].measured = vec2(
                        if inline_extent >= 0.0 {
                            measured_text.x.max(expected_width)
                        } else {
                            measured_text.x.min(expected_width)
                        },
                        measured_text.y,
                    );
                } else {
                    frame.nodes[index].measured = measured;
                }
            } else {
                frame.nodes[index].measured = measured;
            }
            frame.nodes[index].text = text;
        } else {
            frame.nodes[index].measured = measured;
        }
    }
    let descendant_text_color = {
        let frame = active.0.borrow();
        (!frame.nodes[index].children.is_empty()).then_some(frame.nodes[index].color)
    };
    if let Some(color) = descendant_text_color
        && let Some((pseudo_style, _, _)) = pseudo_decoration_style(
            stylesheet,
            target,
            crate::css::PseudoElementKind::FirstLetter,
            state,
            &style,
            units,
            color,
        )
    {
        let update = {
            let mut frame = active.0.borrow_mut();
            let target = first_text_descendant(&frame, index);
            target.and_then(|target| {
                let node = &mut frame.nodes[target];
                let text = node.text.as_mut()?;
                let (trailing, inline_extent) = apply_first_letter_inline_spacing(
                    ui.ctx(),
                    &mut text.job,
                    &pseudo_style,
                    geometry.content().width(),
                    style.format.writing,
                    style.layout.direction,
                )?;
                text.extra.x += trailing;
                let measured = ui
                    .ctx()
                    .fonts_mut(|fonts| fonts.layout_job(text.job.clone()).size())
                    + text.extra;
                let expected_width = (node.measured.x + inline_extent).max(0.0);
                node.measured = vec2(
                    if inline_extent >= 0.0 {
                        measured.x.max(expected_width)
                    } else {
                        measured.x.min(expected_width)
                    },
                    measured.y,
                );
                let paint = node.paint.as_ref()?;
                Some((
                    paint.ui.layer_id(),
                    paint.start,
                    paint.end,
                    text.job.clone(),
                    node.style.format.clone(),
                    if node.style.format.writing.vertical() {
                        node.used.content().height()
                    } else {
                        node.used.content().width()
                    },
                ))
            })
        };
        if let Some((layer, start, end, job, format, width)) = update {
            reflow_text(
                ui.ctx(),
                ShapeRange { layer, start, end },
                &job,
                content_ui.visuals().selection.bg_fill,
                &format,
                width,
            );
        }
    }
    let content_size = active.0.borrow().nodes[index]
        .text
        .as_ref()
        .map(|text| {
            reflow_text(
                ui.ctx(),
                ShapeRange { layer, start, end },
                &text.job,
                content_ui.visuals().selection.bg_fill,
                &style.format,
                if style.format.writing.vertical() {
                    geometry.content().height()
                } else {
                    geometry.content().width()
                },
            )
        })
        .unwrap_or(measured);
    if geometry.flow_region_content {
        ui.ctx().graphics_mut(|layers| {
            let list = layers.entry(layer);
            for index in start.0..end.0 {
                list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                    entry.shape = egui::Shape::Noop;
                });
            }
        });
    } else if !geometry.flow_source
        && geometry
            .fragments
            .iter()
            .any(|fragment| fragment.range.is_some())
    {
        if let Some(text) = active.0.borrow().nodes[index].text.as_ref() {
            replace_fragmented_text(FragmentedTextInput {
                context: ui.ctx(),
                layer,
                start,
                end,
                geometry: &geometry,
                job: &text.job,
                format: &style.format,
                selection_visuals: content_ui.visuals(),
                decorations: &active.0.borrow().nodes[index].text_decorations,
            });
        }
    } else {
        text::decorate_texts(
            ui.ctx(),
            &active.0.borrow(),
            index,
            ShapeRange { layer, start, end },
        );
        if text_vertical {
            rotate_text(ui.ctx(), layer, start, end, &geometry, &style.format);
        }
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
        let mut frame = active.0.borrow_mut();
        let has_complete_layout_cache = frame
            .nodes
            .iter()
            .all(|node| frame.cache.boxes.contains_key(&node.id));
        let cache = solve(&mut frame, ui.ctx());
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
            let can_defer_layout_animation = frame.layout_animation_active
                && frame.layout_animation_can_defer
                && has_complete_layout_cache
                && ui.input(|input| input.events.is_empty());
            // Eligible animations already schedule their next frame; avoid a discarded pass.
            if scroll_changed_last_frame {
                ui.ctx().request_repaint();
            } else if !can_defer_layout_animation {
                request_relayout(ui.ctx(), LayoutInvalidation::Geometry);
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
    let collapsed_decoration_style = final_geometry.collapsed.is_some().then(|| {
        let mut style = style.as_ref().clone();
        style.decorations.radii = [crate::box_decoration::CornerRadius::default(); 4];
        style.decorations.corner_shapes = [crate::box_decoration::CornerShape::Round; 4];
        style
    });
    let decoration_style = collapsed_decoration_style
        .as_ref()
        .unwrap_or_else(|| style.as_ref());
    let border = final_geometry.border;
    if let Some(natural) = image {
        object::fit_image(
            ui.ctx(),
            ShapeRange { layer, start, end },
            response.rect,
            natural,
            final_geometry.content(),
            &style,
        );
    }
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
    } else if response.enabled() {
        let interaction = without_id_clash_warning(ui.ctx(), || {
            content_ui.interact(visual, response.id, response.sense)
        });
        response.union(interaction).with_new_rect(visual)
    } else {
        response.with_new_rect(visual)
    };
    response.interact_rect = if absent || style.hidden {
        Rect::NOTHING
    } else {
        visual.intersect(ui.clip_rect())
    };
    let enabled = if absent || style.hidden {
        ui_enabled
    } else {
        response.enabled()
    };
    let current_state = states::response_state(&response, id, enabled);
    if stylesheet.rule_index.has_element_references {
        let (_, current_element_rules, _, _) = resolve_rules(current_state);
        active.0.borrow_mut().nodes[index].element_references =
            resolved_element_references_from_rules(
                stylesheet,
                target,
                state,
                &current_element_rules,
                &style,
                units,
            );
    }
    let interaction_layout_changed = state != current_state
        && matching_rules(stylesheet, target, units.viewport).any(|rule| {
            rule.state.matches(state) != rule.state.matches(current_state)
                && (style.transitions.enabled()
                    || rule.declarations.iter().any(|declaration| {
                        matches!(
                            declaration,
                            Declaration::Layout(_)
                                | Declaration::Color(_)
                                | Declaration::All(_)
                                | Declaration::FontFamily(_)
                                | Declaration::FontWeight(_)
                                | Declaration::FontStyle(_)
                                | Declaration::FontShorthand(_)
                                | Declaration::Transition(_)
                                | Declaration::CustomProperty { .. }
                                | Declaration::Deferred { .. }
                        )
                    }))
        });
    if interaction_layout_changed {
        request_relayout(ui.ctx(), LayoutInvalidation::Interaction);
    }
    let overlay = shape_index(ui.ctx(), layer);
    let mut outline = None;
    let mut scrollbars = Vec::new();
    let mut fragment_border_images = Vec::new();
    let mut fragment_backgrounds = Vec::new();
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
        let (_, current_state_rules, _, _) = resolve_rules(current_state);
        let mut background_style = resolved_background_from_rules(&current_state_rules);
        if transition.background
            || resolved_rules
                .iter()
                .flat_map(|(_, declarations)| declarations)
                .any(|declaration| {
                    matches!(declaration, Declaration::Layout(layout) if layout.name == "background" || layout.name == "background-color")
                })
        {
            background_style.color = style.background_color;
        }
        let corner_radius = if final_geometry.collapsed.is_some() {
            egui::CornerRadius::ZERO
        } else if frame_stroke.is_some()
            || matching_rules(stylesheet, target, units.viewport).any(|r| {
                r.state.matches(current_state)
                    && r.declarations.iter().any(|declaration| {
                        matches!(declaration, Declaration::BorderRadius(_))
                            || matches!(declaration, Declaration::Layout(layout) if layout.name == "border-radius")
                    })
            }) {
            visuals.corner_radius
        } else {
            egui::CornerRadius::ZERO
        };
        let blocked_element_ids =
            invalid_element_references(ui.ctx(), stylesheet, index, &active.0.borrow());
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
        let inline_slice_background =
            inline_fragmented && !style.format.clone_decoration && surface_count > 1;
        let inline_background_offsets = if inline_slice_background {
            inline_slice_offsets(&surfaces, inline_vertical, inline_forward)
        } else {
            vec![Vec2::ZERO; surface_count]
        };
        let inline_slice_border_image = if inline_slice_background {
            let composite = surfaces
                .iter()
                .zip(&inline_background_offsets)
                .map(|(surface, offset)| surface.translate(*offset))
                .fold(Rect::NOTHING, Rect::union);
            border_image_shapes(
                composite,
                &style,
                &stylesheet.background_images,
                ui.ctx(),
                &blocked_element_ids,
            )
        } else {
            None
        };
        if style.format.clone_decoration && final_geometry.collapsed.is_none() {
            fragment_border_images = final_geometry
                .slices
                .iter()
                .map(|slice| {
                    let source = slice.source.translate(border.min.to_vec2());
                    border_image_shapes(
                        source,
                        &style,
                        &stylesheet.background_images,
                        ui.ctx(),
                        &blocked_element_ids,
                    )
                    .map(|shapes| {
                        clip_border_image_shapes(
                            ui.ctx(),
                            source,
                            &style,
                            final_geometry.stroke,
                            final_geometry.padding,
                            final_geometry.margin,
                            shapes,
                        )
                    })
                })
                .collect();
            fragment_backgrounds = final_geometry
                .slices
                .iter()
                .map(|slice| {
                    let source = slice.source.translate(border.min.to_vec2());
                    let geometry = BoxDecorationGeometry::new(
                        source,
                        decoration_style,
                        final_geometry.stroke,
                        final_geometry.padding,
                        final_geometry.margin,
                    );
                    let resources = ImageResources {
                        context: ui.ctx(),
                        images: &stylesheet.background_images,
                        invalid_elements: &blocked_element_ids,
                    };
                    fragment_background_shapes(
                        &resources,
                        &background_style,
                        &geometry,
                        corner_radius,
                        scroll,
                        Vec2::ZERO,
                        None,
                    )
                })
                .collect();
        }
        let mut shapes = Vec::new();
        let background_area = inline_slice_background.then(|| {
            surfaces
                .iter()
                .zip(&inline_background_offsets)
                .map(|(surface, offset)| surface.translate(*offset))
                .fold(Rect::NOTHING, Rect::union)
        });
        if !suppress_outer_shadow && final_geometry.slices.is_empty() {
            for (surface_index, surface) in surfaces.iter().copied().enumerate() {
                let shadow = crate::box_decoration::shadow_shapes(
                    if inline_fragmented { surface } else { border },
                    decoration_style,
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
                        FragmentPosition {
                            vertical: inline_vertical,
                            forward: inline_forward,
                            index: surface_index,
                            count: surface_count,
                        },
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
        for (surface_index, surface) in surfaces.iter().copied().enumerate() {
            let background_offset = inline_background_offsets[surface_index];
            let resources = ImageResources {
                context: ui.ctx(),
                images: &stylesheet.background_images,
                invalid_elements: &blocked_element_ids,
            };
            let geometry = BoxDecorationGeometry::new(
                surface,
                decoration_style,
                final_geometry.stroke,
                final_geometry.padding,
                final_geometry.margin,
            );
            shapes.extend(fragment_background_shapes(
                &resources,
                &background_style,
                &geometry,
                corner_radius,
                scroll,
                background_offset,
                background_area,
            ));
            let inset_shadow = crate::box_decoration::shadow_shapes(
                surface,
                decoration_style,
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
                    FragmentPosition {
                        vertical: inline_vertical,
                        forward: inline_forward,
                        index: surface_index,
                        count: surface_count,
                    },
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
            if let Some(edges) = final_geometry.collapsed.as_ref() {
                shapes.extend(collapsed_border_shapes(
                    surface,
                    decoration_style,
                    edges,
                    visuals.fg_stroke.color,
                ));
            } else {
                let mut fragment_style = (*decoration_style).clone();
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
                let border_image = if let Some(composite_shapes) = &inline_slice_border_image {
                    let offset = inline_background_offsets[surface_index];
                    let clip = surface.translate(offset) + border_image_outset(&style);
                    let clip = crate::box_decoration::path_clip(
                        vec![vec![
                            clip.left_top(),
                            clip.right_top(),
                            clip.right_bottom(),
                            clip.left_bottom(),
                        ]],
                        crate::box_decoration::FillRule::Nonzero,
                    );
                    Some(
                        composite_shapes
                            .iter()
                            .cloned()
                            .flat_map(|shape| clip_shape_to_paths(ui.ctx(), shape, &clip))
                            .map(|mut shape| {
                                shape
                                    .transform(egui::emath::TSTransform::from_translation(-offset));
                                shape
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    border_image_shapes(
                        surface,
                        &fragment_style,
                        &stylesheet.background_images,
                        ui.ctx(),
                        &blocked_element_ids,
                    )
                };
                if let Some(border_image) = border_image {
                    shapes.extend(clip_border_image_shapes(
                        ui.ctx(),
                        surface,
                        &fragment_style,
                        final_geometry.stroke,
                        final_geometry.padding,
                        final_geometry.margin,
                        border_image,
                    ));
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
            let collapsed_segment_style = BoxStyle::default();
            for segment in &final_geometry.collapsed_segments {
                shapes.extend(collapsed_border_shapes(
                    segment.rect,
                    &collapsed_segment_style,
                    &segment.edges,
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
        if !absent {
            for pseudo in [
                crate::css::PseudoElementKind::FirstLine,
                crate::css::PseudoElementKind::FirstLetter,
            ] {
                let Some((pseudo_style, fill, color)) = pseudo_decoration_style(
                    stylesheet,
                    target,
                    pseudo,
                    state,
                    &style,
                    units,
                    visuals.fg_stroke.color,
                ) else {
                    continue;
                };
                let text_rects = pseudo_text_rects(
                    ui.ctx(),
                    layer,
                    start,
                    end,
                    pseudo,
                    style.format.writing.vertical(),
                );
                if text_rects.is_empty() {
                    continue;
                }
                for text_rect in text_rects {
                    shapes.extend(pseudo_decoration_shapes(PseudoDecorationInput {
                        context: ui.ctx(),
                        stylesheet,
                        rect: text_rect,
                        style: &pseudo_style,
                        percentage_basis: final_geometry.content().width(),
                        fill,
                        current: color,
                        blocked_element_ids: &blocked_element_ids,
                    }));
                }
            }
        }

        painter.set(background, egui::Shape::Vec(shapes));
        scrollbars = scroll_box(&mut content_ui, id, &final_geometry, &style, scroll);
        let outline_surfaces = if final_geometry.slices.is_empty() {
            surfaces
        } else {
            final_geometry
                .slices
                .iter()
                .map(|slice| slice.source.translate(border.min.to_vec2() + slice.offset))
                .collect()
        };
        let outlines = outline_surfaces
            .into_iter()
            .flat_map(|surface| outline_shapes(surface, decoration_style, style.current_color))
            .collect::<Vec<_>>();
        if !outlines.is_empty() {
            let mut outline_painter = painter.clone();
            outline_painter.set_clip_rect(ui.clip_rect());
            outline = Some(outline_painter.add(egui::Shape::Vec(outlines)));
        }
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
        outline,
        scrollbars,
        geometry: final_geometry.clone(),
        clip: ui.clip_rect(),
        fragment_border_images,
        fragment_background_shapes: fragment_backgrounds,
    });
    if first_element_occurrence && let Some(element_id) = element_id {
        if !element_source_is_eligible(&active.0.borrow(), index) {
            remove_element_image(ui.ctx(), &element_id);
        } else if let Some(image) = capture_element_image(
            ui.ctx(),
            layer,
            background,
            paint_end,
            element_image_bounds(visual, &style),
            element_descendant_dependencies(&active.0.borrow())
                .remove(&element_id)
                .unwrap_or_default(),
        ) {
            publish_element_image(ui.ctx(), element_id.clone(), image);
        } else {
            remove_element_image(ui.ctx(), &element_id);
        }
    }
    if root {
        if let Some(cache) = ui.ctx().data(|data| data.get_temp::<Cache>(id)) {
            let frame = active.0.borrow();
            paint_named_flow_texts(&frame, &cache, ui.ctx());
        }
        reorder_paint(&active.0.borrow(), ui.ctx());
        let mut frame = active.0.borrow_mut();
        frame.nodes.clear();
        frame.stack.clear();
        let buffers = (
            std::mem::take(&mut frame.nodes),
            std::mem::take(&mut frame.stack),
        );
        drop(frame);
        FRAME_BUFFERS.with(|pool| pool.borrow_mut().push(buffers));
    }
    response
}

fn clone_decoration(
    shape: &egui::Shape,
    geometry: &BoxDecorationGeometry<'_>,
    color: Color32,
    fragment_border_image: Option<&[egui::Shape]>,
    fragment_background_shapes: &[egui::Shape],
) -> egui::Shape {
    if matches!(shape, egui::Shape::Vec(_)) {
        let mut shapes = fragment_background_shapes.to_vec();
        shapes.extend(crate::box_decoration::shadow_shapes(
            geometry.border,
            geometry.style,
            color,
            crate::box_decoration::ShadowPosition::Inset,
            geometry.stroke,
            geometry.padding,
            geometry.margin,
        ));
        if let Some(fragment_border_image) = fragment_border_image {
            shapes.extend(fragment_border_image.iter().cloned());
        } else {
            shapes.extend(border_shapes(geometry.border, geometry.style, color));
        }
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
    let containing_parent = nearest_box_ancestor(parent, frame);
    let containing = containing_parent
        .map(|index| frame.nodes[index].used.content())
        .unwrap_or(frame.viewport);
    let containing_style = containing_parent.map(|index| &*frame.nodes[index].style);
    let mut scroll_parent = parent;
    let scrollport = loop {
        let Some(index) = scroll_parent else {
            break frame.viewport;
        };
        let node = &frame.nodes[index];
        if !node.style.contents
            && (node.style.layout.overflow.x.is_scroll_container()
                || node.style.layout.overflow.y.is_scroll_container())
        {
            break node.used.border - node.used.stroke - node.used.gutter;
        }
        scroll_parent = node.parent;
    };

    let containing_style = containing_style.unwrap_or(style);
    let direction = containing_style.layout.direction;
    let writing = containing_style.format.writing;
    let orientation = containing_style.format.text_orientation.as_str();
    let block_axis = usize::from(!writing.vertical());
    let insets = [
        style.layout.inset.top,
        style.layout.inset.right,
        style.layout.inset.bottom,
        style.layout.inset.left,
    ];
    let margins = geometry.margin;
    let margin_low = [margins.left, margins.top];
    let margin_high = [margins.right, margins.bottom];
    let mut offset = Vec2::ZERO;

    for axis in 0..2 {
        let (start_name, end_name) = if axis == block_axis {
            ("block-start", "block-end")
        } else {
            ("inline-start", "inline-end")
        };
        let start_side =
            crate::layout_style::logical_side(start_name, direction, writing, orientation).unwrap();
        let end_side =
            crate::layout_style::logical_side(end_name, direction, writing, orientation).unwrap();
        let low_side = if axis == 0 { 3 } else { 0 };
        let high_side = if axis == 0 { 1 } else { 2 };
        let basis = scrollport.size()[axis];
        let resolved = insets.map(|inset| resolve(inset.into_raw(), basis));
        if resolved[low_side].is_none() && resolved[high_side].is_none() {
            continue;
        }

        let mut used_insets = resolved.map(|inset| inset.unwrap_or(0.0));
        let start_inset = used_insets[start_side];
        let end_inset = used_insets[end_side];
        let border_extent = geometry.border.size()[axis];
        if basis - start_inset - end_inset < border_extent {
            used_insets[end_side] = basis - start_inset - border_extent;
        }

        let view_low = scrollport.min[axis] + used_insets[low_side];
        let view_high = scrollport.max[axis] - used_insets[high_side];
        let mut sticky_min = f32::NEG_INFINITY;
        let mut sticky_max = f32::INFINITY;
        if resolved[low_side].is_some() {
            sticky_min = view_low - geometry.border.min[axis];
        }
        if resolved[high_side].is_some() {
            sticky_max = view_high - geometry.border.max[axis];
        }

        let low_margin_edge = geometry.border.min[axis] - margin_low[axis];
        let high_margin_edge = geometry.border.max[axis] + margin_high[axis];
        let low_margin =
            sticky_position_margin(margin_low[axis], low_margin_edge, containing.min[axis]);
        let high_margin =
            sticky_position_margin(margin_high[axis], high_margin_edge, containing.max[axis]);
        let position_low = geometry.border.min[axis] - low_margin;
        let position_high = geometry.border.max[axis] + high_margin;
        let containing_min = containing.min[axis] - position_low;
        let containing_max = containing.max[axis] - position_high;
        let preferred = if sticky_min <= sticky_max {
            0.0_f32.clamp(sticky_min, sticky_max)
        } else if sticky_min > 0.0 {
            sticky_min
        } else {
            sticky_max
        };
        offset[axis] = if containing_min <= containing_max {
            preferred.clamp(containing_min, containing_max)
        } else if (preferred - containing_min).abs() < (preferred - containing_max).abs() {
            containing_min
        } else {
            containing_max
        };
    }
    offset
}

fn sticky_position_margin(margin: f32, margin_edge: f32, containing_edge: f32) -> f32 {
    if margin > 0.0 {
        margin.min((margin_edge - containing_edge).abs())
    } else {
        margin
    }
}

fn nearest_box_ancestor(mut ancestor: Option<usize>, frame: &Frame) -> Option<usize> {
    while let Some(index) = ancestor {
        let node = &frame.nodes[index];
        if !node.style.contents {
            return Some(index);
        }
        ancestor = node.parent;
    }
    None
}

fn fixed_containing_block(frame: &Frame, mut ancestor: Option<usize>) -> Option<usize> {
    while let Some(index) = ancestor {
        let node = &frame.nodes[index];
        if !node.style.contents && (node.style.fixed || node.style.will_change("position")) {
            return Some(index);
        }
        ancestor = node.parent;
    }
    None
}

fn viewport_fixed(frame: &Frame, index: usize) -> bool {
    let mut current = Some(index);
    while let Some(index) = current {
        let node = &frame.nodes[index];
        if node.style.fixed {
            return fixed_containing_block(frame, node.parent).is_none();
        }
        current = node.parent;
    }
    false
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
    let scrollbar_width = style.scrollbar_width.thickness();
    let axes = [
        style.layout.overflow.x == Overflow::Scroll,
        style.layout.overflow.y == Overflow::Scroll,
    ];
    let scroll_containers = [
        style.layout.overflow.x.is_scroll_container(),
        style.layout.overflow.y.is_scroll_container(),
    ];
    for axis in 0..2 {
        if !axes[axis] {
            offset[axis] = 0.0;
            if hovered
                && scroll_containers[axis]
                && style.overscroll_behavior[axis] != crate::layout_style::OverscrollBehavior::Auto
            {
                ui.input_mut(|input| input.smooth_scroll_delta[axis] = 0.0);
            }
            continue;
        }
        if hovered {
            ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta[axis];
                let next = (offset[axis] - delta).clamp(0.0, maximum[axis]);
                if next != offset[axis]
                    || style.overscroll_behavior[axis]
                        != crate::layout_style::OverscrollBehavior::Auto
                {
                    input.smooth_scroll_delta[axis] = 0.0;
                }
                offset[axis] = next;
            });
        }
        offset[axis] = offset[axis].clamp(0.0, maximum[axis]);
        if maximum[axis] <= 0.0 || scrollbar_width == 0.0 {
            continue;
        }
        let track = if axis == 0 {
            Rect::from_min_max(
                pos2(border_box.left(), border_box.bottom() - scrollbar_width),
                border_box.right_bottom(),
            )
        } else {
            Rect::from_min_max(
                pos2(border_box.right() - scrollbar_width, border_box.top()),
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
        let (thumb_color, track_color) = style
            .scrollbar_color
            .resolve(style.current_color)
            .map_or_else(
                || {
                    (
                        Color32::from_gray(if response.hovered() { 100 } else { 150 }),
                        Color32::TRANSPARENT,
                    )
                },
                |[thumb, track]| (thumb, track),
            );
        if track_color != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(track, scrollbar_width * 0.5, track_color);
        }
        ui.painter()
            .rect_filled(thumb, scrollbar_width * 0.5, thumb_color);
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
