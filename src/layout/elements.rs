use super::*;

pub(super) struct ElementImage {
    pub(super) size: Vec2,
    pub(super) meshes: Vec<egui::epaint::Mesh>,
    pub(super) textures: HashMap<egui::TextureId, Arc<egui::ColorImage>>,
    pub(super) dependencies: HashSet<String>,
    pub(super) fingerprint: u64,
    pub(super) captured_frame: u64,
}

pub(super) fn element_image_bounds(visual: Rect, style: &BoxStyle) -> Rect {
    visual + border_image_outset(style)
}

pub(super) fn capture_element_image(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    bounds: Rect,
    dependencies: HashSet<String>,
) -> Option<ElementImage> {
    if !bounds.is_positive() || !bounds.is_finite() {
        return None;
    }
    let shapes = context.graphics(|layers| {
        let list = layers.get(layer)?;
        Some(
            list.all_entries()
                .skip(start.0)
                .take(end.0.saturating_sub(start.0))
                .filter_map(|entry| {
                    let clip_rect = entry.clip_rect.intersect(bounds);
                    clip_rect.is_positive().then(|| {
                        let mut clipped_shape =
                            egui::epaint::ClippedShape::new(clip_rect, entry.shape.clone());
                        clipped_shape.transform_after_tessellation =
                            entry.transform_after_tessellation;
                        clipped_shape
                    })
                })
                .collect::<Vec<_>>(),
        )
    })?;
    let clip = crate::box_decoration::path_clip(
        vec![vec![
            bounds.left_top(),
            bounds.right_top(),
            bounds.right_bottom(),
            bounds.left_bottom(),
        ]],
        crate::box_decoration::FillRule::Nonzero,
    );
    let meshes = context
        .tessellate(shapes, context.pixels_per_point())
        .into_iter()
        .filter_map(|primitive| match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => {
                let mut mesh = crate::box_decoration::clip_mesh_to_paths(mesh, &clip);
                if mesh.indices.is_empty() {
                    return None;
                }
                for vertex in &mut mesh.vertices {
                    vertex.pos -= bounds.min.to_vec2();
                }
                Some(mesh)
            }
            egui::epaint::Primitive::Callback(_) => None,
        })
        .collect::<Vec<_>>();
    let mut textures = HashMap::new();
    for texture_id in meshes
        .iter()
        .map(|mesh| mesh.texture_id)
        .collect::<HashSet<_>>()
    {
        let image = if texture_id == egui::TextureId::default() {
            let image = context.fonts_mut(|fonts| fonts.image());
            crate::background::remember_texture_image(context, texture_id, image);
            crate::background::texture_image(context, texture_id)
        } else {
            crate::background::texture_image(context, texture_id)
        };
        if let Some(image) = image {
            textures.insert(texture_id, image);
        }
    }
    let fingerprint = element_image_fingerprint(bounds.size(), &meshes, &textures);
    Some(ElementImage {
        size: bounds.size(),
        meshes,
        textures,
        dependencies,
        fingerprint,
        captured_frame: context.cumulative_frame_nr(),
    })
}

pub(super) fn element_image_fingerprint(
    size: Vec2,
    meshes: &[egui::epaint::Mesh],
    textures: &HashMap<egui::TextureId, Arc<egui::ColorImage>>,
) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    size.x.to_bits().hash(&mut hasher);
    size.y.to_bits().hash(&mut hasher);
    for mesh in meshes {
        mesh.texture_id.hash(&mut hasher);
        mesh.indices.hash(&mut hasher);
        for vertex in &mesh.vertices {
            vertex.pos.x.to_bits().hash(&mut hasher);
            vertex.pos.y.to_bits().hash(&mut hasher);
            vertex.uv.x.to_bits().hash(&mut hasher);
            vertex.uv.y.to_bits().hash(&mut hasher);
            vertex.color.to_array().hash(&mut hasher);
        }
        if let Some(image) = textures.get(&mesh.texture_id) {
            image.size.hash(&mut hasher);
            for pixel in &image.pixels {
                pixel.to_array().hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

pub(super) fn publish_element_image(context: &Context, id: String, image: ElementImage) {
    let changed = context.data_mut(|data| {
        let key = Id::unique(ELEMENT_IMAGES);
        let mut images = data
            .get_temp::<HashMap<String, Arc<ElementImage>>>(key)
            .unwrap_or_default();
        let changed = images.get(&id).is_none_or(|previous| {
            previous.fingerprint != image.fingerprint || previous.dependencies != image.dependencies
        });
        images.insert(id, Arc::new(image));
        data.insert_temp(key, images);
        changed
    });
    if changed {
        context.request_repaint();
    }
}

pub(super) fn remove_element_image(context: &Context, id: &str) {
    let removed = context.data_mut(|data| {
        let key = Id::unique(ELEMENT_IMAGES);
        let Some(mut images) = data.get_temp::<HashMap<String, Arc<ElementImage>>>(key) else {
            return false;
        };
        let removed = images.remove(id).is_some();
        data.insert_temp(key, images);
        removed
    });
    if removed {
        context.request_repaint();
    }
}

pub(super) fn first_element_occurrence(context: &Context, id: &str) -> bool {
    let pass = context.cumulative_pass_nr();
    context.data_mut(|data| {
        let key = Id::unique(ELEMENT_OCCURRENCES);
        let (seen_pass, mut seen) = data
            .get_temp::<(u64, HashSet<String>)>(key)
            .filter(|(seen_pass, _)| *seen_pass == pass)
            .unwrap_or_else(|| (pass, HashSet::new()));
        let first = seen.insert(id.to_owned());
        data.insert_temp(key, (seen_pass, seen));
        first
    })
}

pub(super) fn element_source_is_eligible(frame: &Frame, index: usize) -> bool {
    let node = &frame.nodes[index];
    if node.absent || node.style.contents {
        return false;
    }
    let style = &node.style;
    if style.fixed
        || matches!(style.layout.position, Position::Fixed | Position::Sticky)
        || style.positioned && style.z_index.is_some()
    {
        return true;
    }
    if style.z_index.is_none() {
        return false;
    }
    let mut parent = node.parent;
    while let Some(index) = parent {
        let ancestor = &frame.nodes[index];
        if ancestor.style.contents {
            parent = ancestor.parent;
            continue;
        }
        return matches!(ancestor.style.layout.display, Display::Flex | Display::Grid);
    }
    false
}

pub(super) fn element_image(context: &Context, id: &str) -> Option<Arc<ElementImage>> {
    let image = context.data(|data| {
        data.get_temp::<HashMap<String, Arc<ElementImage>>>(Id::unique(ELEMENT_IMAGES))
            .and_then(|images| images.get(id).cloned())
    })?;
    (context
        .cumulative_frame_nr()
        .saturating_sub(image.captured_frame)
        <= 1)
        .then_some(image)
}

pub(super) fn element_slice_shapes(
    image: &ElementImage,
    source: Rect,
    target: Rect,
) -> Vec<egui::Shape> {
    if !source.is_positive() || !target.is_positive() {
        return Vec::new();
    }
    let clip = crate::box_decoration::path_clip(
        vec![vec![
            source.left_top(),
            source.right_top(),
            source.right_bottom(),
            source.left_bottom(),
        ]],
        crate::box_decoration::FillRule::Nonzero,
    );
    let scale = target.size() / source.size();
    image
        .meshes
        .iter()
        .filter_map(|source_mesh| {
            let mut mesh = crate::box_decoration::clip_mesh_to_paths(source_mesh.clone(), &clip);
            if mesh.indices.is_empty() {
                return None;
            }
            for vertex in &mut mesh.vertices {
                vertex.pos = target.min + (vertex.pos - source.min) * scale;
            }
            Some(egui::Shape::Mesh(mesh.into()))
        })
        .collect()
}
