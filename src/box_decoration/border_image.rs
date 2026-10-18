use super::*;

pub(crate) fn border_image_urls(value: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for part in crate::layout_style::components(value).unwrap_or_default() {
        if let Ok(image) = crate::css::parse_background_image(&part) {
            image.collect_urls(&mut urls);
        }
    }
    urls
}

pub(crate) fn border_image_source(name: &str, value: &str) -> Option<crate::css::BackgroundImage> {
    match name {
        "border-image-source" => crate::css::parse_background_image(value).ok(),
        "border" => Some(crate::css::BackgroundImage::None),
        "border-image" => {
            let values = crate::layout_style::components(value).ok()?;
            let source = values
                .iter()
                .find(|value| is_border_image_source_token(value));
            Some(source.map_or(crate::css::BackgroundImage::None, |value| {
                crate::css::parse_background_image(value).expect("validated border image source")
            }))
        }
        _ => None,
    }
}

fn is_border_image_source_token(token: &str) -> bool {
    token.eq_ignore_ascii_case("none")
        || token
            .get(..token.find('(').unwrap_or(0))
            .is_some_and(|function| {
                [
                    "url",
                    "element",
                    "image",
                    "linear-gradient",
                    "repeating-linear-gradient",
                    "radial-gradient",
                    "repeating-radial-gradient",
                    "conic-gradient",
                    "repeating-conic-gradient",
                    "cross-fade",
                    "image-set",
                ]
                .iter()
                .any(|name| function.eq_ignore_ascii_case(name))
            })
}

pub(super) fn apply_border_image(
    name: &str,
    value: &str,
    values: &[String],
    image: &mut BorderImage,
    units: Units,
) -> Result<(), &'static str> {
    match name {
        "border-image" => *image = parse_border_image(values, units)?,
        "border-image-source" => {
            image.source = crate::css::parse_background_image(value)
                .map_err(|_| "invalid border image source")?;
        }
        "border-image-slice" => {
            (image.slice, image.fill) = parse_image_slice(values, units)?;
        }
        "border-image-width" => image.width = parse_image_width(values, units)?,
        "border-image-outset" => image.outset = parse_image_outset(values, units)?,
        "border-image-repeat" => image.repeat = parse_image_repeat(values)?,
        _ => return Err("unsupported border-image property"),
    }
    Ok(())
}

fn parse_border_image(values: &[String], units: Units) -> Result<BorderImage, &'static str> {
    let mut sections = vec![Vec::new()];
    for value in values {
        if value == "/" {
            if sections.len() == 3 {
                return Err("too many border-image slashes");
            }
            sections.push(Vec::new());
        } else {
            sections.last_mut().unwrap().push(value.clone());
        }
    }
    let mut image = BorderImage::default();
    let mut initial = Vec::new();
    let mut source_seen = false;
    for token in &sections[0] {
        if let Some(repeat) = image_repeat(token) {
            if image.repeat[0] != ImageRepeat::Stretch {
                image.repeat[1] = repeat;
            } else {
                image.repeat[0] = repeat;
                image.repeat[1] = repeat;
            }
        } else if is_border_image_source_token(token) {
            if source_seen {
                return Err("duplicate border image source");
            }
            image.source = crate::css::parse_background_image(token)
                .map_err(|_| "invalid border image source")?;
            source_seen = true;
        } else {
            initial.push(token.clone());
        }
    }
    if initial.is_empty() && sections.len() > 1 {
        return Err("border-image slash values require a slice");
    }
    if !initial.is_empty() {
        (image.slice, image.fill) = parse_image_slice(&initial, units)?;
    }
    if sections.len() == 2 && sections[1].is_empty() {
        return Err("border-image width requires a value");
    }
    if sections.len() == 3 && sections[2].is_empty() {
        return Err("border-image outset requires a value");
    }
    if sections.len() > 1 {
        let width_values = sections[1].clone();
        if !width_values.is_empty() {
            image.width = parse_image_width(&width_values, units)?;
        }
    }
    if sections.len() > 2 {
        let outset_values = sections[2].clone();
        if !outset_values.is_empty() {
            image.outset = parse_image_outset(&outset_values, units)?;
        }
    }
    let repeat_values = sections[0]
        .iter()
        .filter_map(|value| image_repeat(value))
        .collect::<Vec<_>>();
    if repeat_values.len() > 2 {
        return Err("invalid border-image-repeat");
    }
    if let Some(first) = repeat_values.first() {
        image.repeat = [*first, *repeat_values.get(1).unwrap_or(first)];
    }
    Ok(image)
}

fn parse_image_slice(
    values: &[String],
    units: Units,
) -> Result<([ImageSlice; 4], bool), &'static str> {
    let mut fill = false;
    let mut slices = Vec::with_capacity(values.len());
    for value in values {
        if value.eq_ignore_ascii_case("fill") {
            if fill {
                return Err("duplicate border-image-slice fill");
            }
            fill = true;
            continue;
        }
        let value = value.trim();
        let slice = if let Ok(Some(number)) = crate::math::number(value, units) {
            if number < 0.0 || !number.is_finite() {
                return Err("invalid border-image-slice");
            }
            ImageSlice::Number(number)
        } else if let Ok(Some(percentage)) = crate::math::percentage(value, units, false) {
            ImageSlice::PercentageCalc(percentage)
        } else if let Some(percent) = value.strip_suffix('%') {
            let percent = percent
                .parse::<f32>()
                .map_err(|_| "invalid border-image-slice")?;
            if percent < 0.0 || !percent.is_finite() {
                return Err("invalid border-image-slice");
            }
            ImageSlice::Percentage(percent / 100.0)
        } else {
            let number = value
                .parse::<f32>()
                .map_err(|_| "invalid border-image-slice")?;
            if number < 0.0 || !number.is_finite() {
                return Err("invalid border-image-slice");
            }
            ImageSlice::Number(number)
        };
        slices.push(slice);
    }
    if slices.is_empty() {
        return Err("border-image-slice requires a value");
    }
    let expanded = quad(&slices)?;
    Ok((expanded, fill))
}

fn parse_image_width(values: &[String], units: Units) -> Result<[ImageWidth; 4], &'static str> {
    let values = values
        .iter()
        .map(|value| {
            if value.eq_ignore_ascii_case("auto") {
                return Ok(ImageWidth::Auto);
            }
            if let Ok(Some(number)) = crate::math::number(value, units) {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageWidth::Number(number));
                }
                return Err("invalid border-image-width");
            }
            if let Ok(number) = value.parse::<f32>() {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageWidth::Number(number));
                }
                return Err("invalid border-image-width");
            }
            Ok(ImageWidth::Length(length_percentage(value, units, false)?))
        })
        .collect::<Result<Vec<_>, _>>()?;
    quad(&values)
}

fn parse_image_outset(values: &[String], units: Units) -> Result<[ImageOutset; 4], &'static str> {
    let values = values
        .iter()
        .map(|value| {
            if let Ok(Some(number)) = crate::math::number(value, units) {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageOutset::Number(number));
                }
                return Err("invalid border-image-outset");
            }
            if let Ok(number) = value.parse::<f32>() {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageOutset::Number(number));
                }
                return Err("invalid border-image-outset");
            }
            let length = if let Some(handle) = crate::math::length(value, units, false, false)? {
                crate::math::resolve(handle, 0.0)
            } else {
                let (length, percentage) = crate::layout_style::length_value(value, units, false)?;
                if percentage {
                    return Err("border-image-outset cannot be a percentage");
                }
                length
            };
            Ok(ImageOutset::Length(length))
        })
        .collect::<Result<Vec<_>, _>>()?;
    quad(&values)
}

fn image_repeat(value: &str) -> Option<ImageRepeat> {
    Some(match value.to_ascii_lowercase().as_str() {
        "stretch" => ImageRepeat::Stretch,
        "repeat" => ImageRepeat::Repeat,
        "round" => ImageRepeat::Round,
        "space" => ImageRepeat::Space,
        _ => return None,
    })
}

fn parse_image_repeat(values: &[String]) -> Result<[ImageRepeat; 2], &'static str> {
    if values.is_empty() || values.len() > 2 {
        return Err("invalid border-image-repeat");
    }
    let first = image_repeat(&values[0]).ok_or("invalid border-image-repeat")?;
    let second = values
        .get(1)
        .map(|value| image_repeat(value).ok_or("invalid border-image-repeat"))
        .transpose()?
        .unwrap_or(first);
    Ok([first, second])
}
