use super::*;

pub(crate) fn parse_background_image(
    value: &str,
) -> Result<BackgroundImage, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let image = match input.next()? {
        Token::Ident(name) if name.eq_ignore_ascii_case("none") => BackgroundImage::None,
        Token::UnquotedUrl(url) => BackgroundImage::Url(url.to_string()),
        Token::Function(name) if name.eq_ignore_ascii_case("url") => {
            let url = input.parse_nested_block(|url| {
                let value = match url.next()? {
                    Token::QuotedString(value) | Token::UnquotedUrl(value) => value.to_string(),
                    _ => return Err(ParseError::custom("unsupported background image")),
                };
                url.expect_exhausted()?;
                Ok(value)
            })?;
            BackgroundImage::Url(url)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("element") => {
            let id = input.parse_nested_block(|args| {
                let id = match args.next()? {
                    Token::IDHash(id) => id.to_string(),
                    _ => return Err(ParseError::custom("element() requires an ID selector")),
                };
                args.expect_exhausted()?;
                Ok(id)
            })?;
            BackgroundImage::Element(id)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("linear-gradient")
                || name.eq_ignore_ascii_case("repeating-linear-gradient") =>
        {
            let repeating = name.eq_ignore_ascii_case("repeating-linear-gradient");
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::LinearGradient(parse_linear_gradient(&args, repeating)?)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("conic-gradient")
                || name.eq_ignore_ascii_case("repeating-conic-gradient") =>
        {
            let repeating = name.eq_ignore_ascii_case("repeating-conic-gradient");
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::ConicGradient(parse_conic_gradient(&args, repeating)?)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("radial-gradient")
                || name.eq_ignore_ascii_case("repeating-radial-gradient") =>
        {
            let repeating = name.eq_ignore_ascii_case("repeating-radial-gradient");
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::RadialGradient(parse_radial_gradient(&args, repeating)?)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("cross-fade") => {
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            parse_cross_fade(&args)?
        }
        Token::Function(name) if name.eq_ignore_ascii_case("image") => {
            let color = input.parse_nested_block(parse_color)?;
            BackgroundImage::SolidColor(color)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("image-set") => {
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::ImageSet(parse_image_set(&args)?)
        }
        _ => return Err(ParseError::custom("unsupported background image")),
    };
    input.expect_exhausted()?;
    Ok(image)
}

fn parse_cross_fade(value: &str) -> Result<BackgroundImage, ParseError<&'static str>> {
    let arguments = split_top_level_commas(value);
    if arguments.is_empty() {
        return Err(ParseError::custom(
            "cross-fade() requires at least one argument",
        ));
    }
    let mut parsed = Vec::new();
    let mut specified = 0.0;
    let mut omitted = 0;
    for argument in arguments {
        let components = crate::layout_style::components(argument).map_err(ParseError::custom)?;
        if components.is_empty() {
            return Err(ParseError::custom("empty cross-fade() argument"));
        }
        let mut weight = None;
        let mut image_components = Vec::with_capacity(components.len());
        for component in components {
            if let Ok(parsed_weight) = parse_percentage(&component) {
                if weight.replace(parsed_weight).is_some() {
                    return Err(ParseError::custom("duplicate cross-fade percentage"));
                }
                if !(0.0..=1.0).contains(&parsed_weight) {
                    return Err(ParseError::custom(
                        "cross-fade percentages must be between 0% and 100%",
                    ));
                }
            } else {
                image_components.push(component);
            }
        }
        if let Some(weight) = weight {
            specified += weight;
        } else {
            omitted += 1;
        }
        if image_components.is_empty() {
            return Err(ParseError::custom(
                "cross-fade() argument is missing an image or color",
            ));
        }
        let source = image_components.join(" ");
        let image = parse_background_image(&source).or_else(|_| {
            parse_color(&mut CssParser::new(&source)).map(BackgroundImage::SolidColor)
        })?;
        if image == BackgroundImage::None {
            return Err(ParseError::custom("cross-fade() arguments cannot be none"));
        }
        if image.contains_image_set() {
            return Err(ParseError::custom(
                "image-set() cannot be nested inside another image function",
            ));
        }
        parsed.push((image, weight));
    }
    let omitted_weight = if omitted == 0 {
        0.0
    } else {
        (1.0 - specified).max(0.0) / omitted as f32
    };
    Ok(BackgroundImage::CrossFade(CrossFade {
        items: parsed
            .into_iter()
            .map(|(image, weight)| CrossFadeItem {
                image: Box::new(image),
                weight: weight.unwrap_or(omitted_weight),
            })
            .collect(),
    }))
}

fn parse_percentage(value: &str) -> Result<f32, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let percentage = match input.next()? {
        Token::Percentage { unit_value, .. } => *unit_value,
        _ => return Err(ParseError::custom("expected a percentage")),
    };
    input.expect_exhausted()?;
    Ok(percentage)
}

fn parse_image_set(value: &str) -> Result<Vec<ImageSetCandidate>, ParseError<&'static str>> {
    let mut candidates = Vec::new();
    for candidate in split_top_level_commas(value) {
        let parts = crate::layout_style::components(candidate).map_err(ParseError::custom)?;
        let Some(source) = parts.first() else {
            return Err(ParseError::custom("empty image-set candidate"));
        };
        let image = parse_image_set_source(source)?;
        if image == BackgroundImage::None {
            return Err(ParseError::custom("image-set candidates cannot be none"));
        }
        if image.contains_image_set() {
            return Err(ParseError::custom(
                "image-set() cannot be nested inside another image function",
            ));
        }
        let mut resolution = None;
        let mut type_hint = None;
        for descriptor in &parts[1..] {
            if descriptor
                .split_once('(')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("type"))
            {
                if type_hint.is_some() {
                    return Err(ParseError::custom("duplicate image-set type hint"));
                }
                type_hint = Some(parse_image_type_hint(descriptor)?);
            } else if resolution
                .replace(parse_image_resolution(descriptor)?)
                .is_some()
            {
                return Err(ParseError::custom("duplicate image-set resolution"));
            }
        }
        candidates.push(ImageSetCandidate {
            image: Box::new(image),
            resolution: resolution.unwrap_or(1.0),
            type_hint,
        });
    }
    if candidates.is_empty() {
        return Err(ParseError::custom(
            "image-set() requires at least one image",
        ));
    }
    Ok(candidates)
}

fn parse_image_set_source(value: &str) -> Result<BackgroundImage, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    if let Token::QuotedString(url) = input.next()?.clone() {
        input.expect_exhausted()?;
        return Ok(BackgroundImage::Url(url.to_string()));
    }
    parse_background_image(value)
}

fn parse_image_resolution(value: &str) -> Result<f32, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let resolution = match input.next()? {
        Token::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str() {
            "x" | "dppx" => *value,
            "dpi" => *value / 96.0,
            "dpcm" => *value * 2.54 / 96.0,
            _ => return Err(ParseError::custom("invalid image-set resolution unit")),
        },
        _ => return Err(ParseError::custom("invalid image-set resolution")),
    };
    input.expect_exhausted()?;
    if !resolution.is_finite() || resolution <= 0.0 {
        return Err(ParseError::custom("image-set resolution must be positive"));
    }
    Ok(resolution)
}

fn parse_image_type_hint(value: &str) -> Result<String, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let mime = match input.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("type") => {
            input.parse_nested_block(|args| {
                let mime = args.expect_string()?.to_string();
                if mime.is_empty() {
                    return Err(ParseError::custom("empty image-set type hint"));
                }
                args.expect_exhausted()?;
                Ok::<_, ParseError<&'static str>>(mime)
            })?
        }
        _ => return Err(ParseError::custom("invalid image-set type hint")),
    };
    input.expect_exhausted()?;
    Ok(mime.to_ascii_lowercase())
}

pub(super) fn parse_background_attachments<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<BackgroundAttachment>, ParseError<&'static str>> {
    split_top_level_commas(&consume_value(input)?)
        .into_iter()
        .map(parse_background_attachment_value)
        .collect()
}

pub(super) fn parse_background_attachment_value(
    value: &str,
) -> Result<BackgroundAttachment, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let attachment = match input.expect_ident()?.to_ascii_lowercase().as_str() {
        "scroll" => BackgroundAttachment::Scroll,
        "fixed" => BackgroundAttachment::Fixed,
        "local" => BackgroundAttachment::Local,
        _ => return Err(ParseError::custom("unsupported background attachment")),
    };
    input.expect_exhausted()?;
    Ok(attachment)
}

pub(super) fn parse_background_clips<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<BackgroundClip>, ParseError<&'static str>> {
    split_top_level_commas(&consume_value(input)?)
        .into_iter()
        .map(parse_background_clip_value)
        .collect()
}

pub(super) fn parse_background_clip_value(
    value: &str,
) -> Result<BackgroundClip, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let clip = match input.expect_ident()?.to_ascii_lowercase().as_str() {
        "border-box" => BackgroundClip::Border,
        "padding-box" => BackgroundClip::Padding,
        "content-box" => BackgroundClip::Content,
        _ => return Err(ParseError::custom("unsupported background clip")),
    };
    input.expect_exhausted()?;
    Ok(clip)
}

pub(super) fn consume_value<'i>(
    input: &mut CssParser<'i>,
) -> Result<String, ParseError<&'static str>> {
    let start = input.position();
    while input.next().is_ok() {}
    Ok(input.slice_from(start).trim().to_owned())
}

fn parse_linear_gradient(
    value: &str,
    repeating: bool,
) -> Result<LinearGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid linear gradient"));
    }
    let mut prelude = crate::layout_style::components(parts[0]).map_err(ParseError::custom)?;
    let (color_space, has_color_space) = parse_gradient_color_space(&mut prelude)?;
    let mut angle = 180.0;
    if !prelude.is_empty()
        && let Some(parsed) = parse_gradient_angle(&prelude.join(" "))
    {
        angle = parsed;
        parts.remove(0);
    } else if prelude.is_empty() && has_color_space {
        parts.remove(0);
    } else if prelude
        .first()
        .is_some_and(|value| parse_gradient_angle(value).is_some())
    {
        angle = parse_gradient_angle(&prelude[0]).unwrap_or(angle);
        if prelude.len() != 1 {
            return Err(ParseError::custom("invalid linear gradient prelude"));
        }
        parts.remove(0);
    }
    let stops = parse_gradient_stops(parts.iter().copied(), false)?;
    if stops.len() < 2 {
        return Err(ParseError::custom(
            "a linear gradient needs two color stops",
        ));
    }
    Ok(LinearGradient {
        angle,
        stops,
        repeating,
        color_space,
    })
}

fn parse_conic_gradient(
    value: &str,
    repeating: bool,
) -> Result<ConicGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid conic gradient"));
    }
    let mut from_angle = 0.0;
    let mut center = [GradientPosition::center(); 2];
    let mut prelude = crate::layout_style::components(parts[0]).map_err(ParseError::custom)?;
    let (color_space, has_color_space) = parse_gradient_color_space(&mut prelude)?;
    if has_color_space
        || prelude.first().is_some_and(|value| {
            value.eq_ignore_ascii_case("from") || value.eq_ignore_ascii_case("at")
        })
    {
        let mut index = 0;
        let mut from_seen = false;
        let mut at_seen = false;
        while index < prelude.len() {
            if prelude[index].eq_ignore_ascii_case("from") && !from_seen {
                let value = prelude
                    .get(index + 1)
                    .ok_or_else(|| ParseError::custom("missing conic gradient angle"))?;
                from_angle = parse_gradient_angle(value)
                    .ok_or_else(|| ParseError::custom("invalid conic gradient angle"))?;
                from_seen = true;
                index += 2;
            } else if prelude[index].eq_ignore_ascii_case("at") && !at_seen {
                center = parse_gradient_position(&prelude[index + 1..])?;
                at_seen = true;
                index = prelude.len();
            } else {
                return Err(ParseError::custom("invalid conic gradient prelude"));
            }
        }
        parts.remove(0);
    }
    let stops = parse_gradient_stops(parts.iter().copied(), true)?;
    if stops.len() < 2 {
        return Err(ParseError::custom("a conic gradient needs two color stops"));
    }
    Ok(ConicGradient {
        from_angle,
        center,
        stops,
        repeating,
        color_space,
    })
}

fn parse_radial_gradient(
    value: &str,
    repeating: bool,
) -> Result<RadialGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid radial gradient"));
    }
    let mut shape = RadialShape::Ellipse;
    let mut size = RadialSize::Extent(RadialExtent::FarthestCorner);
    let mut center = [GradientPosition::center(); 2];
    let mut prelude = crate::layout_style::components(parts[0]).map_err(ParseError::custom)?;
    let (color_space, has_color_space) = parse_gradient_color_space(&mut prelude)?;
    let mut index = 0;
    let mut has_prelude = has_color_space;
    if has_color_space
        || prelude.first().is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "circle"
                    | "ellipse"
                    | "closest-side"
                    | "farthest-side"
                    | "closest-corner"
                    | "farthest-corner"
                    | "at"
            )
        })
        || prelude
            .first()
            .is_some_and(|value| parse_radial_radius(value).is_ok())
    {
        has_prelude = true;
        if prelude[index].eq_ignore_ascii_case("circle") {
            shape = RadialShape::Circle;
            index += 1;
        } else if prelude[index].eq_ignore_ascii_case("ellipse") {
            shape = RadialShape::Ellipse;
            index += 1;
        }

        let mut explicit_radii = Vec::new();
        while index < prelude.len() && !prelude[index].eq_ignore_ascii_case("at") {
            let extent = match prelude[index].to_ascii_lowercase().as_str() {
                "closest-side" => Some(RadialExtent::ClosestSide),
                "farthest-side" => Some(RadialExtent::FarthestSide),
                "closest-corner" => Some(RadialExtent::ClosestCorner),
                "farthest-corner" => Some(RadialExtent::FarthestCorner),
                _ => None,
            };
            if let Some(extent) = extent {
                if !explicit_radii.is_empty() {
                    return Err(ParseError::custom("conflicting radial gradient sizes"));
                }
                size = RadialSize::Extent(extent);
                index += 1;
                break;
            }
            explicit_radii.push(parse_radial_radius(&prelude[index])?);
            if explicit_radii.len() > 2 {
                return Err(ParseError::custom("too many radial gradient radii"));
            }
            index += 1;
        }
        if !explicit_radii.is_empty() {
            if shape == RadialShape::Circle && explicit_radii.len() > 1 {
                return Err(ParseError::custom("circle gradients accept one radius"));
            }
            let radii = if explicit_radii.len() == 1 {
                [explicit_radii[0]; 2]
            } else {
                [explicit_radii[0], explicit_radii[1]]
            };
            size = RadialSize::Explicit(radii);
        }
        if index < prelude.len() {
            if !prelude[index].eq_ignore_ascii_case("at") || index + 1 >= prelude.len() {
                return Err(ParseError::custom("invalid radial gradient prelude"));
            }
            center = parse_gradient_position(&prelude[index + 1..])?;
        }
        parts.remove(0);
    }
    if !has_prelude {
        center = [GradientPosition::center(); 2];
    }
    let stops = parse_gradient_stops(parts.iter().copied(), false)?;
    if stops.len() < 2 {
        return Err(ParseError::custom(
            "a radial gradient needs two color stops",
        ));
    }
    Ok(RadialGradient {
        shape,
        size,
        center,
        stops,
        repeating,
        color_space,
    })
}

fn parse_gradient_color_space(
    prelude: &mut Vec<String>,
) -> Result<(GradientColorSpace, bool), ParseError<&'static str>> {
    let Some(index) = prelude
        .iter()
        .position(|value| value.eq_ignore_ascii_case("in"))
    else {
        return Ok((GradientColorSpace::default(), false));
    };
    let space = prelude
        .get(index + 1)
        .ok_or_else(|| ParseError::custom("missing gradient interpolation color space"))?;
    let color_space = if space.eq_ignore_ascii_case("srgb") {
        GradientColorSpace::Srgb
    } else if space.eq_ignore_ascii_case("srgb-linear") {
        GradientColorSpace::SrgbLinear
    } else if space.eq_ignore_ascii_case("oklab") {
        GradientColorSpace::Oklab
    } else {
        return Err(ParseError::custom(
            "unsupported gradient interpolation color space",
        ));
    };
    prelude.drain(index..=index + 1);
    Ok((color_space, true))
}

fn parse_radial_radius(value: &str) -> Result<RadialRadius, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let radius = match input.next()? {
        Token::Number { value, .. } if *value == 0.0 => RadialRadius::Length(0.0),
        Token::Percentage { unit_value, .. } if *unit_value >= 0.0 => {
            RadialRadius::Percentage(*unit_value)
        }
        Token::Dimension { value, unit, .. } => {
            let factor = match unit.to_ascii_lowercase().as_str() {
                "px" => 1.0,
                "pt" => 96.0 / 72.0,
                "pc" => 16.0,
                "in" => 96.0,
                "cm" => 96.0 / 2.54,
                "mm" => 96.0 / 25.4,
                "q" => 96.0 / 101.6,
                _ => return Err(ParseError::custom("unsupported radial gradient length")),
            };
            if *value < 0.0 {
                return Err(ParseError::custom(
                    "radial gradient radii cannot be negative",
                ));
            }
            RadialRadius::Length(*value * factor)
        }
        _ => return Err(ParseError::custom("invalid radial gradient radius")),
    };
    input.expect_exhausted()?;
    Ok(radius)
}

fn parse_gradient_position(
    values: &[String],
) -> Result<[GradientPosition; 2], ParseError<&'static str>> {
    if values.is_empty() || values.len() > 4 {
        return Err(ParseError::custom("invalid conic gradient position"));
    }
    let mut position = [None, None];
    let mut index = 0;
    while index < values.len() {
        let lower = values[index].to_ascii_lowercase();
        let edge = match lower.as_str() {
            "left" => Some((0, false, 0.0)),
            "right" => Some((0, true, 1.0)),
            "top" => Some((1, false, 0.0)),
            "bottom" => Some((1, true, 1.0)),
            _ => None,
        };
        if let Some((axis, from_end, percentage)) = edge {
            let has_offset = values.len() > 2
                && values.get(index + 1).is_some_and(|next| {
                    !matches!(
                        next.to_ascii_lowercase().as_str(),
                        "left" | "right" | "top" | "bottom" | "center"
                    )
                });
            let coordinate = if has_offset {
                let mut coordinate = parse_gradient_coordinate(&values[index + 1])?;
                coordinate.from_end = from_end;
                index += 2;
                coordinate
            } else {
                index += 1;
                GradientPosition {
                    percentage,
                    length: 0.0,
                    from_end: false,
                }
            };
            if position[axis].replace(coordinate).is_some() {
                return Err(ParseError::custom("duplicate gradient position axis"));
            }
        } else if lower == "center" {
            let axis = if position[0].is_none() {
                0
            } else if position[1].is_none() {
                1
            } else {
                return Err(ParseError::custom("too many gradient position coordinates"));
            };
            position[axis] = Some(GradientPosition::center());
            index += 1;
        } else {
            let axis = if position[0].is_none() {
                0
            } else if position[1].is_none() {
                1
            } else {
                return Err(ParseError::custom("too many gradient position coordinates"));
            };
            position[axis] = Some(parse_gradient_coordinate(&values[index])?);
            index += 1;
        }
    }
    Ok([
        position[0].unwrap_or_else(GradientPosition::center),
        position[1].unwrap_or_else(GradientPosition::center),
    ])
}

fn parse_gradient_coordinate(value: &str) -> Result<GradientPosition, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let (percentage, length) = match input.next()? {
        Token::Percentage { unit_value, .. } => (*unit_value, 0.0),
        Token::Number { value, .. } if *value == 0.0 => (0.0, 0.0),
        Token::Dimension { value, unit, .. } => {
            let factor = match unit.to_ascii_lowercase().as_str() {
                "px" => 1.0,
                "pt" => 96.0 / 72.0,
                "pc" => 16.0,
                "in" => 96.0,
                "cm" => 96.0 / 2.54,
                "mm" => 96.0 / 25.4,
                "q" => 96.0 / 101.6,
                _ => return Err(ParseError::custom("unsupported gradient position length")),
            };
            (0.0, *value * factor)
        }
        _ => return Err(ParseError::custom("invalid gradient position coordinate")),
    };
    input.expect_exhausted()?;
    if !percentage.is_finite() || !length.is_finite() {
        return Err(ParseError::custom("invalid gradient position coordinate"));
    }
    Ok(GradientPosition {
        percentage,
        length,
        from_end: false,
    })
}

fn parse_gradient_stops<'a>(
    parts: impl IntoIterator<Item = &'a str>,
    conic: bool,
) -> Result<Vec<GradientStop>, ParseError<&'static str>> {
    let mut stops: Vec<GradientStop> = Vec::new();
    let mut pending_hint = None;
    for part in parts {
        let parsed = if conic {
            parse_conic_gradient_stop(part)
        } else {
            parse_gradient_stop(part)
        };
        match parsed {
            Ok(mut next) => {
                if let Some(hint) = pending_hint.take() {
                    let Some(previous) = stops.last_mut() else {
                        return Err(ParseError::custom(
                            "gradient hint requires a preceding stop",
                        ));
                    };
                    previous.hint_after = Some(hint);
                }
                stops.append(&mut next);
            }
            Err(error) => {
                let hint = parse_gradient_hint(part, conic);
                if let Ok(hint) = hint {
                    if stops.is_empty() || pending_hint.replace(hint).is_some() {
                        return Err(ParseError::custom("invalid gradient hint position"));
                    }
                } else {
                    return Err(error);
                }
            }
        }
    }
    if pending_hint.is_some() {
        return Err(ParseError::custom(
            "gradient hint requires a following stop",
        ));
    }
    Ok(stops)
}

fn parse_gradient_hint(value: &str, conic: bool) -> Result<f32, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let position = match input.next()? {
        Token::Percentage { unit_value, .. } => *unit_value,
        Token::Number { value, .. } if *value == 0.0 => 0.0,
        Token::Dimension { .. } if conic => {
            parse_gradient_angle(value)
                .ok_or_else(|| ParseError::custom("invalid conic gradient hint"))?
                / 360.0
        }
        _ => return Err(ParseError::custom("invalid gradient hint")),
    };
    input.expect_exhausted()?;
    Ok(position)
}

fn parse_conic_gradient_stop(value: &str) -> Result<Vec<GradientStop>, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let start = input.position();
    let token = input.next()?.clone();
    if matches!(token, Token::Function(_) | Token::ParenthesisBlock) {
        input.parse_nested_block(|args| {
            while args.next().is_ok() {}
            Ok::<_, ParseError<&'static str>>(())
        })?;
    }
    let color_text = input.slice_from(start).trim().to_owned();
    let color = parse_color(&mut CssParser::new(&color_text))?;
    input.skip_whitespace();
    let mut positions = Vec::new();
    while !input.is_exhausted() {
        let start = input.position();
        let token = input.next()?.clone();
        let position = match token {
            Token::Percentage { unit_value, .. } => unit_value,
            Token::Number { value: 0.0, .. } => 0.0,
            Token::Dimension { .. } => {
                parse_gradient_angle(input.slice_from(start).trim())
                    .ok_or_else(|| ParseError::custom("invalid conic gradient stop"))?
                    / 360.0
            }
            _ => return Err(ParseError::custom("invalid conic gradient stop")),
        };
        positions.push(position);
        if positions.len() > 2 {
            return Err(ParseError::custom("too many conic gradient stop positions"));
        }
        input.skip_whitespace();
    }
    Ok(gradient_stops(color, positions))
}

pub(super) fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_u32;
    let mut quote = None;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
        } else {
            match character {
                '\'' | '"' => quote = Some(character),
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    parts.push(value[start..index].trim());
                    start = index + character.len_utf8();
                }
                _ => {}
            }
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn parse_gradient_angle(value: &str) -> Option<f32> {
    let lower = value.to_ascii_lowercase();
    if let Some(direction) = lower.strip_prefix("to ") {
        return Some(match direction.trim() {
            "top" => 0.0,
            "right" => 90.0,
            "bottom" => 180.0,
            "left" => 270.0,
            "top right" | "right top" => 45.0,
            "bottom right" | "right bottom" => 135.0,
            "bottom left" | "left bottom" => 225.0,
            "top left" | "left top" => 315.0,
            _ => return None,
        });
    }
    let mut input = CssParser::new(value);
    let angle = match input.next().ok()? {
        Token::Number { value, .. } if *value == 0.0 => 0.0,
        Token::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str() {
            "deg" => *value,
            "grad" => *value * 0.9,
            "rad" => value.to_degrees(),
            "turn" => *value * 360.0,
            _ => return None,
        },
        _ => return None,
    };
    input.expect_exhausted().ok()?;
    Some(angle)
}

fn parse_gradient_stop(value: &str) -> Result<Vec<GradientStop>, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let start = input.position();
    let token = input.next()?.clone();
    if matches!(token, Token::Function(_) | Token::ParenthesisBlock) {
        input.parse_nested_block(|args| {
            while args.next().is_ok() {}
            Ok::<_, ParseError<&'static str>>(())
        })?;
    }
    let color_text = input.slice_from(start).trim().to_owned();
    let color = parse_color(&mut CssParser::new(&color_text))?;
    input.skip_whitespace();
    let mut positions = Vec::new();
    while !input.is_exhausted() {
        let position = match input.next()? {
            Token::Percentage { unit_value, .. } => *unit_value,
            _ => return Err(ParseError::custom("unsupported gradient stop")),
        };
        positions.push(position);
        if positions.len() > 2 {
            return Err(ParseError::custom("too many gradient stop positions"));
        }
        input.skip_whitespace();
    }
    Ok(gradient_stops(color, positions))
}

fn gradient_stops(color: Color32, positions: Vec<f32>) -> Vec<GradientStop> {
    if positions.is_empty() {
        vec![GradientStop {
            color,
            position: None,
            hint_after: None,
        }]
    } else {
        positions
            .into_iter()
            .map(|position| GradientStop {
                color,
                position: Some(position),
                hint_after: None,
            })
            .collect()
    }
}
