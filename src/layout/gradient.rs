use super::*;

const GRADIENT_MESH_CACHE: &str = "ecss.gradient-mesh-cache";

#[derive(Clone, PartialEq)]
pub(super) enum GradientPaint {
    Linear(crate::css::LinearGradient),
    Radial(crate::css::RadialGradient),
    Conic(crate::css::ConicGradient),
}

#[derive(Clone, PartialEq)]
struct GradientMeshKey {
    gradient: GradientPaint,
    surface: Rect,
    paint_area: Rect,
    sample_offset: Vec2,
    clip: Option<crate::box_decoration::PathClip>,
}

#[derive(Default)]
struct GradientMeshCache {
    entries: Vec<(GradientMeshKey, Arc<egui::epaint::Mesh>)>,
}

pub(super) fn gradient_paint_shape(
    context: &Context,
    surface: Rect,
    paint_area: Rect,
    sample_offset: Vec2,
    gradient: GradientPaint,
    clip: Option<&crate::box_decoration::PathClip>,
) -> egui::Shape {
    let key = GradientMeshKey {
        gradient,
        surface,
        paint_area,
        sample_offset,
        clip: clip.cloned(),
    };
    let cache = context.data_mut(|data| {
        let id = Id::unique(GRADIENT_MESH_CACHE);
        if let Some(cache) = data.get_temp::<Arc<std::sync::Mutex<GradientMeshCache>>>(id) {
            cache
        } else {
            let cache = Arc::new(std::sync::Mutex::new(GradientMeshCache::default()));
            data.insert_temp(id, cache.clone());
            cache
        }
    });
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, mesh)) = cache.entries.iter().find(|(entry, _)| entry == &key) {
        return egui::Shape::Mesh(mesh.clone());
    }

    let mesh = match &key.gradient {
        GradientPaint::Linear(gradient) => {
            linear_gradient_mesh(surface, paint_area, sample_offset, gradient)
        }
        GradientPaint::Radial(gradient) => {
            radial_gradient_mesh(surface, paint_area, sample_offset, gradient)
        }
        GradientPaint::Conic(gradient) => {
            conic_gradient_mesh(surface, paint_area, sample_offset, gradient)
        }
    };
    let egui::Shape::Mesh(mesh) =
        clip_background_shape(egui::Shape::Mesh(mesh.into()), key.clip.as_ref())
    else {
        unreachable!()
    };
    if cache.entries.len() == 32 {
        cache.entries.remove(0);
    }
    cache.entries.push((key, mesh.clone()));
    egui::Shape::Mesh(mesh)
}

fn linear_gradient_mesh(
    clip: Rect,
    paint_area: Rect,
    sample_offset: Vec2,
    gradient: &crate::css::LinearGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 32;
    let radians = gradient.angle.to_radians();
    let direction = vec2(radians.sin(), -radians.cos());
    let paint_corners = [
        paint_area.left_top(),
        paint_area.right_top(),
        paint_area.right_bottom(),
        paint_area.left_bottom(),
    ];
    let min = paint_corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::INFINITY, f32::min);
    let max = paint_corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::NEG_INFINITY, f32::max);
    let range = (max - min).max(f32::EPSILON);
    let clip_corners = [
        clip.left_top(),
        clip.right_top(),
        clip.right_bottom(),
        clip.left_bottom(),
    ];
    let offset_projection = sample_offset.dot(direction);
    let clip_min = (clip_corners
        .iter()
        .map(|point| (point.to_vec2().dot(direction) + offset_projection - min) / range)
        .fold(f32::INFINITY, f32::min))
    .min(1.0);
    let clip_max = (clip_corners
        .iter()
        .map(|point| (point.to_vec2().dot(direction) + offset_projection - min) / range)
        .fold(f32::NEG_INFINITY, f32::max))
    .max(0.0);
    let active_min = clip_min.max(0.0);
    let active_max = clip_max.min(1.0);
    let mut boundaries = vec![clip_min, clip_max];
    if clip_min < 0.0 && clip_max > 0.0 {
        boundaries.push(0.0);
    }
    if clip_min < 1.0 && clip_max > 1.0 {
        boundaries.push(1.0);
    }
    if active_max > active_min {
        for step in 1..STEPS {
            boundaries.push(egui::emath::lerp(
                active_min..=active_max,
                step as f32 / STEPS as f32,
            ));
        }
    }
    boundaries.sort_by(f32::total_cmp);
    boundaries.dedup_by(|first, second| (*first - *second).abs() <= f32::EPSILON);

    let stops = PreparedGradient::new(&gradient.stops, gradient.repeating, gradient.color_space);
    let boundary_colors = boundaries
        .iter()
        .map(|position| stops.sample(position.clamp(0.0, 1.0)))
        .collect::<smallvec::SmallVec<[Color32; 36]>>();
    let mut mesh = Mesh::default();
    let intervals = boundaries.len().saturating_sub(1);
    mesh.vertices.reserve(intervals * 8);
    mesh.indices.reserve(intervals * 18);
    let mut polygon = smallvec::SmallVec::<[LinearGradientPoint; 8]>::new();
    let mut scratch = smallvec::SmallVec::<[LinearGradientPoint; 8]>::new();
    for (index, interval) in boundaries.windows(2).enumerate() {
        let lower = min + interval[0] * range - offset_projection;
        let upper = min + interval[1] * range - offset_projection;
        polygon.clear();
        polygon.extend(clip_corners.map(|position| {
            let projection = position.to_vec2().dot(direction);
            LinearGradientPoint {
                position,
                projection,
                gradient_position: (projection + offset_projection - min) / range,
            }
        }));
        clip_gradient_polygon(&mut polygon, &mut scratch, lower, interval[0], true);
        clip_gradient_polygon(&mut polygon, &mut scratch, upper, interval[1], false);
        if polygon.len() < 3 {
            continue;
        }
        let first = mesh.vertices.len() as u32;
        for point in &polygon {
            let color = if point.gradient_position == interval[0] {
                boundary_colors[index]
            } else if point.gradient_position == interval[1] {
                boundary_colors[index + 1]
            } else {
                stops.sample(point.gradient_position.clamp(0.0, 1.0))
            };
            mesh.vertices.push(Vertex {
                pos: point.position,
                uv: Pos2::ZERO,
                color,
            });
        }
        for index in 1..polygon.len() - 1 {
            mesh.add_triangle(first, first + index as u32, first + index as u32 + 1);
        }
    }
    mesh
}

#[derive(Clone, Copy)]
struct LinearGradientPoint {
    position: Pos2,
    projection: f32,
    gradient_position: f32,
}

fn clip_gradient_polygon(
    polygon: &mut smallvec::SmallVec<[LinearGradientPoint; 8]>,
    scratch: &mut smallvec::SmallVec<[LinearGradientPoint; 8]>,
    boundary: f32,
    gradient_position: f32,
    keep_greater: bool,
) {
    if polygon.is_empty() {
        return;
    }
    scratch.clear();
    let mut previous_index = polygon.len() - 1;
    let mut previous_distance = polygon[previous_index].projection - boundary;
    for current_index in 0..polygon.len() {
        let previous = polygon[previous_index];
        let current = polygon[current_index];
        let current_distance = current.projection - boundary;
        let previous_inside = if keep_greater {
            previous_distance >= -1.0e-5
        } else {
            previous_distance <= 1.0e-5
        };
        let current_inside = if keep_greater {
            current_distance >= -1.0e-5
        } else {
            current_distance <= 1.0e-5
        };
        if previous_inside != current_inside {
            let amount = previous_distance / (previous_distance - current_distance);
            scratch.push(LinearGradientPoint {
                position: previous.position.lerp(current.position, amount),
                projection: boundary,
                gradient_position,
            });
        }
        if current_inside {
            scratch.push(current);
        }
        previous_index = current_index;
        previous_distance = current_distance;
    }
    std::mem::swap(polygon, scratch);
}

pub(super) fn linear_gradient_slice_mesh(
    clip: Rect,
    source: Rect,
    image_size: Vec2,
    gradient: &crate::css::LinearGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    let steps = 32;
    let radians = gradient.angle.to_radians();
    let direction = vec2(radians.sin(), -radians.cos());
    let image = Rect::from_min_size(Pos2::ZERO, image_size);
    let corners = [
        image.left_top(),
        image.right_top(),
        image.right_bottom(),
        image.left_bottom(),
    ];
    let min = corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::INFINITY, f32::min);
    let max = corners
        .iter()
        .map(|point| point.to_vec2().dot(direction))
        .fold(f32::NEG_INFINITY, f32::max);
    let stops = PreparedGradient::new(&gradient.stops, gradient.repeating, gradient.color_space);
    let mut mesh = Mesh::default();
    reserve_gradient_grid(&mut mesh, steps);
    for y in 0..=steps {
        for x in 0..=steps {
            let u = x as f32 / steps as f32;
            let v = y as f32 / steps as f32;
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), u),
                egui::emath::lerp(clip.top()..=clip.bottom(), v),
            );
            let source_point = pos2(
                egui::emath::lerp(source.left()..=source.right(), u),
                egui::emath::lerp(source.top()..=source.bottom(), v),
            );
            let t = ((source_point.to_vec2().dot(direction) - min) / (max - min).max(f32::EPSILON))
                .clamp(0.0, 1.0);
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: stops.sample(t),
            });
        }
    }
    for y in 0..steps {
        for x in 0..steps {
            let top_left = y * (steps + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + steps + 1;
            let bottom_right = bottom_left + 1;
            mesh.add_triangle(top_left as u32, top_right as u32, bottom_right as u32);
            mesh.add_triangle(top_left as u32, bottom_right as u32, bottom_left as u32);
        }
    }
    mesh
}

fn radial_gradient_mesh(
    clip: Rect,
    paint_area: Rect,
    sample_offset: Vec2,
    gradient: &crate::css::RadialGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 48;
    let (center, radii, corner_extent) = radial_gradient_parameters(paint_area, gradient);
    let stops = PreparedGradient::new(&gradient.stops, gradient.repeating, gradient.color_space);
    let mut mesh = Mesh::default();
    reserve_gradient_grid(&mut mesh, STEPS);
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), x as f32 / STEPS as f32),
                egui::emath::lerp(clip.top()..=clip.bottom(), y as f32 / STEPS as f32),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: stops.sample(radial_gradient_position(
                    point + sample_offset,
                    center,
                    radii,
                    corner_extent,
                )),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

pub(super) fn radial_gradient_slice_mesh(
    clip: Rect,
    source: Rect,
    image_size: Vec2,
    gradient: &crate::css::RadialGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 32;
    let image = Rect::from_min_size(Pos2::ZERO, image_size);
    let (center, radii, corner_extent) = radial_gradient_parameters(image, gradient);
    let stops = PreparedGradient::new(&gradient.stops, gradient.repeating, gradient.color_space);
    let mut mesh = Mesh::default();
    reserve_gradient_grid(&mut mesh, STEPS);
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let u = x as f32 / STEPS as f32;
            let v = y as f32 / STEPS as f32;
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), u),
                egui::emath::lerp(clip.top()..=clip.bottom(), v),
            );
            let source_point = pos2(
                egui::emath::lerp(source.left()..=source.right(), u),
                egui::emath::lerp(source.top()..=source.bottom(), v),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: stops.sample(radial_gradient_position(
                    source_point,
                    center,
                    radii,
                    corner_extent,
                )),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

pub(super) fn radial_gradient_parameters(
    area: Rect,
    gradient: &crate::css::RadialGradient,
) -> (Pos2, Vec2, bool) {
    let center = pos2(
        area.left() + gradient.center[0].resolve(area.width()),
        area.top() + gradient.center[1].resolve(area.height()),
    );
    match gradient.size {
        crate::css::RadialSize::Explicit(values) => {
            let radius = |value: crate::css::RadialRadius, axis: usize| match value {
                crate::css::RadialRadius::Length(value) => value,
                crate::css::RadialRadius::Percentage(value) => {
                    if gradient.shape == crate::css::RadialShape::Circle {
                        value * (area.width().powi(2) + area.height().powi(2)).sqrt()
                            / std::f32::consts::SQRT_2
                    } else if axis == 0 {
                        value * area.width()
                    } else {
                        value * area.height()
                    }
                }
            };
            let x = radius(values[0], 0);
            let y = if gradient.shape == crate::css::RadialShape::Circle {
                x
            } else {
                radius(values[1], 1)
            };
            (center, vec2(x, y), false)
        }
        crate::css::RadialSize::Extent(extent) => {
            let horizontal = [center.x - area.left(), area.right() - center.x];
            let vertical = [center.y - area.top(), area.bottom() - center.y];
            let corner_extent = matches!(
                extent,
                crate::css::RadialExtent::ClosestCorner | crate::css::RadialExtent::FarthestCorner
            );
            if gradient.shape == crate::css::RadialShape::Circle {
                let radius = match extent {
                    crate::css::RadialExtent::ClosestSide => horizontal
                        .into_iter()
                        .chain(vertical)
                        .fold(f32::INFINITY, f32::min),
                    crate::css::RadialExtent::FarthestSide => {
                        horizontal.into_iter().chain(vertical).fold(0.0, f32::max)
                    }
                    crate::css::RadialExtent::ClosestCorner
                    | crate::css::RadialExtent::FarthestCorner => {
                        let distances = [
                            vec2(horizontal[0], vertical[0]).length(),
                            vec2(horizontal[1], vertical[0]).length(),
                            vec2(horizontal[1], vertical[1]).length(),
                            vec2(horizontal[0], vertical[1]).length(),
                        ];
                        if extent == crate::css::RadialExtent::ClosestCorner {
                            distances.into_iter().fold(f32::INFINITY, f32::min)
                        } else {
                            distances.into_iter().fold(0.0, f32::max)
                        }
                    }
                };
                (center, Vec2::splat(radius), false)
            } else if corner_extent {
                let distances = [
                    (horizontal[0], vertical[0]),
                    (horizontal[1], vertical[0]),
                    (horizontal[1], vertical[1]),
                    (horizontal[0], vertical[1]),
                ];
                let selected = if extent == crate::css::RadialExtent::ClosestCorner {
                    distances.into_iter().min_by(|first, second| {
                        (first.0 * first.0 + first.1 * first.1)
                            .total_cmp(&(second.0 * second.0 + second.1 * second.1))
                    })
                } else {
                    distances.into_iter().max_by(|first, second| {
                        (first.0 * first.0 + first.1 * first.1)
                            .total_cmp(&(second.0 * second.0 + second.1 * second.1))
                    })
                }
                .unwrap_or((0.0, 0.0));
                (center, vec2(selected.0, selected.1), true)
            } else {
                let choose = |values: [f32; 2]| {
                    if extent == crate::css::RadialExtent::ClosestSide {
                        values.into_iter().fold(f32::INFINITY, f32::min)
                    } else {
                        values.into_iter().fold(0.0, f32::max)
                    }
                };
                (center, vec2(choose(horizontal), choose(vertical)), false)
            }
        }
    }
}

pub(super) fn radial_gradient_position(
    point: Pos2,
    center: Pos2,
    radii: Vec2,
    corner_extent: bool,
) -> f32 {
    if point == center {
        return 0.0;
    }
    let distance = vec2(
        (point.x - center.x) / radii.x.max(f32::EPSILON),
        (point.y - center.y) / radii.y.max(f32::EPSILON),
    )
    .length();
    if corner_extent {
        distance / std::f32::consts::SQRT_2
    } else {
        distance
    }
}

fn conic_gradient_mesh(
    clip: Rect,
    paint_area: Rect,
    sample_offset: Vec2,
    gradient: &crate::css::ConicGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 64;
    let center = pos2(
        paint_area.left() + gradient.center[0].resolve(paint_area.width()),
        paint_area.top() + gradient.center[1].resolve(paint_area.height()),
    );
    let stops = PreparedGradient::new(&gradient.stops, gradient.repeating, gradient.color_space);
    let mut mesh = Mesh::default();
    reserve_gradient_grid(&mut mesh, STEPS);
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), x as f32 / STEPS as f32),
                egui::emath::lerp(clip.top()..=clip.bottom(), y as f32 / STEPS as f32),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: stops.sample(conic_gradient_position(
                    point + sample_offset,
                    center,
                    gradient.from_angle,
                )),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

pub(super) fn conic_gradient_slice_mesh(
    clip: Rect,
    source: Rect,
    image_size: Vec2,
    gradient: &crate::css::ConicGradient,
) -> egui::epaint::Mesh {
    use egui::epaint::{Mesh, Vertex};

    const STEPS: usize = 32;
    let center = pos2(
        gradient.center[0].resolve(image_size.x),
        gradient.center[1].resolve(image_size.y),
    );
    let stops = PreparedGradient::new(&gradient.stops, gradient.repeating, gradient.color_space);
    let mut mesh = Mesh::default();
    reserve_gradient_grid(&mut mesh, STEPS);
    for y in 0..=STEPS {
        for x in 0..=STEPS {
            let u = x as f32 / STEPS as f32;
            let v = y as f32 / STEPS as f32;
            let point = pos2(
                egui::emath::lerp(clip.left()..=clip.right(), u),
                egui::emath::lerp(clip.top()..=clip.bottom(), v),
            );
            let source_point = pos2(
                egui::emath::lerp(source.left()..=source.right(), u),
                egui::emath::lerp(source.top()..=source.bottom(), v),
            );
            mesh.vertices.push(Vertex {
                pos: point,
                uv: Pos2::ZERO,
                color: stops.sample(conic_gradient_position(
                    source_point,
                    center,
                    gradient.from_angle,
                )),
            });
        }
    }
    add_grid_triangles(&mut mesh, STEPS);
    mesh
}

pub(super) fn conic_gradient_position(point: Pos2, center: Pos2, from_angle: f32) -> f32 {
    if point == center {
        return 0.0;
    }
    let angle = (point.y - center.y).atan2(point.x - center.x).to_degrees() + 90.0 - from_angle;
    angle.rem_euclid(360.0) / 360.0
}

fn add_grid_triangles(mesh: &mut egui::epaint::Mesh, steps: usize) {
    for y in 0..steps {
        for x in 0..steps {
            let top_left = y * (steps + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + steps + 1;
            let bottom_right = bottom_left + 1;
            mesh.add_triangle(top_left as u32, top_right as u32, bottom_right as u32);
            mesh.add_triangle(top_left as u32, bottom_right as u32, bottom_left as u32);
        }
    }
}

fn reserve_gradient_grid(mesh: &mut egui::epaint::Mesh, steps: usize) {
    let side = steps + 1;
    mesh.vertices.reserve(side * side);
    mesh.indices.reserve(steps * steps * 6);
}

struct PreparedGradient {
    stops: smallvec::SmallVec<[PreparedGradientStop; 8]>,
    repeating: bool,
    color_space: crate::css::GradientColorSpace,
}

struct PreparedGradientStop {
    position: f32,
    color: Color32,
    alpha: f32,
    components: [f32; 3],
    hint_after: Option<PreparedGradientHint>,
}

enum PreparedGradientHint {
    AtStart,
    AtEnd,
    Power(f32),
}

impl PreparedGradient {
    fn new(
        source: &[crate::css::GradientStop],
        repeating: bool,
        color_space: crate::css::GradientColorSpace,
    ) -> Self {
        let mut positions = source
            .iter()
            .map(|stop| stop.position)
            .collect::<smallvec::SmallVec<[Option<f32>; 8]>>();
        if positions[0].is_none() {
            positions[0] = Some(0.0);
        }
        let last = positions.len() - 1;
        if positions[last].is_none() {
            positions[last] = Some(1.0);
        }
        let mut previous_fixed = 0;
        for index in 1..positions.len() {
            if let Some(value) = positions[index] {
                let previous = positions[previous_fixed].unwrap_or(0.0);
                let value = value.max(previous);
                positions[index] = Some(value);
                let gap = index - previous_fixed;
                for missing in 1..gap {
                    let amount = missing as f32 / gap as f32;
                    positions[previous_fixed + missing] =
                        Some(egui::emath::lerp(previous..=value, amount));
                }
                previous_fixed = index;
            }
        }

        let stops = source
            .iter()
            .enumerate()
            .map(|(index, stop)| {
                let position = positions[index].unwrap_or(0.0);
                let hint_after = stop.hint_after.and_then(|hint| {
                    let end = positions.get(index + 1).copied().flatten()?;
                    let hint_amount =
                        ((hint - position) / (end - position).max(f32::EPSILON)).clamp(0.0, 1.0);
                    Some(if hint_amount <= f32::EPSILON {
                        PreparedGradientHint::AtStart
                    } else if hint_amount >= 1.0 - f32::EPSILON {
                        PreparedGradientHint::AtEnd
                    } else {
                        PreparedGradientHint::Power(0.5_f32.ln() / hint_amount.ln())
                    })
                });
                let rgba = stop.color.to_array();
                PreparedGradientStop {
                    position,
                    color: stop.color,
                    alpha: f32::from(rgba[3]) / 255.0,
                    components: gradient_color_components(rgba, color_space),
                    hint_after,
                }
            })
            .collect::<smallvec::SmallVec<[PreparedGradientStop; 8]>>();
        Self {
            stops,
            repeating,
            color_space,
        }
    }

    fn sample(&self, position: f32) -> Color32 {
        let position = if self.repeating {
            let start = self.stops[0].position;
            let end = self.stops[self.stops.len() - 1].position;
            let period = end - start;
            if period <= f32::EPSILON {
                return self.stops[self.stops.len() - 1].color;
            }
            (position - start).rem_euclid(period) + start
        } else {
            position
        };
        let after = self
            .stops
            .iter()
            .position(|stop| stop.position >= position)
            .unwrap_or(self.stops.len() - 1);
        if after == 0 {
            return self.stops[0].color;
        }
        let start = &self.stops[after - 1];
        let end = &self.stops[after];
        let mut amount = ((position - start.position)
            / (end.position - start.position).max(f32::EPSILON))
        .clamp(0.0, 1.0);
        if let Some(hint) = &start.hint_after {
            amount = match hint {
                PreparedGradientHint::AtStart => {
                    if amount <= 0.0 {
                        0.0
                    } else {
                        1.0
                    }
                }
                PreparedGradientHint::AtEnd => {
                    if amount >= 1.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                PreparedGradientHint::Power(exponent) => amount.powf(*exponent),
            };
        }
        interpolate_prepared_gradient_colors(start, end, amount, self.color_space)
    }
}

fn interpolate_prepared_gradient_colors(
    start: &PreparedGradientStop,
    end: &PreparedGradientStop,
    amount: f32,
    color_space: crate::css::GradientColorSpace,
) -> Color32 {
    let alpha = egui::emath::lerp(start.alpha..=end.alpha, amount);
    let mut color = std::array::from_fn(|index| {
        egui::emath::lerp(
            (start.components[index] * start.alpha)..=(end.components[index] * end.alpha),
            amount,
        )
    });
    if alpha > f32::EPSILON {
        color.iter_mut().for_each(|component| *component /= alpha);
    } else {
        color = [0.0; 3];
    }
    let channels = gradient_color_channels(color, alpha, color_space);
    Color32::from_rgba_unmultiplied(channels[0], channels[1], channels[2], channels[3])
}

pub(super) fn gradient_color(
    stops: &[crate::css::GradientStop],
    position: f32,
    repeating: bool,
    color_space: crate::css::GradientColorSpace,
) -> Color32 {
    let mut positions = stops.iter().map(|stop| stop.position).collect::<Vec<_>>();
    if positions[0].is_none() {
        positions[0] = Some(0.0);
    }
    let last = positions.len() - 1;
    if positions[last].is_none() {
        positions[last] = Some(1.0);
    }
    let mut previous_fixed = 0;
    for index in 1..positions.len() {
        if let Some(value) = positions[index] {
            let previous = positions[previous_fixed].unwrap_or(0.0);
            let value = value.max(previous);
            positions[index] = Some(value);
            let gap = index - previous_fixed;
            for missing in 1..gap {
                let amount = missing as f32 / gap as f32;
                positions[previous_fixed + missing] =
                    Some(egui::emath::lerp(previous..=value, amount));
            }
            previous_fixed = index;
        }
    }

    let stops = stops
        .iter()
        .zip(positions)
        .map(|(stop, position)| (position.unwrap_or(0.0), stop.color, stop.hint_after))
        .collect::<Vec<_>>();
    let position = if repeating {
        let start = stops[0].0;
        let end = stops[stops.len() - 1].0;
        let period = end - start;
        if period <= f32::EPSILON {
            return stops[stops.len() - 1].1;
        }
        (position - start).rem_euclid(period) + start
    } else {
        position
    };
    let after = stops
        .iter()
        .position(|(stop_position, _, _)| *stop_position >= position)
        .unwrap_or(stops.len() - 1);
    if after == 0 {
        return stops[0].1;
    }
    let (start, start_color, hint) = stops[after - 1];
    let (end, end_color, _) = stops[after];
    let mut amount = ((position - start) / (end - start).max(f32::EPSILON)).clamp(0.0, 1.0);
    if let Some(hint) = hint {
        let hint_amount = ((hint - start) / (end - start).max(f32::EPSILON)).clamp(0.0, 1.0);
        amount = if hint_amount <= f32::EPSILON {
            if amount <= 0.0 { 0.0 } else { 1.0 }
        } else if hint_amount >= 1.0 - f32::EPSILON {
            if amount >= 1.0 { 1.0 } else { 0.0 }
        } else {
            amount.powf(0.5_f32.ln() / hint_amount.ln())
        };
    }
    interpolate_gradient_colors(start_color, end_color, amount, color_space)
}

fn interpolate_gradient_colors(
    start: Color32,
    end: Color32,
    amount: f32,
    color_space: crate::css::GradientColorSpace,
) -> Color32 {
    let start_rgba = start.to_array();
    let end_rgba = end.to_array();
    let start_alpha = f32::from(start_rgba[3]) / 255.0;
    let end_alpha = f32::from(end_rgba[3]) / 255.0;
    let alpha = egui::emath::lerp(start_alpha..=end_alpha, amount);
    let start_color = gradient_color_components(start_rgba, color_space);
    let end_color = gradient_color_components(end_rgba, color_space);
    let mut color = std::array::from_fn(|index| {
        egui::emath::lerp(
            (start_color[index] * start_alpha)..=(end_color[index] * end_alpha),
            amount,
        )
    });
    if alpha > f32::EPSILON {
        color.iter_mut().for_each(|component| *component /= alpha);
    } else {
        color = [0.0; 3];
    }
    let channels = gradient_color_channels(color, alpha, color_space);
    Color32::from_rgba_unmultiplied(channels[0], channels[1], channels[2], channels[3])
}

fn gradient_color_components(
    rgba: [u8; 4],
    color_space: crate::css::GradientColorSpace,
) -> [f32; 3] {
    let srgb = [
        f32::from(rgba[0]) / 255.0,
        f32::from(rgba[1]) / 255.0,
        f32::from(rgba[2]) / 255.0,
    ];
    match color_space {
        crate::css::GradientColorSpace::Srgb => srgb,
        crate::css::GradientColorSpace::SrgbLinear => srgb.map(srgb_to_linear),
        crate::css::GradientColorSpace::Oklab => {
            let [red, green, blue] = srgb.map(srgb_to_linear);
            let lightness = (0.41222147 * red + 0.53633255 * green + 0.051445995 * blue).cbrt();
            let m = (0.2119035 * red + 0.6806995 * green + 0.10739696 * blue).cbrt();
            let s = (0.08830246 * red + 0.28171885 * green + 0.6299787 * blue).cbrt();
            [
                0.21045426 * lightness + 0.7936178 * m - 0.004072047 * s,
                1.9779985 * lightness - 2.4285922 * m + 0.4505937 * s,
                0.025904037 * lightness + 0.78277177 * m - 0.80867577 * s,
            ]
        }
    }
}

fn gradient_color_channels(
    components: [f32; 3],
    alpha: f32,
    color_space: crate::css::GradientColorSpace,
) -> [u8; 4] {
    let srgb = match color_space {
        crate::css::GradientColorSpace::Srgb => components,
        crate::css::GradientColorSpace::SrgbLinear => components.map(linear_to_srgb),
        crate::css::GradientColorSpace::Oklab => {
            let [lightness, a, b] = components;
            let l = (lightness + 0.39633778 * a + 0.21580376 * b).powi(3);
            let m = (lightness - 0.105561346 * a - 0.06385417 * b).powi(3);
            let s = (lightness - 0.08948418 * a - 1.2914855 * b).powi(3);
            [
                linear_to_srgb(4.076742 * l - 3.3077116 * m + 0.23096994 * s),
                linear_to_srgb(-1.268438 * l + 2.6097574 * m - 0.3413194 * s),
                linear_to_srgb(-0.0041960863 * l - 0.7034186 * m + 1.7076147 * s),
            ]
        }
    };
    [
        (srgb[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (srgb[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.max(0.0).powf(1.0 / 2.4) - 0.055
    }
}
