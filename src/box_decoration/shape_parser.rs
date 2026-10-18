use super::*;

pub(super) fn parse_border_shape(
    values: &[String],
    units: Units,
) -> Result<Option<BorderShape>, &'static str> {
    if values.len() == 1 && values[0].eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let mut shapes = Vec::new();
    let mut geometry_seen = false;
    for value in values {
        if let Some(geometry) = geometry_box(value) {
            if geometry_seen {
                return Err("duplicate border-shape geometry box");
            }
            let Some((_, existing)) = shapes.last_mut() else {
                return Err("border-shape geometry box needs a preceding shape");
            };
            *existing = geometry;
            geometry_seen = true;
            continue;
        }
        if shapes.len() == 2 {
            return Err("border-shape accepts at most two shapes");
        }
        let shape = parse_basic_shape(value, units)?;
        let default_box = match shapes.len() {
            0 if values.iter().filter(|value| value.contains('(')).count() == 1 => {
                GeometryBox::HalfBorder
            }
            0 => GeometryBox::Border,
            _ => GeometryBox::Padding,
        };
        shapes.push((shape, default_box));
        geometry_seen = false;
    }
    if shapes.is_empty() {
        return Err("border-shape requires a shape or none");
    }
    Ok(Some(BorderShape { shapes }))
}

fn geometry_box(value: &str) -> Option<GeometryBox> {
    match value.to_ascii_lowercase().as_str() {
        "border-box" => Some(GeometryBox::Border),
        "padding-box" => Some(GeometryBox::Padding),
        "content-box" => Some(GeometryBox::Content),
        "margin-box" => Some(GeometryBox::Margin),
        "half-border-box" => Some(GeometryBox::HalfBorder),
        _ => None,
    }
}

fn parse_basic_shape(value: &str, units: Units) -> Result<BasicShape, &'static str> {
    let open = value.find('(').ok_or("expected a basic shape function")?;
    if !value.ends_with(')') {
        return Err("invalid basic shape function");
    }
    let name = value[..open].to_ascii_lowercase();
    let inner = &value[open + 1..value.len() - 1];
    let components = crate::layout_style::components(inner)?;
    match name.as_str() {
        "circle" => {
            let at = components
                .iter()
                .position(|value| value.eq_ignore_ascii_case("at"));
            let radius_values = &components[..at.unwrap_or(components.len())];
            if radius_values.len() > 1 {
                return Err("circle() takes one radius");
            }
            let radius = radius_values
                .first()
                .map(|value| shape_radius(value, units, false))
                .transpose()?;
            let position = at
                .map(|at| shape_position(&components[at + 1..], units))
                .transpose()?
                .unwrap_or_default();
            Ok(BasicShape::Circle { radius, position })
        }
        "ellipse" => {
            let at = components
                .iter()
                .position(|value| value.eq_ignore_ascii_case("at"));
            let radius_values = &components[..at.unwrap_or(components.len())];
            if radius_values.len() > 2 {
                return Err("ellipse() takes two radii");
            }
            let radii = match radius_values {
                [] => [None, None],
                [value] => {
                    let radius = Some(shape_radius(value, units, false)?);
                    [radius, radius]
                }
                [x, y] => [
                    Some(shape_radius(x, units, false)?),
                    Some(shape_radius(y, units, false)?),
                ],
                _ => unreachable!(),
            };
            let position = at
                .map(|at| shape_position(&components[at + 1..], units))
                .transpose()?
                .unwrap_or_default();
            Ok(BasicShape::Ellipse { radii, position })
        }
        "inset" => {
            let round = components
                .iter()
                .position(|value| value.eq_ignore_ascii_case("round"));
            let inset_values = &components[..round.unwrap_or(components.len())];
            let inset_values = quad(inset_values)?;
            let insets = [
                length_percentage(&inset_values[0], units, true)?,
                length_percentage(&inset_values[1], units, true)?,
                length_percentage(&inset_values[2], units, true)?,
                length_percentage(&inset_values[3], units, true)?,
            ];
            let radii = if let Some(round) = round {
                let (horizontal, vertical) = slash_pair(&components[round + 1..])?;
                let horizontal = radius_quad(horizontal, units)?;
                let vertical = vertical
                    .map(|values| radius_quad(values, units))
                    .transpose()?
                    .unwrap_or(horizontal);
                Some(std::array::from_fn(|index| CornerRadius {
                    x: horizontal[index],
                    y: vertical[index],
                }))
            } else {
                None
            };
            Ok(BasicShape::Inset { insets, radii })
        }
        "polygon" => {
            let mut points = Vec::new();
            let mut fill_rule = FillRule::Nonzero;
            for (index, pair) in split_commas(inner).into_iter().enumerate() {
                let pair = crate::layout_style::components(pair)?;
                if index == 0
                    && pair.len() == 1
                    && (pair[0].eq_ignore_ascii_case("evenodd")
                        || pair[0].eq_ignore_ascii_case("nonzero"))
                {
                    fill_rule = if pair[0].eq_ignore_ascii_case("evenodd") {
                        FillRule::Evenodd
                    } else {
                        FillRule::Nonzero
                    };
                    continue;
                }
                if pair.len() != 2 {
                    return Err("polygon() requires coordinate pairs");
                }
                points.push((
                    length_percentage(&pair[0], units, true)?,
                    length_percentage(&pair[1], units, true)?,
                ));
            }
            if points.len() < 3 {
                return Err("polygon() requires at least three points");
            }
            Ok(BasicShape::Polygon { points, fill_rule })
        }
        "rect" => {
            if components.len() != 4 {
                return Err("rect() requires four edges");
            }
            let values = [
                rect_edge(&components[0], 0, units)?,
                rect_edge(&components[1], 1, units)?,
                rect_edge(&components[2], 2, units)?,
                rect_edge(&components[3], 3, units)?,
            ];
            Ok(BasicShape::Rect(values))
        }
        "xywh" => {
            if components.len() != 4 {
                return Err("xywh() requires four values");
            }
            let values = [
                length_percentage(&components[0], units, true)?,
                length_percentage(&components[1], units, true)?,
                length_percentage(&components[2], units, false)?,
                length_percentage(&components[3], units, false)?,
            ];
            Ok(BasicShape::Xywh(values))
        }
        "shape" => Ok(BasicShape::Shape(parse_shape(inner, units)?)),
        "path" => {
            let (contours, fill_rule) = parse_path(inner)?;
            Ok(BasicShape::Path {
                contours,
                fill_rule,
            })
        }
        _ => Err("unsupported border-shape basic shape"),
    }
}

fn parse_shape(value: &str, units: Units) -> Result<ShapeFunction, &'static str> {
    let commands = split_commas(value);
    if commands.len() < 2 {
        return Err("shape() requires a starting point and a command");
    }
    let mut header = crate::layout_style::components(commands[0])?;
    let fill_rule = if header
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("evenodd"))
    {
        header.remove(0);
        FillRule::Evenodd
    } else if header
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("nonzero"))
    {
        header.remove(0);
        FillRule::Nonzero
    } else {
        FillRule::Nonzero
    };
    if !header
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("from"))
    {
        return Err("shape() requires a from position");
    }
    let start = shape_position(&header[1..], units)?;
    let mut parsed = Vec::with_capacity(commands.len() - 1);
    for command in commands.iter().skip(1) {
        let values = crate::layout_style::components(command)?;
        let Some(name) = values.first().map(|value| value.to_ascii_lowercase()) else {
            return Err("empty shape() command");
        };
        match name.as_str() {
            "close" if values.len() == 1 => parsed.push(ShapeCommand::Close),
            "line" | "move" => {
                let endpoint = parse_shape_endpoint(&values[1..], units)?;
                parsed.push(if name == "line" {
                    ShapeCommand::Line(endpoint)
                } else {
                    ShapeCommand::Move(endpoint)
                });
            }
            "hline" | "vline" => {
                let [mode, value] = &values[1..] else {
                    return Err("hline() and vline() require one coordinate");
                };
                let mode = mode.to_ascii_lowercase();
                let relative = match mode.as_str() {
                    "to" => false,
                    "by" => true,
                    _ => return Err("hline() and vline() require to or by"),
                };
                let value = shape_axis_value(value, name == "hline", units, relative)?;
                parsed.push(if name == "hline" {
                    ShapeCommand::Horizontal { relative, value }
                } else {
                    ShapeCommand::Vertical { relative, value }
                });
            }
            "curve" => {
                let with = values
                    .iter()
                    .position(|value| value.eq_ignore_ascii_case("with"))
                    .ok_or("curve command requires with control points")?;
                let endpoint = parse_shape_endpoint(&values[1..with], units)?;
                let control_groups = split_shape_controls(&values[with + 1..])?;
                let relative = matches!(endpoint, ShapeEndpoint::By(_));
                let controls = control_groups
                    .into_iter()
                    .map(|control| parse_shape_control(control, units, relative))
                    .collect::<Result<Vec<_>, _>>()?;
                if controls.is_empty() || controls.len() > 2 {
                    return Err("curve command requires one or two control points");
                }
                parsed.push(ShapeCommand::Curve { endpoint, controls });
            }
            "smooth" => {
                let with = values
                    .iter()
                    .position(|value| value.eq_ignore_ascii_case("with"));
                let endpoint_end = with.unwrap_or(values.len());
                let endpoint = parse_shape_endpoint(&values[1..endpoint_end], units)?;
                let control = with
                    .map(|index| {
                        parse_shape_control(
                            &values[index + 1..],
                            units,
                            matches!(endpoint, ShapeEndpoint::By(_)),
                        )
                    })
                    .transpose()?;
                parsed.push(ShapeCommand::Smooth { endpoint, control });
            }
            "arc" => parsed.push(parse_shape_arc(&values[1..], units)?),
            _ => return Err("unsupported shape() command"),
        }
    }
    Ok(ShapeFunction {
        start,
        commands: parsed,
        fill_rule,
    })
}

fn parse_shape_endpoint(values: &[String], units: Units) -> Result<ShapeEndpoint, &'static str> {
    let Some(mode) = values.first().map(|value| value.to_ascii_lowercase()) else {
        return Err("shape command requires to or by");
    };
    match mode.as_str() {
        "to" => Ok(ShapeEndpoint::To(shape_position(&values[1..], units)?)),
        "by" => {
            let [x, y] = &values[1..] else {
                return Err("relative shape position requires two coordinates");
            };
            Ok(ShapeEndpoint::By(ShapePoint {
                x: length_percentage(x, units, true)?,
                y: length_percentage(y, units, true)?,
            }))
        }
        _ => Err("shape command requires to or by"),
    }
}

fn parse_shape_control(
    values: &[String],
    units: Units,
    relative_only: bool,
) -> Result<ShapeControl, &'static str> {
    let from = values
        .iter()
        .position(|value| value.eq_ignore_ascii_case("from"));
    let (coordinates, origin) = if let Some(from) = from {
        let origin = match values.get(from + 1).map(|value| value.to_ascii_lowercase()) {
            Some(value) if value == "start" => ControlOrigin::Start,
            Some(value) if value == "end" => ControlOrigin::End,
            Some(value) if value == "origin" => ControlOrigin::Origin,
            _ => return Err("control point from requires start, end, or origin"),
        };
        if from + 2 != values.len() {
            return Err("invalid control point reference");
        }
        (&values[..from], origin)
    } else {
        (values, ControlOrigin::Start)
    };
    if from.is_some() || relative_only {
        let [x, y] = coordinates else {
            return Err("relative control point requires two coordinates");
        };
        Ok(ShapeControl::Relative(
            ShapePoint {
                x: length_percentage(x, units, true)?,
                y: length_percentage(y, units, true)?,
            },
            origin,
        ))
    } else {
        Ok(ShapeControl::Absolute(shape_position(coordinates, units)?))
    }
}

fn split_shape_controls(values: &[String]) -> Result<Vec<&[String]>, &'static str> {
    let slash = values.iter().position(|value| value == "/");
    match slash {
        Some(index) if index > 0 && index + 1 < values.len() => {
            Ok(vec![&values[..index], &values[index + 1..]])
        }
        Some(_) => Err("invalid curve control point separator"),
        None if !values.is_empty() => Ok(vec![values]),
        None => Err("curve command requires control points"),
    }
}

fn shape_axis_value(
    value: &str,
    horizontal: bool,
    units: Units,
    relative: bool,
) -> Result<LengthPercentage, &'static str> {
    let keyword = match (horizontal, value.to_ascii_lowercase().as_str()) {
        (true, "left" | "x-start") | (false, "top" | "y-start") => Some(0.0),
        (_, "center") => Some(0.5),
        (true, "right" | "x-end") | (false, "bottom" | "y-end") => Some(1.0),
        _ => None,
    };
    if let Some(value) = keyword {
        if relative {
            return Err("relative hline/vline coordinates cannot use position keywords");
        }
        return Ok(LengthPercentage::Percentage(value));
    }
    length_percentage(value, units, true)
}

fn parse_shape_arc(values: &[String], units: Units) -> Result<ShapeCommand, &'static str> {
    let parameter = values
        .iter()
        .position(|value| is_shape_arc_parameter(value))
        .unwrap_or(values.len());
    let endpoint = parse_shape_endpoint(&values[..parameter], units)?;
    let mut radii = None;
    let mut direction_agnostic_radius = false;
    let mut clockwise = false;
    let mut sweep_seen = false;
    let mut large = false;
    let mut size_seen = false;
    let mut rotation = 0.0;
    let mut rotation_seen = false;
    let mut index = parameter;
    while index < values.len() {
        let value = values[index].to_ascii_lowercase();
        match value.as_str() {
            "of" => {
                if radii.is_some() {
                    return Err("duplicate arc radius");
                }
                index += 1;
                let start = index;
                while index < values.len() && !is_shape_arc_parameter(&values[index]) {
                    index += 1;
                }
                match &values[start..index] {
                    [radius] => {
                        let radius = length_percentage(radius, units, true)?;
                        radii = Some([radius, radius]);
                        direction_agnostic_radius = true;
                    }
                    [x, y] => {
                        radii = Some([
                            length_percentage(x, units, true)?,
                            length_percentage(y, units, true)?,
                        ]);
                    }
                    _ => return Err("arc of requires one or two radii"),
                }
            }
            "cw" | "ccw" => {
                if sweep_seen {
                    return Err("duplicate arc sweep direction");
                }
                clockwise = value == "cw";
                sweep_seen = true;
                index += 1;
            }
            "large" | "small" => {
                if size_seen {
                    return Err("duplicate arc size");
                }
                large = value == "large";
                size_seen = true;
                index += 1;
            }
            "rotate" => {
                if rotation_seen || index + 1 >= values.len() {
                    return Err("rotate requires one angle");
                }
                rotation = parse_shape_angle(&values[index + 1])?;
                rotation_seen = true;
                index += 2;
            }
            _ => return Err("invalid arc parameter"),
        }
    }
    Ok(ShapeCommand::Arc {
        endpoint,
        radii: radii.ok_or("arc command requires an of radius")?,
        direction_agnostic_radius,
        clockwise,
        large,
        rotation,
    })
}

fn is_shape_arc_parameter(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "of" | "cw" | "ccw" | "large" | "small" | "rotate"
    )
}

fn parse_shape_angle(value: &str) -> Result<f32, &'static str> {
    let mut parser = Parser::new(value);
    let angle = match parser.next().map_err(|_| "invalid shape angle")? {
        cssparser::Token::Number { value, .. } if *value == 0.0 => 0.0,
        cssparser::Token::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str()
        {
            "deg" => *value,
            "grad" => *value * 0.9,
            "rad" => value.to_degrees(),
            "turn" => *value * 360.0,
            _ => return Err("invalid shape angle unit"),
        },
        _ => return Err("invalid shape angle"),
    };
    parser
        .expect_exhausted()
        .map_err(|_| "invalid shape angle")?;
    angle
        .is_finite()
        .then_some(angle)
        .ok_or("invalid shape angle")
}

#[derive(Clone, Copy)]
enum PathToken {
    Command(char),
    Number(f32),
}

fn parse_path(value: &str) -> Result<(Vec<Vec<Vec2>>, FillRule), &'static str> {
    let mut values = crate::layout_style::components(value)?;
    values.retain(|value| value != ",");
    let fill_rule = if values
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("evenodd"))
    {
        values.remove(0);
        FillRule::Evenodd
    } else if values
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("nonzero"))
    {
        values.remove(0);
        FillRule::Nonzero
    } else {
        FillRule::Nonzero
    };
    if values.len() != 1 {
        return Err("path() requires one quoted path string");
    }
    let mut parser = Parser::new(&values[0]);
    let data = match parser.next().map_err(|_| "invalid path() string")? {
        cssparser::Token::QuotedString(value) => value.to_string(),
        _ => return Err("path() requires one quoted path string"),
    };
    parser
        .expect_exhausted()
        .map_err(|_| "invalid path() string")?;
    Ok((parse_svg_path(&data)?, fill_rule))
}

fn parse_svg_path(data: &str) -> Result<Vec<Vec<Vec2>>, &'static str> {
    let tokens = path_tokens(data)?;
    let mut index = 0;
    let mut command = None;
    let mut current = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut cubic_control = None;
    let mut quadratic_control = None;
    let mut moved = false;
    while index < tokens.len() {
        if let PathToken::Command(next) = tokens[index] {
            command = Some(next);
            index += 1;
        }
        let command_value = command.ok_or("path data must begin with a command")?;
        if matches!(command_value, 'Z' | 'z') {
            if !moved {
                return Err("path close command has no subpath");
            }
            if points
                .last()
                .is_some_and(|point: &Vec2| (*point - start).length() > 0.001)
            {
                points.push(start);
            }
            current = start;
            command = None;
            cubic_control = None;
            quadratic_control = None;
            continue;
        }
        let arity = match command_value.to_ascii_lowercase() {
            'm' | 'l' | 't' => 2,
            'h' | 'v' => 1,
            'c' => 6,
            's' | 'q' => 4,
            'a' => 7,
            _ => return Err("unsupported SVG path command"),
        };
        if index + arity > tokens.len()
            || tokens[index..index + arity]
                .iter()
                .any(|token| matches!(token, PathToken::Command(_)))
        {
            return Err("incomplete SVG path command");
        }
        let mut numbers = Vec::with_capacity(arity);
        for token in &tokens[index..index + arity] {
            let PathToken::Number(value) = token else {
                unreachable!()
            };
            numbers.push(*value);
        }
        index += arity;
        let relative = command_value.is_ascii_lowercase();
        let point = |x: f32, y: f32| {
            if relative {
                current + vec2(x, y)
            } else {
                vec2(x, y)
            }
        };
        match command_value.to_ascii_lowercase() {
            'm' => {
                if !points.is_empty() {
                    contours.push(std::mem::take(&mut points));
                }
                current = point(numbers[0], numbers[1]);
                start = current;
                points.push(current);
                moved = true;
                command = Some(if relative { 'l' } else { 'L' });
                cubic_control = None;
                quadratic_control = None;
            }
            'l' => {
                current = point(numbers[0], numbers[1]);
                points.push(current);
                cubic_control = None;
                quadratic_control = None;
            }
            'h' => {
                current.x = if relative {
                    current.x + numbers[0]
                } else {
                    numbers[0]
                };
                points.push(current);
                cubic_control = None;
                quadratic_control = None;
            }
            'v' => {
                current.y = if relative {
                    current.y + numbers[0]
                } else {
                    numbers[0]
                };
                points.push(current);
                cubic_control = None;
                quadratic_control = None;
            }
            'c' => {
                let first = point(numbers[0], numbers[1]);
                let second = point(numbers[2], numbers[3]);
                let end = point(numbers[4], numbers[5]);
                append_cubic(&mut points, current, first, second, end);
                current = end;
                cubic_control = Some(second);
                quadratic_control = None;
            }
            's' => {
                let first = cubic_control.map_or(current, |control: Vec2| current * 2.0 - control);
                let second = point(numbers[0], numbers[1]);
                let end = point(numbers[2], numbers[3]);
                append_cubic(&mut points, current, first, second, end);
                current = end;
                cubic_control = Some(second);
                quadratic_control = None;
            }
            'q' => {
                let control = point(numbers[0], numbers[1]);
                let end = point(numbers[2], numbers[3]);
                append_quadratic(&mut points, current, control, end);
                current = end;
                quadratic_control = Some(control);
                cubic_control = None;
            }
            't' => {
                let control =
                    quadratic_control.map_or(current, |control: Vec2| current * 2.0 - control);
                let end = point(numbers[0], numbers[1]);
                append_quadratic(&mut points, current, control, end);
                current = end;
                quadratic_control = Some(control);
                cubic_control = None;
            }
            'a' => {
                let end = point(numbers[5], numbers[6]);
                append_arc(
                    &mut points,
                    current,
                    end,
                    ArcParameters {
                        radius_x: numbers[0],
                        radius_y: numbers[1],
                        rotation: numbers[2],
                        large_arc: numbers[3] != 0.0,
                        sweep: numbers[4] != 0.0,
                    },
                );
                current = end;
                cubic_control = None;
                quadratic_control = None;
            }
            _ => unreachable!(),
        }
        if !points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite())
        {
            return Err("path coordinates must be finite");
        }
    }
    if !points.is_empty() {
        contours.push(points);
    }
    if contours.is_empty() || contours.iter().any(|points| points.len() < 3) {
        return Err("path() requires a subpath with at least three points");
    }
    for points in &mut contours {
        if points
            .last()
            .is_some_and(|point| (*point - points[0]).length() <= 0.001)
        {
            points.pop();
        }
    }
    Ok(contours)
}

fn path_tokens(data: &str) -> Result<Vec<PathToken>, &'static str> {
    let bytes = data.as_bytes();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() || bytes[index] == b',' {
            index += 1;
            continue;
        }
        if bytes[index].is_ascii_alphabetic() {
            tokens.push(PathToken::Command(bytes[index] as char));
            index += 1;
            continue;
        }
        let start = index;
        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let mut digits = 0;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            digits += 1;
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                digits += 1;
                index += 1;
            }
        }
        if digits == 0 {
            return Err("invalid SVG path number");
        }
        if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
            let exponent = index;
            index += 1;
            if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            let exponent_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if exponent_start == index {
                index = exponent;
            }
        }
        let value = data[start..index]
            .parse::<f32>()
            .map_err(|_| "invalid SVG path number")?;
        if !value.is_finite() {
            return Err("path coordinates must be finite");
        }
        tokens.push(PathToken::Number(value));
    }
    Ok(tokens)
}

pub(super) fn append_cubic(
    points: &mut Vec<Vec2>,
    start: Vec2,
    first: Vec2,
    second: Vec2,
    end: Vec2,
) {
    for step in 1..=12 {
        let t = step as f32 / 12.0;
        let inverse = 1.0 - t;
        points.push(
            start * inverse.powi(3)
                + first * (3.0 * inverse.powi(2) * t)
                + second * (3.0 * inverse * t.powi(2))
                + end * t.powi(3),
        );
    }
}

pub(super) fn append_quadratic(points: &mut Vec<Vec2>, start: Vec2, control: Vec2, end: Vec2) {
    for step in 1..=12 {
        let t = step as f32 / 12.0;
        let inverse = 1.0 - t;
        points.push(start * inverse.powi(2) + control * (2.0 * inverse * t) + end * t.powi(2));
    }
}

#[derive(Clone, Copy)]
pub(super) struct ArcParameters {
    pub(super) radius_x: f32,
    pub(super) radius_y: f32,
    pub(super) rotation: f32,
    pub(super) large_arc: bool,
    pub(super) sweep: bool,
}

pub(super) fn append_arc(
    points: &mut Vec<Vec2>,
    start: Vec2,
    end: Vec2,
    parameters: ArcParameters,
) {
    let ArcParameters {
        radius_x,
        radius_y,
        rotation,
        large_arc,
        sweep,
    } = parameters;
    let (mut rx, mut ry) = (radius_x.abs(), radius_y.abs());
    if rx == 0.0 || ry == 0.0 || (start - end).length() <= f32::EPSILON {
        points.push(end);
        return;
    }
    let phi = rotation.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let half = (start - end) * 0.5;
    let x_prime = cos_phi * half.x + sin_phi * half.y;
    let y_prime = -sin_phi * half.x + cos_phi * half.y;
    let scale = (x_prime.powi(2) / rx.powi(2) + y_prime.powi(2) / ry.powi(2)).sqrt();
    if scale > 1.0 {
        rx *= scale;
        ry *= scale;
    }
    let numerator =
        (rx.powi(2) * ry.powi(2) - rx.powi(2) * y_prime.powi(2) - ry.powi(2) * x_prime.powi(2))
            .max(0.0);
    let denominator = rx.powi(2) * y_prime.powi(2) + ry.powi(2) * x_prime.powi(2);
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coefficient = sign * (numerator / denominator.max(f32::EPSILON)).sqrt();
    let center_x_prime = coefficient * rx * y_prime / ry;
    let center_y_prime = coefficient * -ry * x_prime / rx;
    let center = vec2(
        cos_phi * center_x_prime - sin_phi * center_y_prime + (start.x + end.x) * 0.5,
        sin_phi * center_x_prime + cos_phi * center_y_prime + (start.y + end.y) * 0.5,
    );
    let first = vec2(
        (x_prime - center_x_prime) / rx,
        (y_prime - center_y_prime) / ry,
    );
    let second = vec2(
        (-x_prime - center_x_prime) / rx,
        (-y_prime - center_y_prime) / ry,
    );
    let start_angle = first.y.atan2(first.x);
    let mut sweep_angle = cross_vec(first, second).atan2(first.dot(second));
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    }
    let steps = (sweep_angle.abs() / (std::f32::consts::PI / 24.0))
        .ceil()
        .clamp(1.0, 96.0) as usize;
    for step in 1..=steps {
        let angle = start_angle + sweep_angle * step as f32 / steps as f32;
        let (sin, cos) = angle.sin_cos();
        points.push(vec2(
            center.x + cos_phi * rx * cos - sin_phi * ry * sin,
            center.y + sin_phi * rx * cos + cos_phi * ry * sin,
        ));
    }
}

fn shape_radius(value: &str, units: Units, negative: bool) -> Result<ShapeRadius, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "closest-side" => Ok(ShapeRadius::ClosestSide),
        "farthest-side" => Ok(ShapeRadius::FarthestSide),
        _ => Ok(ShapeRadius::LengthPercentage(length_percentage(
            value, units, negative,
        )?)),
    }
}

fn shape_position(values: &[String], units: Units) -> Result<ShapePosition, &'static str> {
    if values.is_empty() {
        return Err("position after at cannot be empty");
    }
    if values.len() > 4 {
        return Err("unsupported basic shape position");
    }
    if values.len() == 1 {
        return Ok(match values[0].to_ascii_lowercase().as_str() {
            "left" => ShapePosition {
                x: LengthPercentage::Percentage(0.0),
                ..ShapePosition::default()
            },
            "right" => ShapePosition {
                x: LengthPercentage::Percentage(1.0),
                ..ShapePosition::default()
            },
            "top" => ShapePosition {
                y: LengthPercentage::Percentage(0.0),
                ..ShapePosition::default()
            },
            "bottom" => ShapePosition {
                y: LengthPercentage::Percentage(1.0),
                ..ShapePosition::default()
            },
            "center" => ShapePosition::default(),
            _ => ShapePosition {
                x: length_percentage(&values[0], units, true)?,
                ..ShapePosition::default()
            },
        });
    }

    let mut x = None;
    let mut y = None;
    let mut x_from_end = false;
    let mut y_from_end = false;
    let mut index = 0;
    while index < values.len() {
        let value = values[index].to_ascii_lowercase();
        let edge_axis = match value.as_str() {
            "left" => Some((true, false)),
            "right" => Some((true, true)),
            "top" => Some((false, false)),
            "bottom" => Some((false, true)),
            _ => None,
        };
        if values.len() > 2
            && let Some((horizontal, from_end)) = edge_axis
            && values.get(index + 1).is_some_and(|next| {
                !matches!(
                    next.to_ascii_lowercase().as_str(),
                    "left" | "right" | "top" | "bottom" | "center"
                )
            })
        {
            let offset = length_percentage(&values[index + 1], units, true)?;
            let axis = if horizontal { &mut x } else { &mut y };
            if axis.replace(offset).is_some() {
                return Err("duplicate axis in shape position");
            }
            if horizontal {
                x_from_end = from_end;
            } else {
                y_from_end = from_end;
            }
            index += 2;
            continue;
        }
        match value.as_str() {
            "left" | "right" => {
                let axis = if value == "left" { 0.0 } else { 1.0 };
                if x.replace(LengthPercentage::Percentage(axis)).is_some() {
                    return Err("duplicate horizontal axis in shape position");
                }
            }
            "top" | "bottom" => {
                let axis = if value == "top" { 0.0 } else { 1.0 };
                if y.replace(LengthPercentage::Percentage(axis)).is_some() {
                    return Err("duplicate vertical axis in shape position");
                }
            }
            "center" => {
                if x.is_none() {
                    x = Some(LengthPercentage::Percentage(0.5));
                } else if y.is_none() {
                    y = Some(LengthPercentage::Percentage(0.5));
                } else {
                    return Err("too many coordinates in shape position");
                }
            }
            _ => {
                let coordinate = length_percentage(&values[index], units, true)?;
                if x.is_none() {
                    x = Some(coordinate);
                } else if y.is_none() {
                    y = Some(coordinate);
                } else {
                    return Err("too many coordinates in shape position");
                }
            }
        }
        index += 1;
    }
    Ok(ShapePosition {
        x: x.unwrap_or(LengthPercentage::Percentage(0.5)),
        y: y.unwrap_or(LengthPercentage::Percentage(0.5)),
        x_from_end,
        y_from_end,
    })
}

fn rect_edge(value: &str, edge: usize, units: Units) -> Result<LengthPercentage, &'static str> {
    if value.eq_ignore_ascii_case("auto") {
        return Ok(LengthPercentage::Percentage(if matches!(edge, 1 | 2) {
            1.0
        } else {
            0.0
        }));
    }
    length_percentage(value, units, true)
}
