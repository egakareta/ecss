use std::{
    collections::VecDeque,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex},
};

use cssparser::Parser;
use egui::epaint::{Mesh, Shape, TextShape};
use egui::{Color32, Context, Id, Pos2, Rect, Stroke, Vec2, pos2, vec2};

use crate::layout_style::Units;

const CACHE_ID: &str = "ecss.text-shadow-cache";
const CACHE_CAPACITY: usize = 48;
pub(crate) const MAX_MASK_PIXELS: usize = 1_048_576;
const MAX_MASK_BYTES: usize = 8 * 1024 * 1024;
const MAX_ACTIVE_BYTES: usize = 48 * 1024 * 1024;
const MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shadow {
    pub(crate) offset: Vec2,
    pub(crate) blur: f32,
    pub(crate) color: Option<Color32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MaskKey {
    glyphs: u64,
    blur: u32,
    bounds: [u32; 4],
}

struct CachedMask {
    key: MaskKey,
    texture: egui::TextureHandle,
    bounds: Rect,
    bytes: usize,
}

struct EvictedMask {
    frame: u64,
    texture: egui::TextureHandle,
    bytes: usize,
}

#[derive(Default)]
struct Cache {
    masks: VecDeque<CachedMask>,
    evicted: VecDeque<EvictedMask>,
    active_bytes: usize,
    retained_bytes: usize,
}

pub(crate) fn supported(name: &str) -> bool {
    name == "text-shadow"
}

pub(crate) fn parse(value: &str, units: Units<'_>) -> Result<Vec<Shadow>, &'static str> {
    if value.trim().eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    let values = crate::box_decoration::split_commas(value);
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err("empty text shadow");
    }
    values
        .into_iter()
        .map(|value| parse_shadow(value, units))
        .collect()
}

fn parse_shadow(value: &str, units: Units<'_>) -> Result<Shadow, &'static str> {
    let values = crate::layout_style::components(value)?;
    let mut color = None;
    let mut color_seen = false;
    let mut lengths = Vec::new();
    for value in values {
        if let Some(parsed) = color_value(&value) {
            if color_seen {
                return Err("duplicate text shadow color");
            }
            color_seen = true;
            color = parsed;
        } else {
            if lengths.len() == 3 {
                return Err("invalid text shadow");
            }
            lengths.push(shadow_length(&value, units, lengths.len() < 2)?);
        }
    }
    if !(2..=3).contains(&lengths.len()) {
        return Err("text shadow requires two offsets");
    }
    Ok(Shadow {
        offset: vec2(lengths[0], lengths[1]),
        blur: lengths.get(2).copied().unwrap_or(0.0),
        color,
    })
}

fn shadow_length(value: &str, units: Units<'_>, negative: bool) -> Result<f32, &'static str> {
    if let Some(handle) = crate::math::length(value, units, negative, false)? {
        return Ok(crate::math::resolve(handle, 0.0));
    }
    let (value, percentage) = crate::layout_style::length_value(value, units, negative)?;
    if percentage {
        return Err("text shadow lengths cannot be percentages");
    }
    Ok(value)
}

fn color_value(value: &str) -> Option<Option<Color32>> {
    if value.eq_ignore_ascii_case("currentcolor") {
        return Some(None);
    }
    crate::css::parse_color(&mut Parser::new(value))
        .ok()
        .map(Some)
}

pub(crate) fn paint(
    context: &Context,
    text: &TextShape,
    clip: Rect,
    shadows: &[Shadow],
    current_color: Color32,
) -> Shape {
    paint_with_selection(context, text, clip, shadows, current_color, None, true)
}

pub(crate) fn paint_selected(
    context: &Context,
    text: &TextShape,
    clip: Rect,
    shadows: &[Shadow],
    current_color: Color32,
    selection: &[Rect],
) -> Shape {
    paint_with_selection(
        context,
        text,
        clip,
        shadows,
        current_color,
        Some(selection),
        false,
    )
}

fn paint_with_selection(
    context: &Context,
    text: &TextShape,
    clip: Rect,
    shadows: &[Shadow],
    current_color: Color32,
    selection: Option<&[Rect]>,
    include_text: bool,
) -> Shape {
    if shadows.is_empty() {
        return if include_text {
            Shape::Text(text.clone())
        } else {
            Shape::Noop
        };
    }
    let mask_bounds = mask_bounds(text, selection);
    if !mask_bounds.is_positive() {
        return if include_text {
            Shape::Text(text.clone())
        } else {
            Shape::Noop
        };
    }
    if selection.is_some_and(<[Rect]>::is_empty) {
        return Shape::Noop;
    }
    let glyphs = glyph_key(text, selection);
    let cache = cache(context);
    let mut source_meshes = None;
    let mut shapes = Vec::with_capacity(shadows.len() + 1);
    for shadow in shadows.iter().rev() {
        let color = shadow.color.unwrap_or(current_color);
        if color.a() == 0 {
            continue;
        }
        let padding = if shadow.blur.is_finite() {
            (shadow.blur * 1.5).min(1_000_000.0)
        } else {
            1_000_000.0
        };
        let output_bounds = mask_bounds
            .expand(padding)
            .intersect(clip.translate(-(text.pos.to_vec2() + shadow.offset)));
        if !output_bounds.is_positive() {
            continue;
        }
        let key = MaskKey {
            glyphs,
            blur: shadow.blur.to_bits(),
            bounds: [
                output_bounds.min.x.to_bits(),
                output_bounds.min.y.to_bits(),
                output_bounds.max.x.to_bits(),
                output_bounds.max.y.to_bits(),
            ],
        };
        let mask = cached_mask(&cache, key).or_else(|| {
            if source_meshes.is_none() {
                source_meshes = Some(glyph_meshes(context, text, selection));
            }
            let meshes = source_meshes.as_ref()?;
            let (image, bounds) =
                crate::layout::rasterize_alpha_mask(context, meshes, output_bounds, shadow.blur)?;
            let bytes = image
                .pixels
                .len()
                .checked_mul(std::mem::size_of::<Color32>() * 2)?;
            if bytes > MAX_MASK_BYTES || !reserve_mask(&cache, bytes, context.cumulative_frame_nr())
            {
                return None;
            }
            let texture = context.load_texture(
                format!("ecss-text-shadow-{:016x}-{:08x}", key.glyphs, key.blur),
                image.clone(),
                egui::TextureOptions::LINEAR,
            );
            crate::background::remember_texture_image(context, texture.id(), image);
            let mask = CachedMask {
                key,
                texture,
                bounds,
                bytes,
            };
            insert_reserved_mask(&cache, mask, context.cumulative_frame_nr());
            cached_mask(&cache, key)
        });
        if let Some(mask) = mask {
            shapes.push(mask_shape(mask, text.pos, shadow.offset, color));
        }
    }
    if include_text {
        shapes.push(Shape::Text(text.clone()));
    }
    if shapes.is_empty() {
        Shape::Noop
    } else {
        Shape::Vec(shapes)
    }
}

fn mask_shape(mask: CachedMask, position: Pos2, offset: Vec2, color: Color32) -> Shape {
    let rect = mask.bounds.translate(position.to_vec2() + offset);
    let mut mesh = Mesh::with_texture(mask.texture.id());
    mesh.add_rect_with_uv(rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), color);
    Shape::Mesh(mesh.into())
}

fn glyph_meshes(context: &Context, text: &TextShape, selection: Option<&[Rect]>) -> Vec<Mesh> {
    let mut glyph_text = text.clone();
    glyph_text.pos = Pos2::ZERO;
    glyph_text.underline = Stroke::NONE;
    glyph_text.fallback_color = Color32::WHITE;
    glyph_text.override_text_color = Some(Color32::WHITE);
    let galley = Arc::make_mut(&mut glyph_text.galley);
    galley.num_vertices = 0;
    galley.num_indices = 0;
    galley.mesh_bounds = Rect::NOTHING;
    for placed in &mut galley.rows {
        let position = placed.pos;
        let row = Arc::make_mut(&mut placed.row);
        let original = &row.visuals.mesh;
        let ranges = selection.map_or_else(
            || {
                vec![
                    row.visuals.glyph_vertex_range.clone(),
                    row.visuals.glyph_vertex_range.end..original.vertices.len(),
                ]
            },
            |selection| selected_glyph_ranges(text, position, row, selection),
        );
        let mut mesh = Mesh::with_texture(original.texture_id);
        let mut remap = vec![None; original.vertices.len()];
        for range in &ranges {
            let start = range.start.min(original.vertices.len());
            let end = range.end.min(original.vertices.len()).max(start);
            let destination = mesh.vertices.len() as u32;
            for (offset, vertex) in original.vertices[start..end].iter().enumerate() {
                remap[start + offset] = Some(destination + offset as u32);
                let mut vertex = *vertex;
                let in_glyph_range = row.visuals.glyph_vertex_range.contains(&(start + offset));
                vertex.color = if in_glyph_range {
                    Color32::WHITE
                } else {
                    Color32::from_white_alpha(vertex.color.a())
                };
                mesh.vertices.push(vertex);
            }
        }
        for &[a, b, c] in original.indices.as_chunks::<3>().0 {
            let [a, b, c] = [a as usize, b as usize, c as usize];
            let included = ranges
                .iter()
                .any(|range| range.contains(&a) && range.contains(&b) && range.contains(&c));
            if included && let (Some(a), Some(b), Some(c)) = (remap[a], remap[b], remap[c]) {
                mesh.indices.extend([a, b, c]);
            }
        }
        for glyph in &mut row.glyphs {
            glyph.first_vertex = remap
                .get(glyph.first_vertex as usize)
                .copied()
                .flatten()
                .unwrap_or(0);
        }
        row.visuals.glyph_vertex_range = 0..mesh.vertices.len();
        row.visuals.glyph_index_start = 0;
        row.visuals.mesh_bounds = mesh.calc_bounds();
        galley.num_vertices += mesh.vertices.len();
        galley.num_indices += mesh.indices.len();
        galley.mesh_bounds = galley
            .mesh_bounds
            .union(row.visuals.mesh_bounds.translate(position.to_vec2()));
        row.visuals.mesh = mesh;
    }
    context
        .tessellate(
            vec![egui::epaint::ClippedShape::new(
                Rect::EVERYTHING,
                Shape::Text(glyph_text),
            )],
            context.pixels_per_point(),
        )
        .into_iter()
        .filter_map(|primitive| match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => Some(mesh),
            egui::epaint::Primitive::Callback(_) => None,
        })
        .collect()
}

fn selected_glyph_ranges(
    text: &TextShape,
    position: Pos2,
    row: &egui::epaint::text::Row,
    selection: &[Rect],
) -> Vec<std::ops::Range<usize>> {
    let glyph_range = row.visuals.glyph_vertex_range.clone();
    let rotation = egui::emath::Rot2::from_angle(text.angle);
    row.glyphs
        .iter()
        .enumerate()
        .filter_map(|(index, glyph)| {
            let start = (glyph.first_vertex as usize).max(glyph_range.start);
            let end = row
                .glyphs
                .get(index + 1)
                .map_or(glyph_range.end, |next| next.first_vertex as usize)
                .min(glyph_range.end)
                .min(row.visuals.mesh.vertices.len());
            if start >= end {
                return None;
            }
            let bounds = row.visuals.mesh.vertices[start..end]
                .iter()
                .map(|vertex| text.pos + rotation * (position.to_vec2() + vertex.pos.to_vec2()))
                .fold(Rect::NOTHING, |bounds, point| {
                    bounds.union(Rect::from_min_max(point, point))
                });
            selection
                .iter()
                .any(|rect| rect.contains(bounds.center()))
                .then_some(start..end)
        })
        .collect()
}

fn mask_bounds(text: &TextShape, selection: Option<&[Rect]>) -> Rect {
    let rotation = egui::emath::Rot2::from_angle(text.angle);
    text.galley
        .rows
        .iter()
        .flat_map(|placed| {
            let row = &placed.row;
            let ranges = selection.map_or_else(
                || {
                    vec![
                        row.visuals.glyph_vertex_range.clone(),
                        row.visuals.glyph_vertex_range.end..row.visuals.mesh.vertices.len(),
                    ]
                },
                |selection| selected_glyph_ranges(text, placed.pos, row, selection),
            );
            let vertices = &row.visuals.mesh.vertices;
            ranges.into_iter().flat_map(move |range| {
                vertices[range]
                    .iter()
                    .map(move |vertex| rotation * (placed.pos.to_vec2() + vertex.pos.to_vec2()))
            })
        })
        .fold(Rect::NOTHING, |bounds, point| {
            let point = Pos2::ZERO + point;
            bounds.union(Rect::from_min_max(point, point))
        })
}

fn glyph_key(text: &TextShape, selection: Option<&[Rect]>) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.angle.to_bits().hash(&mut hasher);
    text.opacity_factor.to_bits().hash(&mut hasher);
    text.galley.pixels_per_point.to_bits().hash(&mut hasher);
    text.galley.rows.len().hash(&mut hasher);
    if let Some(selection) = selection {
        for rect in selection {
            rect.min.x.to_bits().hash(&mut hasher);
            rect.min.y.to_bits().hash(&mut hasher);
            rect.max.x.to_bits().hash(&mut hasher);
            rect.max.y.to_bits().hash(&mut hasher);
        }
    }
    for placed in &text.galley.rows {
        placed.pos.x.to_bits().hash(&mut hasher);
        placed.pos.y.to_bits().hash(&mut hasher);
        let row = &placed.row;
        row.visuals.mesh.texture_id.hash(&mut hasher);
        let ranges = [
            row.visuals.glyph_vertex_range.clone(),
            row.visuals.glyph_vertex_range.end..row.visuals.mesh.vertices.len(),
        ];
        for range in &ranges {
            range.start.hash(&mut hasher);
            range.end.hash(&mut hasher);
            for vertex in &row.visuals.mesh.vertices[range.clone()] {
                vertex.pos.x.to_bits().hash(&mut hasher);
                vertex.pos.y.to_bits().hash(&mut hasher);
                vertex.uv.x.to_bits().hash(&mut hasher);
                vertex.uv.y.to_bits().hash(&mut hasher);
                vertex.color.a().hash(&mut hasher);
            }
        }
        for triangle in row.visuals.mesh.indices.as_chunks::<3>().0 {
            if triangle.iter().all(|index| {
                ranges
                    .iter()
                    .any(|range| range.contains(&(*index as usize)))
            }) {
                triangle.hash(&mut hasher);
            }
        }
        for glyph in &row.glyphs {
            glyph.first_vertex.hash(&mut hasher);
            glyph.is_color.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn cache(context: &Context) -> Arc<Mutex<Cache>> {
    let cache = context.data_mut(|data| {
        let id = Id::unique(CACHE_ID);
        if let Some(cache) = data.get_temp::<Arc<Mutex<Cache>>>(id) {
            cache
        } else {
            let cache = Arc::new(Mutex::new(Cache::default()));
            data.insert_temp(id, cache.clone());
            cache
        }
    });
    let frame = context.cumulative_frame_nr();
    let expired = {
        let mut cache = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut ids = Vec::new();
        while cache
            .evicted
            .front()
            .is_some_and(|entry| entry.frame < frame)
        {
            if let Some(entry) = cache.evicted.pop_front() {
                cache.retained_bytes = cache.retained_bytes.saturating_sub(entry.bytes);
                ids.push(entry.texture.id());
            }
        }
        ids
    };
    for id in expired {
        crate::background::forget_texture_image(context, id);
    }
    cache
}

fn cached_mask(cache: &Arc<Mutex<Cache>>, key: MaskKey) -> Option<CachedMask> {
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let index = cache.masks.iter().position(|mask| mask.key == key)?;
    let mask = cache.masks.remove(index)?;
    let cached = CachedMask {
        key: mask.key,
        texture: mask.texture.clone(),
        bounds: mask.bounds,
        bytes: mask.bytes,
    };
    cache.masks.push_back(mask);
    Some(cached)
}

fn reserve_mask(cache: &Arc<Mutex<Cache>>, bytes: usize, frame: u64) -> bool {
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if bytes > MAX_MASK_BYTES
        || cache
            .retained_bytes
            .checked_add(bytes)
            .is_none_or(|total| total > MAX_RETAINED_BYTES)
    {
        return false;
    }
    while cache.active_bytes + bytes > MAX_ACTIVE_BYTES || cache.masks.len() >= CACHE_CAPACITY {
        let Some(mask) = cache.masks.pop_front() else {
            return false;
        };
        cache.active_bytes = cache.active_bytes.saturating_sub(mask.bytes);
        cache.evicted.push_back(EvictedMask {
            frame,
            texture: mask.texture,
            bytes: mask.bytes,
        });
    }
    cache.active_bytes += bytes;
    cache.retained_bytes += bytes;
    true
}

fn insert_reserved_mask(cache: &Arc<Mutex<Cache>>, mask: CachedMask, frame: u64) {
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(index) = cache.masks.iter().position(|cached| cached.key == mask.key)
        && let Some(previous) = cache.masks.remove(index)
    {
        cache.active_bytes = cache.active_bytes.saturating_sub(previous.bytes);
        cache.evicted.push_back(EvictedMask {
            frame,
            texture: previous.texture,
            bytes: previous.bytes,
        });
    }
    cache.masks.push_back(mask);
}
