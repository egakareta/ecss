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
        Token::Function(name) if name.eq_ignore_ascii_case("hwb") => {
            parse_functional_color(input, FunctionalColorKind::Hwb)
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
    Hwb,
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
    let legacy = values
        .iter()
        .any(|value| matches!(value, FunctionalColorToken::Comma));
    if legacy {
        let valid = match kind {
            FunctionalColorKind::Rgb => {
                [first, second, third]
                    .iter()
                    .all(|value| matches!(value, FunctionalColorToken::Number(_)))
                    || [first, second, third]
                        .iter()
                        .all(|value| matches!(value, FunctionalColorToken::Percentage(_)))
            }
            FunctionalColorKind::Hsl => {
                matches!(second, FunctionalColorToken::Percentage(_))
                    && matches!(third, FunctionalColorToken::Percentage(_))
            }
            FunctionalColorKind::Hwb => false,
        };
        if !valid {
            return Err(ParseError::custom("invalid legacy color components"));
        }
    }
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
        FunctionalColorKind::Hwb => hwb_components(first, second, third),
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
    let hue = hue_degrees(hue);
    let saturation = hundred_scale(saturation);
    let lightness = hundred_scale(lightness);
    let (Some(hue), Some(saturation), Some(lightness)) = (hue, saturation, lightness) else {
        return (None, None, None);
    };
    let [red, green, blue] = hsl_to_rgb(hue, saturation.max(0.0), lightness);
    (Some(red), Some(green), Some(blue))
}

fn hue_degrees(hue: &FunctionalColorToken) -> Option<f32> {
    let hue = match hue {
        FunctionalColorToken::Number(value) => f64::from(*value),
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("deg") => {
            f64::from(*value)
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("grad") => {
            f64::from(*value) * 0.9
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("rad") => {
            f64::from(*value).to_degrees()
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("turn") => {
            f64::from(*value) * 360.0
        }
        _ => return None,
    };
    hue.is_finite().then(|| hue.rem_euclid(360.0) as f32)
}

fn hwb_components(
    hue: &FunctionalColorToken,
    white: &FunctionalColorToken,
    black: &FunctionalColorToken,
) -> (Option<f32>, Option<f32>, Option<f32>) {
    let (Some(hue), Some(white), Some(black)) =
        (hue_degrees(hue), hundred_scale(white), hundred_scale(black))
    else {
        return (None, None, None);
    };
    let white = f64::from(white);
    let black = f64::from(black);
    let rgb = if white + black >= 1.0 {
        [(white / (white + black)) as f32; 3]
    } else {
        hsl_to_rgb(hue, 1.0, 0.5)
            .map(|channel| (f64::from(channel) * (1.0 - white - black) + white) as f32)
    };
    (Some(rgb[0]), Some(rgb[1]), Some(rgb[2]))
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [f32; 3] {
    let lightness = f64::from(lightness);
    let amplitude = f64::from(saturation) * lightness.min(1.0 - lightness);
    [0.0, 8.0, 4.0].map(|offset| {
        let phase = (offset + f64::from(hue) / 30.0) % 12.0;
        (lightness - amplitude * (phase - 3.0).min(9.0 - phase).clamp(-1.0, 1.0)) as f32
    })
}

fn hundred_scale(value: &FunctionalColorToken) -> Option<f32> {
    match value {
        FunctionalColorToken::Percentage(value) if value.is_finite() => Some(*value),
        FunctionalColorToken::Number(value) if value.is_finite() => Some(*value / 100.0),
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
