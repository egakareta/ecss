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
        "border-image-source"
            if ["initial", "unset", "revert"]
                .iter()
                .any(|keyword| value.trim().eq_ignore_ascii_case(keyword)) =>
        {
            Some(crate::css::BackgroundImage::None)
        }
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
    let [source, slice, width, outset, repeat] = border_image_components(values)?;
    let mut image = BorderImage::default();
    if let Some(source) = source.first() {
        image.source = crate::css::parse_background_image(source)
            .map_err(|_| "invalid border image source")?;
    }
    if !slice.is_empty() {
        (image.slice, image.fill) = parse_image_slice(&slice, units)?;
    }
    if !width.is_empty() {
        image.width = parse_image_width(&width, units)?;
    }
    if !outset.is_empty() {
        image.outset = parse_image_outset(&outset, units)?;
    }
    if !repeat.is_empty() {
        image.repeat = parse_image_repeat(&repeat)?;
    }
    Ok(image)
}

pub(crate) fn border_image_components(values: &[String]) -> Result<[Vec<String>; 5], &'static str> {
    let mut components: [Vec<String>; 5] = Default::default();
    let mut values = values.iter().peekable();
    let is_boundary = |value: &str| {
        value == "/" || is_border_image_source_token(value) || image_repeat(value).is_some()
    };
    while let Some(value) = values.next() {
        if is_border_image_source_token(value) {
            if !components[0].is_empty() {
                return Err("duplicate border image source");
            }
            components[0].push(value.clone());
        } else if image_repeat(value).is_some() {
            if !components[4].is_empty() {
                return Err("invalid border-image-repeat");
            }
            components[4].push(value.clone());
            if let Some(value) = values.next_if(|value| image_repeat(value).is_some()) {
                components[4].push(value.clone());
            }
        } else {
            if !components[1].is_empty() || value == "/" {
                return Err("invalid border-image slice");
            }
            components[1].push(value.clone());
            while let Some(value) = values.next_if(|value| !is_boundary(value)) {
                components[1].push(value.clone());
            }
            if values.next_if(|value| value.as_str() == "/").is_none() {
                continue;
            }
            while let Some(value) = values.next_if(|value| !is_boundary(value)) {
                components[2].push(value.clone());
            }
            if values.next_if(|value| value.as_str() == "/").is_some() {
                while let Some(value) = values.next_if(|value| !is_boundary(value)) {
                    components[3].push(value.clone());
                }
                if components[3].is_empty() {
                    return Err("border-image outset requires a value");
                }
            } else if components[2].is_empty() {
                return Err("border-image width requires a value");
            }
        }
    }
    Ok(components)
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
