use super::*;

pub(crate) fn parse_color<'i>(
    input: &mut CssParser<'i>,
) -> Result<Color32, ParseError<&'static str>> {
    let color = match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("transparent") => {
            Ok(Color32::TRANSPARENT)
        }
        Token::Hash(hex) | Token::IDHash(hex) => {
            parse_hex_color(hex).ok_or_else(|| ParseError::custom("unsupported color value"))
        }
        Token::Ident(value) => cssparser::color::parse_named_color(value)
            .map(|(r, g, b)| Color32::from_rgb(r, g, b))
            .map_err(|_| ParseError::custom("unsupported color value")),
        Token::Function(name)
            if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") =>
        {
            parse_functional_color(input, FunctionalColorKind::Rgb)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("hsl") || name.eq_ignore_ascii_case("hsla") =>
        {
            parse_functional_color(input, FunctionalColorKind::Hsl)
        }
        _ => Err(ParseError::custom("unsupported color value")),
    }?;
    input.expect_exhausted()?;
    Ok(color)
}

#[derive(Clone, Copy)]
enum FunctionalColorKind {
    Rgb,
    Hsl,
}

enum FunctionalColorToken {
    Number(f32),
    Percentage(f32),
    Dimension(f32, String),
    Comma,
    Slash,
}

fn parse_functional_color<'i>(
    input: &mut CssParser<'i>,
    kind: FunctionalColorKind,
) -> Result<Color32, ParseError<&'static str>> {
    let values = input.parse_nested_block(|args| {
        let mut values = Vec::new();
        while !args.is_exhausted() {
            let value = match args.next()? {
                Token::Number { value, .. } => FunctionalColorToken::Number(*value),
                Token::Percentage { unit_value, .. } => {
                    FunctionalColorToken::Percentage(*unit_value)
                }
                Token::Dimension { value, unit, .. } => {
                    FunctionalColorToken::Dimension(*value, unit.to_string())
                }
                Token::Comma => FunctionalColorToken::Comma,
                Token::Delim('/') => FunctionalColorToken::Slash,
                _ => return Err(ParseError::custom("unsupported color value")),
            };
            values.push(value);
        }
        Ok(values)
    })?;

    let (first, second, third, alpha) = functional_components(&values)
        .ok_or_else(|| ParseError::custom("unsupported color value"))?;
    let alpha = match alpha {
        Some(alpha) => {
            color_alpha(alpha).ok_or_else(|| ParseError::custom("unsupported color value"))?
        }
        None => 1.0,
    };

    let (red, green, blue) = match kind {
        FunctionalColorKind::Rgb => (
            rgb_component(first),
            rgb_component(second),
            rgb_component(third),
        ),
        FunctionalColorKind::Hsl => hsl_components(first, second, third),
    };
    let (Some(red), Some(green), Some(blue)) = (red, green, blue) else {
        return Err(ParseError::custom("unsupported color value"));
    };

    Ok(Color32::from_rgba_unmultiplied(
        color_byte(red),
        color_byte(green),
        color_byte(blue),
        color_byte(alpha),
    ))
}

fn functional_components(
    values: &[FunctionalColorToken],
) -> Option<(
    &FunctionalColorToken,
    &FunctionalColorToken,
    &FunctionalColorToken,
    Option<&FunctionalColorToken>,
)> {
    match values {
        [
            first,
            FunctionalColorToken::Comma,
            second,
            FunctionalColorToken::Comma,
            third,
        ] => Some((first, second, third, None)),
        [
            first,
            FunctionalColorToken::Comma,
            second,
            FunctionalColorToken::Comma,
            third,
            FunctionalColorToken::Comma,
            alpha,
        ] => Some((first, second, third, Some(alpha))),
        [first, second, third] => Some((first, second, third, None)),
        [first, second, third, FunctionalColorToken::Slash, alpha] => {
            Some((first, second, third, Some(alpha)))
        }
        _ => None,
    }
}

fn rgb_component(value: &FunctionalColorToken) -> Option<f32> {
    let channel = match value {
        FunctionalColorToken::Number(value) => value / 255.0,
        FunctionalColorToken::Percentage(value) => *value,
        _ => return None,
    };
    channel.is_finite().then(|| channel.clamp(0.0, 1.0))
}

fn color_alpha(value: &FunctionalColorToken) -> Option<f32> {
    let alpha = match value {
        FunctionalColorToken::Number(value) => *value,
        FunctionalColorToken::Percentage(value) => *value,
        _ => return None,
    };
    alpha.is_finite().then(|| alpha.clamp(0.0, 1.0))
}

fn hsl_components(
    hue: &FunctionalColorToken,
    saturation: &FunctionalColorToken,
    lightness: &FunctionalColorToken,
) -> (Option<f32>, Option<f32>, Option<f32>) {
    let hue = match hue {
        FunctionalColorToken::Number(value) => Some(*value),
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("deg") => {
            Some(*value)
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("grad") => {
            Some(*value * 0.9)
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("rad") => {
            Some(value.to_degrees())
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("turn") => {
            Some(*value * 360.0)
        }
        _ => None,
    };
    let saturation = hsl_percentage(saturation);
    let lightness = hsl_percentage(lightness);
    let (Some(hue), Some(saturation), Some(lightness)) = (hue, saturation, lightness) else {
        return (None, None, None);
    };
    if !hue.is_finite() {
        return (None, None, None);
    }

    let hue = hue.rem_euclid(360.0) / 60.0;
    let saturation = saturation.clamp(0.0, 1.0);
    let lightness = lightness.clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let secondary = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = lightness - chroma / 2.0;
    (
        Some(red + offset),
        Some(green + offset),
        Some(blue + offset),
    )
}

fn hsl_percentage(value: &FunctionalColorToken) -> Option<f32> {
    match value {
        FunctionalColorToken::Percentage(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn color_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn parse_hex_color(hex: &str) -> Option<Color32> {
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |index: usize| u8::from_str_radix(&hex[index..index + 1], 16).ok();
    let byte = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    match hex.len() {
        3 => Some(Color32::from_rgb(
            nibble(0)?.saturating_mul(17),
            nibble(1)?.saturating_mul(17),
            nibble(2)?.saturating_mul(17),
        )),
        4 => Some(Color32::from_rgba_unmultiplied(
            nibble(0)?.saturating_mul(17),
            nibble(1)?.saturating_mul(17),
            nibble(2)?.saturating_mul(17),
            nibble(3)?.saturating_mul(17),
        )),
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}
