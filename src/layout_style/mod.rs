use cssparser::{Parser, Token};
use egui::{Color32, Vec2};
use taffy::{Overflow, prelude::*};

mod grid;
mod outline;
mod values;

use grid::*;
pub(crate) use values::{components, dimension, length_value};
use values::{gap, number};

#[derive(Clone, Debug)]
pub(crate) struct Declaration {
    pub(crate) name: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug)]
pub(crate) struct BoxStyle {
    pub layout: Style,
    pub format: crate::formatting::Formatting,
    pub(crate) computed_declarations: std::sync::Arc<Vec<crate::css::Declaration>>,
    pub(crate) custom_properties: crate::css::variables::CustomProperties,
    template_grid: bool,
    pub display_set: bool,
    pub inline: bool,
    pub contents: bool,
    pub positioned: bool,
    pub fixed: bool,
    pub(crate) will_change: Vec<String>,
    pub hidden: bool,
    pub collapsed: bool,
    pub(crate) opacity: f32,
    pub margin_trim: [bool; 4],
    pub(crate) current_color: Color32,
    pub(crate) background_color: Color32,
    pub(crate) background_set: bool,
    pub(crate) background_current_color: bool,
    pub(crate) transitions: crate::transition::Settings,
    pub(crate) object_fit: crate::object::Fit,
    pub(crate) object_position: crate::object::Position,
    pub(crate) scrollbar_color: ScrollbarColor,
    pub(crate) scrollbar_width: ScrollbarWidth,
    pub(crate) overscroll_behavior: [OverscrollBehavior; 2],
    pub intrinsic_min: Size<Option<IntrinsicSizing>>,
    pub intrinsic_max: Size<Option<IntrinsicSizing>>,
    pub scrollbar_gutter: ScrollbarGutter,
    pub overflow_clip_box: OverflowClipBox,
    pub overflow_clip_margin: [f32; 4],
    pub order: i32,
    pub z_index: Option<i32>,
    pub padding_set: bool,
    pub border_set: bool,
    pub border_colors: [Option<Color32>; 4],
    pub border_stripes: [Option<Vec<crate::box_decoration::BorderStripe>>; 4],
    pub border_styles: [BorderStyle; 4],
    pub border_widths: [f32; 4],
    pub(crate) outline: outline::Outline,
    pub(crate) text_decoration: crate::text_decoration::TextDecoration,
    pub decorations: crate::box_decoration::DecorationStyle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum BorderStyle {
    #[default]
    None,
    Hidden,
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OverflowClipBox {
    Content,
    #[default]
    Padding,
    Border,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ScrollbarGutter {
    #[default]
    Auto,
    Stable,
    StableBothEdges,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ScrollbarWidth {
    #[default]
    Auto,
    Thin,
    None,
}

impl ScrollbarWidth {
    pub(crate) fn thickness(self) -> f32 {
        match self {
            Self::Auto => 6.0,
            Self::Thin => 4.0,
            Self::None => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OverscrollBehavior {
    #[default]
    Auto,
    Contain,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ScrollbarColor {
    #[default]
    Auto,
    Colors([ScrollbarColorValue; 2]),
}

impl ScrollbarColor {
    pub(crate) fn inherited(self, current_color: Color32) -> Self {
        match self {
            Self::Auto => Self::Auto,
            Self::Colors(colors) => Self::Colors(colors.map(|color| match color {
                ScrollbarColorValue::CurrentColor => ScrollbarColorValue::Color(current_color),
                color => color,
            })),
        }
    }

    pub(crate) fn resolve(self, current_color: Color32) -> Option<[Color32; 2]> {
        match self {
            Self::Auto => None,
            Self::Colors(colors) => Some(colors.map(|color| match color {
                ScrollbarColorValue::CurrentColor => current_color,
                ScrollbarColorValue::Color(color) => color,
            })),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollbarColorValue {
    CurrentColor,
    Color(Color32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntrinsicSizing {
    MinContent,
    MaxContent,
}

impl Default for BoxStyle {
    fn default() -> Self {
        Self {
            format: Default::default(),
            computed_declarations: Default::default(),
            custom_properties: Default::default(),
            layout: Style {
                display: Display::Block,
                box_sizing: BoxSizing::ContentBox,
                ..Style::default()
            },
            template_grid: false,
            display_set: false,
            inline: false,
            contents: false,
            positioned: false,
            fixed: false,
            will_change: Vec::new(),
            hidden: false,
            collapsed: false,
            opacity: 1.0,
            margin_trim: [false; 4],
            current_color: Color32::BLACK,
            background_color: Color32::TRANSPARENT,
            background_set: false,
            background_current_color: false,
            transitions: crate::transition::Settings::default(),
            scrollbar_color: ScrollbarColor::Auto,
            scrollbar_width: ScrollbarWidth::Auto,
            overscroll_behavior: [OverscrollBehavior::Auto; 2],
            intrinsic_min: Size {
                width: None,
                height: None,
            },
            intrinsic_max: Size {
                width: None,
                height: None,
            },
            scrollbar_gutter: ScrollbarGutter::Auto,
            overflow_clip_box: OverflowClipBox::default(),
            overflow_clip_margin: [0.0; 4],
            order: 0,
            z_index: None,
            padding_set: false,
            border_set: false,
            border_colors: [None; 4],
            border_stripes: std::array::from_fn(|_| None),
            border_styles: [BorderStyle::None; 4],
            border_widths: [3.0; 4],
            outline: outline::Outline::default(),
            text_decoration: Default::default(),
            object_fit: Default::default(),
            object_position: Default::default(),
            decorations: crate::box_decoration::DecorationStyle::default(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Units<'a> {
    pub math: &'a crate::math::Arena,
    pub font: f32,
    pub root_font: f32,
    pub viewport: Vec2,
    pub pixels_per_point: f32,
}

pub(crate) fn parse(
    name: &str,
    input: &mut Parser<'_>,
) -> Result<Option<Declaration>, &'static str> {
    let name = name.to_ascii_lowercase();
    if !supported(&name) {
        return Ok(None);
    }
    let start = input.position();
    while input.next().is_ok() {}
    let value = input.slice_from(start).trim().to_owned();
    let declaration = Declaration { name, value };
    let arena = crate::math::Arena::default();
    if declaration.value.eq_ignore_ascii_case("revert-layer") && declaration.name != "will-change" {
        return Err("cascade layers are unsupported");
    }
    if !["inherit", "initial", "unset", "revert", "revert-layer"]
        .iter()
        .any(|keyword| declaration.value.eq_ignore_ascii_case(keyword))
    {
        declaration.apply(
            &mut BoxStyle::default(),
            Units {
                math: &arena,
                font: 16.0,
                root_font: 16.0,
                viewport: Vec2::splat(1000.0),
                pixels_per_point: 1.0,
            },
        )?;
    }
    Ok(Some(declaration))
}

fn supported(name: &str) -> bool {
    crate::formatting::supported(name)
        || outline::supported(name)
        || crate::text_decoration::supported(name)
        || crate::box_decoration::supported(name)
        || matches!(
            name,
            "display"
                | "box-sizing"
                | "width"
                | "height"
                | "min-width"
                | "min-height"
                | "max-width"
                | "max-height"
                | "inline-size"
                | "block-size"
                | "min-inline-size"
                | "min-block-size"
                | "max-inline-size"
                | "max-block-size"
                | "aspect-ratio"
                | "object-fit"
                | "object-position"
                | "position"
                | "will-change"
                | "opacity"
                | "scroll-behavior"
                | "top"
                | "right"
                | "bottom"
                | "left"
                | "inset"
                | "inset-inline"
                | "inset-block"
                | "inset-inline-start"
                | "inset-inline-end"
                | "inset-block-start"
                | "inset-block-end"
                | "margin"
                | "margin-top"
                | "margin-right"
                | "margin-bottom"
                | "margin-left"
                | "margin-trim"
                | "margin-inline"
                | "margin-block"
                | "margin-inline-start"
                | "margin-inline-end"
                | "margin-block-start"
                | "margin-block-end"
                | "padding"
                | "padding-top"
                | "padding-right"
                | "padding-bottom"
                | "padding-left"
                | "padding-inline"
                | "padding-block"
                | "padding-inline-start"
                | "padding-inline-end"
                | "padding-block-start"
                | "padding-block-end"
                | "border"
                | "border-top"
                | "border-right"
                | "border-bottom"
                | "border-left"
                | "border-width"
                | "border-style"
                | "border-color"
                | "border-top-width"
                | "border-right-width"
                | "border-bottom-width"
                | "border-left-width"
                | "border-top-style"
                | "border-right-style"
                | "border-bottom-style"
                | "border-left-style"
                | "border-top-color"
                | "border-right-color"
                | "border-bottom-color"
                | "border-left-color"
                | "border-inline"
                | "border-block"
                | "border-inline-width"
                | "border-block-width"
                | "border-inline-style"
                | "border-block-style"
                | "border-inline-color"
                | "border-block-color"
                | "border-inline-start"
                | "border-inline-end"
                | "border-block-start"
                | "border-block-end"
                | "border-inline-start-width"
                | "border-inline-end-width"
                | "border-block-start-width"
                | "border-block-end-width"
                | "border-inline-start-style"
                | "border-inline-end-style"
                | "border-block-start-style"
                | "border-block-end-style"
                | "border-inline-start-color"
                | "border-inline-end-color"
                | "border-block-start-color"
                | "border-block-end-color"
                | "overflow"
                | "overflow-x"
                | "overflow-y"
                | "overflow-block"
                | "overflow-inline"
                | "overflow-clip-margin"
                | "overflow-clip-margin-block"
                | "overflow-clip-margin-inline"
                | "overflow-clip-margin-top"
                | "overflow-clip-margin-right"
                | "overflow-clip-margin-bottom"
                | "overflow-clip-margin-left"
                | "overflow-clip-margin-block-start"
                | "overflow-clip-margin-block-end"
                | "overflow-clip-margin-inline-start"
                | "overflow-clip-margin-inline-end"
                | "scrollbar-gutter"
                | "scrollbar-color"
                | "scrollbar-width"
                | "overscroll-behavior"
                | "overscroll-behavior-x"
                | "overscroll-behavior-y"
                | "overscroll-behavior-inline"
                | "overscroll-behavior-block"
                | "visibility"
                | "order"
                | "z-index"
                | "gap"
                | "row-gap"
                | "column-gap"
                | "flex"
                | "flex-grow"
                | "flex-shrink"
                | "flex-basis"
                | "flex-direction"
                | "flex-wrap"
                | "flex-flow"
                | "align-items"
                | "align-self"
                | "align-content"
                | "justify-items"
                | "justify-self"
                | "justify-content"
                | "place-items"
                | "place-self"
                | "place-content"
                | "grid"
                | "grid-template-columns"
                | "grid-template-rows"
                | "grid-auto-columns"
                | "grid-auto-rows"
                | "grid-auto-flow"
                | "grid-column"
                | "grid-row"
                | "grid-column-start"
                | "grid-column-end"
                | "grid-row-start"
                | "grid-row-end"
                | "grid-area"
                | "grid-template"
                | "grid-template-areas"
                | "flow"
                | "chains"
                | "float"
                | "clear"
                | "direction"
        )
}

impl Declaration {
    pub(crate) fn apply_with_parent(
        &self,
        output: &mut BoxStyle,
        parent: &BoxStyle,
        units: Units,
    ) -> Result<(), &'static str> {
        if self.name == "font-size" {
            output.format.font_size_set = true;
        }
        let keyword = self.value.trim();
        if keyword.eq_ignore_ascii_case("currentcolor") {
            if self.name == "color" {
                output.current_color = parent.current_color;
                return Ok(());
            }
            if self.name == "background-color" {
                output.background_current_color = true;
                output.background_set = true;
                return Ok(());
            }
        }
        if keyword.eq_ignore_ascii_case("match-parent")
            && matches!(
                self.name.as_str(),
                "text-align" | "text-align-all" | "text-align-last"
            )
        {
            let resolve = |value: &str| match value {
                "start" if parent.layout.direction == taffy::Direction::Ltr => "left".into(),
                "start" => "right".into(),
                "end" if parent.layout.direction == taffy::Direction::Ltr => "right".into(),
                "end" => "left".into(),
                "match-parent" => "start".into(),
                value => value.to_owned(),
            };
            match self.name.as_str() {
                "text-align" => {
                    output.format.text_align = resolve(&parent.format.text_align).into();
                    output.format.text_align_last = resolve(&parent.format.text_align_last).into();
                }
                "text-align-all" => {
                    output.format.text_align = resolve(&parent.format.text_align).into();
                }
                "text-align-last" => {
                    output.format.text_align_last = resolve(&parent.format.text_align_last).into();
                }
                _ => unreachable!(),
            }
            return Ok(());
        }
        if keyword.eq_ignore_ascii_case("inherit")
            || keyword.eq_ignore_ascii_case("unset") && inherited(&self.name)
            || self.name == "object-position" && keyword.eq_ignore_ascii_case("revert")
        {
            copy_property(&self.name, output, parent);
            return Ok(());
        }
        if ["initial", "unset", "revert"]
            .iter()
            .any(|value| keyword.eq_ignore_ascii_case(value))
        {
            if outline::supported(&self.name) {
                outline::copy_property(
                    &self.name,
                    &mut output.outline,
                    outline::Outline::default(),
                );
            } else if crate::text_decoration::supported(&self.name) {
                crate::text_decoration::copy_property(
                    &self.name,
                    &mut output.text_decoration,
                    Default::default(),
                );
            } else {
                copy_property(&self.name, output, &BoxStyle::default());
            }
            return Ok(());
        }
        if keyword.eq_ignore_ascii_case("revert-layer") {
            if self.name == "will-change" {
                copy_property(&self.name, output, &BoxStyle::default());
                return Ok(());
            }
            return Err("cascade layers are unsupported");
        }
        self.apply(output, units)
    }

    pub(crate) fn apply(&self, output: &mut BoxStyle, units: Units) -> Result<(), &'static str> {
        let value = self.value.trim();
        let name = self.name.as_str();
        if ["inherit", "initial", "unset", "revert", "revert-layer"]
            .iter()
            .any(|keyword| value.eq_ignore_ascii_case(keyword))
        {
            return Err("CSS-wide value requires its computed parent style");
        }
        if crate::formatting::apply(name, value, output, units)? {
            return Ok(());
        }
        if name == "scroll-behavior" {
            return if value.eq_ignore_ascii_case("auto") || value.eq_ignore_ascii_case("smooth") {
                Ok(())
            } else {
                Err("invalid scroll-behavior")
            };
        }
        if crate::box_decoration::apply(name, value, output, units)? {
            return Ok(());
        }
        if outline::supported(name) {
            return outline::apply(name, value, &mut output.outline, units);
        }
        if crate::text_decoration::supported(name) {
            return crate::text_decoration::apply(name, value, &mut output.text_decoration, units);
        }
        if name == "opacity" {
            output.opacity = opacity_value(value, units)?;
            return Ok(());
        }
        let parts = components(value)?;
        let s = &mut output.layout;
        match name {
            "object-fit" => output.object_fit = crate::object::Fit::parse(value)?,
            "object-position" => {
                output.object_position = crate::object::Position::parse(&parts, units)?
            }
            "direction" => s.direction = value.parse().map_err(|_| "invalid direction")?,
            "float" => s.float = value.parse().map_err(|_| "invalid float")?,
            "clear" => s.clear = value.parse().map_err(|_| "invalid clear")?,
            "display" => {
                output.format.display = crate::formatting::DisplayKind::Normal;
                if let Some((kind, inline)) = crate::formatting::display(value) {
                    output.format.display = kind;
                    output.inline = inline;
                    output.display_set = true;
                    output.contents = false;
                    s.display = Display::Block;
                    return Ok(());
                }

                output.display_set = true;
                output.inline = matches!(
                    value,
                    "inline-block"
                        | "inline-flex"
                        | "inline-grid"
                        | "inline flow-root"
                        | "inline flex"
                        | "inline grid"
                );
                output.contents = value == "contents";
                s.display = match value {
                    "block" | "block flow" | "contents" => Display::Block,
                    "flow-root" | "inline-block" | "block flow-root" | "inline flow-root" => {
                        Display::FlowRoot
                    }
                    "flex" | "inline-flex" | "block flex" | "inline flex" => Display::Flex,
                    "grid" | "inline-grid" | "block grid" | "inline grid" => Display::Grid,
                    "none" => Display::None,
                    _ => return Err("unsupported display value"),
                };
            }
            "box-sizing" => {
                s.box_sizing = match value {
                    "content-box" => BoxSizing::ContentBox,
                    "border-box" => BoxSizing::BorderBox,
                    _ => return Err("invalid box-sizing"),
                }
            }
            "width" | "inline-size" => s.size.width = dimension(value, units)?,
            "height" | "block-size" => s.size.height = dimension(value, units)?,
            "min-width" | "min-inline-size" => {
                s.min_size.width = limit(value, units)?;
                output.intrinsic_min.width = intrinsic_sizing(value);
            }
            "min-height" | "min-block-size" => {
                s.min_size.height = limit(value, units)?;
                output.intrinsic_min.height = intrinsic_sizing(value);
            }
            "max-width" | "max-inline-size" => {
                s.max_size.width = if value == "none" {
                    auto()
                } else {
                    limit(value, units)?
                };
                output.intrinsic_max.width = intrinsic_sizing(value);
            }
            "max-height" | "max-block-size" => {
                s.max_size.height = if value == "none" {
                    auto()
                } else {
                    limit(value, units)?
                };
                output.intrinsic_max.height = intrinsic_sizing(value);
            }
            "aspect-ratio" => {
                s.aspect_ratio = if value == "auto" {
                    None
                } else {
                    let (a, b) = value.split_once('/').unwrap_or((value, "1"));
                    let ratio = number(a.trim(), units)? / number(b.trim(), units)?;
                    if !ratio.is_finite() || ratio <= 0.0 {
                        return Err("invalid aspect-ratio");
                    }
                    Some(ratio)
                };
            }
            "position" => {
                output.positioned = value != "static";
                output.fixed = value == "fixed";
                s.position = match value {
                    "static" => Position::Static,
                    "relative" => Position::Relative,
                    "sticky" => Position::Sticky,
                    "absolute" | "fixed" => Position::Absolute,
                    _ => return Err("unsupported position value"),
                };
            }
            "will-change" => output.will_change = parse_will_change(value)?,
            "margin" | "padding" | "inset" => {
                let edges = quad(&parts)?;
                for (index, value) in edges.iter().enumerate() {
                    set_edge(output, name, index, value, units)?;
                }
            }
            "margin-trim" => {
                apply_margin_trim(&mut output.margin_trim, &parts)?;
            }
            "top" | "right" | "bottom" | "left" => {
                set_edge(output, "inset", side(name).unwrap(), value, units)?
            }
            _ if name.starts_with("margin-")
                || name.starts_with("padding-")
                || name.starts_with("inset-") =>
            {
                let (group, suffix) = name.split_once('-').unwrap();
                if suffix == "inline" || suffix == "block" {
                    if parts.is_empty() || parts.len() > 2 {
                        return Err("expected one or two edge values");
                    }
                    let indices = if suffix == "inline" {
                        [
                            logical_side(
                                "inline-start",
                                output.layout.direction,
                                output.format.writing,
                                &output.format.text_orientation,
                            )
                            .unwrap(),
                            logical_side(
                                "inline-end",
                                output.layout.direction,
                                output.format.writing,
                                &output.format.text_orientation,
                            )
                            .unwrap(),
                        ]
                    } else {
                        [
                            logical_side(
                                "block-start",
                                output.layout.direction,
                                output.format.writing,
                                &output.format.text_orientation,
                            )
                            .unwrap(),
                            logical_side(
                                "block-end",
                                output.layout.direction,
                                output.format.writing,
                                &output.format.text_orientation,
                            )
                            .unwrap(),
                        ]
                    };
                    set_edge(output, group, indices[0], &parts[0], units)?;
                    set_edge(
                        output,
                        group,
                        indices[1],
                        parts.get(1).unwrap_or(&parts[0]),
                        units,
                    )?;
                } else {
                    set_edge(
                        output,
                        group,
                        logical_side(
                            suffix,
                            output.layout.direction,
                            output.format.writing,
                            &output.format.text_orientation,
                        )
                        .ok_or("invalid side")?,
                        value,
                        units,
                    )?;
                }
            }
            _ if name.starts_with("border-inline") || name.starts_with("border-block") => {
                let physical = physical_borders(
                    name,
                    output.layout.direction,
                    output.format.writing,
                    &output.format.text_orientation,
                );
                let pair = physical.len() == 2
                    && matches!(name.rsplit('-').next(), Some("width" | "style" | "color"));
                if pair && (parts.is_empty() || parts.len() > 2) {
                    return Err("expected one or two border values");
                }
                for (index, physical) in physical.iter().enumerate() {
                    if pair {
                        apply_border(
                            output,
                            physical,
                            std::slice::from_ref(parts.get(index).unwrap_or(&parts[0])),
                            units,
                        )?;
                    } else {
                        apply_border(output, physical, &parts, units)?;
                    }
                }
            }
            _ if name.starts_with("border") => apply_border(output, name, &parts, units)?,
            "overflow" => {
                if parts.is_empty() || parts.len() > 2 {
                    return Err("invalid overflow");
                }
                s.overflow.x = overflow(&parts[0])?;
                s.overflow.y = overflow(parts.get(1).unwrap_or(&parts[0]))?;
            }
            "overflow-x" => s.overflow.x = overflow(value)?,
            "overflow-y" => s.overflow.y = overflow(value)?,
            "overflow-block" => {
                let value = overflow(value)?;
                if output.format.writing.vertical() {
                    s.overflow.x = value;
                } else {
                    s.overflow.y = value;
                }
            }
            "overflow-inline" => {
                let value = overflow(value)?;
                if output.format.writing.vertical() {
                    s.overflow.y = value;
                } else {
                    s.overflow.x = value;
                }
            }
            "overflow-clip-margin" => apply_clip_margin(
                &mut output.overflow_clip_box,
                &mut output.overflow_clip_margin,
                &parts,
                units,
            )?,
            "overscroll-behavior" => {
                if parts.is_empty() || parts.len() > 2 {
                    return Err("invalid overscroll-behavior");
                }
                let x = overscroll_behavior(&parts[0])?;
                let y = parts
                    .get(1)
                    .map_or(Ok(x), |value| overscroll_behavior(value))?;
                output.overscroll_behavior = [x, y];
            }
            "overscroll-behavior-x"
            | "overscroll-behavior-y"
            | "overscroll-behavior-inline"
            | "overscroll-behavior-block" => {
                if parts.len() != 1 {
                    return Err("invalid overscroll-behavior");
                }
                let axis = overscroll_axis(name, output.format.writing).unwrap();
                output.overscroll_behavior[axis] = overscroll_behavior(&parts[0])?;
            }
            "scrollbar-gutter" => {
                output.scrollbar_gutter = match parts.as_slice() {
                    [value] if value == "auto" => ScrollbarGutter::Auto,
                    [value] if value == "stable" => ScrollbarGutter::Stable,
                    [stable, both_edges] if stable == "stable" && both_edges == "both-edges" => {
                        ScrollbarGutter::StableBothEdges
                    }
                    _ => return Err("invalid scrollbar-gutter"),
                };
            }
            "scrollbar-color" => {
                output.scrollbar_color =
                    if parts.len() == 1 && parts[0].eq_ignore_ascii_case("auto") {
                        ScrollbarColor::Auto
                    } else {
                        let [thumb, track] = parts.as_slice() else {
                            return Err("invalid scrollbar-color");
                        };
                        ScrollbarColor::Colors([scrollbar_color(thumb)?, scrollbar_color(track)?])
                    };
            }
            "scrollbar-width" => {
                output.scrollbar_width = match value.to_ascii_lowercase().as_str() {
                    "auto" => ScrollbarWidth::Auto,
                    "thin" => ScrollbarWidth::Thin,
                    "none" => ScrollbarWidth::None,
                    _ => return Err("invalid scrollbar-width"),
                };
            }
            "overflow-clip-margin-block" | "overflow-clip-margin-inline" => {
                if parts.is_empty() || parts.len() > 2 {
                    return Err("invalid overflow-clip-margin");
                }
                let inline = name.ends_with("inline");
                let start = logical_side(
                    if inline {
                        "inline-start"
                    } else {
                        "block-start"
                    },
                    s.direction,
                    output.format.writing,
                    &output.format.text_orientation,
                )
                .unwrap();
                let end = logical_side(
                    if inline { "inline-end" } else { "block-end" },
                    s.direction,
                    output.format.writing,
                    &output.format.text_orientation,
                )
                .unwrap();
                output.overflow_clip_margin[start] = clip_margin_length(&parts[0], units)?;
                output.overflow_clip_margin[end] =
                    clip_margin_length(parts.get(1).unwrap_or(&parts[0]), units)?;
            }
            _ if name.starts_with("overflow-clip-margin-") => {
                let side_name = name.trim_start_matches("overflow-clip-margin-");
                let edge = logical_side(
                    side_name,
                    s.direction,
                    output.format.writing,
                    &output.format.text_orientation,
                )
                .or_else(|| side(side_name))
                .ok_or("invalid overflow-clip-margin side")?;
                output.overflow_clip_margin[edge] = clip_margin_length(value, units)?;
            }
            "visibility" => match value {
                "visible" => {
                    output.hidden = false;
                    output.collapsed = false;
                }
                "hidden" => {
                    output.hidden = true;
                    output.collapsed = false;
                }
                "collapse" => {
                    output.hidden = true;
                    output.collapsed = true;
                }
                _ => return Err("invalid visibility"),
            },
            "order" => output.order = value.parse().map_err(|_| "invalid order")?,
            "z-index" => {
                output.z_index = if value.eq_ignore_ascii_case("auto") {
                    None
                } else {
                    Some(value.parse().map_err(|_| "invalid z-index")?)
                }
            }
            "gap" => {
                output.format.normal_gaps = [value == "normal"; 2];
                if parts.is_empty() || parts.len() > 2 {
                    return Err("invalid gap");
                }
                s.gap.height = gap(&parts[0], units)?;
                s.gap.width = gap(parts.get(1).unwrap_or(&parts[0]), units)?;
            }
            "row-gap" => {
                output.format.normal_gaps[1] = value == "normal";
                s.gap.height = gap(value, units)?;
            }
            "column-gap" => {
                output.format.normal_gaps[0] = value == "normal";
                s.gap.width = gap(value, units)?;
            }
            "flex-grow" => s.flex_grow = number(value, units)?,
            "flex-shrink" => s.flex_shrink = number(value, units)?,
            "flex-basis" => s.flex_basis = dimension(value, units)?,
            "flex-direction" => s.flex_direction = direction(value)?,
            "flex-wrap" => s.flex_wrap = wrap(value)?,
            "flex-flow" => {
                s.flex_direction = FlexDirection::Row;
                s.flex_wrap = FlexWrap::NoWrap;
                let mut has_direction = false;
                let mut has_wrap = false;
                for part in &parts {
                    if let Ok(d) = direction(part) {
                        if has_direction {
                            return Err("duplicate flex direction");
                        }
                        s.flex_direction = d;
                        has_direction = true;
                    } else {
                        if has_wrap {
                            return Err("duplicate flex wrap");
                        }
                        s.flex_wrap = wrap(part)?;
                        has_wrap = true;
                    }
                }
                if parts.is_empty() {
                    return Err("invalid flex-flow");
                }
            }
            "flex" => apply_flex(s, value, &parts, units)?,
            "align-items" => s.align_items = alignment(value)?.unwrap_or_default(),
            "align-self" => s.align_self = alignment(value)?,
            "justify-items" => s.justify_items = alignment(value)?.unwrap_or_default(),
            "justify-self" => s.justify_self = alignment(value)?,
            "align-content" => s.align_content = distribution(value)?.unwrap_or_default(),
            "justify-content" => s.justify_content = distribution(value)?.unwrap_or_default(),
            "place-items" | "place-self" | "place-content" => {
                if parts.is_empty() || parts.len() > 2 {
                    return Err("invalid alignment shorthand");
                }
                let a = &parts[0];
                let b = parts.get(1).unwrap_or(a);
                match name {
                    "place-items" => {
                        s.align_items = alignment(a)?.unwrap_or_default();
                        s.justify_items = alignment(b)?.unwrap_or_default();
                    }
                    "place-self" => {
                        s.align_self = alignment(a)?;
                        s.justify_self = alignment(b)?;
                    }
                    _ => {
                        s.align_content = distribution(a)?.unwrap_or_default();
                        s.justify_content = distribution(b)?.unwrap_or_default();
                    }
                }
            }
            "grid-template-columns" | "grid-template-rows" => {
                output.template_grid = true;
                let axis = usize::from(name == "grid-template-rows");
                output.format.subgrid[axis] = parts
                    .first()
                    .is_some_and(|part| part.eq_ignore_ascii_case("subgrid"));
                output.format.subgrid_names[axis].clear();
                if output.format.subgrid[axis] {
                    for part in &parts[1..] {
                        let inner = part
                            .strip_prefix('[')
                            .and_then(|v| v.strip_suffix(']'))
                            .ok_or("expected subgrid line names")?;
                        let mut parser = Parser::new(inner);
                        let mut names = Vec::new();
                        while !parser.is_exhausted() {
                            names.push(
                                parser
                                    .expect_ident_cloned()
                                    .map_err(|_| "invalid subgrid line name")?
                                    .to_string(),
                            );
                        }
                        output.format.subgrid_names[axis].push(names);
                    }
                    return Ok(());
                }
                let (tracks, names) = named_tracks(value, units)?;
                if name == "grid-template-columns" {
                    s.grid_template_columns = tracks;
                    s.grid_template_column_names = names;
                } else {
                    s.grid_template_rows = tracks;
                    s.grid_template_row_names = names;
                }
            }
            "grid" => {
                apply_grid_shorthand(value, s, &mut output.format, units)?;
                output.template_grid = true;
            }
            "grid-template" => {
                apply_grid_template(value, s, &mut output.format, units)?;
                output.template_grid = true;
            }
            "grid-template-areas" => {
                s.grid_template_areas = areas(value)?;
                output.template_grid = true;
            }
            "flow" => output.format.flow = parse_flow(value)?,
            "chains" => output.format.chains = parse_chains(value)?,
            "grid-auto-columns" => {
                s.grid_auto_columns = parts
                    .iter()
                    .map(|v| track(v, units))
                    .collect::<Result<_, _>>()?
            }
            "grid-auto-rows" => {
                s.grid_auto_rows = parts
                    .iter()
                    .map(|v| track(v, units))
                    .collect::<Result<_, _>>()?
            }
            "grid-auto-flow" => {
                s.grid_auto_flow = match value {
                    "row" => GridAutoFlow::Row,
                    "column" => GridAutoFlow::Column,
                    "dense" | "row dense" | "dense row" => GridAutoFlow::RowDense,
                    "column dense" | "dense column" => GridAutoFlow::ColumnDense,
                    _ => return Err("invalid grid-auto-flow"),
                }
            }
            "grid-column" => s.grid_column = placement_pair(value)?,
            "grid-row" => s.grid_row = placement_pair(value)?,
            "grid-column-start" => s.grid_column.start = placement(value)?,
            "grid-column-end" => s.grid_column.end = placement(value)?,
            "grid-row-start" => s.grid_row.start = placement(value)?,
            "grid-row-end" => s.grid_row.end = placement(value)?,
            "grid-area" => {
                let values: Vec<_> = value.split('/').map(str::trim).collect();
                if values.is_empty() || values.len() > 4 {
                    return Err("invalid grid-area");
                }
                s.grid_row.start = placement(values[0])?;
                let default = if matches!(s.grid_row.start, GridPlacement::NamedLine(_, _)) {
                    values[0]
                } else {
                    "auto"
                };
                s.grid_column.start = placement(values.get(1).copied().unwrap_or(default))?;
                s.grid_row.end = placement(values.get(2).copied().unwrap_or(default))?;
                s.grid_column.end = placement(values.get(3).copied().unwrap_or(default))?;
            }
            _ => return Err("unsupported layout property"),
        }
        Ok(())
    }
}

fn inherited(name: &str) -> bool {
    matches!(
        name,
        "font-size"
            | "color"
            | "font-kerning"
            | "direction"
            | "visibility"
            | "writing-mode"
            | "text-orientation"
            | "line-height"
            | "text-align"
            | "text-align-all"
            | "text-align-last"
            | "text-justify"
            | "text-overflow"
            | "white-space"
            | "letter-spacing"
            | "word-spacing"
            | "tab-size"
            | "text-indent"
            | "text-transform"
            | "line-break"
            | "overflow-wrap"
            | "word-break"
            | "hyphens"
            | "hanging-punctuation"
            | "border-collapse"
            | "border-spacing"
            | "caption-side"
            | "empty-cells"
            | "orphans"
            | "widows"
            | "scrollbar-color"
            | "object-position"
    )
}

fn copy_property(name: &str, out: &mut BoxStyle, source: &BoxStyle) {
    if name == "opacity" {
        out.opacity = source.opacity;
        return;
    }
    if name == "object-fit" {
        out.object_fit = source.object_fit;
        return;
    }
    if name == "object-position" {
        out.object_position = source.object_position.clone();
        return;
    }
    if crate::text_decoration::supported(name) {
        let mut decoration = source.text_decoration.clone();
        decoration.color = Some(decoration.color.unwrap_or(source.current_color));
        crate::text_decoration::copy_property(name, &mut out.text_decoration, decoration);
        return;
    }
    if name == "color" {
        out.current_color = source.current_color;
        return;
    }
    if name == "background-color" {
        out.background_color = source.background_color;
        out.background_set = true;
        out.background_current_color = false;
        return;
    }
    if outline::supported(name) {
        outline::copy_property(
            name,
            &mut out.outline,
            source.outline.inherited(source.current_color),
        );
        return;
    }
    if crate::box_decoration::copy_property(name, out, source) {
        return;
    }
    if crate::formatting::supported(name) {
        copy_format_property(name, &mut out.format, &source.format);
        return;
    }
    if name == "overflow-clip-margin" {
        out.overflow_clip_box = source.overflow_clip_box;
        out.overflow_clip_margin = source.overflow_clip_margin;
        return;
    }
    if name == "overscroll-behavior" {
        out.overscroll_behavior = source.overscroll_behavior;
        return;
    }
    if let Some(target_axis) = overscroll_axis(name, out.format.writing) {
        let source_axis = overscroll_axis(name, source.format.writing).unwrap();
        out.overscroll_behavior[target_axis] = source.overscroll_behavior[source_axis];
        return;
    }
    if name == "flow" {
        out.format.flow.clone_from(&source.format.flow);
        return;
    }
    if name == "chains" {
        out.format.chains.clone_from(&source.format.chains);
        return;
    }
    if name == "margin-trim" {
        out.margin_trim = source.margin_trim;
        return;
    }
    if name == "scrollbar-gutter" {
        out.scrollbar_gutter = source.scrollbar_gutter;
        return;
    }
    if name == "scrollbar-color" {
        out.scrollbar_color = source.scrollbar_color.inherited(source.current_color);
        return;
    }
    if name == "scrollbar-width" {
        out.scrollbar_width = source.scrollbar_width;
        return;
    }
    if name == "will-change" {
        out.will_change.clone_from(&source.will_change);
        return;
    }
    if let Some(side_name) = name.strip_prefix("overflow-clip-margin-") {
        let copy_side = |side_name: &str, out: &mut BoxStyle, source: &BoxStyle| {
            let target = logical_side(
                side_name,
                out.layout.direction,
                out.format.writing,
                &out.format.text_orientation,
            )
            .or_else(|| side(side_name))
            .unwrap();
            let source_edge = logical_side(
                side_name,
                source.layout.direction,
                source.format.writing,
                &source.format.text_orientation,
            )
            .or_else(|| side(side_name))
            .unwrap();
            out.overflow_clip_margin[target] = source.overflow_clip_margin[source_edge];
        };
        match side_name {
            "block" | "inline" => {
                let pair = if side_name == "inline" {
                    ["inline-start", "inline-end"]
                } else {
                    ["block-start", "block-end"]
                };
                for side_name in pair {
                    copy_side(side_name, out, source);
                }
            }
            _ => copy_side(side_name, out, source),
        }
        return;
    }
    if name.ends_with("inline-size") || name.ends_with("block-size") {
        let inline = name.ends_with("inline-size");
        fn copy<T: Copy>(
            source: Size<T>,
            to: &mut Size<T>,
            source_width: bool,
            target_width: bool,
        ) {
            let value = if source_width {
                source.width
            } else {
                source.height
            };
            if target_width {
                to.width = value;
            } else {
                to.height = value;
            }
        }
        let a = inline != source.format.writing.vertical();
        let b = inline != out.format.writing.vertical();
        if name.starts_with("min-") {
            copy(source.layout.min_size, &mut out.layout.min_size, a, b);
            copy(source.intrinsic_min, &mut out.intrinsic_min, a, b);
        } else if name.starts_with("max-") {
            copy(source.layout.max_size, &mut out.layout.max_size, a, b);
            copy(source.intrinsic_max, &mut out.intrinsic_max, a, b);
        } else {
            copy(source.layout.size, &mut out.layout.size, a, b);
        }
        return;
    }
    if let Some((group, logical)) = name.split_once('-')
        && matches!(group, "margin" | "padding" | "inset")
        && (logical.starts_with("inline") || logical.starts_with("block"))
    {
        let sides = match logical {
            "inline" => vec!["inline-start", "inline-end"],
            "block" => vec!["block-start", "block-end"],
            _ => vec![logical],
        };
        for side in sides {
            let to = logical_side(
                side,
                out.layout.direction,
                out.format.writing,
                &out.format.text_orientation,
            )
            .unwrap();
            let from = logical_side(
                side,
                source.layout.direction,
                source.format.writing,
                &source.format.text_orientation,
            )
            .unwrap();
            match group {
                "margin" => {
                    *edge_mut(&mut out.layout.margin, to) =
                        *edge_mut(&mut source.layout.margin.clone(), from)
                }
                "inset" => {
                    *edge_mut(&mut out.layout.inset, to) =
                        *edge_mut(&mut source.layout.inset.clone(), from)
                }
                _ => {
                    *edge_mut(&mut out.layout.padding, to) =
                        *edge_mut(&mut source.layout.padding.clone(), from);
                    out.padding_set = true;
                }
            }
        }
        return;
    }

    if name.starts_with("border-inline") || name.starts_with("border-block") {
        for physical in physical_borders(
            name,
            out.layout.direction,
            out.format.writing,
            &out.format.text_orientation,
        ) {
            copy_property(&physical, out, source);
        }
        return;
    }
    let to = &mut out.layout;
    let from = &source.layout;
    match name {
        "display" => {
            to.display = from.display;
            out.display_set = true;
            out.inline = source.inline;
            out.contents = source.contents;
            out.format.display = source.format.display;
        }
        "box-sizing" => to.box_sizing = from.box_sizing,
        "direction" => to.direction = from.direction,
        "float" => to.float = from.float,
        "clear" => to.clear = from.clear,
        "position" => {
            to.position = from.position;
            out.positioned = source.positioned;
            out.fixed = source.fixed;
        }
        "width" | "inline-size" => to.size.width = from.size.width,
        "height" | "block-size" => to.size.height = from.size.height,
        "min-width" | "min-inline-size" => {
            to.min_size.width = from.min_size.width;
            out.intrinsic_min.width = source.intrinsic_min.width;
        }
        "min-height" | "min-block-size" => {
            to.min_size.height = from.min_size.height;
            out.intrinsic_min.height = source.intrinsic_min.height;
        }
        "max-width" | "max-inline-size" => {
            to.max_size.width = from.max_size.width;
            out.intrinsic_max.width = source.intrinsic_max.width;
        }
        "max-height" | "max-block-size" => {
            to.max_size.height = from.max_size.height;
            out.intrinsic_max.height = source.intrinsic_max.height;
        }
        "aspect-ratio" => to.aspect_ratio = from.aspect_ratio,
        "margin-trim" => out.margin_trim = source.margin_trim,
        "visibility" => {
            out.hidden = source.hidden;
            out.collapsed = source.collapsed;
        }
        "order" => out.order = source.order,
        "z-index" => out.z_index = source.z_index,
        "overflow" => to.overflow = from.overflow,
        "overflow-x" => to.overflow.x = from.overflow.x,
        "overflow-y" => to.overflow.y = from.overflow.y,
        "overflow-block" => {
            let source_axis = usize::from(!source.format.writing.vertical());
            let target_axis = usize::from(!out.format.writing.vertical());
            let value = if source_axis == 0 {
                source.layout.overflow.x
            } else {
                source.layout.overflow.y
            };
            if target_axis == 0 {
                to.overflow.x = value;
            } else {
                to.overflow.y = value;
            }
        }
        "overflow-inline" => {
            let source_axis = usize::from(source.format.writing.vertical());
            let target_axis = usize::from(out.format.writing.vertical());
            let value = if source_axis == 0 {
                source.layout.overflow.x
            } else {
                source.layout.overflow.y
            };
            if target_axis == 0 {
                to.overflow.x = value;
            } else {
                to.overflow.y = value;
            }
        }
        "gap" => to.gap = from.gap,
        "row-gap" => to.gap.height = from.gap.height,
        "column-gap" => to.gap.width = from.gap.width,
        "flex-direction" => to.flex_direction = from.flex_direction,
        "flex-wrap" => to.flex_wrap = from.flex_wrap,
        "flex-flow" => {
            to.flex_direction = from.flex_direction;
            to.flex_wrap = from.flex_wrap;
        }
        "flex-grow" => to.flex_grow = from.flex_grow,
        "flex-shrink" => to.flex_shrink = from.flex_shrink,
        "flex-basis" => to.flex_basis = from.flex_basis,
        "flex" => {
            to.flex_grow = from.flex_grow;
            to.flex_shrink = from.flex_shrink;
            to.flex_basis = from.flex_basis;
        }
        "align-items" => to.align_items = from.align_items,
        "justify-items" => to.justify_items = from.justify_items,
        "align-self" => to.align_self = from.align_self,
        "justify-self" => to.justify_self = from.justify_self,
        "align-content" => to.align_content = from.align_content,
        "justify-content" => to.justify_content = from.justify_content,
        "place-items" => {
            to.align_items = from.align_items;
            to.justify_items = from.justify_items;
        }
        "place-self" => {
            to.align_self = from.align_self;
            to.justify_self = from.justify_self;
        }
        "place-content" => {
            to.align_content = from.align_content;
            to.justify_content = from.justify_content;
        }
        "grid-template-columns" => {
            out.format.subgrid[0] = source.format.subgrid[0];
            out.format.subgrid_names[0] = source.format.subgrid_names[0].clone();
            to.grid_template_columns = from.grid_template_columns.clone();
            to.grid_template_column_names = from.grid_template_column_names.clone();
        }
        "grid-template-rows" => {
            out.format.subgrid[1] = source.format.subgrid[1];
            out.format.subgrid_names[1] = source.format.subgrid_names[1].clone();
            to.grid_template_rows = from.grid_template_rows.clone();
            to.grid_template_row_names = from.grid_template_row_names.clone();
        }
        "grid-template" => {
            out.format.subgrid = source.format.subgrid;
            out.format.subgrid_names = source.format.subgrid_names.clone();
            to.grid_template_rows = from.grid_template_rows.clone();
            to.grid_template_columns = from.grid_template_columns.clone();
            to.grid_template_row_names = from.grid_template_row_names.clone();
            to.grid_template_column_names = from.grid_template_column_names.clone();
            to.grid_template_areas = from.grid_template_areas.clone();
        }
        "grid" => {
            out.format.subgrid = source.format.subgrid;
            out.format.subgrid_names = source.format.subgrid_names.clone();
            to.grid_template_rows = from.grid_template_rows.clone();
            to.grid_template_columns = from.grid_template_columns.clone();
            to.grid_template_row_names = from.grid_template_row_names.clone();
            to.grid_template_column_names = from.grid_template_column_names.clone();
            to.grid_template_areas = from.grid_template_areas.clone();
            to.grid_auto_rows = from.grid_auto_rows.clone();
            to.grid_auto_columns = from.grid_auto_columns.clone();
            to.grid_auto_flow = from.grid_auto_flow;
        }
        "grid-template-areas" => to.grid_template_areas = from.grid_template_areas.clone(),
        "grid-auto-columns" => to.grid_auto_columns = from.grid_auto_columns.clone(),
        "grid-auto-rows" => to.grid_auto_rows = from.grid_auto_rows.clone(),
        "grid-auto-flow" => to.grid_auto_flow = from.grid_auto_flow,
        "grid-column" => to.grid_column = from.grid_column.clone(),
        "grid-row" => to.grid_row = from.grid_row.clone(),
        "grid-column-start" => to.grid_column.start = from.grid_column.start.clone(),
        "grid-column-end" => to.grid_column.end = from.grid_column.end.clone(),
        "grid-row-start" => to.grid_row.start = from.grid_row.start.clone(),
        "grid-row-end" => to.grid_row.end = from.grid_row.end.clone(),
        "grid-area" => {
            to.grid_column = from.grid_column.clone();
            to.grid_row = from.grid_row.clone();
        }
        _ if name.starts_with("border") => {
            out.border_set = true;
            let parts: Vec<_> = name.split('-').skip(1).collect();
            for i in 0..4 {
                if parts
                    .first()
                    .is_some_and(|part| side(part).is_some_and(|s| s != i))
                {
                    continue;
                }
                let property = parts.last().copied().unwrap_or("");
                if property == "width" || !matches!(property, "color" | "style") {
                    out.border_widths[i] = source.border_widths[i];
                }
                if property == "style" || !matches!(property, "color" | "width") {
                    out.border_styles[i] = source.border_styles[i];
                }
                if property == "color" || !matches!(property, "style" | "width") {
                    out.border_colors[i] = source.border_colors[i];
                    out.border_stripes[i] = source.border_stripes[i].clone();
                }
            }
        }
        _ => {
            let (group, suffix) = name.split_once('-').unwrap_or((name, ""));
            for i in 0..4 {
                let included = match suffix {
                    "" => true,
                    "inline" => i == 1 || i == 3,
                    "block" => i == 0 || i == 2,
                    _ => {
                        logical_side(
                            suffix,
                            to.direction,
                            out.format.writing,
                            &out.format.text_orientation,
                        ) == Some(i)
                    }
                };
                if !included {
                    continue;
                }
                match group {
                    "margin" => {
                        *edge_mut(&mut to.margin, i) = *edge_mut(&mut from.margin.clone(), i)
                    }
                    "padding" => {
                        *edge_mut(&mut to.padding, i) = *edge_mut(&mut from.padding.clone(), i);
                        out.padding_set = true;
                    }
                    "inset" => *edge_mut(&mut to.inset, i) = *edge_mut(&mut from.inset.clone(), i),
                    _ => {
                        if let Some(i) = side(name) {
                            *edge_mut(&mut to.inset, i) = *edge_mut(&mut from.inset.clone(), i);
                        }
                    }
                }
            }
        }
    }
}

impl BoxStyle {
    pub(crate) fn finish(&mut self) {
        if self.template_grid && !self.display_set {
            self.layout.display = Display::Grid;
        }
        if self.will_change("position") && !self.positioned {
            self.layout.position = Position::Relative;
        }
        let widths = std::array::from_fn::<_, 4, _>(|i| {
            if matches!(
                self.border_styles[i],
                BorderStyle::None | BorderStyle::Hidden
            ) {
                0.0
            } else {
                self.border_widths[i]
            }
        });
        self.layout.border = taffy::Rect {
            top: length(widths[0]),
            right: length(widths[1]),
            bottom: length(widths[2]),
            left: length(widths[3]),
        };
        if !self.positioned {
            self.layout.inset = taffy::Rect::auto();
        }
        let x = self.layout.overflow.x;
        let y = self.layout.overflow.y;
        let scroll = |v| !matches!(v, Overflow::Visible | Overflow::Clip);
        let computed = |v| match v {
            Overflow::Visible => Overflow::Scroll,
            Overflow::Clip => Overflow::Hidden,
            _ => v,
        };
        if scroll(y) {
            self.layout.overflow.x = computed(x);
        }
        if scroll(x) {
            self.layout.overflow.y = computed(y);
        }
    }

    pub(crate) fn will_change(&self, property: &str) -> bool {
        self.will_change
            .iter()
            .any(|value| value.eq_ignore_ascii_case(property))
    }
}

fn opacity_value(value: &str, units: Units) -> Result<f32, &'static str> {
    if let Some(percentage) = crate::math::percentage(value, units, true)? {
        return Ok(crate::math::resolve(percentage, 1.0).clamp(0.0, 1.0));
    }
    if let Some(number) = crate::math::number(value, units)? {
        return number
            .is_finite()
            .then(|| number.clamp(0.0, 1.0))
            .ok_or("invalid opacity");
    }

    let mut input = Parser::new(value);
    let number = match input.next().map_err(|_| "invalid opacity")? {
        Token::Number { value, .. } => *value,
        Token::Percentage { unit_value, .. } => *unit_value,
        _ => return Err("expected opacity number or percentage"),
    };
    input.expect_exhausted().map_err(|_| "invalid opacity")?;
    number
        .is_finite()
        .then(|| number.clamp(0.0, 1.0))
        .ok_or("invalid opacity")
}

fn parse_will_change(value: &str) -> Result<Vec<String>, &'static str> {
    let mut input = Parser::new(value);
    let first = input
        .expect_ident_cloned()
        .map_err(|_| "invalid will-change")?;
    if first.eq_ignore_ascii_case("auto") {
        return if input.is_exhausted() {
            Ok(Vec::new())
        } else {
            Err("invalid will-change")
        };
    }

    let mut properties = Vec::new();
    let mut current = Some(first);
    loop {
        let Some(value) = current.take() else {
            break;
        };
        if ["will-change", "none", "all", "auto"]
            .iter()
            .any(|keyword| value.eq_ignore_ascii_case(keyword))
        {
            return Err("invalid will-change feature");
        }
        if !["scroll-position", "contents"]
            .iter()
            .any(|keyword| value.eq_ignore_ascii_case(keyword))
            && ["inherit", "initial", "unset", "revert", "revert-layer"]
                .iter()
                .any(|keyword| value.eq_ignore_ascii_case(keyword))
        {
            return Err("invalid will-change feature");
        }
        properties.push(value.to_string());

        if input.is_exhausted() {
            break;
        }
        input.expect_comma().map_err(|_| "invalid will-change")?;
        current = Some(
            input
                .expect_ident_cloned()
                .map_err(|_| "invalid will-change")?,
        );
    }
    Ok(properties)
}

fn clip_margin_length(value: &str, units: Units) -> Result<f32, &'static str> {
    if let Some(handle) = crate::math::length(value, units, false, false)? {
        return Ok(crate::math::resolve(handle, 0.0));
    }
    let (value, percentage) = length_value(value, units, false)?;
    if percentage {
        return Err("percentage overflow-clip-margin is invalid");
    }
    Ok(value)
}

fn apply_clip_margin(
    clip_box: &mut OverflowClipBox,
    margins: &mut [f32; 4],
    parts: &[String],
    units: Units,
) -> Result<(), &'static str> {
    let (visual_box, value) = match parts {
        [value] => (OverflowClipBox::Padding, value.as_str()),
        [visual_box, value] => (
            match visual_box.as_str() {
                "content-box" => OverflowClipBox::Content,
                "padding-box" => OverflowClipBox::Padding,
                "border-box" => OverflowClipBox::Border,
                _ => return Err("invalid overflow-clip-margin visual box"),
            },
            value.as_str(),
        ),
        _ => return Err("invalid overflow-clip-margin"),
    };
    let value = clip_margin_length(value, units)?;
    *clip_box = visual_box;
    *margins = [value; 4];
    Ok(())
}

fn apply_margin_trim(trim: &mut [bool; 4], parts: &[String]) -> Result<(), &'static str> {
    if parts.is_empty() {
        return Err("invalid margin-trim");
    }
    let parts = parts
        .iter()
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if parts.as_slice() == ["none"] {
        *trim = [false; 4];
        return Ok(());
    }
    let mut parsed = [false; 4];
    for part in &parts {
        match part.as_str() {
            "block" if parts.len() == 1 => set_trim(&mut parsed, &[0, 1])?,
            "block-start" => set_trim(&mut parsed, &[0])?,
            "block-end" => set_trim(&mut parsed, &[1])?,
            _ => return Err("invalid margin-trim"),
        }
    }
    *trim = parsed;
    Ok(())
}

fn set_trim(trim: &mut [bool; 4], sides: &[usize]) -> Result<(), &'static str> {
    if sides.iter().any(|side| trim[*side]) {
        return Err("duplicate margin-trim side");
    }
    for side in sides {
        trim[*side] = true;
    }
    Ok(())
}

fn quad(parts: &[String]) -> Result<[&str; 4], &'static str> {
    match parts {
        [a] => Ok([a, a, a, a]),
        [a, b] => Ok([a, b, a, b]),
        [a, b, c] => Ok([a, b, c, b]),
        [a, b, c, d] => Ok([a, b, c, d]),
        _ => Err("expected one to four values"),
    }
}

fn side(value: &str) -> Option<usize> {
    match value {
        "top" | "block-start" => Some(0),
        "right" | "inline-end" => Some(1),
        "bottom" | "block-end" => Some(2),
        "left" | "inline-start" => Some(3),
        _ => None,
    }
}

fn edge_mut<T>(rect: &mut taffy::Rect<T>, side: usize) -> &mut T {
    match side {
        0 => &mut rect.top,
        1 => &mut rect.right,
        2 => &mut rect.bottom,
        _ => &mut rect.left,
    }
}

fn set_edge(
    style: &mut BoxStyle,
    group: &str,
    side: usize,
    value: &str,
    units: Units,
) -> Result<(), &'static str> {
    if group == "padding" {
        *edge_mut(&mut style.layout.padding, side) = gap(value, units)?;
        style.padding_set = true;
    } else {
        let length = if value == "auto" {
            auto()
        } else if let Some(handle) = crate::math::length(value, units, true, true)? {
            LengthPercentageAuto::calc(handle)
        } else {
            let (v, p) = length_value(value, units, true)?;
            if p { percent(v) } else { length(v) }
        };
        *edge_mut(
            if group == "margin" {
                &mut style.layout.margin
            } else {
                &mut style.layout.inset
            },
            side,
        ) = length;
    }
    Ok(())
}

fn overflow(value: &str) -> Result<Overflow, &'static str> {
    match value {
        "visible" => Ok(Overflow::Visible),
        "hidden" => Ok(Overflow::Hidden),
        "clip" => Ok(Overflow::Clip),
        "scroll" => Ok(Overflow::Scroll),
        "auto" => Ok(Overflow::Scroll),
        _ => Err("invalid overflow"),
    }
}

fn overscroll_behavior(value: &str) -> Result<OverscrollBehavior, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => Ok(OverscrollBehavior::Auto),
        "contain" => Ok(OverscrollBehavior::Contain),
        "none" => Ok(OverscrollBehavior::None),
        _ => Err("invalid overscroll-behavior"),
    }
}

fn overscroll_axis(name: &str, writing: crate::formatting::Writing) -> Option<usize> {
    match name {
        "overscroll-behavior-x" => Some(0),
        "overscroll-behavior-y" => Some(1),
        "overscroll-behavior-inline" => Some(usize::from(writing.vertical())),
        "overscroll-behavior-block" => Some(usize::from(!writing.vertical())),
        _ => None,
    }
}

fn scrollbar_color(value: &str) -> Result<ScrollbarColorValue, &'static str> {
    if value.eq_ignore_ascii_case("currentcolor") {
        return Ok(ScrollbarColorValue::CurrentColor);
    }
    crate::css::parse_color(&mut Parser::new(value))
        .map(ScrollbarColorValue::Color)
        .map_err(|_| "invalid scrollbar-color")
}

fn direction(value: &str) -> Result<FlexDirection, &'static str> {
    match value {
        "row" => Ok(FlexDirection::Row),
        "row-reverse" => Ok(FlexDirection::RowReverse),
        "column" => Ok(FlexDirection::Column),
        "column-reverse" => Ok(FlexDirection::ColumnReverse),
        _ => Err("invalid flex-direction"),
    }
}
fn wrap(value: &str) -> Result<FlexWrap, &'static str> {
    match value {
        "nowrap" => Ok(FlexWrap::NoWrap),
        "wrap" => Ok(FlexWrap::Wrap),
        "wrap-reverse" => Ok(FlexWrap::WrapReverse),
        _ => Err("invalid flex-wrap"),
    }
}
fn alignment(value: &str) -> Result<Option<AlignItems>, &'static str> {
    if value == "auto" {
        return Ok(None);
    }
    value.parse().map(Some).map_err(|_| "invalid alignment")
}
fn distribution(value: &str) -> Result<Option<AlignContent>, &'static str> {
    value
        .parse()
        .map(Some)
        .map_err(|_| "invalid content alignment")
}

fn apply_flex(
    s: &mut Style,
    value: &str,
    parts: &[String],
    units: Units,
) -> Result<(), &'static str> {
    match value {
        "none" => {
            s.flex_grow = 0.0;
            s.flex_shrink = 0.0;
            s.flex_basis = auto();
        }
        "auto" => {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = auto();
        }
        "initial" => {
            s.flex_grow = 0.0;
            s.flex_shrink = 1.0;
            s.flex_basis = auto();
        }
        _ => {
            s.flex_grow = 1.0;
            s.flex_shrink = 1.0;
            s.flex_basis = percent(0.0_f32);
            match parts {
                [a] => {
                    if let Ok(n) = number(a, units) {
                        s.flex_grow = n;
                    } else {
                        s.flex_basis = dimension(a, units)?;
                    }
                }
                [a, b] => {
                    s.flex_grow = number(a, units)?;
                    if let Ok(n) = number(b, units) {
                        s.flex_shrink = n;
                    } else {
                        s.flex_basis = dimension(b, units)?;
                    }
                }
                [a, b, c] => {
                    s.flex_grow = number(a, units)?;
                    s.flex_shrink = number(b, units)?;
                    s.flex_basis = dimension(c, units)?;
                }
                _ => return Err("invalid flex"),
            }
        }
    }
    Ok(())
}

pub(crate) fn resolve_flow(child: &mut BoxStyle, parent: &BoxStyle) {
    if parent.layout.display != Display::Grid {
        return;
    }
    let Some(areas) = parent.layout.grid_template_areas.as_ref() else {
        return;
    };
    let name = child.format.flow.clone().or_else(|| {
        areas
            .areas
            .iter()
            .find(|area| area.name == "*")
            .or_else(|| {
                (!parent.format.chains.is_empty())
                    .then(|| areas.areas.first())
                    .flatten()
            })
            .map(|area| area.name.clone())
    });
    let Some(name) = name else {
        return;
    };
    let Some(area) = areas.areas.iter().find(|area| area.name == name) else {
        return;
    };
    let line = |value: u16| placement(&value.to_string()).expect("grid area lines are valid");
    child.layout.grid_row = taffy::Line {
        start: line(area.row_start),
        end: line(area.row_end),
    };
    child.layout.grid_column = taffy::Line {
        start: line(area.column_start),
        end: line(area.column_end),
    };
    child.format.flow = Some(name);
}

pub(crate) fn border_style(value: &str) -> Result<BorderStyle, &'static str> {
    Ok(match value.to_ascii_lowercase().as_str() {
        "none" => BorderStyle::None,
        "hidden" => BorderStyle::Hidden,
        "solid" => BorderStyle::Solid,
        "dashed" => BorderStyle::Dashed,
        "dotted" => BorderStyle::Dotted,
        "double" => BorderStyle::Double,
        "groove" => BorderStyle::Groove,
        "ridge" => BorderStyle::Ridge,
        "inset" => BorderStyle::Inset,
        "outset" => BorderStyle::Outset,
        _ => return Err("invalid border style"),
    })
}
pub(crate) fn border_width(value: &str, units: Units) -> Result<f32, &'static str> {
    if let Some(handle) = crate::math::length(value, units, false, false)? {
        return Ok(crate::math::resolve(handle, 0.0));
    }
    match value.to_ascii_lowercase().as_str() {
        "hairline" => Ok(1.0 / units.pixels_per_point.max(f32::MIN_POSITIVE)),
        "thin" => Ok(1.0),
        "medium" => Ok(3.0),
        "thick" => Ok(5.0),
        _ => {
            let (v, p) = length_value(value, units, false)?;
            if p {
                Err("percentage border width")
            } else {
                Ok(v)
            }
        }
    }
}
pub(crate) fn border_color(value: &str) -> Result<Option<Color32>, &'static str> {
    if value.eq_ignore_ascii_case("currentcolor") {
        return Ok(None);
    }
    crate::css::parse_color(&mut Parser::new(value))
        .map(Some)
        .map_err(|_| "invalid border color")
}

fn border_paint(
    value: &str,
    units: Units,
) -> Result<
    (
        Option<Color32>,
        Option<Vec<crate::box_decoration::BorderStripe>>,
    ),
    &'static str,
> {
    if value.to_ascii_lowercase().starts_with("stripes(") && value.ends_with(')') {
        let stripes = crate::box_decoration::parse_border_stripes(value, units)?;
        return Ok((None, Some(stripes)));
    }
    Ok((border_color(value)?, None))
}
fn apply_border(
    s: &mut BoxStyle,
    name: &str,
    parts: &[String],
    units: Units,
) -> Result<(), &'static str> {
    s.border_set = true;
    if name == "border" {
        s.decorations.border_image = crate::box_decoration::BorderImage::default();
    }
    let suffix = name.strip_prefix("border-").unwrap_or("");
    if matches!(suffix, "width" | "style" | "color") {
        for (i, v) in quad(parts)?.iter().enumerate() {
            match suffix {
                "width" => s.border_widths[i] = border_width(v, units)?,
                "style" => s.border_styles[i] = border_style(v)?,
                _ => {
                    let (color, stripes) = border_paint(v, units)?;
                    s.border_colors[i] = color;
                    s.border_stripes[i] = stripes;
                }
            }
        }
    } else if let Some((edge, property)) = suffix.split_once('-') {
        let i = side(edge).ok_or("invalid border side")?;
        if parts.len() != 1 {
            return Err("invalid border value");
        }
        match property {
            "width" => s.border_widths[i] = border_width(&parts[0], units)?,
            "style" => s.border_styles[i] = border_style(&parts[0])?,
            "color" => {
                let (color, stripes) = border_paint(&parts[0], units)?;
                s.border_colors[i] = color;
                s.border_stripes[i] = stripes;
            }
            _ => return Err("invalid border property"),
        }
    } else {
        let mut width = None;
        let mut style = None;
        let mut color: Option<(
            Option<Color32>,
            Option<Vec<crate::box_decoration::BorderStripe>>,
        )> = None;
        for value in parts {
            if let Ok(v) = border_style(value) {
                if style.replace(v).is_some() {
                    return Err("duplicate border style");
                }
            } else if let Ok(v) = border_width(value, units) {
                if width.replace(v).is_some() {
                    return Err("duplicate border width");
                }
            } else {
                let (parsed, stripes) = border_paint(value, units)?;
                if color.replace((parsed, stripes)).is_some() {
                    return Err("duplicate border color");
                }
            }
        }
        if parts.is_empty() {
            return Err("empty border");
        }
        for i in 0..4 {
            if suffix.is_empty() || side(suffix) == Some(i) {
                s.border_widths[i] = width.unwrap_or(3.0);
                s.border_styles[i] = style.unwrap_or_default();
                if let Some((color, stripes)) = &color {
                    s.border_colors[i] = *color;
                    s.border_stripes[i] = stripes.clone();
                } else {
                    s.border_colors[i] = None;
                    s.border_stripes[i] = None;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn limit(value: &str, units: Units) -> Result<LengthPercentageAuto, &'static str> {
    if intrinsic_sizing(value).is_some() {
        return Ok(auto());
    }
    if value == "auto" {
        return Ok(auto());
    }
    if let Some(handle) = crate::math::length(value, units, false, true)? {
        return Ok(taffy::LengthPercentage::calc(handle).into());
    }
    let (v, p) = length_value(value, units, false)?;
    Ok(if p { percent(v) } else { length(v) })
}

pub(crate) fn intrinsic_sizing(value: &str) -> Option<IntrinsicSizing> {
    match value.trim().to_ascii_lowercase().as_str() {
        "min-content" => Some(IntrinsicSizing::MinContent),
        "max-content" => Some(IntrinsicSizing::MaxContent),
        _ => None,
    }
}

pub(crate) fn logical_side(
    value: &str,
    direction: taffy::Direction,
    writing: crate::formatting::Writing,
    text_orientation: &str,
) -> Option<usize> {
    let direction = if writing.vertical() && text_orientation.eq_ignore_ascii_case("upright") {
        taffy::Direction::Ltr
    } else {
        direction
    };
    if writing.vertical() {
        return match value {
            "inline-start" => Some(if direction == taffy::Direction::Rtl {
                2
            } else {
                0
            }),
            "inline-end" => Some(if direction == taffy::Direction::Rtl {
                0
            } else {
                2
            }),
            "block-start" => Some(if writing.reverse() { 1 } else { 3 }),
            "block-end" => Some(if writing.reverse() { 3 } else { 1 }),
            _ => side(value),
        };
    }
    if direction == taffy::Direction::Rtl {
        match value {
            "inline-start" => return Some(1),
            "inline-end" => return Some(3),
            _ => {}
        }
    }
    side(value)
}

fn physical_borders(
    name: &str,
    direction: taffy::Direction,
    writing: crate::formatting::Writing,
    text_orientation: &str,
) -> Vec<String> {
    let parts: Vec<_> = name.split('-').skip(1).collect();
    let inline = parts[0] == "inline";
    let names = ["top", "right", "bottom", "left"];
    let start = if inline {
        "inline-start"
    } else {
        "block-start"
    };
    let end = if inline { "inline-end" } else { "block-end" };
    let edges = [
        names[logical_side(start, direction, writing, text_orientation).unwrap()],
        names[logical_side(end, direction, writing, text_orientation).unwrap()],
    ];
    let (indices, suffix): (&[usize], &[&str]) = match parts.get(1) {
        Some(&"start") => (&[0], &parts[2..]),
        Some(&"end") => (&[1], &parts[2..]),
        _ => (&[0, 1], &parts[1..]),
    };
    indices
        .iter()
        .map(|i| {
            let mut property = format!("border-{}", edges[*i]);
            if !suffix.is_empty() {
                property.push('-');
                property.push_str(&suffix.join("-"));
            }
            property
        })
        .collect()
}

fn copy_format_property(
    name: &str,
    to: &mut crate::formatting::Formatting,
    from: &crate::formatting::Formatting,
) {
    match name {
        "font-size" => to.em = from.em,
        "font-kerning" => to.font_kerning = from.font_kerning,
        "writing-mode" => to.writing = from.writing,
        "text-orientation" => to.text_orientation = from.text_orientation.clone(),
        "flow-from" => to.flow_from = from.flow_from.clone(),
        "flow-into" => {
            to.flow_into = from.flow_into.clone();
            to.flow_into_content = from.flow_into_content;
        }
        "region-fragment" => to.region_fragment = from.region_fragment,
        "column-count" => to.column_count = from.column_count,
        "column-width" => to.column_width = from.column_width,
        "columns" => {
            to.column_count = from.column_count;
            to.column_width = from.column_width;
        }
        "column-fill" => to.column_balance = from.column_balance,
        "column-span" => to.column_span = from.column_span,
        "column-rule" => to.column_rule = from.column_rule,
        "column-rule-width" => to.column_rule.width = from.column_rule.width,
        "column-rule-color" => to.column_rule.color = from.column_rule.color,
        "column-rule-style" => to.column_rule.style = from.column_rule.style,
        "break-before" | "page-break-before" => to.before = from.before,
        "break-after" | "page-break-after" => to.after = from.after,
        "break-inside" | "page-break-inside" => to.inside = from.inside,
        "orphans" => to.orphans = from.orphans,
        "widows" => to.widows = from.widows,
        "box-decoration-break" => to.clone_decoration = from.clone_decoration,
        "table-layout" => to.table_fixed = from.table_fixed,
        "border-collapse" => to.collapse = from.collapse,
        "border-spacing" => to.spacing = from.spacing,
        "caption-side" => to.caption_bottom = from.caption_bottom,
        "empty-cells" => to.hide_empty = from.hide_empty,
        "vertical-align" => to.vertical_align = from.vertical_align.clone(),
        "line-height" => to.line_height = from.line_height,
        "text-align" => {
            to.text_align = from.text_align.clone();
            to.text_align_last = from.text_align_last.clone();
        }
        "text-align-all" => to.text_align = from.text_align.clone(),
        "text-align-last" => to.text_align_last = from.text_align_last.clone(),
        "text-justify" => to.text_justify = from.text_justify.clone(),
        "text-overflow" => to.text_overflow = from.text_overflow.clone(),
        "white-space" => to.white_space = from.white_space.clone(),
        "letter-spacing" => to.letter_spacing = from.letter_spacing,
        "word-spacing" => to.word_spacing = from.word_spacing,
        "tab-size" => {
            to.tab_size = from.tab_size;
            to.tab_size_length = from.tab_size_length;
        }
        "text-indent" => {
            to.text_indent = from.text_indent;
            to.text_indent_each_line = from.text_indent_each_line;
            to.text_indent_hanging = from.text_indent_hanging;
        }
        "text-transform" => to.text_transform = from.text_transform.clone(),
        "line-break" => to.line_break = from.line_break.clone(),
        "overflow-wrap" => to.overflow_wrap = from.overflow_wrap.clone(),
        "word-break" => to.word_break = from.word_break.clone(),
        "hyphens" => to.hyphens = from.hyphens.clone(),
        "hanging-punctuation" => to.hanging_punctuation = from.hanging_punctuation.clone(),
        _ => {}
    }
}
