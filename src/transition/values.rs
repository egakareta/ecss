use egui::Color32;

use crate::{
    box_decoration::{CornerRadius, LengthPercentage},
    layout_style::BoxStyle,
    math::{Arena, ComputedLength},
};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Value {
    Color(Color32),
    Number(f32),
    Length(ComputedLength),
    Radius(ComputedLength, ComputedLength),
    Position(crate::object::Position),
}

impl Value {
    pub(super) fn interpolate(&self, to: &Self, progress: f32) -> Option<Self> {
        Some(match (self, to) {
            (Self::Color(from), Self::Color(to)) => {
                let channel = |a: u8, b: u8| {
                    (f32::from(a) + (f32::from(b) - f32::from(a)) * progress)
                        .round()
                        .clamp(0.0, 255.0) as u8
                };
                let alpha = channel(from.a(), to.a());
                Self::Color(Color32::from_rgba_premultiplied(
                    channel(from.r(), to.r()).min(alpha),
                    channel(from.g(), to.g()).min(alpha),
                    channel(from.b(), to.b()).min(alpha),
                    alpha,
                ))
            }
            (Self::Number(from), Self::Number(to)) => Self::Number(from + (to - from) * progress),
            (Self::Length(from), Self::Length(to)) => Self::Length(from.interpolate(to, progress)),
            (Self::Radius(x, y), Self::Radius(to_x, to_y)) => {
                Self::Radius(x.interpolate(to_x, progress), y.interpolate(to_y, progress))
            }
            (Self::Position(from), Self::Position(to)) => {
                Self::Position(from.interpolate(to, progress))
            }
            _ => return None,
        })
    }
}

macro_rules! lengths {
    ($($variant:ident => $name:literal, $($field:ident).+, $nonnegative:literal;)+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(super) enum Length { $($variant,)+ }

        impl Length {
            const ALL: &[Self] = &[$(Self::$variant,)+];

            fn name(self) -> &'static str { match self { $(Self::$variant => $name,)+ } }

            fn nonnegative(self) -> bool { match self { $(Self::$variant => $nonnegative,)+ } }

            fn get(self, style: &BoxStyle) -> Option<Value> {
                ComputedLength::from_raw(match self { $(Self::$variant => style.$($field).+.into_raw(),)+ }).map(Value::Length)
            }

            fn set(self, style: &mut BoxStyle, value: &ComputedLength, arena: &Arena) {
                match self { $(Self::$variant => {
                    let value = if $nonnegative { value.clone().nonnegative() } else { value.clone() };
                    style.$($field).+ = value.encode(arena).into();
                },)+ }
            }
        }
    }
}

lengths! {
    Width => "width", layout.size.width, true;
    Height => "height", layout.size.height, true;
    MinWidth => "min-width", layout.min_size.width, true;
    MinHeight => "min-height", layout.min_size.height, true;
    MaxWidth => "max-width", layout.max_size.width, true;
    MaxHeight => "max-height", layout.max_size.height, true;
    FlexBasis => "flex-basis", layout.flex_basis, true;
    MarginTop => "margin-top", layout.margin.top, false;
    MarginRight => "margin-right", layout.margin.right, false;
    MarginBottom => "margin-bottom", layout.margin.bottom, false;
    MarginLeft => "margin-left", layout.margin.left, false;
    PaddingTop => "padding-top", layout.padding.top, true;
    PaddingRight => "padding-right", layout.padding.right, true;
    PaddingBottom => "padding-bottom", layout.padding.bottom, true;
    PaddingLeft => "padding-left", layout.padding.left, true;
    Top => "top", layout.inset.top, false;
    Right => "right", layout.inset.right, false;
    Bottom => "bottom", layout.inset.bottom, false;
    Left => "left", layout.inset.left, false;
    ColumnGap => "column-gap", layout.gap.width, true;
    RowGap => "row-gap", layout.gap.height, true;
}

macro_rules! numbers {
    ($($variant:ident => $name:literal, $($field:ident).+, $nonnegative:literal;)+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(super) enum Number { $($variant,)+ }

        impl Number {
            const ALL: &[Self] = &[$(Self::$variant,)+];

            fn name(self) -> &'static str { match self { $(Self::$variant => $name,)+ } }

            fn constrain(self, value: f32) -> f32 {
                if self == Self::Opacity {
                    value.clamp(0.0, 1.0)
                } else {
                    match self { $(Self::$variant => if $nonnegative { value.max(0.0) } else { value },)+ }
                }
            }

            fn get(self, style: &BoxStyle) -> Value { Value::Number(match self { $(Self::$variant => style.$($field).+,)+ }) }

            fn set(self, style: &mut BoxStyle, value: f32) {
                match self { $(Self::$variant => style.$($field).+ = if self == Self::Opacity { value.clamp(0.0, 1.0) } else if $nonnegative { value.max(0.0) } else { value },)+ }
                if self == Self::FontSize { style.format.font_size_set = true; }
            }
        }
    }
}

numbers! {
    FontSize => "font-size", format.em, true;
    LetterSpacing => "letter-spacing", format.letter_spacing, false;
    WordSpacing => "word-spacing", format.word_spacing, false;
    FlexGrow => "flex-grow", layout.flex_grow, true;
    FlexShrink => "flex-shrink", layout.flex_shrink, true;
    OutlineWidth => "outline-width", outline.width, true;
    OutlineOffset => "outline-offset", outline.offset, false;
    ColumnRuleWidth => "column-rule-width", format.column_rule.width, true;
    BorderSpacingX => "border-spacing", format.spacing.width, true;
    BorderSpacingY => "border-spacing", format.spacing.height, true;
    Opacity => "opacity", opacity, true;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PropertyId {
    Color,
    Background,
    BorderColor(usize),
    OutlineColor,
    ColumnRuleColor,
    Radius(usize),
    BorderWidth(usize),
    Length(Length),
    Number(Number),
    LineHeight,
    ColumnWidth,
    ColumnCount,
    TabSize,
    Order,
    ZIndex,
    ObjectPosition,
}

impl PropertyId {
    pub(super) fn all() -> impl Iterator<Item = Self> {
        [
            Self::Color,
            Self::Background,
            Self::OutlineColor,
            Self::ColumnRuleColor,
            Self::LineHeight,
            Self::ColumnWidth,
            Self::ColumnCount,
            Self::TabSize,
            Self::Order,
            Self::ZIndex,
            Self::ObjectPosition,
        ]
        .into_iter()
        .chain((0..4).flat_map(|side| {
            [
                Self::BorderColor(side),
                Self::BorderWidth(side),
                Self::Radius(side),
            ]
        }))
        .chain(Length::ALL.iter().copied().map(Self::Length))
        .chain(Number::ALL.iter().copied().map(Self::Number))
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Background => "background-color",
            Self::BorderColor(side) => [
                "border-top-color",
                "border-right-color",
                "border-bottom-color",
                "border-left-color",
            ][side],
            Self::BorderWidth(side) => [
                "border-top-width",
                "border-right-width",
                "border-bottom-width",
                "border-left-width",
            ][side],
            Self::OutlineColor => "outline-color",
            Self::ColumnRuleColor => "column-rule-color",
            Self::Radius(corner) => [
                "border-top-left-radius",
                "border-top-right-radius",
                "border-bottom-right-radius",
                "border-bottom-left-radius",
            ][corner],
            Self::Length(property) => property.name(),
            Self::Number(property) => property.name(),
            Self::LineHeight => "line-height",
            Self::ColumnWidth => "column-width",
            Self::ColumnCount => "column-count",
            Self::TabSize => "tab-size",
            Self::Order => "order",
            Self::ZIndex => "z-index",
            Self::ObjectPosition => "object-position",
        }
    }

    pub(super) fn matches(self, name: &str, style: &BoxStyle) -> bool {
        if self.name() == name {
            return true;
        }
        match (self, name) {
            (Self::Background, "background")
            | (Self::Radius(_), "border-radius")
            | (Self::BorderColor(_) | Self::BorderWidth(_), "border")
            | (Self::BorderColor(_), "border-color")
            | (Self::BorderWidth(_), "border-width")
            | (Self::OutlineColor | Self::Number(Number::OutlineWidth), "outline")
            | (Self::ColumnRuleColor | Self::Number(Number::ColumnRuleWidth), "column-rule")
            | (Self::ColumnWidth | Self::ColumnCount, "columns")
            | (
                Self::Length(Length::FlexBasis)
                | Self::Number(Number::FlexGrow | Number::FlexShrink),
                "flex",
            ) => return true,
            _ => {}
        }
        let own = self.name();
        if matches!(name, "margin" | "padding") {
            return own.starts_with(&format!("{name}-"));
        }
        if name == "gap" {
            return matches!(self, Self::Length(Length::ColumnGap | Length::RowGap));
        }
        if name == "inset" {
            return matches!(
                self,
                Self::Length(Length::Top | Length::Right | Length::Bottom | Length::Left)
            );
        }
        if matches!(self, Self::BorderColor(_) | Self::BorderWidth(_))
            && let Some(side) = own
                .strip_prefix("border-")
                .and_then(|name| name.split_once('-'))
                .and_then(|(side, _)| {
                    ["top", "right", "bottom", "left"]
                        .iter()
                        .position(|value| *value == side)
                })
            && name == ["border-top", "border-right", "border-bottom", "border-left"][side]
        {
            return true;
        }
        for (logical, physical) in [
            ("inline-size", "width"),
            ("block-size", "height"),
            ("min-inline-size", "min-width"),
            ("min-block-size", "min-height"),
            ("max-inline-size", "max-width"),
            ("max-block-size", "max-height"),
        ] {
            if name == logical && own == physical {
                return true;
            }
        }
        for group in ["margin", "padding", "inset", "border"] {
            let Some(suffix) = name
                .strip_prefix(group)
                .and_then(|suffix| suffix.strip_prefix('-'))
            else {
                continue;
            };
            for logical in ["block-start", "block-end", "inline-start", "inline-end"] {
                let axis = logical.split_once('-').unwrap().0;
                let Some(rest) = suffix
                    .strip_prefix(logical)
                    .or_else(|| suffix.strip_prefix(axis))
                else {
                    continue;
                };
                if !matches!(rest, "" | "-width" | "-color") {
                    continue;
                }
                let side = crate::layout_style::logical_side(
                    logical,
                    style.layout.direction,
                    style.format.writing,
                    &style.format.text_orientation,
                )
                .unwrap();
                let physical = ["top", "right", "bottom", "left"][side];
                if group == "inset" && own == physical
                    || group != "inset"
                        && (own == format!("{group}-{physical}{rest}")
                            || group == "border"
                                && rest.is_empty()
                                && (own == format!("border-{physical}-width")
                                    || own == format!("border-{physical}-color")))
                {
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn affects_layout(self) -> bool {
        !matches!(
            self,
            Self::Color
                | Self::Background
                | Self::BorderColor(_)
                | Self::OutlineColor
                | Self::ColumnRuleColor
                | Self::Radius(_)
                | Self::ObjectPosition
                | Self::Number(
                    Number::OutlineWidth
                        | Number::OutlineOffset
                        | Number::ColumnRuleWidth
                        | Number::Opacity
                )
        )
    }

    pub(super) fn constrain(self, value: Value) -> Value {
        match (self, value) {
            (Self::Length(property), Value::Length(value)) if property.nonnegative() => {
                Value::Length(value.nonnegative())
            }
            (Self::Number(property), Value::Number(value)) => {
                Value::Number(property.constrain(value))
            }
            (Self::Radius(_), Value::Radius(x, y)) => {
                Value::Radius(x.nonnegative(), y.nonnegative())
            }
            (
                Self::BorderWidth(_) | Self::LineHeight | Self::ColumnWidth | Self::TabSize,
                Value::Number(value),
            ) => Value::Number(value.max(0.0)),
            (Self::TabSize, Value::Length(value)) => Value::Length(value.nonnegative()),
            (Self::ColumnCount, Value::Number(value)) => {
                Value::Number((value + 0.5).floor().max(1.0))
            }
            (Self::Order | Self::ZIndex, Value::Number(value)) => {
                Value::Number((value + 0.5).floor())
            }
            (_, value) => value,
        }
    }

    pub(super) fn get(self, style: &BoxStyle) -> Option<Value> {
        Some(match self {
            Self::Color => Value::Color(style.current_color),
            Self::ObjectPosition => Value::Position(style.object_position.clone()),
            Self::Background => Value::Color(style.background_color),
            Self::BorderColor(side) => {
                if style.border_stripes[side].is_some() {
                    return None;
                }
                Value::Color(style.border_colors[side].unwrap_or(style.current_color))
            }
            Self::OutlineColor => Value::Color(style.outline.color.unwrap_or(style.current_color)),
            Self::ColumnRuleColor => Value::Color(
                style
                    .format
                    .column_rule
                    .color
                    .unwrap_or(style.current_color),
            ),
            Self::Radius(corner) => {
                let radius = style.decorations.radii[corner];
                Value::Radius(decoration_length(radius.x), decoration_length(radius.y))
            }
            Self::BorderWidth(side) => Value::Number(style.border_widths[side]),
            Self::Length(property) => return property.get(style),
            Self::Number(property) => property.get(style),
            Self::LineHeight => Value::Number(style.format.line_height?),
            Self::ColumnWidth => Value::Number(style.format.column_width?),
            Self::ColumnCount => Value::Number(style.format.column_count? as f32),
            Self::TabSize if style.format.tab_size_length => {
                Value::Length(ComputedLength::length(style.format.tab_size))
            }
            Self::TabSize => Value::Number(style.format.tab_size),
            Self::Order => Value::Number(style.order as f32),
            Self::ZIndex => Value::Number(style.z_index? as f32),
        })
    }

    pub(super) fn set(self, style: &mut BoxStyle, value: &Value, arena: &Arena) {
        match (self, value) {
            (Self::Color, Value::Color(value)) => style.current_color = *value,
            (Self::Background, Value::Color(value)) => style.background_color = *value,
            (Self::BorderColor(side), Value::Color(value)) => {
                style.border_colors[side] = Some(*value)
            }
            (Self::ObjectPosition, Value::Position(value)) => style.object_position = value.clone(),
            (Self::OutlineColor, Value::Color(value)) => style.outline.color = Some(*value),
            (Self::ColumnRuleColor, Value::Color(value)) => {
                style.format.column_rule.color = Some(*value)
            }
            (Self::Radius(corner), Value::Radius(x, y)) => {
                style.decorations.radii[corner] = CornerRadius {
                    x: x.clone().nonnegative().decoration(arena),
                    y: y.clone().nonnegative().decoration(arena),
                }
            }
            (Self::BorderWidth(side), Value::Number(value)) => {
                style.border_widths[side] = value.max(0.0)
            }
            (Self::Length(property), Value::Length(value)) => property.set(style, value, arena),
            (Self::Number(property), Value::Number(value)) => property.set(style, *value),
            (Self::LineHeight, Value::Number(value)) => {
                style.format.line_height = Some(value.max(0.0))
            }
            (Self::ColumnWidth, Value::Number(value)) => {
                style.format.column_width = Some(value.max(0.0))
            }
            (Self::ColumnCount, Value::Number(value)) => {
                style.format.column_count = Some((value + 0.5).floor().max(1.0) as usize)
            }
            (Self::TabSize, Value::Number(value)) => {
                style.format.tab_size = value.max(0.0);
                style.format.tab_size_length = false;
            }
            (Self::TabSize, Value::Length(value)) => {
                style.format.tab_size = value.resolve(0.0).max(0.0);
                style.format.tab_size_length = true;
            }
            (Self::Order, Value::Number(value)) => style.order = (value + 0.5).floor() as i32,
            (Self::ZIndex, Value::Number(value)) => {
                style.z_index = Some((value + 0.5).floor() as i32)
            }
            _ => {}
        }
    }
}

fn decoration_length(value: LengthPercentage) -> ComputedLength {
    match value {
        LengthPercentage::Length(value) => ComputedLength::length(value),
        LengthPercentage::Percentage(value) => ComputedLength::percentage(value),
        LengthPercentage::Calc(handle) => ComputedLength::from_handle(handle),
    }
}
