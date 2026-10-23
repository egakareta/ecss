use super::*;

pub(super) fn image_size(context: &Context, range: ShapeRange, rect: Rect) -> Option<Vec2> {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || !rect.is_finite() {
        return None;
    }
    let (texture, uv) = context.graphics(|layers| {
        let list = layers.get(range.layer)?;
        let mut image = None;
        for entry in list
            .all_entries()
            .skip(range.start.0)
            .take(range.end.0 - range.start.0)
        {
            match &entry.shape {
                egui::Shape::Noop => {}
                egui::Shape::Rect(shape)
                    if close_rect(shape.rect, rect) && shape.stroke.is_empty() =>
                {
                    if let Some(brush) = &shape.brush
                        && image.replace((brush.fill_texture_id, brush.uv)).is_some()
                    {
                        return None;
                    }
                }
                egui::Shape::Mesh(mesh) if mesh.vertices.len() == 4 && mesh.indices.len() == 6 => {
                    let uv = mesh.vertices.iter().fold(Rect::NOTHING, |rect, vertex| {
                        rect.union(Rect::from_pos(vertex.uv))
                    });
                    if image.replace((mesh.texture_id, uv)).is_some() {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        image
    })?;
    if texture == egui::TextureId::default() {
        return None;
    }
    let size = context
        .tex_manager()
        .read()
        .meta(texture)
        .map(|meta| vec2(meta.size[0] as f32, meta.size[1] as f32) * uv.size().abs())
        .unwrap_or(rect.size());
    (size.is_finite() && size.x > 0.0 && size.y > 0.0).then_some(size)
}

pub(super) fn fit_image(
    context: &Context,
    range: ShapeRange,
    source: Rect,
    natural: Vec2,
    area: Rect,
    style: &BoxStyle,
) {
    let target = style.object_position.rect(style.object_fit, natural, area);
    context.graphics_mut(|layers| {
        let list = layers.entry(range.layer);
        for index in range.start.0..range.end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                entry.clip_rect = entry.clip_rect.intersect(area);
                if !target.is_finite() || target.width() <= 0.0 || target.height() <= 0.0 {
                    entry.shape = egui::Shape::Noop;
                    return;
                }
                match &mut entry.shape {
                    egui::Shape::Rect(shape) => shape.rect = target,
                    egui::Shape::Mesh(mesh) => {
                        for vertex in &mut Arc::make_mut(mesh).vertices {
                            vertex.pos = target.min
                                + (vertex.pos - source.min) / source.size() * target.size();
                        }
                    }
                    _ => {}
                }
            });
        }
    });
}
