use egui::{Rect, Vec2, vec2};

use crate::{layout_style::Units, math::ComputedLength};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Fit {
    #[default]
    Fill,
    Contain,
    Cover,
    None,
    ScaleDown,
    CoverScaleDown,
}

impl Fit {
    pub(crate) fn parse(value: &str) -> Result<Self, &'static str> {
        let parts = crate::layout_style::components(&value.to_ascii_lowercase())?;
        let parts = parts.iter().map(String::as_str).collect::<Vec<_>>();
        match parts.as_slice() {
            ["fill"] => Ok(Self::Fill),
            ["contain"] => Ok(Self::Contain),
            ["cover"] => Ok(Self::Cover),
            ["none"] => Ok(Self::None),
            ["scale-down"] | ["contain", "scale-down"] | ["scale-down", "contain"] => {
                Ok(Self::ScaleDown)
            }
            ["cover", "scale-down"] | ["scale-down", "cover"] => Ok(Self::CoverScaleDown),
            _ => Err("invalid object-fit"),
        }
    }

    fn size(self, natural: Vec2, area: Vec2) -> Vec2 {
        let ratios = area / natural;
        match self {
            Self::Fill => area,
            Self::Contain => natural * ratios.min_elem(),
            Self::Cover => natural * ratios.max_elem(),
            Self::None => natural,
            Self::ScaleDown => natural * ratios.min_elem().min(1.0),
            Self::CoverScaleDown => natural * ratios.max_elem().min(1.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Position([ComputedLength; 2]);

impl Default for Position {
    fn default() -> Self {
        Self([
            ComputedLength::percentage(0.5),
            ComputedLength::percentage(0.5),
        ])
    }
}

impl Position {
    pub(crate) fn parse(parts: &[String], units: Units<'_>) -> Result<Self, &'static str> {
        let [x, y] = position_values(parts)?;
        let length = |value: &str| {
            let value = crate::box_decoration::length_percentage(value, units, true)?;
            Ok::<_, &'static str>(match value {
                crate::box_decoration::LengthPercentage::Length(value) => {
                    ComputedLength::length(value)
                }
                crate::box_decoration::LengthPercentage::Percentage(value) => {
                    ComputedLength::percentage(value)
                }
                crate::box_decoration::LengthPercentage::Calc(handle) => {
                    ComputedLength::from_handle(handle)
                }
            })
        };
        Ok(Self([length(&x)?, length(&y)?]))
    }

    pub(crate) fn interpolate(&self, to: &Self, progress: f32) -> Self {
        Self(std::array::from_fn(|axis| {
            self.0[axis].interpolate(&to.0[axis], progress)
        }))
    }

    pub(crate) fn rect(&self, fit: Fit, natural: Vec2, area: Rect) -> Rect {
        let size = fit.size(natural, area.size());
        let free = area.size() - size;
        let offset = vec2(self.0[0].resolve(free.x), self.0[1].resolve(free.y));
        Rect::from_min_size(area.min + offset, size)
    }
}

pub(crate) fn position_values(parts: &[String]) -> Result<[String; 2], &'static str> {
    let keyword = |value: &str| match value.to_ascii_lowercase().as_str() {
        "left" => Some((0, "0%")),
        "right" => Some((0, "100%")),
        "top" => Some((1, "0%")),
        "bottom" => Some((1, "100%")),
        "center" => Some((2, "50%")),
        _ => None,
    };
    let component = |value: &str, axis: usize| {
        if let Some((direction, percent)) = keyword(value) {
            if direction != axis && direction != 2 {
                return Err("invalid object-position axis");
            }
            Ok(percent.to_owned())
        } else {
            Ok(value.to_owned())
        }
    };
    match parts {
        [value] => {
            let mut position = ["50%".to_owned(), "50%".to_owned()];
            let axis = usize::from(keyword(value).is_some_and(|(axis, _)| axis == 1));
            position[axis] = component(value, axis)?;
            Ok(position)
        }
        [x, y] => {
            if let (Some((a, _)), Some((b, _))) = (keyword(x), keyword(y))
                && (a == 1 || b == 0)
            {
                return Ok([component(y, 0)?, component(x, 1)?]);
            }
            Ok([component(x, 0)?, component(y, 1)?])
        }
        [a, offset_a, b, offset_b] => {
            let edge = |value: &str, offset: &str| {
                let (axis, percent) = keyword(value).ok_or("invalid object-position edge")?;
                if axis == 2 {
                    return Err("invalid object-position edge");
                }
                if keyword(offset).is_some() {
                    return Err("invalid object-position offset");
                }
                Ok((
                    axis,
                    if percent == "0%" {
                        offset.to_owned()
                    } else {
                        format!("calc(100% - ({offset}))")
                    },
                ))
            };
            let (a, offset_a) = edge(a, offset_a)?;
            let (b, offset_b) = edge(b, offset_b)?;
            if a == b {
                return Err("duplicate object-position axis");
            }
            Ok(if a == 0 {
                [offset_a, offset_b]
            } else {
                [offset_b, offset_a]
            })
        }
        _ => Err("object-position requires one, two, or four values"),
    }
}
