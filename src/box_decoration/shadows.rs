use super::*;

pub(super) fn parse_shadows(value: &str, units: Units) -> Result<Vec<Shadow>, &'static str> {
    if value.trim().eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    split_commas(value)
        .into_iter()
        .map(|value| parse_shadow(value, units))
        .collect()
}

pub(super) fn set_shadows(style: &mut DecorationStyle, shadows: Vec<Shadow>) {
    if shadows.is_empty() {
        style.shadow_offsets = smallvec::smallvec![None];
        style.shadow_blurs = smallvec::smallvec![0.0];
        style.shadow_spreads = smallvec::smallvec![0.0];
        style.shadow_colors = smallvec::smallvec![None];
        style.shadow_positions = smallvec::smallvec![ShadowPosition::Outset];
    } else {
        style.shadow_offsets = shadows.iter().map(|shadow| shadow.offset).collect();
        style.shadow_blurs = shadows.iter().map(|shadow| shadow.blur).collect();
        style.shadow_spreads = shadows.iter().map(|shadow| shadow.spread).collect();
        style.shadow_colors = shadows.iter().map(|shadow| shadow.color).collect();
        style.shadow_positions = shadows.iter().map(|shadow| shadow.position).collect();
    }
}

pub(super) fn copy_shadows(out: &mut DecorationStyle, source: &DecorationStyle) {
    out.shadow_offsets.clone_from(&source.shadow_offsets);
    out.shadow_blurs.clone_from(&source.shadow_blurs);
    out.shadow_spreads.clone_from(&source.shadow_spreads);
    out.shadow_colors.clone_from(&source.shadow_colors);
    out.shadow_positions.clone_from(&source.shadow_positions);
}

pub(super) fn apply_shadow_longhand(
    name: &str,
    value: &str,
    style: &mut DecorationStyle,
    units: Units,
) -> Result<(), &'static str> {
    let values = split_commas(value);
    if values.is_empty() || values.iter().any(|value| value.is_empty()) {
        return Err("empty shadow list");
    }
    match name {
        "box-shadow-color" => {
            style.shadow_colors = values
                .iter()
                .map(|value| color_value(value))
                .collect::<Result<_, _>>()?;
        }
        "box-shadow-offset" => {
            style.shadow_offsets = values
                .iter()
                .map(|value| {
                    let parts = crate::layout_style::components(value)?;
                    if parts.len() == 1 && parts[0].eq_ignore_ascii_case("none") {
                        return Ok(None);
                    }
                    if parts.is_empty() || parts.len() > 2 {
                        return Err("invalid shadow offset");
                    }
                    let x = shadow_length(&parts[0], units, true)?;
                    let y = if let Some(second) = parts.get(1) {
                        shadow_length(second, units, true)?
                    } else {
                        x
                    };
                    Ok(Some(vec2(x, y)))
                })
                .collect::<Result<_, _>>()?;
        }
        "box-shadow-blur" => {
            style.shadow_blurs = values
                .iter()
                .map(|value| shadow_length(value, units, false))
                .collect::<Result<_, _>>()?;
        }
        "box-shadow-spread" => {
            style.shadow_spreads = values
                .iter()
                .map(|value| shadow_length(value, units, true))
                .collect::<Result<_, _>>()?;
        }
        _ => {
            style.shadow_positions = values
                .iter()
                .map(|value| match value.to_ascii_lowercase().as_str() {
                    "inset" => Ok(ShadowPosition::Inset),
                    "outset" => Ok(ShadowPosition::Outset),
                    _ => Err("invalid shadow position"),
                })
                .collect::<Result<_, _>>()?;
        }
    }
    Ok(())
}

fn shadow_length(value: &str, units: Units, negative: bool) -> Result<f32, &'static str> {
    if let Some(handle) = crate::math::length(value, units, negative, false)? {
        return Ok(crate::math::resolve(handle, 0.0));
    }
    let (value, percentage) = crate::layout_style::length_value(value, units, negative)?;
    if percentage {
        return Err("box-shadow lengths cannot be percentages");
    }
    Ok(value)
}

fn shadows(style: &DecorationStyle) -> Vec<Shadow> {
    let count = style.shadow_offsets.len();
    (0..count)
        .map(|index| Shadow {
            offset: style.shadow_offsets[index],
            blur: coordinated(&style.shadow_blurs, index),
            spread: coordinated(&style.shadow_spreads, index),
            color: coordinated(&style.shadow_colors, index),
            position: coordinated(&style.shadow_positions, index),
        })
        .collect()
}

fn coordinated<T: Copy>(values: &[T], index: usize) -> T {
    values[index % values.len()]
}

fn parse_shadow(value: &str, units: Units) -> Result<Shadow, &'static str> {
    let values = crate::layout_style::components(value)?;
    let mut color = None;
    let mut color_seen = false;
    let mut position = ShadowPosition::Outset;
    let mut position_seen = false;
    let mut offset: Option<Option<Vec2>> = None;
    let mut lengths = Vec::new();
    for value in &values {
        match value.to_ascii_lowercase().as_str() {
            "inset" | "outset" => {
                if position_seen {
                    return Err("duplicate shadow position");
                }
                position = if value.eq_ignore_ascii_case("inset") {
                    ShadowPosition::Inset
                } else {
                    ShadowPosition::Outset
                };
                position_seen = true;
            }
            "none" => {
                if offset.is_some() || !lengths.is_empty() {
                    return Err("invalid none shadow offset");
                }
                offset = Some(None);
            }
            _ => {
                if let Ok(parsed) = color_value(value) {
                    if color_seen {
                        return Err("duplicate shadow color");
                    }
                    color = parsed;
                    color_seen = true;
                } else {
                    let length = shadow_length(value, units, true)?;
                    lengths.push(length);
                }
            }
        }
    }
    let no_offset = offset == Some(None);
    if lengths.len() > if no_offset { 2 } else { 4 } {
        return Err("invalid box-shadow");
    }
    let shadow_offset = if no_offset {
        None
    } else if lengths.len() >= 2 {
        Some(vec2(lengths[0], lengths[1]))
    } else {
        return Err("box-shadow requires two offsets");
    };
    let blur = lengths
        .get(if no_offset { 0 } else { 2 })
        .copied()
        .unwrap_or(0.0);
    if blur < 0.0 {
        return Err("negative shadow blur");
    }
    let spread = lengths
        .get(if no_offset { 1 } else { 3 })
        .copied()
        .unwrap_or(0.0);
    Ok(Shadow {
        offset: shadow_offset,
        blur,
        spread,
        color: if !color_seen && no_offset {
            Some(Color32::TRANSPARENT)
        } else {
            color
        },
        position,
    })
}

pub(super) fn color_value(value: &str) -> Result<Option<Color32>, &'static str> {
    if value.eq_ignore_ascii_case("currentcolor") {
        return Ok(None);
    }
    crate::css::parse_color(&mut Parser::new(value))
        .map(Some)
        .map_err(|_| "invalid shadow color")
}

pub(super) fn split_commas(value: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    let mut quote = None;
    for (index, character) in value.char_indices() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(value[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    result.push(value[start..].trim());
    result
}

pub(crate) fn shadow_shapes(
    rect: Rect,
    style: &BoxStyle,
    current: Color32,
    position: ShadowPosition,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
) -> Vec<Shape> {
    let radii = resolved_radii(style, rect);
    let border_widths = style.layout.border.map(|value| {
        let value = value.into_raw();
        match value.tag() {
            taffy::CompactLength::CALC_TAG => crate::math::resolve(value.calc_value(), 0.0),
            taffy::CompactLength::LENGTH_TAG => value.value(),
            _ => 0.0,
        }
    });
    let widths = [
        border_widths.top,
        border_widths.right,
        border_widths.bottom,
        border_widths.left,
    ];
    let mut shapes = Vec::new();
    for shadow in shadows(&style.decorations).into_iter().rev() {
        if shadow.position != position {
            continue;
        }
        let Some(offset) = shadow.offset else {
            continue;
        };
        let color = shadow.color.unwrap_or(current);
        if position == ShadowPosition::Inset {
            shapes.extend(inset_shadow_shapes(InsetShadowInput {
                rect,
                style,
                outer_radii: radii,
                shadow: &shadow,
                color,
                stroke,
                padding,
                margin,
                widths,
            }));
            continue;
        }

        let blur = shadow.blur;
        let count = if blur > 0.0 { 96 } else { 1 };
        let sigma = blur * 0.5;
        for step in 0..count {
            let (distance, alpha) = if count == 1 {
                (shadow.spread, f32::from(color.a()))
            } else {
                let outer_distance = shadow.spread + sigma * 3.0;
                let inner_distance = shadow.spread - sigma * 3.0;
                let previous_distance = outer_distance
                    + (inner_distance - outer_distance) * ((step as f32 - 0.5) / count as f32);
                let distance = outer_distance
                    + (inner_distance - outer_distance) * ((step as f32 + 0.5) / count as f32);
                let previous_opacity = if step == 0 {
                    0.0
                } else {
                    shadow_opacity(previous_distance, sigma)
                };
                let opacity = shadow_opacity(distance, sigma);
                (
                    distance,
                    (opacity - previous_opacity) / (1.0 - previous_opacity) * f32::from(color.a()),
                )
            };
            let target = rect.translate(offset).expand(distance);
            if !target.is_positive() || alpha <= 0.0 {
                continue;
            }
            let target_radii =
                outset_radii(rect, radii, [distance; 4]).map(|radius| radius.max(Vec2::ZERO));
            let paint = shadow_color(color, alpha);
            if let Some(border_shape) = &style.decorations.border_shape {
                let fill_rule = border_shape.outer_fill_rule();
                let source_paths = border_shape.outer_paths(rect, stroke, padding, margin, widths);
                let paths = offset_filled_paths(&source_paths, fill_rule, distance)
                    .into_iter()
                    .map(|path| path.into_iter().map(|point| point + offset).collect())
                    .collect();
                shapes.push(filled_paths_difference(
                    paths,
                    fill_rule,
                    source_paths,
                    fill_rule,
                    paint,
                ));
            } else {
                let points = rounded_path(target, target_radii, style.decorations.corner_shapes);
                let source = rounded_path(rect, radii, style.decorations.corner_shapes);
                shapes.push(filled_paths_difference(
                    vec![points],
                    FillRule::Nonzero,
                    vec![source],
                    FillRule::Nonzero,
                    paint,
                ));
            }
        }
    }
    shapes
}

pub(crate) fn inset_shadow_extent(rect: Rect, style: &BoxStyle) -> f32 {
    shadows(&style.decorations)
        .into_iter()
        .filter(|shadow| shadow.position == ShadowPosition::Inset)
        .filter_map(|shadow| {
            let offset = shadow.offset?;
            Some(
                (shadow.spread + shadow.blur * 1.5 + offset.x.abs().max(offset.y.abs()))
                    .max(0.0)
                    .min(rect.width().min(rect.height()) * 0.5),
            )
        })
        .fold(0.0, f32::max)
}

struct InsetShadowInput<'a> {
    rect: Rect,
    style: &'a BoxStyle,
    outer_radii: [Vec2; 4],
    shadow: &'a Shadow,
    color: Color32,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    widths: [f32; 4],
}

fn inset_shadow_shapes(input: InsetShadowInput<'_>) -> Vec<Shape> {
    use egui::epaint::{Mesh, Vertex};

    let InsetShadowInput {
        rect,
        style,
        outer_radii,
        shadow,
        color,
        stroke,
        padding,
        margin,
        widths,
    } = input;
    let Some(offset) = shadow.offset else {
        return Vec::new();
    };
    let blur = shadow.blur;
    let thickness = (shadow.spread + blur * 1.5 + offset.x.abs().max(offset.y.abs()))
        .max(0.0)
        .min(rect.width().min(rect.height()) * 0.5);
    if thickness <= 0.0 {
        return Vec::new();
    }
    let count = if blur > 0.0 {
        64
    } else {
        thickness.ceil().max(1.0) as usize
    };
    let (base_paths, fill_rule) = if let Some(border_shape) = &style.decorations.border_shape {
        match border_shape.inner_paths(rect, stroke, padding, margin, widths) {
            Some(paths) => (
                paths,
                border_shape.inner_fill_rule().unwrap_or(FillRule::Nonzero),
            ),
            None => (
                border_shape.outer_paths(rect, stroke, padding, margin, widths),
                border_shape.outer_fill_rule(),
            ),
        }
    } else {
        let padding_box = crate::nonnegative_rect(rect - stroke);
        let radii = [
            vec2(
                (outer_radii[0].x - stroke.left).max(0.0),
                (outer_radii[0].y - stroke.top).max(0.0),
            ),
            vec2(
                (outer_radii[1].x - stroke.right).max(0.0),
                (outer_radii[1].y - stroke.top).max(0.0),
            ),
            vec2(
                (outer_radii[2].x - stroke.right).max(0.0),
                (outer_radii[2].y - stroke.bottom).max(0.0),
            ),
            vec2(
                (outer_radii[3].x - stroke.left).max(0.0),
                (outer_radii[3].y - stroke.bottom).max(0.0),
            ),
        ];
        (
            vec![rounded_path(
                padding_box,
                radii,
                style.decorations.corner_shapes,
            )],
            FillRule::Nonzero,
        )
    };
    let base_paths = base_paths
        .into_iter()
        .filter(|path| path.len() >= 3)
        .collect::<Vec<_>>();
    if base_paths.is_empty() {
        return Vec::new();
    }
    let directions = (0..base_paths.len())
        .map(|index| contour_expansion_direction(&base_paths, index, fill_rule))
        .collect::<Vec<_>>();
    let sigma = blur * 0.5;
    let mut mesh = Mesh::default();
    for (base, contour_direction) in base_paths.iter().zip(directions) {
        for ring in 0..count {
            let outer_inset = thickness * ring as f32 / count as f32;
            let inner_inset = thickness * (ring + 1) as f32 / count as f32;
            let outer_points = offset_polygon(base, -contour_direction * outer_inset);
            let inner_points = offset_polygon(base, -contour_direction * inner_inset);
            if outer_points.len() != inner_points.len() || outer_points.len() < 3 {
                break;
            }
            for index in 0..outer_points.len() {
                let next = (index + 1) % outer_points.len();
                let direction = base[next] - base[index];
                let normal = outward_normal(base, index, direction) * contour_direction;
                let offset = offset.dot(normal) - shadow.spread;
                let outer_alpha = if blur == 0.0 {
                    if outer_inset + offset < 0.0 {
                        f32::from(color.a())
                    } else {
                        0.0
                    }
                } else {
                    shadow_opacity(outer_inset + offset, sigma) * f32::from(color.a())
                };
                let inner_alpha = if blur == 0.0 {
                    if inner_inset + offset < 0.0 {
                        f32::from(color.a())
                    } else {
                        0.0
                    }
                } else {
                    shadow_opacity(inner_inset + offset, sigma) * f32::from(color.a())
                };
                let first = mesh.vertices.len() as u32;
                mesh.vertices.extend([
                    Vertex {
                        pos: outer_points[index],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, outer_alpha),
                    },
                    Vertex {
                        pos: outer_points[next],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, outer_alpha),
                    },
                    Vertex {
                        pos: inner_points[next],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, inner_alpha),
                    },
                    Vertex {
                        pos: inner_points[index],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, inner_alpha),
                    },
                ]);
                mesh.add_triangle(first, first + 1, first + 2);
                mesh.add_triangle(first, first + 2, first + 3);
            }
        }
    }
    vec![Shape::Mesh(mesh.into())]
}

pub(super) fn offset_filled_paths(
    paths: &[Vec<Pos2>],
    fill_rule: FillRule,
    distance: f32,
) -> Vec<Vec<Pos2>> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let direction = contour_expansion_direction(paths, index, fill_rule);
            offset_polygon(path, distance * direction)
        })
        .collect()
}

fn contour_expansion_direction(paths: &[Vec<Pos2>], index: usize, fill_rule: FillRule) -> f32 {
    let path = &paths[index];
    if path.len() < 3 {
        return 1.0;
    }
    let Some((edge_index, direction)) = (0..path.len()).find_map(|edge_index| {
        let direction = path[(edge_index + 1) % path.len()] - path[edge_index];
        (direction.length_sq() > 0.000_001).then_some((edge_index, direction))
    }) else {
        return 1.0;
    };
    let extent = path.iter().map(|point| point.to_vec2()).fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(min, max), point| (min.min(point), max.max(point)),
    );
    let epsilon = (extent.1 - extent.0).max_elem() * 0.0001;
    let sample = path[edge_index].lerp(path[(edge_index + 1) % path.len()], 0.5)
        - outward_normal(path, edge_index, direction) * epsilon.max(0.0001);
    let own_winding = path_winding_number(path, sample);
    let other_winding = paths
        .iter()
        .enumerate()
        .filter(|(other_index, _)| *other_index != index)
        .map(|(_, path)| path_winding_number(path, sample));
    match fill_rule {
        FillRule::Evenodd => {
            if other_winding.map(i32::unsigned_abs).sum::<u32>() % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        }
        FillRule::Nonzero => {
            let other_winding = other_winding.sum::<i32>();
            match (other_winding == 0, other_winding + own_winding == 0) {
                (true, false) => 1.0,
                (false, true) => -1.0,
                _ => 0.0,
            }
        }
    }
}

fn path_winding_number(path: &[Pos2], point: Pos2) -> i32 {
    let mut winding = 0;
    for index in 0..path.len() {
        let start = path[index];
        let end = path[(index + 1) % path.len()];
        let cross = cross_vec(end - start, point - start);
        if start.y <= point.y {
            if end.y > point.y && cross > 0.0 {
                winding += 1;
            }
        } else if end.y <= point.y && cross < 0.0 {
            winding -= 1;
        }
    }
    winding
}

fn offset_polygon(points: &[Pos2], amount: f32) -> Vec<Pos2> {
    if points.len() < 3 || amount.abs() < 0.0001 {
        return points.to_vec();
    }
    (0..points.len())
        .map(|index| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let point = points[index];
            let next = points[(index + 1) % points.len()];
            let before = (point - previous).normalized();
            let after = (next - point).normalized();
            let n_before = outward_normal(points, index, before);
            let n_after = outward_normal(points, (index + 1) % points.len(), after);
            let a = point + n_before * amount;
            let b = point + n_after * amount;
            let denominator = cross_vec(before, after);
            if denominator.abs() < 0.0001 {
                point + (n_before + n_after).normalized() * amount
            } else {
                a + before * (cross_vec(b - a, after) / denominator)
            }
        })
        .collect()
}

fn outward_normal(points: &[Pos2], _index: usize, direction: Vec2) -> Vec2 {
    let area = (0..points.len())
        .map(|index| {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>();
    if area >= 0.0 {
        vec2(direction.y, -direction.x).normalized()
    } else {
        vec2(-direction.y, direction.x).normalized()
    }
}

pub(super) fn cross_vec(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn shadow_opacity(distance: f32, sigma: f32) -> f32 {
    if sigma <= 0.0 {
        return if distance < 0.0 { 1.0 } else { 0.0 };
    }
    0.5 * (1.0 - erf(distance / (sigma * std::f32::consts::SQRT_2)))
}

fn erf(value: f32) -> f32 {
    let sign = value.signum();
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let polynomial = (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_8) * t - 0.284_496_72)
        * t
        + 0.254_829_6)
        * t;
    sign * (1.0 - polynomial * (-x * x).exp())
}

fn shadow_color(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        alpha.round().clamp(0.0, 255.0) as u8,
    )
}
