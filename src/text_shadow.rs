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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shadow {
    pub(crate) offset: Vec2,
    pub(crate) blur: f32,
    pub(crate) color: Option<Color32>,
}

pub(crate) fn resolve_current_color(shadows: &mut [Shadow], color: Color32) {
    for shadow in shadows {
        if shadow.color.is_none() {
            shadow.color = Some(color);
        }
    }
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
}

struct EvictedMask {
    frame: u64,
    texture: egui::TextureHandle,
}

#[derive(Default)]
struct Cache {
    masks: VecDeque<CachedMask>,
    evicted: VecDeque<EvictedMask>,
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
    if shadows.is_empty() {
        return Shape::Text(text.clone());
    }
    let glyph_bounds = glyph_bounds(text);
    if !glyph_bounds.is_positive() {
        return Shape::Text(text.clone());
    }
    let glyphs = glyph_key(text);
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
        let output_bounds = glyph_bounds
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
                source_meshes = Some(glyph_meshes(context, text));
            }
            let meshes = source_meshes.as_ref()?;
            let (image, bounds) =
                crate::layout::rasterize_alpha_mask(context, meshes, output_bounds, shadow.blur)?;
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
            };
            insert_mask(&cache, mask, context.cumulative_frame_nr());
            cached_mask(&cache, key)
        });
        if let Some(mask) = mask {
            shapes.push(mask_shape(mask, text.pos, shadow.offset, color));
        }
    }
    shapes.push(Shape::Text(text.clone()));
    Shape::Vec(shapes)
}

fn mask_shape(mask: CachedMask, position: Pos2, offset: Vec2, color: Color32) -> Shape {
    let rect = mask.bounds.translate(position.to_vec2() + offset);
    let mut mesh = Mesh::with_texture(mask.texture.id());
    mesh.add_rect_with_uv(rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), color);
    Shape::Mesh(mesh.into())
}

fn glyph_meshes(context: &Context, text: &TextShape) -> Vec<Mesh> {
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
        let row = Arc::make_mut(&mut placed.row);
        let original = &row.visuals.mesh;
        let range = row.visuals.glyph_vertex_range.clone();
        let start = range.start.min(original.vertices.len());
        let end = range.end.min(original.vertices.len()).max(start);
        let mut mesh = Mesh::with_texture(original.texture_id);
        mesh.vertices
            .extend_from_slice(&original.vertices[start..end]);
        for vertex in &mut mesh.vertices {
            vertex.color = Color32::WHITE;
        }
        for &[a, b, c] in original.indices.as_chunks::<3>().0 {
            let [a, b, c] = [a as usize, b as usize, c as usize];
            if (start..end).contains(&a) && (start..end).contains(&b) && (start..end).contains(&c) {
                mesh.indices
                    .extend([(a - start) as u32, (b - start) as u32, (c - start) as u32]);
            }
        }
        for glyph in &mut row.glyphs {
            glyph.first_vertex = (glyph.first_vertex as usize)
                .saturating_sub(start)
                .min(mesh.vertices.len()) as u32;
        }
        row.visuals.glyph_vertex_range = 0..mesh.vertices.len();
        row.visuals.glyph_index_start = 0;
        row.visuals.mesh_bounds = mesh.calc_bounds();
        galley.num_vertices += mesh.vertices.len();
        galley.num_indices += mesh.indices.len();
        galley.mesh_bounds = galley
            .mesh_bounds
            .union(row.visuals.mesh_bounds.translate(placed.pos.to_vec2()));
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

fn glyph_bounds(text: &TextShape) -> Rect {
    let rotation = egui::emath::Rot2::from_angle(text.angle);
    text.galley
        .rows
        .iter()
        .flat_map(|placed| {
            let range = placed.row.visuals.glyph_vertex_range.clone();
            placed.row.visuals.mesh.vertices[range]
                .iter()
                .map(move |vertex| rotation * (placed.pos.to_vec2() + vertex.pos.to_vec2()))
        })
        .fold(Rect::NOTHING, |bounds, point| {
            let point = Pos2::ZERO + point;
            bounds.union(Rect::from_min_max(point, point))
        })
}

fn glyph_key(text: &TextShape) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.angle.to_bits().hash(&mut hasher);
    text.opacity_factor.to_bits().hash(&mut hasher);
    text.galley.pixels_per_point.to_bits().hash(&mut hasher);
    text.galley.rows.len().hash(&mut hasher);
    for placed in &text.galley.rows {
        placed.pos.x.to_bits().hash(&mut hasher);
        placed.pos.y.to_bits().hash(&mut hasher);
        let row = &placed.row;
        let range = row.visuals.glyph_vertex_range.clone();
        range.start.hash(&mut hasher);
        range.end.hash(&mut hasher);
        row.visuals.mesh.texture_id.hash(&mut hasher);
        for vertex in &row.visuals.mesh.vertices[range.clone()] {
            vertex.pos.x.to_bits().hash(&mut hasher);
            vertex.pos.y.to_bits().hash(&mut hasher);
            vertex.uv.x.to_bits().hash(&mut hasher);
            vertex.uv.y.to_bits().hash(&mut hasher);
        }
        for triangle in row.visuals.mesh.indices.as_chunks::<3>().0 {
            if triangle
                .iter()
                .all(|index| range.contains(&(*index as usize)))
            {
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
    };
    cache.masks.push_back(mask);
    Some(cached)
}

fn insert_mask(cache: &Arc<Mutex<Cache>>, mask: CachedMask, frame: u64) {
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.masks.retain(|cached| cached.key != mask.key);
    cache.masks.push_back(mask);
    if cache.masks.len() > CACHE_CAPACITY
        && let Some(mask) = cache.masks.pop_front()
    {
        cache.evicted.push_back(EvictedMask {
            frame,
            texture: mask.texture,
        });
    }
}
