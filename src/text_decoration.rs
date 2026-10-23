use crate::layout_style::{Units, border_color, components, length_value};
use egui::{Color32, Context, Rect, Shape, Stroke, pos2};
use std::sync::Arc;

const UNDERLINE: u8 = 1;
const OVERLINE: u8 = 2;
const LINE_THROUGH: u8 = 4;
const BLINK: u8 = 8;

#[derive(Clone, Copy, Debug, Default)]
enum LineStyle {
    #[default]
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Clone, Debug, Default)]
enum Thickness {
    #[default]
    Auto,
    FromFont,
    Value(crate::math::ComputedLength),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TextDecoration {
    lines: u8,
    style: LineStyle,
    pub(crate) color: Option<Color32>,
    thickness: Thickness,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Decoration {
    lines: u8,
    style: LineStyle,
    color: Color32,
    thickness: f32,
    em: f32,
}

impl TextDecoration {
    pub(crate) fn resolve(&self, color: Color32, em: f32) -> Option<Decoration> {
        (self.lines & !BLINK != 0).then(|| Decoration {
            lines: self.lines,
            style: self.style,
            color: self.color.unwrap_or(color),
            thickness: match &self.thickness {
                // egui does not expose underline metrics for from-font.
                Thickness::Auto | Thickness::FromFont => em / 14.0,
                Thickness::Value(value) => value.resolve(em),
            },
            em,
        })
    }
}

pub(crate) fn supported(name: &str) -> bool {
    matches!(
        name,
        "text-decoration"
            | "text-decoration-line"
            | "text-decoration-style"
            | "text-decoration-color"
            | "text-decoration-thickness"
    )
}

fn line(value: &str) -> Option<u8> {
    Some(match value.to_ascii_lowercase().as_str() {
        "none" => 0,
        "underline" => UNDERLINE,
        "overline" => OVERLINE,
        "line-through" => LINE_THROUGH,
        "blink" => BLINK,
        _ => return None,
    })
}

fn style(value: &str) -> Result<LineStyle, &'static str> {
    Ok(match value.to_ascii_lowercase().as_str() {
        "solid" => LineStyle::Solid,
        "double" => LineStyle::Double,
        "dotted" => LineStyle::Dotted,
        "dashed" => LineStyle::Dashed,
        "wavy" => LineStyle::Wavy,
        _ => return Err("invalid text decoration style"),
    })
}

fn thickness(value: &str, units: Units) -> Result<Thickness, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "auto" => return Ok(Thickness::Auto),
        "from-font" => return Ok(Thickness::FromFont),
        _ => {}
    }
    if let Some(handle) = crate::math::length(value, units, false, true)? {
        return Ok(Thickness::Value(crate::math::ComputedLength::from_handle(
            handle,
        )));
    }
    let (value, percentage) = length_value(value, units, false)?;
    Ok(Thickness::Value(if percentage {
        crate::math::ComputedLength::percentage(value)
    } else {
        crate::math::ComputedLength::length(value)
    }))
}

pub(crate) fn apply(
    name: &str,
    value: &str,
    out: &mut TextDecoration,
    units: Units,
) -> Result<(), &'static str> {
    let values = components(value)?;
    if values.is_empty() {
        return Err("empty text decoration");
    }
    if !matches!(name, "text-decoration" | "text-decoration-line") && values.len() != 1 {
        return Err("expected one text decoration value");
    }
    match name {
        "text-decoration-color" => out.color = border_color(value)?,
        "text-decoration-style" => out.style = style(value)?,
        "text-decoration-thickness" => out.thickness = thickness(value, units)?,
        "text-decoration" | "text-decoration-line" => {
            let mut result = TextDecoration::default();
            let mut lines = None;
            let mut line_style = None;
            let mut color = None;
            let mut width = None;
            for value in values {
                if let Some(parsed) = line(&value) {
                    if let Some(previous) = lines
                        && (previous == 0 || parsed == 0 || previous & parsed != 0)
                    {
                        return Err("duplicate or incompatible text decoration lines");
                    }
                    lines = Some(lines.unwrap_or(0) | parsed);
                } else if name == "text-decoration-line" {
                    return Err("invalid text decoration line");
                } else if let Ok(parsed) = style(&value) {
                    if line_style.replace(parsed).is_some() {
                        return Err("duplicate text decoration style");
                    }
                } else if let Ok(parsed) = thickness(&value, units) {
                    if width.replace(parsed).is_some() {
                        return Err("duplicate text decoration thickness");
                    }
                } else if color.replace(border_color(&value)?).is_some() {
                    return Err("duplicate text decoration color");
                }
            }
            result.lines = lines.unwrap_or(0);
            if name == "text-decoration-line" {
                out.lines = result.lines;
            } else {
                result.style = line_style.unwrap_or_default();
                result.color = color.flatten();
                result.thickness = width.unwrap_or_default();
                *out = result;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(crate) fn copy_property(name: &str, out: &mut TextDecoration, source: TextDecoration) {
    match name {
        "text-decoration" => *out = source,
        "text-decoration-line" => out.lines = source.lines,
        "text-decoration-style" => out.style = source.style,
        "text-decoration-color" => out.color = source.color,
        "text-decoration-thickness" => out.thickness = source.thickness,
        _ => unreachable!(),
    }
}

pub(crate) fn decorate_galley(
    context: &Context,
    galley: &mut Arc<egui::Galley>,
    decorations: &[Decoration],
    trailing_space: f32,
) {
    if decorations.is_empty() {
        return;
    }
    let galley = Arc::make_mut(galley);
    for placed in &mut galley.rows {
        let glyphs = &placed.row.glyphs;
        let Some(first) = glyphs.iter().find(|glyph| !glyph.chr.is_whitespace()) else {
            continue;
        };
        let last = glyphs
            .iter()
            .rfind(|glyph| !glyph.chr.is_whitespace())
            .unwrap();
        let (left, right, baseline, ascent) = (
            first.pos.x,
            last.max_x() + trailing_space,
            first.pos.y,
            first.font_ascent,
        );
        let mut below = Vec::new();
        let mut above = Vec::new();
        for decoration in decorations {
            let thickness = decoration.thickness.max(1.0 / context.pixels_per_point());
            for (line, y) in [
                (UNDERLINE, baseline + decoration.em * 0.08),
                (OVERLINE, baseline - ascent),
                (LINE_THROUGH, baseline - decoration.em * 0.3),
            ] {
                if decoration.lines & line != 0 {
                    let shapes = if line == LINE_THROUGH {
                        &mut above
                    } else {
                        &mut below
                    };
                    line_shapes(shapes, left, right, y, thickness, *decoration);
                }
            }
        }
        let row = Arc::make_mut(&mut placed.row);
        let foreground = row
            .visuals
            .mesh
            .indices
            .split_off(row.visuals.glyph_index_start);
        append_shapes(context, &mut row.visuals.mesh, below);
        row.visuals.glyph_index_start = row.visuals.mesh.indices.len();
        row.visuals.mesh.indices.extend(foreground);
        append_shapes(context, &mut row.visuals.mesh, above);
        row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
    }
    galley.num_vertices = galley
        .rows
        .iter()
        .map(|row| row.visuals.mesh.vertices.len())
        .sum();
    galley.num_indices = galley
        .rows
        .iter()
        .map(|row| row.visuals.mesh.indices.len())
        .sum();
    galley.mesh_bounds = galley.rows.iter().fold(Rect::NOTHING, |bounds, row| {
        bounds.union(row.visuals.mesh_bounds.translate(row.pos.to_vec2()))
    });
}

fn append_shapes(context: &Context, mesh: &mut egui::Mesh, shapes: Vec<Shape>) {
    if shapes.is_empty() {
        return;
    }
    let primitives = context.tessellate(
        vec![egui::epaint::ClippedShape::new(
            Rect::EVERYTHING,
            Shape::Vec(shapes),
        )],
        context.pixels_per_point(),
    );
    let texture_size = context.fonts(|fonts| fonts.font_image_size());
    for primitive in primitives {
        if let egui::epaint::Primitive::Mesh(mut decoration) = primitive.primitive {
            // Row meshes store texel coordinates; tessellation returns normalized UVs,
            // including atlas-backed primitives such as small dotted-line circles.
            for vertex in &mut decoration.vertices {
                vertex.uv.x *= texture_size[0] as f32;
                vertex.uv.y *= texture_size[1] as f32;
            }
            mesh.append(decoration);
        }
    }
}

fn line_shapes(
    out: &mut Vec<Shape>,
    left: f32,
    right: f32,
    y: f32,
    thickness: f32,
    decoration: Decoration,
) {
    let color = decoration.color;
    let rect = |left, right, y| {
        Shape::rect_filled(
            Rect::from_min_max(
                pos2(left, y - thickness * 0.5),
                pos2(right, y + thickness * 0.5),
            ),
            0.0,
            color,
        )
    };
    match decoration.style {
        LineStyle::Solid => out.push(rect(left, right, y)),
        LineStyle::Double => {
            out.push(rect(left, right, y - thickness));
            out.push(rect(left, right, y + thickness));
        }
        LineStyle::Dashed | LineStyle::Dotted => {
            let dotted = matches!(decoration.style, LineStyle::Dotted);
            let length = thickness * if dotted { 1.0 } else { 3.0 };
            let mut x = left;
            while x < right {
                let end = (x + length).min(right);
                if dotted {
                    let radius = ((end - x) * 0.5).min(thickness * 0.5);
                    out.push(Shape::circle_filled(pos2(x + radius, y), radius, color));
                } else {
                    out.push(rect(x, end, y));
                }
                x += length + thickness * 2.0;
            }
        }
        LineStyle::Wavy => {
            let wavelength = thickness * 6.0;
            let steps = ((right - left) / (thickness * 0.5)).ceil().max(1.0) as usize;
            let points = (0..=steps)
                .map(|step| {
                    let x = left + (right - left) * step as f32 / steps as f32;
                    pos2(
                        x,
                        y + ((x - left) * std::f32::consts::TAU / wavelength).sin() * thickness,
                    )
                })
                .collect();
            out.push(Shape::line(points, Stroke::new(thickness, color)));
        }
    }
}
