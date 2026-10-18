use super::*;

pub(crate) fn components(value: &str) -> Result<Vec<String>, &'static str> {
    let mut input = Parser::new(value);
    let mut parts = Vec::new();
    while !input.is_exhausted() {
        input.skip_whitespace();
        let start = input.position();
        let token = input.next().map_err(|_| "invalid layout value")?.clone();
        if matches!(
            token,
            Token::Function(_) | Token::ParenthesisBlock | Token::SquareBracketBlock
        ) {
            input
                .parse_nested_block::<_, (), ()>(|p| {
                    while p.next().is_ok() {}
                    Ok(())
                })
                .map_err(|_| "invalid layout function")?;
        }
        parts.push(input.slice_from(start).trim().to_owned());
    }
    Ok(parts)
}

pub(super) fn number(value: &str, units: Units) -> Result<f32, &'static str> {
    let n = if let Some(n) = crate::math::number(value, units)? {
        n.max(0.0)
    } else {
        value.parse::<f32>().map_err(|_| "invalid number")?
    };
    if n.is_finite() && n >= 0.0 {
        Ok(n)
    } else {
        Err("expected nonnegative finite number")
    }
}

pub(crate) fn length_value(
    value: &str,
    units: Units,
    negative: bool,
) -> Result<(f32, bool), &'static str> {
    let mut input = Parser::new(value);
    let (value, percentage) = match input.next().map_err(|_| "expected length")? {
        Token::Number { value, .. } if *value == 0.0 => (*value, false),
        Token::Percentage { unit_value, .. } => (*unit_value, true),
        Token::Dimension { value, unit, .. } => {
            let factor = match unit.to_ascii_lowercase().as_str() {
                "px" => 1.0,
                "pt" => 96.0 / 72.0,
                "pc" => 16.0,
                "in" => 96.0,
                "cm" => 96.0 / 2.54,
                "mm" => 96.0 / 25.4,
                "q" => 96.0 / 101.6,
                "em" => units.font,
                "rem" => units.root_font,
                "vw" | "svw" | "lvw" | "dvw" => units.viewport.x / 100.0,
                "vh" | "svh" | "lvh" | "dvh" => units.viewport.y / 100.0,
                "vmin" => units.viewport.min_elem() / 100.0,
                "vmax" => units.viewport.max_elem() / 100.0,
                _ => return Err("unsupported length unit"),
            };
            (*value * factor, false)
        }
        _ => return Err("expected CSS length or percentage"),
    };
    if input.expect_exhausted().is_err() || !value.is_finite() || (!negative && value < 0.0) {
        return Err("invalid length");
    }
    Ok((value, percentage))
}

pub(crate) fn dimension(value: &str, units: Units) -> Result<Dimension, &'static str> {
    match value {
        "min-content" => return Ok(Dimension::min_content()),
        "max-content" => return Ok(Dimension::max_content()),
        "fit-content" => return Ok(Dimension::fit_content()),
        "stretch" => return Ok(Dimension::stretch()),
        "content" => return Ok(Dimension::content()),
        _ => {}
    }
    if let Some(inner) = value
        .strip_prefix("fit-content(")
        .and_then(|v| v.strip_suffix(')'))
    {
        if let Some(handle) = crate::math::length(inner.trim(), units, false, true)? {
            return Ok(Dimension::fit_content_calc(handle));
        }
        let (v, p) = length_value(inner.trim(), units, false)?;
        return Ok(if p {
            Dimension::fit_content_percent(v)
        } else {
            Dimension::fit_content_px(v)
        });
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

pub(super) fn gap(value: &str, units: Units) -> Result<LengthPercentage, &'static str> {
    if value == "normal" {
        return Ok(length(0.0_f32));
    }
    if let Some(handle) = crate::math::length(value, units, false, true)? {
        return Ok(taffy::LengthPercentage::calc(handle));
    }
    let (v, p) = length_value(value, units, false)?;
    Ok(if p { percent(v) } else { length(v) })
}
