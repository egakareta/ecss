use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Outline {
    pub(crate) color: Option<Color32>,
    pub(crate) style: BorderStyle,
    pub(crate) width: f32,
    pub(crate) offset: f32,
}

impl Default for Outline {
    fn default() -> Self {
        Self {
            color: None,
            style: BorderStyle::None,
            width: 3.0,
            offset: 0.0,
        }
    }
}

impl Outline {
    pub(crate) fn inherited(mut self, current_color: Color32) -> Self {
        self.color = Some(self.color.unwrap_or(current_color));
        self
    }
}

pub(super) fn supported(name: &str) -> bool {
    matches!(
        name,
        "outline" | "outline-color" | "outline-style" | "outline-width" | "outline-offset"
    )
}

fn style(value: &str) -> Result<BorderStyle, &'static str> {
    if value.eq_ignore_ascii_case("auto") {
        return Ok(BorderStyle::Solid);
    }
    let style = border_style(value).map_err(|_| "invalid outline style")?;
    if style == BorderStyle::Hidden {
        return Err("invalid outline style");
    }
    Ok(style)
}

pub(super) fn apply(
    name: &str,
    value: &str,
    out: &mut Outline,
    units: Units,
) -> Result<(), &'static str> {
    let values = components(value)?;
    if name != "outline" && values.len() != 1 {
        return Err("expected one outline value");
    }
    match name {
        "outline-color" => out.color = border_color(value)?,
        "outline-style" => out.style = style(value)?,
        "outline-width" => out.width = border_width(value, units)?,
        "outline-offset" => {
            out.offset = if let Some(handle) = crate::math::length(value, units, true, false)? {
                crate::math::resolve(handle, 0.0)
            } else {
                let (offset, percentage) = length_value(value, units, true)?;
                if percentage {
                    return Err("percentage outline offset");
                }
                offset
            };
        }
        "outline" => {
            if values.is_empty() {
                return Err("empty outline");
            }
            let mut color = None;
            let mut line_style = None;
            let mut width = None;
            for value in &values {
                if let Ok(parsed) = style(value) {
                    if line_style.replace(parsed).is_some() {
                        return Err("duplicate outline style");
                    }
                } else if let Ok(parsed) = border_width(value, units) {
                    if width.replace(parsed).is_some() {
                        return Err("duplicate outline width");
                    }
                } else if color.replace(border_color(value)?).is_some() {
                    return Err("duplicate outline color");
                }
            }
            out.color = color.flatten();
            out.style = line_style.unwrap_or_default();
            out.width = width.unwrap_or(3.0);
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(super) fn copy_property(name: &str, out: &mut Outline, source: Outline) {
    match name {
        "outline" => {
            out.color = source.color;
            out.style = source.style;
            out.width = source.width;
        }
        "outline-color" => out.color = source.color,
        "outline-style" => out.style = source.style,
        "outline-width" => out.width = source.width,
        "outline-offset" => out.offset = source.offset,
        _ => unreachable!(),
    }
}
