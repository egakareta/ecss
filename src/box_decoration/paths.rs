use super::*;

pub(crate) fn rounded_corner_segments(radii: [Vec2; 4]) -> usize {
    let radius = radii
        .iter()
        .map(|radius| radius.max_elem())
        .fold(0.0_f32, f32::max);
    if !radius.is_finite() {
        return MAX_ROUNDED_CORNER_SEGMENTS;
    }
    if radius <= MAX_ROUNDED_CORNER_ERROR {
        return MIN_ROUNDED_CORNER_SEGMENTS;
    }
    // Bound the circular chord sagitta: r * (1 - cos(pi / (4n))) <= error.
    let half_angle = (1.0 - MAX_ROUNDED_CORNER_ERROR / radius).acos();
    let segments = (std::f32::consts::FRAC_PI_2 / (2.0 * half_angle)).ceil() as usize;
    segments.clamp(MIN_ROUNDED_CORNER_SEGMENTS, MAX_ROUNDED_CORNER_SEGMENTS)
}

pub(crate) fn rounded_path(rect: Rect, radii: [Vec2; 4], shapes: [CornerShape; 4]) -> Vec<Pos2> {
    rounded_path_with_segments(rect, radii, shapes, rounded_corner_segments(radii))
}

pub(crate) fn rounded_path_with_segments(
    rect: Rect,
    radii: [Vec2; 4],
    shapes: [CornerShape; 4],
    segments: usize,
) -> Vec<Pos2> {
    let mut points = Vec::with_capacity((segments + 1) * 4);
    for corner in 0..4 {
        let radius = radii[corner];
        if radius.x <= 0.0 || radius.y <= 0.0 {
            let point = match corner {
                0 => pos2(rect.left(), rect.top()),
                1 => pos2(rect.right(), rect.top()),
                2 => pos2(rect.right(), rect.bottom()),
                _ => pos2(rect.left(), rect.bottom()),
            };
            points.extend(std::iter::repeat_n(point, segments + 1));
            continue;
        }
        for step in 0..=segments {
            let t = step as f32 / segments as f32;
            let k = shapes[corner].parameter();
            let (mut x, mut y) = if k == 0.0 {
                (t, 1.0 - t)
            } else if k.is_sign_positive() && k.is_infinite() {
                if t < 0.5 {
                    (0.0, 1.0 - 2.0 * t)
                } else {
                    (2.0 * t - 1.0, 0.0)
                }
            } else if k.is_sign_negative() && k.is_infinite() {
                if t < 0.5 {
                    (2.0 * t, 1.0)
                } else {
                    (1.0, 2.0 - 2.0 * t)
                }
            } else if k > 0.0 {
                let angle = t * std::f32::consts::FRAC_PI_2;
                let exponent = 1.0 / k;
                let (sin, cos) = angle.sin_cos();
                (
                    1.0 - cos.clamp(0.0, 1.0).powf(exponent),
                    1.0 - sin.clamp(0.0, 1.0).powf(exponent),
                )
            } else {
                let exponent = 1.0 / (2.0 * k.abs());
                (t.powf(exponent), (1.0 - t).powf(exponent))
            };
            if corner == 1 || corner == 3 {
                std::mem::swap(&mut x, &mut y);
            }
            let point = match corner {
                0 => pos2(rect.left() + radius.x * x, rect.top() + radius.y * y),
                1 => pos2(rect.right() - radius.x * x, rect.top() + radius.y * y),
                2 => pos2(rect.right() - radius.x * x, rect.bottom() - radius.y * y),
                _ => pos2(rect.left() + radius.x * x, rect.bottom() - radius.y * y),
            };
            points.push(point);
        }
    }
    points
}

pub(crate) fn filled_path(points: Vec<Pos2>, color: Color32) -> Shape {
    filled_paths(vec![points], FillRule::Nonzero, color)
}

pub(crate) fn filled_paths(paths: Vec<Vec<Pos2>>, fill_rule: FillRule, color: Color32) -> Shape {
    Shape::Vec(
        path_fill_quads(paths, fill_rule)
            .into_iter()
            .map(|quad| Shape::convex_polygon(quad.to_vec(), color, Stroke::NONE))
            .collect(),
    )
}

pub(super) fn filled_paths_difference(
    paths: Vec<Vec<Pos2>>,
    fill_rule: FillRule,
    excluded: Vec<Vec<Pos2>>,
    excluded_fill_rule: FillRule,
    color: Color32,
) -> Shape {
    Shape::Vec(
        path_difference_quads(paths, fill_rule, excluded, excluded_fill_rule)
            .into_iter()
            .map(|quad| Shape::convex_polygon(quad.to_vec(), color, Stroke::NONE))
            .collect(),
    )
}

pub(crate) fn path_clip(paths: Vec<Vec<Pos2>>, fill_rule: FillRule) -> PathClip {
    PathClip {
        quads: path_fill_quads(paths, fill_rule),
    }
}

pub(crate) fn clip_mesh_to_paths(mesh: egui::epaint::Mesh, clip: &PathClip) -> egui::epaint::Mesh {
    let mut output = egui::epaint::Mesh::with_texture(mesh.texture_id);
    output.vertices.reserve(mesh.vertices.len());
    output.indices.reserve(mesh.indices.len());
    let quad_bounds = clip
        .quads
        .iter()
        .map(|quad| {
            quad.iter()
                .skip(1)
                .fold(Rect::from_min_max(quad[0], quad[0]), |bounds, point| {
                    bounds.union(Rect::from_min_max(*point, *point))
                })
        })
        .collect::<Vec<_>>();
    let mut polygon = Vec::with_capacity(8);
    let mut scratch = Vec::with_capacity(8);
    let (triangles, _) = mesh.indices.as_chunks::<3>();
    for triangle in triangles {
        let vertices = [
            mesh.vertices[triangle[0] as usize],
            mesh.vertices[triangle[1] as usize],
            mesh.vertices[triangle[2] as usize],
        ];
        let bounds = vertices.iter().fold(
            Rect::from_min_max(vertices[0].pos, vertices[0].pos),
            |bounds, vertex| bounds.union(Rect::from_min_max(vertex.pos, vertex.pos)),
        );
        // path_fill_quads emits scanline bands in ascending y order.
        let first_quad = quad_bounds.partition_point(|quad| quad.max.y < bounds.min.y);
        for (quad, quad_bounds) in clip.quads[first_quad..]
            .iter()
            .zip(&quad_bounds[first_quad..])
        {
            if quad_bounds.min.y > bounds.max.y {
                break;
            }
            if bounds.max.x < quad_bounds.min.x || bounds.min.x > quad_bounds.max.x {
                continue;
            }
            clip_vertex_polygon(&vertices, quad, &mut polygon, &mut scratch);
            if polygon.len() < 3 {
                continue;
            }
            for index in 1..polygon.len() - 1 {
                let first = output.vertices.len() as u32;
                output
                    .vertices
                    .extend([polygon[0], polygon[index], polygon[index + 1]]);
                output.add_triangle(first, first + 1, first + 2);
            }
        }
    }
    output
}

pub(super) fn clip_vertex_polygon(
    vertices: &[egui::epaint::Vertex; 3],
    clip: &[Pos2; 4],
    polygon: &mut Vec<egui::epaint::Vertex>,
    scratch: &mut Vec<egui::epaint::Vertex>,
) {
    polygon.clear();
    polygon.extend(vertices.iter().cloned());
    let orientation = (0..4)
        .map(|index| {
            let a = clip[index];
            let b = clip[(index + 1) % 4];
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>()
        .signum();
    for edge in 0..4 {
        if polygon.is_empty() {
            break;
        }
        let start = clip[edge];
        let end = clip[(edge + 1) % 4];
        let direction = end - start;
        scratch.clear();
        let mut previous_index = polygon.len() - 1;
        let mut previous_distance =
            cross_vec(direction, polygon[previous_index].pos - start) * orientation;
        for current_index in 0..polygon.len() {
            let previous = &polygon[previous_index];
            let current = &polygon[current_index];
            let current_distance = cross_vec(direction, current.pos - start) * orientation;
            let previous_inside = previous_distance >= -0.001;
            let current_inside = current_distance >= -0.001;
            if previous_inside != current_inside {
                let amount = previous_distance / (previous_distance - current_distance);
                scratch.push(interpolate_vertex(previous, current, amount));
            }
            if current_inside {
                scratch.push(*current);
            }
            previous_index = current_index;
            previous_distance = current_distance;
        }
        std::mem::swap(polygon, scratch);
    }
}

pub(super) fn interpolate_vertex(
    first: &egui::epaint::Vertex,
    second: &egui::epaint::Vertex,
    amount: f32,
) -> egui::epaint::Vertex {
    let first_color = first.color.to_array();
    let second_color = second.color.to_array();
    let color: [u8; 4] = std::array::from_fn(|index| {
        (first_color[index] as f32
            + (second_color[index] as f32 - first_color[index] as f32) * amount)
            .round() as u8
    });
    egui::epaint::Vertex {
        pos: first.pos.lerp(second.pos, amount),
        uv: first.uv.lerp(second.uv, amount),
        color: Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
    }
}

pub(super) fn path_fill_quads(paths: Vec<Vec<Pos2>>, fill_rule: FillRule) -> Vec<[Pos2; 4]> {
    let mut edges = Vec::new();
    let mut levels = Vec::new();
    for points in paths {
        let mut points = points;
        points.dedup_by(|a, b| a.distance(*b) <= 0.001);
        if points.len() > 2 && points[0].distance(*points.last().unwrap()) <= 0.001 {
            points.pop();
        }
        if points.len() < 3 {
            continue;
        }
        levels.extend(points.iter().map(|point| point.y));
        edges.extend(
            (0..points.len()).map(|index| (points[index], points[(index + 1) % points.len()])),
        );
    }
    if edges.is_empty() {
        return Vec::new();
    }
    for first in 0..edges.len() {
        for second in first + 1..edges.len() {
            let (a, b) = edges[first];
            let (c, d) = edges[second];
            let r = b - a;
            let s = d - c;
            let denominator = cross_vec(r, s);
            if denominator.abs() <= 0.001 {
                continue;
            }
            let t = cross_vec(c - a, s) / denominator;
            let u = cross_vec(c - a, r) / denominator;
            if (0.001..0.999).contains(&t) && (0.001..0.999).contains(&u) {
                levels.push(a.y + t * r.y);
            }
        }
    }
    levels.sort_by(f32::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() <= 0.001);

    let mut quads = Vec::new();
    for band in levels.windows(2) {
        let top = band[0];
        let bottom = band[1];
        if bottom - top <= 0.001 {
            continue;
        }
        let middle = (top + bottom) * 0.5;
        let mut crossings: Vec<_> = edges
            .iter()
            .copied()
            .filter(|(a, b)| (a.y < middle && b.y > middle) || (b.y < middle && a.y > middle))
            .map(|(a, b)| {
                let direction = if b.y > a.y { 1_i32 } else { -1_i32 };
                let x = x_at_y(a, b, middle);
                (x, direction, (a, b))
            })
            .collect();
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0_i32;
        let mut parity = false;
        let mut left = None;
        let mut index = 0;
        while index < crossings.len() {
            let group_start = index;
            let x = crossings[index].0;
            index += 1;
            while index < crossings.len() && (crossings[index].0 - x).abs() <= 0.001 {
                index += 1;
            }
            let was_inside = match fill_rule {
                FillRule::Nonzero => winding != 0,
                FillRule::Evenodd => parity,
            };
            let group = &crossings[group_start..index];
            if fill_rule == FillRule::Evenodd {
                if group.len() % 2 == 1 {
                    parity = !parity;
                }
            } else {
                winding += group.iter().map(|crossing| crossing.1).sum::<i32>();
            }
            let is_inside = match fill_rule {
                FillRule::Nonzero => winding != 0,
                FillRule::Evenodd => parity,
            };
            if !was_inside && is_inside {
                left = group.first().map(|crossing| crossing.2);
            } else if was_inside
                && !is_inside
                && let (Some((left_a, left_b)), Some((right_a, right_b))) =
                    (left.take(), group.first().map(|crossing| crossing.2))
            {
                let quad = [
                    pos2(x_at_y(left_a, left_b, top), top),
                    pos2(x_at_y(right_a, right_b, top), top),
                    pos2(x_at_y(right_a, right_b, bottom), bottom),
                    pos2(x_at_y(left_a, left_b, bottom), bottom),
                ];
                if (quad[1] - quad[0]).length_sq() + (quad[2] - quad[3]).length_sq() > 0.0001 {
                    quads.push([quad[0], quad[1], quad[2], quad[3]]);
                }
            }
        }
    }
    quads
}

pub(super) fn path_difference_quads(
    target_paths: Vec<Vec<Pos2>>,
    target_rule: FillRule,
    excluded_paths: Vec<Vec<Pos2>>,
    excluded_rule: FillRule,
) -> Vec<[Pos2; 4]> {
    let mut edges = Vec::new();
    let mut levels = Vec::new();
    for (shape, paths) in [(0, target_paths), (1, excluded_paths)] {
        for mut points in paths {
            points.dedup_by(|a, b| a.distance(*b) <= 0.001);
            if points.len() > 2 && points[0].distance(*points.last().unwrap()) <= 0.001 {
                points.pop();
            }
            if points.len() < 3 {
                continue;
            }
            levels.extend(points.iter().map(|point| point.y));
            edges.extend(
                (0..points.len())
                    .map(|index| (points[index], points[(index + 1) % points.len()], shape)),
            );
        }
    }
    if edges.is_empty() {
        return Vec::new();
    }
    for first in 0..edges.len() {
        for second in first + 1..edges.len() {
            let (a, b, _) = edges[first];
            let (c, d, _) = edges[second];
            let r = b - a;
            let s = d - c;
            let denominator = cross_vec(r, s);
            if denominator.abs() <= 0.001 {
                continue;
            }
            let t = cross_vec(c - a, s) / denominator;
            let u = cross_vec(c - a, r) / denominator;
            if (0.001..0.999).contains(&t) && (0.001..0.999).contains(&u) {
                levels.push(a.y + t * r.y);
            }
        }
    }
    levels.sort_by(f32::total_cmp);
    levels.dedup_by(|a, b| (*a - *b).abs() <= 0.001);

    let rules = [target_rule, excluded_rule];
    let shape_is_inside = |shape: usize, winding: &[i32; 2], parity: &[bool; 2]| match rules[shape]
    {
        FillRule::Nonzero => winding[shape] != 0,
        FillRule::Evenodd => parity[shape],
    };
    let mut quads = Vec::new();
    for band in levels.windows(2) {
        let top = band[0];
        let bottom = band[1];
        if bottom - top <= 0.001 {
            continue;
        }
        let middle = (top + bottom) * 0.5;
        let mut crossings: Vec<_> = edges
            .iter()
            .copied()
            .filter(|(a, b, _)| (a.y < middle && b.y > middle) || (b.y < middle && a.y > middle))
            .map(|(a, b, shape)| {
                let direction = if b.y > a.y { 1_i32 } else { -1_i32 };
                (x_at_y(a, b, middle), direction, (a, b), shape)
            })
            .collect();
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = [0_i32; 2];
        let mut parity = [false; 2];
        let mut left = None;
        let mut index = 0;
        while index < crossings.len() {
            let group_start = index;
            let x = crossings[index].0;
            index += 1;
            while index < crossings.len() && (crossings[index].0 - x).abs() <= 0.001 {
                index += 1;
            }
            let was_inside =
                shape_is_inside(0, &winding, &parity) && !shape_is_inside(1, &winding, &parity);
            let group = &crossings[group_start..index];
            for shape in 0..2 {
                let shape_crossings = group
                    .iter()
                    .filter(|crossing| crossing.3 == shape)
                    .collect::<Vec<_>>();
                if rules[shape] == FillRule::Evenodd {
                    if shape_crossings.len() % 2 == 1 {
                        parity[shape] = !parity[shape];
                    }
                } else {
                    winding[shape] += shape_crossings
                        .iter()
                        .map(|crossing| crossing.1)
                        .sum::<i32>();
                }
            }
            let is_inside =
                shape_is_inside(0, &winding, &parity) && !shape_is_inside(1, &winding, &parity);
            if !was_inside && is_inside {
                left = group.first().map(|crossing| crossing.2);
            } else if was_inside
                && !is_inside
                && let (Some((left_a, left_b)), Some((right_a, right_b))) =
                    (left.take(), group.first().map(|crossing| crossing.2))
            {
                let quad = [
                    pos2(x_at_y(left_a, left_b, top), top),
                    pos2(x_at_y(right_a, right_b, top), top),
                    pos2(x_at_y(right_a, right_b, bottom), bottom),
                    pos2(x_at_y(left_a, left_b, bottom), bottom),
                ];
                if (quad[1] - quad[0]).length_sq() + (quad[2] - quad[3]).length_sq() > 0.0001 {
                    quads.push(quad);
                }
            }
        }
    }
    quads
}

pub(super) fn x_at_y(a: Pos2, b: Pos2, y: f32) -> f32 {
    if (b.y - a.y).abs() <= f32::EPSILON {
        a.x.min(b.x)
    } else {
        a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y)
    }
}
