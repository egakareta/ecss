use crate::layout_style::{BoxStyle, Units};
use cssparser::{Parser, Token};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CollapsedBorder {
    pub width: f32,
    pub color: Color32,
    pub style: crate::layout_style::BorderStyle,
    pub draw: bool,
    pub priority: u8,
}
impl Default for CollapsedBorder {
    fn default() -> Self {
        Self {
            width: 0.0,
            color: Color32::BLACK,
            style: Default::default(),
            draw: false,
            priority: 0,
        }
    }
}
use egui::Color32;
use taffy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Writing {
    #[default]
    Horizontal,
    VerticalRl,
    VerticalLr,
    SidewaysRl,
    SidewaysLr,
}
impl Writing {
    pub fn vertical(self) -> bool {
        self != Self::Horizontal
    }
    pub fn reverse(self) -> bool {
        matches!(self, Self::VerticalRl | Self::SidewaysRl)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DisplayKind {
    #[default]
    Normal,
    Inline,
    Table,
    Row,
    Cell,
    RowGroup,
    Header,
    Footer,
    Column,
    ColumnGroup,
    Caption,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Break {
    #[default]
    Auto,
    Avoid,
    Column,
    Page,
    Always,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ColumnRule {
    pub width: f32,
    pub color: Option<Color32>,
    pub style: crate::layout_style::BorderStyle,
}
impl Default for ColumnRule {
    fn default() -> Self {
        Self {
            width: 3.0,
            color: None,
            style: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct Formatting {
    pub em: f32,
    pub writing: Writing,
    pub display: DisplayKind,
    pub inline_level: bool,
    pub subgrid: [bool; 2],
    pub subgrid_names: [Vec<Vec<String>>; 2],
    pub column_count: Option<usize>,
    pub column_width: Option<f32>,
    pub column_balance: bool,
    pub normal_gaps: [bool; 2],
    pub column_span: bool,
    pub column_rule: ColumnRule,
    pub before: Break,
    pub after: Break,
    pub inside: Break,
    pub widows: usize,
    pub orphans: usize,
    pub clone_decoration: bool,
    pub table_fixed: bool,
    pub collapse: bool,
    pub spacing: Size<f32>,
    pub caption_bottom: bool,
    pub hide_empty: bool,
    pub vertical_align: String,
    pub line_height: Option<f32>,
    pub text_align: String,
    pub text_overflow: [String; 2],
    pub white_space: String,
    pub text_orientation: String,
}
impl Default for Formatting {
    fn default() -> Self {
        Self {
            em: 16.0,
            writing: Writing::Horizontal,
            display: DisplayKind::Normal,
            inline_level: false,
            subgrid: [false; 2],
            subgrid_names: Default::default(),
            column_count: None,
            column_width: None,
            column_balance: true,
            normal_gaps: [true; 2],
            column_span: false,
            column_rule: Default::default(),
            before: Break::Auto,
            after: Break::Auto,
            inside: Break::Auto,
            widows: 2,
            orphans: 2,
            clone_decoration: false,
            table_fixed: false,
            collapse: false,
            spacing: Size::ZERO,
            caption_bottom: false,
            hide_empty: false,
            vertical_align: "baseline".into(),
            line_height: None,
            text_align: "start".into(),
            text_overflow: ["clip".into(), "clip".into()],
            white_space: "normal".into(),
            text_orientation: "mixed".into(),
        }
    }
}

pub(crate) fn supported(name: &str) -> bool {
    matches!(
        name,
        "writing-mode"
            | "text-orientation"
            | "column-count"
            | "column-width"
            | "columns"
            | "column-fill"
            | "column-span"
            | "column-rule"
            | "column-rule-width"
            | "column-rule-style"
            | "column-rule-color"
            | "break-before"
            | "break-after"
            | "break-inside"
            | "page-break-before"
            | "page-break-after"
            | "page-break-inside"
            | "orphans"
            | "widows"
            | "box-decoration-break"
            | "table-layout"
            | "border-collapse"
            | "border-spacing"
            | "caption-side"
            | "empty-cells"
            | "vertical-align"
            | "line-height"
            | "text-align"
            | "text-overflow"
            | "white-space"
    )
}

pub(crate) fn apply(
    name: &str,
    value: &str,
    out: &mut BoxStyle,
    units: Units,
) -> Result<bool, &'static str> {
    let vertical = out.format.writing.vertical();
    let inline = name.ends_with("inline-size");
    if inline || name.ends_with("block-size") {
        let axis = if inline != vertical { 0 } else { 1 };
        if name.starts_with("min-") {
            let intrinsic = crate::layout_style::intrinsic_sizing(value);
            let value = crate::layout_style::limit(value, units)?;
            if axis == 0 {
                out.layout.min_size.width = value;
                out.intrinsic_min.width = intrinsic;
            } else {
                out.layout.min_size.height = value;
                out.intrinsic_min.height = intrinsic;
            }
        } else if name.starts_with("max-") {
            let intrinsic = crate::layout_style::intrinsic_sizing(value);
            let value = if value == "none" {
                auto()
            } else {
                crate::layout_style::limit(value, units)?
            };
            if axis == 0 {
                out.layout.max_size.width = value;
                out.intrinsic_max.width = intrinsic;
            } else {
                out.layout.max_size.height = value;
                out.intrinsic_max.height = intrinsic;
            }
        } else {
            let value = crate::layout_style::dimension(value, units)?;
            if axis == 0 {
                out.layout.size.width = value;
            } else {
                out.layout.size.height = value;
            }
        }
        return Ok(true);
    }
    let f = &mut out.format;
    match name {
        "writing-mode" => {
            f.writing = match value {
                "horizontal-tb" => Writing::Horizontal,
                "vertical-rl" => Writing::VerticalRl,
                "vertical-lr" => Writing::VerticalLr,
                "sideways-rl" => Writing::SidewaysRl,
                "sideways-lr" => Writing::SidewaysLr,
                _ => return Err("invalid writing-mode"),
            }
        }
        "text-orientation" => {
            if !matches!(value, "mixed" | "upright" | "sideways") {
                return Err("invalid text-orientation");
            }
            f.text_orientation = value.into();
        }
        "column-count" => {
            f.column_count = if value == "auto" {
                None
            } else {
                Some(positive(value)?)
            }
        }
        "column-width" => {
            f.column_width = if value == "auto" {
                None
            } else {
                Some(length(value, units)?)
            }
        }
        "columns" => {
            f.column_count = None;
            f.column_width = None;
            for part in value.split_whitespace() {
                if part == "auto" {
                    continue;
                }
                if let Ok(count) = positive(part) {
                    if f.column_count.replace(count).is_some() {
                        return Err("duplicate column count");
                    }
                } else if f.column_width.replace(length(part, units)?).is_some() {
                    return Err("duplicate column width");
                }
            }
        }
        "column-fill" => {
            f.column_balance = match value {
                "balance" | "balance-all" => true,
                "auto" => false,
                _ => return Err("invalid column-fill"),
            }
        }
        "column-span" => {
            f.column_span = match value {
                "all" => true,
                "none" => false,
                _ => return Err("invalid column-span"),
            }
        }
        "break-before" | "page-break-before" => f.before = parse_break(value)?,
        "break-after" | "page-break-after" => f.after = parse_break(value)?,
        "break-inside" | "page-break-inside" => {
            f.inside = parse_break(value)?;
            if !matches!(f.inside, Break::Auto | Break::Avoid) {
                return Err("invalid break-inside");
            }
        }
        "orphans" => f.orphans = positive(value)?,
        "widows" => f.widows = positive(value)?,
        "box-decoration-break" => {
            f.clone_decoration = match value {
                "slice" => false,
                "clone" => true,
                _ => return Err("invalid box-decoration-break"),
            }
        }
        "table-layout" => {
            f.table_fixed = match value {
                "auto" => false,
                "fixed" => true,
                _ => return Err("invalid table-layout"),
            }
        }
        "border-collapse" => {
            f.collapse = match value {
                "separate" => false,
                "collapse" => true,
                _ => return Err("invalid border-collapse"),
            }
        }
        "border-spacing" => {
            let parts: Vec<_> = value.split_whitespace().collect();
            if parts.is_empty() || parts.len() > 2 {
                return Err("invalid border-spacing");
            }
            f.spacing = Size {
                width: length(parts[0], units)?,
                height: length(parts.get(1).copied().unwrap_or(parts[0]), units)?,
            };
        }
        "caption-side" => {
            f.caption_bottom = match value {
                "top" => false,
                "bottom" => true,
                _ => return Err("invalid caption-side"),
            }
        }
        "empty-cells" => {
            if !matches!(value, "show" | "hide") {
                return Err("invalid empty-cells");
            }
            f.hide_empty = value == "hide";
        }
        "vertical-align" => {
            if !matches!(
                value,
                "baseline"
                    | "top"
                    | "middle"
                    | "bottom"
                    | "text-top"
                    | "text-bottom"
                    | "sub"
                    | "super"
            ) {
                crate::layout_style::length_value(value, units, true)?;
            }
            f.vertical_align = value.into();
        }
        "line-height" => {
            f.line_height = if value == "normal" {
                None
            } else if let Ok(Some(n)) = crate::math::number(value, units) {
                Some(n.max(0.0) * units.font)
            } else if let Ok(n) = value.parse::<f32>() {
                if !n.is_finite() || n < 0.0 {
                    return Err("invalid line-height");
                }
                Some(n * units.font)
            } else if let Some(handle) = crate::math::length(value, units, false, true)? {
                Some(crate::math::resolve(handle, units.font))
            } else {
                let (n, p) = crate::layout_style::length_value(value, units, false)?;
                Some(if p { n * units.font } else { n })
            }
        }
        "text-align" => {
            if !matches!(
                value,
                "start" | "end" | "left" | "right" | "center" | "justify" | "match-parent"
            ) {
                return Err("invalid text-align");
            }
            f.text_align = value.into();
        }
        "text-overflow" => {
            let mut parser = Parser::new(value);
            let mut values = Vec::new();
            while !parser.is_exhausted() {
                parser.skip_whitespace();
                if parser.is_exhausted() {
                    break;
                }
                let value = match parser.next().map_err(|_| "invalid text-overflow")? {
                    Token::Ident(value) if matches_ignore_ascii_case(value, "clip") => "clip",
                    Token::Ident(value) if matches_ignore_ascii_case(value, "ellipsis") => {
                        "ellipsis"
                    }
                    _ => return Err("unsupported text-overflow value"),
                };
                values.push(value.to_owned());
                if values.len() > 2 {
                    return Err("too many text-overflow values");
                }
            }
            if values.is_empty() {
                return Err("invalid text-overflow");
            }
            f.text_overflow = if values.len() == 1 {
                ["clip".into(), values.remove(0)]
            } else {
                [values.remove(0), values.remove(0)]
            };
        }
        "white-space" => {
            if !matches!(
                value,
                "normal" | "nowrap" | "pre" | "pre-wrap" | "pre-line" | "break-spaces"
            ) {
                return Err("invalid white-space");
            }
            f.white_space = value.into();
        }
        "column-rule" => {
            let mut rule = ColumnRule::default();
            for part in crate::layout_style::components(value)? {
                if let Ok(n) = crate::layout_style::border_width(&part, units) {
                    rule.width = n;
                } else if let Ok(style) = crate::layout_style::border_style(&part) {
                    rule.style = style;
                } else {
                    rule.color = crate::layout_style::border_color(&part)?;
                }
            }
            f.column_rule = rule;
        }
        "column-rule-width" => {
            f.column_rule.width = crate::layout_style::border_width(value, units)?;
        }
        "column-rule-color" => {
            f.column_rule.color = crate::layout_style::border_color(value)?;
        }
        "column-rule-style" => {
            f.column_rule.style = crate::layout_style::border_style(value)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
fn matches_ignore_ascii_case(value: &str, expected: &str) -> bool {
    value.eq_ignore_ascii_case(expected)
}
fn positive(value: &str) -> Result<usize, &'static str> {
    let n: usize = value.parse().map_err(|_| "expected positive integer")?;
    if n == 0 || n > 65535 {
        Err("integer out of range")
    } else {
        Ok(n)
    }
}
fn length(value: &str, units: Units) -> Result<f32, &'static str> {
    if let Some(handle) = crate::math::length(value, units, false, false)? {
        return Ok(crate::math::resolve(handle, 0.0));
    }
    let (v, p) = crate::layout_style::length_value(value, units, false)?;
    if p { Err("expected length") } else { Ok(v) }
}
fn parse_break(value: &str) -> Result<Break, &'static str> {
    Ok(match value {
        "auto" => Break::Auto,
        "avoid" | "avoid-page" | "avoid-column" => Break::Avoid,
        "column" => Break::Column,
        "page" | "left" | "right" | "recto" | "verso" => Break::Page,
        "always" | "all" => Break::Always,
        _ => return Err("invalid break value"),
    })
}

pub(crate) fn display(value: &str) -> Option<(DisplayKind, bool)> {
    Some(match value {
        "inline" | "inline flow" => (DisplayKind::Inline, true),
        "table" | "block table" => (DisplayKind::Table, false),
        "inline-table" | "inline table" => (DisplayKind::Table, true),
        "table-row" => (DisplayKind::Row, false),
        "table-cell" => (DisplayKind::Cell, false),
        "table-row-group" => (DisplayKind::RowGroup, false),
        "table-header-group" => (DisplayKind::Header, false),
        "table-footer-group" => (DisplayKind::Footer, false),
        "table-column" => (DisplayKind::Column, false),
        "table-column-group" => (DisplayKind::ColumnGroup, false),
        "table-caption" => (DisplayKind::Caption, false),
        _ => return None,
    })
}

impl Writing {
    pub fn rect(self, rect: egui::Rect, width: f32) -> egui::Rect {
        if !self.vertical() {
            return rect;
        }
        let x = if self.reverse() {
            width - rect.max.y
        } else {
            rect.min.y
        };
        egui::Rect::from_min_size(
            egui::pos2(x, rect.min.x),
            egui::vec2(rect.height(), rect.width()),
        )
    }
    pub fn inverse_rect(self, rect: egui::Rect, width: f32) -> egui::Rect {
        if !self.vertical() {
            return rect;
        }
        let y = if self.reverse() {
            width - rect.max.x
        } else {
            rect.min.x
        };
        egui::Rect::from_min_size(
            egui::pos2(rect.min.y, y),
            egui::vec2(rect.height(), rect.width()),
        )
    }
    pub fn vector(self, value: egui::Vec2) -> egui::Vec2 {
        if !self.vertical() {
            value
        } else {
            egui::vec2(if self.reverse() { -value.y } else { value.y }, value.x)
        }
    }
}
