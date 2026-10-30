use crate::layout_style::{BoxStyle, Units};
use cssparser::{Parser, Token};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CollapsedStripeThickness {
    Length(f32),
    Flex(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CollapsedBorderStripe {
    pub color: Color32,
    pub thickness: CollapsedStripeThickness,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CollapsedBorder {
    pub width: f32,
    pub color: Color32,
    pub style: crate::layout_style::BorderStyle,
    pub stripes: Option<Arc<[CollapsedBorderStripe]>>,
    pub source_side: usize,
    pub draw: bool,
    pub priority: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CollapsedBorderSegment {
    pub rect: egui::Rect,
    pub edges: [CollapsedBorder; 4],
}

impl Default for CollapsedBorder {
    fn default() -> Self {
        Self {
            width: 0.0,
            color: Color32::BLACK,
            style: Default::default(),
            stripes: None,
            source_side: 0,
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
    AvoidRegion,
    Column,
    Page,
    Region,
    Always,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RegionFragment {
    #[default]
    Auto,
    Break,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TextOverflow {
    Clip,
    Ellipsis,
    String(String),
}
impl TextOverflow {
    pub(crate) fn marker(&self) -> Option<&str> {
        match self {
            Self::Clip => None,
            Self::Ellipsis => Some("…"),
            Self::String(value) => Some(value),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Formatting {
    pub em: f32,
    pub font_size_set: bool,
    pub font_family: Option<Vec<String>>,
    pub font_weight: u16,
    pub font_italic: bool,
    pub font_kerning: FontKerning,
    pub writing: Writing,
    pub display: DisplayKind,
    pub inline_level: bool,
    pub subgrid: [bool; 2],
    pub subgrid_names: [Vec<Vec<String>>; 2],
    pub flow: Option<String>,
    pub flow_from: Option<String>,
    pub flow_into: Option<String>,
    pub flow_into_content: bool,
    pub region_fragment: RegionFragment,
    pub chains: Vec<Vec<String>>,
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
    pub text_align_last: String,
    pub text_justify: String,
    pub text_overflow: [TextOverflow; 2],
    pub white_space: String,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub tab_size: f32,
    pub tab_size_length: bool,
    pub text_indent: (f32, bool),
    pub text_indent_each_line: bool,
    pub text_indent_hanging: bool,
    pub text_transform: String,
    pub line_break: String,
    pub overflow_wrap: String,
    pub word_break: String,
    pub hyphens: String,
    pub hanging_punctuation: String,
    pub text_orientation: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FontKerning {
    #[default]
    Auto,
    Normal,
    None,
}

impl Default for Formatting {
    fn default() -> Self {
        Self {
            em: 16.0,
            font_size_set: false,
            font_family: None,
            font_weight: 400,
            font_italic: false,
            font_kerning: FontKerning::Auto,
            writing: Writing::Horizontal,
            display: DisplayKind::Normal,
            inline_level: false,
            subgrid: [false; 2],
            subgrid_names: Default::default(),
            flow: None,
            flow_from: None,
            flow_into: None,
            flow_into_content: false,
            region_fragment: RegionFragment::Auto,
            chains: Vec::new(),
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
            text_align_last: "auto".into(),
            text_justify: "auto".into(),
            text_overflow: [TextOverflow::Clip, TextOverflow::Clip],
            white_space: "normal".into(),
            letter_spacing: 0.0,
            word_spacing: 0.0,
            tab_size: 8.0,
            tab_size_length: false,
            text_indent: (0.0, false),
            text_indent_each_line: false,
            text_indent_hanging: false,
            text_transform: "none".into(),
            line_break: "auto".into(),
            overflow_wrap: "normal".into(),
            word_break: "normal".into(),
            hyphens: "manual".into(),
            hanging_punctuation: "none".into(),
            text_orientation: "mixed".into(),
        }
    }
}

pub(crate) fn supported(name: &str) -> bool {
    matches!(
        name,
        "font-size"
            | "font-kerning"
            | "writing-mode"
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
            | "flow-from"
            | "flow-into"
            | "page-break-before"
            | "page-break-after"
            | "page-break-inside"
            | "region-fragment"
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
        "font-kerning" => {
            f.font_kerning = match value.to_ascii_lowercase().as_str() {
                "auto" => FontKerning::Auto,
                "normal" => FontKerning::Normal,
                "none" => FontKerning::None,
                _ => return Err("invalid font-kerning"),
            }
        }
        "font-size" => {
            let size = match value.to_ascii_lowercase().as_str() {
                "xx-small" => 9.0,
                "x-small" => 10.0,
                "small" => 13.0,
                "medium" => 16.0,
                "large" => 18.0,
                "x-large" => 24.0,
                "xx-large" => 32.0,
                "xxx-large" => 48.0,
                "smaller" => units.font / 1.2,
                "larger" => units.font * 1.2,
                _ => {
                    if let Some(handle) = crate::math::length(value, units, true, true)? {
                        crate::math::resolve(handle, units.font)
                    } else {
                        let (size, percentage) =
                            crate::layout_style::length_value(value, units, true)?;
                        if percentage { size * units.font } else { size }
                    }
                }
            };
            if !size.is_finite() || size < 0.0 {
                return Err("invalid font-size");
            }
            f.em = size;
            f.font_size_set = true;
        }
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
        "flow-from" => f.flow_from = parse_flow_name(value)?,
        "flow-into" => {
            let (name, content) = parse_flow_into(value)?;
            f.flow_into = name;
            f.flow_into_content = content;
        }
        "region-fragment" => {
            f.region_fragment = match value.to_ascii_lowercase().as_str() {
                "auto" => RegionFragment::Auto,
                "break" => RegionFragment::Break,
                _ => return Err("invalid region-fragment"),
            }
        }
        "break-inside" | "page-break-inside" => {
            f.inside = parse_break(value)?;
            if !matches!(f.inside, Break::Auto | Break::Avoid | Break::AvoidRegion) {
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
            f.line_height = if value.eq_ignore_ascii_case("normal") {
                None
            } else if let Ok(Some(n)) = crate::math::number(value, units) {
                if !n.is_finite() || n < 0.0 {
                    return Err("invalid line-height");
                }
                Some(n * units.font)
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
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "start"
                    | "end"
                    | "left"
                    | "right"
                    | "center"
                    | "justify"
                    | "justify-all"
                    | "match-parent"
            ) {
                return Err("invalid text-align");
            }
            match value.as_str() {
                "justify-all" => {
                    f.text_align = "justify".into();
                    f.text_align_last = "justify".into();
                }
                "match-parent" => {
                    f.text_align = value.clone();
                    f.text_align_last = value;
                }
                _ => {
                    f.text_align = value;
                    f.text_align_last = "auto".into();
                }
            }
        }
        "text-align-all" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "start" | "end" | "left" | "right" | "center" | "justify" | "match-parent"
            ) {
                return Err("invalid text-align-all");
            }
            f.text_align = value;
        }
        "text-align-last" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "auto" | "start" | "end" | "left" | "right" | "center" | "justify" | "match-parent"
            ) {
                return Err("invalid text-align-last");
            }
            f.text_align_last = value;
        }
        "text-justify" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "auto" | "none" | "inter-word" | "inter-character"
            ) {
                return Err("invalid text-justify");
            }
            f.text_justify = value;
        }
        "letter-spacing" => {
            f.letter_spacing = if value.eq_ignore_ascii_case("normal") {
                0.0
            } else if let Some(handle) = crate::math::length(value, units, true, false)? {
                crate::math::resolve(handle, 0.0)
            } else {
                length(value, units)?
            };
        }
        "word-spacing" => {
            f.word_spacing = if value.eq_ignore_ascii_case("normal") {
                0.0
            } else if let Some(handle) = crate::math::length(value, units, true, false)? {
                crate::math::resolve(handle, 0.0)
            } else {
                let (spacing, percentage) = crate::layout_style::length_value(value, units, true)?;
                if percentage {
                    spacing * units.font
                } else {
                    spacing
                }
            };
        }
        "tab-size" => {
            if let Ok(size) = value.parse::<u32>() {
                if size > 65535 {
                    return Err("invalid tab-size");
                }
                f.tab_size = size as f32;
                f.tab_size_length = false;
            } else {
                f.tab_size = length(value, units)?;
                if f.tab_size < 0.0 {
                    return Err("invalid tab-size");
                }
                f.tab_size_length = true;
            }
        }
        "text-indent" => {
            let parts = crate::layout_style::components(value)?;
            if parts.is_empty() || parts.len() > 3 {
                return Err("invalid text-indent");
            }
            let mut length = None;
            let mut each_line = false;
            let mut hanging = false;
            for part in &parts {
                match part.to_ascii_lowercase().as_str() {
                    "each-line" if !each_line => each_line = true,
                    "hanging" if !hanging => hanging = true,
                    "each-line" | "hanging" => return Err("duplicate text-indent modifier"),
                    _ if length.is_none() => {
                        length = Some(crate::layout_style::length_value(part, units, true)?);
                    }
                    _ => return Err("invalid text-indent"),
                }
            }
            let Some(length) = length else {
                return Err("text-indent requires a length");
            };
            f.text_indent = length;
            f.text_indent_each_line = each_line;
            f.text_indent_hanging = hanging;
        }
        "text-transform" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "none" | "capitalize" | "uppercase" | "lowercase" | "full-width" | "full-size-kana"
            ) {
                return Err("invalid text-transform");
            }
            f.text_transform = value;
        }
        "line-break" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "auto" | "loose" | "normal" | "strict" | "anywhere"
            ) {
                return Err("invalid line-break");
            }
            f.line_break = value;
        }
        "overflow-wrap" => {
            let value = value.to_ascii_lowercase();
            if !matches!(value.as_str(), "normal" | "break-word" | "anywhere") {
                return Err("invalid overflow-wrap");
            }
            f.overflow_wrap = value;
        }
        "word-break" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "normal" | "break-all" | "keep-all" | "break-word"
            ) {
                return Err("invalid word-break");
            }
            f.word_break = value;
        }
        "hyphens" => {
            let value = value.to_ascii_lowercase();
            if !matches!(value.as_str(), "none" | "manual" | "auto") {
                return Err("invalid hyphens");
            }
            f.hyphens = value;
        }
        "hanging-punctuation" => {
            let value = value.to_ascii_lowercase();
            let values = value.split_whitespace().collect::<Vec<_>>();
            if values.is_empty()
                || values.len() > 3
                || values.iter().any(|value| {
                    !matches!(
                        *value,
                        "none" | "first" | "last" | "allow-end" | "force-end"
                    )
                })
                || values.len() > 1 && values.contains(&"none")
            {
                return Err("invalid hanging-punctuation");
            }
            f.hanging_punctuation = value;
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
                    Token::Ident(value) if matches_ignore_ascii_case(value, "clip") => {
                        TextOverflow::Clip
                    }
                    Token::Ident(value) if matches_ignore_ascii_case(value, "ellipsis") => {
                        TextOverflow::Ellipsis
                    }
                    Token::QuotedString(value) => TextOverflow::String(value.to_string()),
                    _ => return Err("unsupported text-overflow value"),
                };
                values.push(value);
                if values.len() > 2 {
                    return Err("too many text-overflow values");
                }
            }
            if values.is_empty() {
                return Err("invalid text-overflow");
            }
            f.text_overflow = if values.len() == 1 {
                [TextOverflow::Clip, values.remove(0)]
            } else {
                [values.remove(0), values.remove(0)]
            };
        }
        "white-space" => {
            let value = value.to_ascii_lowercase();
            if !matches!(
                value.as_str(),
                "normal" | "nowrap" | "pre" | "pre-wrap" | "pre-line" | "break-spaces"
            ) {
                return Err("invalid white-space");
            }
            f.white_space = value;
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

pub(crate) fn text_alignment_offset(
    value: &str,
    free: f32,
    direction: taffy::Direction,
    writing: Writing,
) -> f32 {
    let rtl = direction == taffy::Direction::Rtl;
    match value {
        "center" => {
            if rtl {
                -free * 0.5
            } else {
                free * 0.5
            }
        }
        "left" | "right" => {
            let line_left_is_low = writing != Writing::SidewaysLr;
            let line_left = value == "left";
            let low = line_left == line_left_is_low;
            if rtl {
                if low { -free } else { 0.0 }
            } else if low {
                0.0
            } else {
                free
            }
        }
        "end" => {
            if rtl {
                -free
            } else {
                free
            }
        }
        _ => 0.0,
    }
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
        "avoid-region" => Break::AvoidRegion,
        "column" => Break::Column,
        "page" | "left" | "right" | "recto" | "verso" => Break::Page,
        "region" => Break::Region,
        "always" | "all" => Break::Always,
        _ => return Err("invalid break value"),
    })
}

fn parse_flow_name(value: &str) -> Result<Option<String>, &'static str> {
    if value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let mut parser = Parser::new(value);
    let name = parser
        .expect_ident_cloned()
        .map_err(|_| "invalid flow name")?;
    if [
        "none", "inherit", "initial", "unset", "revert", "default", "auto",
    ]
    .iter()
    .any(|reserved| name.eq_ignore_ascii_case(reserved))
    {
        return Err("invalid flow name");
    }
    parser.expect_exhausted().map_err(|_| "invalid flow name")?;
    Ok(Some(name.to_string()))
}

fn parse_flow_into(value: &str) -> Result<(Option<String>, bool), &'static str> {
    if value.eq_ignore_ascii_case("none") {
        return Ok((None, false));
    }
    let mut parser = Parser::new(value);
    let name = parser
        .expect_ident_cloned()
        .map_err(|_| "invalid flow name")?;
    if [
        "none", "inherit", "initial", "unset", "revert", "default", "auto",
    ]
    .iter()
    .any(|reserved| name.eq_ignore_ascii_case(reserved))
    {
        return Err("invalid flow name");
    }
    let mode = parser.try_parse(|parser| parser.expect_ident_cloned()).ok();
    parser.expect_exhausted().map_err(|_| "invalid flow-into")?;
    let content = match mode.map(|mode| mode.to_string().to_ascii_lowercase()) {
        None => false,
        Some(mode) if mode == "element" => false,
        Some(mode) if mode == "content" => true,
        _ => return Err("invalid flow-into"),
    };
    Ok((Some(name.to_string()), content))
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
