use super::*;

#[derive(Clone, Default)]
pub(super) struct TextureCache(pub(super) HashMap<Id, egui::TextureHandle>);

pub(super) fn composite(
    context: &Context,
    cache: &mut TextureCache,
    id: Id,
    opacity: f32,
    entries: Vec<egui::epaint::ClippedShape>,
) -> Vec<egui::epaint::ClippedShape> {
    let pixels_per_point = context.pixels_per_point();
    let primitives = context.tessellate(entries, pixels_per_point);
    if primitives.is_empty() {
        return Vec::new();
    }

    if primitives
        .iter()
        .any(|primitive| matches!(primitive.primitive, egui::epaint::Primitive::Callback(_)))
    {
        return multiply_shape_opacity(primitives, opacity);
    }

    let texture_ids = primitives
        .iter()
        .filter_map(|primitive| match &primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => Some(mesh.texture_id),
            egui::epaint::Primitive::Callback(_) => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let mut images = HashMap::new();
    for texture_id in texture_ids {
        let image = if texture_id == egui::TextureId::default() {
            let image = context.fonts_mut(|fonts| fonts.image());
            crate::background::remember_texture_image(context, texture_id, image.clone());
            Some(Arc::new(image))
        } else {
            crate::background::texture_image(context, texture_id)
        };
        if let Some(image) = image {
            images.insert(texture_id, image);
        }
    }
    if primitives
        .iter()
        .any(|primitive| match &primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => !images.contains_key(&mesh.texture_id),
            egui::epaint::Primitive::Callback(_) => false,
        })
    {
        return multiply_shape_opacity(primitives, opacity);
    }

    let mut bounds = Rect::NOTHING;
    for primitive in &primitives {
        let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
            unreachable!();
        };
        let mesh_bounds = mesh.vertices.iter().fold(Rect::NOTHING, |bounds, vertex| {
            bounds.union(Rect::from_min_max(vertex.pos, vertex.pos))
        });
        bounds = bounds.union(mesh_bounds.intersect(primitive.clip_rect));
    }
    if !bounds.is_positive() || !bounds.is_finite() {
        return Vec::new();
    }

    let origin = [
        (bounds.min.x * pixels_per_point).floor() as i32,
        (bounds.min.y * pixels_per_point).floor() as i32,
    ];
    let end = [
        (bounds.max.x * pixels_per_point).ceil() as i32,
        (bounds.max.y * pixels_per_point).ceil() as i32,
    ];
    let size = [
        usize::try_from(end[0].saturating_sub(origin[0])).unwrap_or(0),
        usize::try_from(end[1].saturating_sub(origin[1])).unwrap_or(0),
    ];
    let max_texture_side = context.input(|input| input.max_texture_side);
    let Some(pixel_count) = size[0].checked_mul(size[1]) else {
        return multiply_shape_opacity(primitives, opacity);
    };
    if size[0] == 0 || size[1] == 0 || size[0] > max_texture_side || size[1] > max_texture_side {
        return multiply_shape_opacity(primitives, opacity);
    }

    let mut pixels = vec![Color32::TRANSPARENT; pixel_count];
    for primitive in &primitives {
        let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
            continue;
        };
        let image = &images[&mesh.texture_id];
        rasterize_mesh(
            mesh,
            image,
            primitive.clip_rect,
            pixels_per_point,
            origin,
            size,
            &mut pixels,
        );
    }
    for pixel in &mut pixels {
        *pixel = pixel.gamma_multiply(opacity);
    }

    let image = egui::ColorImage::new(size, pixels);
    let texture = if let Some(texture) = cache.0.get_mut(&id) {
        texture.set(image.clone(), egui::TextureOptions::LINEAR);
        texture.clone()
    } else {
        let texture = context.load_texture(
            format!("ecss.opacity.{id:?}"),
            image.clone(),
            egui::TextureOptions::LINEAR,
        );
        cache.0.insert(id, texture.clone());
        texture
    };
    crate::background::remember_texture_image(context, texture.id(), image);

    let bounds = Rect::from_min_max(
        pos2(
            origin[0] as f32 / pixels_per_point,
            origin[1] as f32 / pixels_per_point,
        ),
        pos2(
            end[0] as f32 / pixels_per_point,
            end[1] as f32 / pixels_per_point,
        ),
    );
    vec![egui::epaint::ClippedShape::new(
        bounds,
        egui::Shape::image(
            texture.id(),
            bounds,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        ),
    )]
}

fn multiply_shape_opacity(
    primitives: Vec<egui::epaint::ClippedPrimitive>,
    opacity: f32,
) -> Vec<egui::epaint::ClippedShape> {
    primitives
        .into_iter()
        .map(|primitive| {
            let shape = match primitive.primitive {
                egui::epaint::Primitive::Mesh(mut mesh) => {
                    for vertex in &mut mesh.vertices {
                        vertex.color = vertex.color.gamma_multiply(opacity);
                    }
                    egui::Shape::Mesh(mesh.into())
                }
                egui::epaint::Primitive::Callback(callback) => egui::Shape::Callback(callback),
            };
            egui::epaint::ClippedShape::new(primitive.clip_rect, shape)
        })
        .collect()
}

fn rasterize_mesh(
    mesh: &egui::epaint::Mesh,
    image: &egui::ColorImage,
    clip: Rect,
    pixels_per_point: f32,
    origin: [i32; 2],
    size: [usize; 2],
    output: &mut [Color32],
) {
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let (Some(a), Some(b), Some(c)) = (
            mesh.vertices.get(triangle[0] as usize),
            mesh.vertices.get(triangle[1] as usize),
            mesh.vertices.get(triangle[2] as usize),
        ) else {
            continue;
        };
        let (mut b, mut c) = (b, c);
        let points = [a.pos, b.pos, c.pos];
        if points.iter().any(|point| !point.is_finite()) {
            continue;
        }
        let mut area = cross(b.pos - a.pos, c.pos - a.pos);
        if !area.is_finite() || area.abs() <= f32::EPSILON {
            continue;
        }
        if area < 0.0 {
            std::mem::swap(&mut b, &mut c);
            area = -area;
        }
        let min_x = points
            .iter()
            .map(|point| (point.x * pixels_per_point).floor() as i32 - origin[0])
            .min()
            .unwrap_or(0)
            .clamp(0, size[0] as i32);
        let max_x = points
            .iter()
            .map(|point| (point.x * pixels_per_point).ceil() as i32 - origin[0])
            .max()
            .unwrap_or(0)
            .clamp(0, size[0] as i32);
        let min_y = points
            .iter()
            .map(|point| (point.y * pixels_per_point).floor() as i32 - origin[1])
            .min()
            .unwrap_or(0)
            .clamp(0, size[1] as i32);
        let max_y = points
            .iter()
            .map(|point| (point.y * pixels_per_point).ceil() as i32 - origin[1])
            .max()
            .unwrap_or(0)
            .clamp(0, size[1] as i32);

        for y in min_y..max_y {
            for x in min_x..max_x {
                let point = pos2((origin[0] + x) as f32 + 0.5, (origin[1] + y) as f32 + 0.5)
                    / pixels_per_point;
                if !clip.contains(point) {
                    continue;
                }
                let weights = [
                    cross(b.pos - point, c.pos - point) / area,
                    cross(c.pos - point, a.pos - point) / area,
                    cross(a.pos - point, b.pos - point) / area,
                ];
                let edges = [(b.pos, c.pos), (c.pos, a.pos), (a.pos, b.pos)];
                if weights.iter().zip(edges).any(|(weight, (start, end))| {
                    *weight < -1.0e-5 || weight.abs() <= 1.0e-5 && !include_edge(start, end)
                }) {
                    continue;
                }
                let uv = vec2(
                    a.uv.x * weights[0] + b.uv.x * weights[1] + c.uv.x * weights[2],
                    a.uv.y * weights[0] + b.uv.y * weights[1] + c.uv.y * weights[2],
                );
                let color = interpolate_color([a.color, b.color, c.color], weights);
                let texel = sample_image(image, uv);
                let source = multiply_color(color, texel);
                let pixel = &mut output[y as usize * size[0] + x as usize];
                *pixel = source_over(source, *pixel);
            }
        }
    }
}

fn include_edge(start: Pos2, end: Pos2) -> bool {
    let delta = end - start;
    delta.y > 0.0 || delta.y == 0.0 && delta.x < 0.0
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn interpolate_color(colors: [Color32; 3], weights: [f32; 3]) -> [f32; 4] {
    let colors = colors.map(|color| color.to_array());
    std::array::from_fn(|channel| {
        (f32::from(colors[0][channel]) * weights[0]
            + f32::from(colors[1][channel]) * weights[1]
            + f32::from(colors[2][channel]) * weights[2])
            .clamp(0.0, 255.0)
    })
}

fn sample_image(image: &egui::ColorImage, uv: Vec2) -> [f32; 4] {
    let x = (uv.x * image.size[0] as f32 - 0.5).clamp(0.0, image.size[0] as f32 - 1.0);
    let y = (uv.y * image.size[1] as f32 - 0.5).clamp(0.0, image.size[1] as f32 - 1.0);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(image.size[0] - 1);
    let y1 = (y0 + 1).min(image.size[1] - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let pixels = [
        image.pixels[y0 * image.size[0] + x0],
        image.pixels[y0 * image.size[0] + x1],
        image.pixels[y1 * image.size[0] + x0],
        image.pixels[y1 * image.size[0] + x1],
    ]
    .map(|pixel| pixel.to_array());
    std::array::from_fn(|channel| {
        let top = f32::from(pixels[0][channel]) * (1.0 - tx) + f32::from(pixels[1][channel]) * tx;
        let bottom =
            f32::from(pixels[2][channel]) * (1.0 - tx) + f32::from(pixels[3][channel]) * tx;
        top * (1.0 - ty) + bottom * ty
    })
}

fn multiply_color(left: [f32; 4], right: [f32; 4]) -> Color32 {
    let channels: [u8; 4] = std::array::from_fn(|channel| {
        (left[channel] * right[channel] / 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    });
    Color32::from_rgba_premultiplied(channels[0], channels[1], channels[2], channels[3])
}

fn source_over(source: Color32, destination: Color32) -> Color32 {
    let source = source.to_array();
    let destination = destination.to_array();
    let inverse_alpha = 255.0 - f32::from(source[3]);
    let channels: [u8; 4] = std::array::from_fn(|channel| {
        (f32::from(source[channel]) + f32::from(destination[channel]) * inverse_alpha / 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    });
    Color32::from_rgba_premultiplied(channels[0], channels[1], channels[2], channels[3])
}
