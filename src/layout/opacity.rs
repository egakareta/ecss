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

pub(crate) fn rasterize_alpha_mask(
    context: &Context,
    meshes: &[egui::epaint::Mesh],
    bounds: Rect,
    blur: f32,
) -> Option<(egui::ColorImage, Rect)> {
    if !bounds.is_positive() || !blur.is_finite() || blur < 0.0 {
        return None;
    }
    let pixels_per_point = context.pixels_per_point();
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    if meshes
        .iter()
        .any(|mesh| mesh.texture_id != egui::TextureId::default())
    {
        return None;
    }

    let pixel_edge = |value: f32, ceil: bool| {
        let value = value * pixels_per_point;
        let value = if ceil { value.ceil() } else { value.floor() };
        value.clamp(i32::MIN as f32, i32::MAX as f32) as i32
    };
    let target_origin = [
        pixel_edge(bounds.min.x, false),
        pixel_edge(bounds.min.y, false),
    ];
    let target_end = [
        pixel_edge(bounds.max.x, true),
        pixel_edge(bounds.max.y, true),
    ];
    let target_size = [
        target_end[0].checked_sub(target_origin[0])? as usize,
        target_end[1].checked_sub(target_origin[1])? as usize,
    ];
    if target_size.contains(&0) {
        return None;
    }

    let max_texture_side = context.input(|input| input.max_texture_side).max(1);
    let max_pixels = 16 * 1024 * 1024;
    if target_size.iter().any(|side| *side > max_texture_side)
        || target_size[0].checked_mul(target_size[1])? > max_pixels
    {
        return None;
    }

    let sigma = (blur * 0.5 * pixels_per_point).min(max_texture_side as f32);
    let requested_padding = (sigma * 3.0).ceil() as usize;
    let max_padding =
        (max_texture_side - target_size[0]).min(max_texture_side - target_size[1]) / 2;
    let mut low = 0;
    let mut high = requested_padding.min(max_padding);
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let width = target_size[0] + middle * 2;
        let height = target_size[1] + middle * 2;
        if width
            .checked_mul(height)
            .is_some_and(|pixels| pixels <= max_pixels)
        {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let padding = low;
    let source_size = [target_size[0] + padding * 2, target_size[1] + padding * 2];
    let source_origin = [
        target_origin[0].saturating_sub(padding as i32),
        target_origin[1].saturating_sub(padding as i32),
    ];
    let source_clip = Rect::from_min_max(
        pos2(
            source_origin[0] as f32 / pixels_per_point,
            source_origin[1] as f32 / pixels_per_point,
        ),
        pos2(
            (source_origin[0] as f32 + source_size[0] as f32) / pixels_per_point,
            (source_origin[1] as f32 + source_size[1] as f32) / pixels_per_point,
        ),
    );
    let font_image = context.fonts_mut(|fonts| fonts.image());
    crate::background::remember_texture_image(
        context,
        egui::TextureId::default(),
        font_image.clone(),
    );
    let mut source = vec![Color32::TRANSPARENT; source_size[0] * source_size[1]];
    for mesh in meshes {
        rasterize_mesh(
            mesh,
            &font_image,
            source_clip,
            pixels_per_point,
            source_origin,
            source_size,
            &mut source,
        );
    }
    let effective_sigma = sigma.min(padding as f32 / 3.0);
    let alpha = blur_alpha(
        source.iter().map(|pixel| f32::from(pixel.a())).collect(),
        source_size,
        effective_sigma,
        padding,
    );
    let mut pixels = Vec::with_capacity(target_size[0] * target_size[1]);
    for y in 0..target_size[1] {
        for x in 0..target_size[0] {
            let value = alpha[(y + padding) * source_size[0] + x + padding]
                .round()
                .clamp(0.0, 255.0) as u8;
            pixels.push(Color32::from_rgba_unmultiplied(255, 255, 255, value));
        }
    }
    let image = egui::ColorImage::new(target_size, pixels);
    let output_bounds = Rect::from_min_max(
        pos2(
            target_origin[0] as f32 / pixels_per_point,
            target_origin[1] as f32 / pixels_per_point,
        ),
        pos2(
            target_end[0] as f32 / pixels_per_point,
            target_end[1] as f32 / pixels_per_point,
        ),
    );
    Some((image, output_bounds))
}

fn blur_alpha(mut alpha: Vec<f32>, size: [usize; 2], sigma: f32, padding: usize) -> Vec<f32> {
    if sigma <= 0.0 || padding == 0 {
        return alpha;
    }
    let radius = (sigma * 3.0).ceil().min(padding as f32) as usize;
    let kernel_len = radius * 2 + 1;
    if alpha.len().saturating_mul(kernel_len).saturating_mul(2) > 8_000_000 {
        let radius = sigma.round().max(1.0).min(padding as f32) as usize;
        for _ in 0..3 {
            alpha = box_blur_axis(&alpha, size, radius, true);
            alpha = box_blur_axis(&alpha, size, radius, false);
        }
        return alpha;
    }

    let mut weights = (-i32::try_from(radius).unwrap_or(i32::MAX)
        ..=i32::try_from(radius).unwrap_or(i32::MAX))
        .map(|offset| {
            crate::box_decoration::shadow_opacity(offset as f32 - 0.5, sigma)
                - crate::box_decoration::shadow_opacity(offset as f32 + 0.5, sigma)
        })
        .collect::<Vec<_>>();
    let total = weights.iter().sum::<f32>();
    if total <= 0.0 || !total.is_finite() {
        return alpha;
    }
    weights.iter_mut().for_each(|weight| *weight /= total);
    alpha = gaussian_blur_axis(&alpha, size, &weights, true);
    gaussian_blur_axis(&alpha, size, &weights, false)
}

fn gaussian_blur_axis(
    alpha: &[f32],
    size: [usize; 2],
    weights: &[f32],
    horizontal: bool,
) -> Vec<f32> {
    let radius = weights.len() / 2;
    let mut output = vec![0.0; alpha.len()];
    for y in 0..size[1] {
        for x in 0..size[0] {
            let mut value = 0.0;
            for (kernel, weight) in weights.iter().enumerate() {
                let offset = kernel as isize - radius as isize;
                let (sample_x, sample_y) = if horizontal {
                    (x as isize + offset, y as isize)
                } else {
                    (x as isize, y as isize + offset)
                };
                if sample_x >= 0
                    && sample_x < size[0] as isize
                    && sample_y >= 0
                    && sample_y < size[1] as isize
                {
                    value += alpha[sample_y as usize * size[0] + sample_x as usize] * weight;
                }
            }
            output[y * size[0] + x] = value;
        }
    }
    output
}

fn box_blur_axis(alpha: &[f32], size: [usize; 2], radius: usize, horizontal: bool) -> Vec<f32> {
    let (lines, line_length) = if horizontal {
        (size[1], size[0])
    } else {
        (size[0], size[1])
    };
    let mut output = vec![0.0; alpha.len()];
    let window = radius.saturating_mul(2).saturating_add(1) as f32;
    for line in 0..lines {
        let mut prefix = Vec::with_capacity(line_length + 1);
        prefix.push(0.0);
        for index in 0..line_length {
            let position = if horizontal {
                line * size[0] + index
            } else {
                index * size[0] + line
            };
            prefix.push(prefix[index] + alpha[position]);
        }
        for index in 0..line_length {
            let start = index.saturating_sub(radius);
            let end = index
                .saturating_add(radius)
                .saturating_add(1)
                .min(line_length);
            let position = if horizontal {
                line * size[0] + index
            } else {
                index * size[0] + line
            };
            output[position] = (prefix[end] - prefix[start]) / window;
        }
    }
    output
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
