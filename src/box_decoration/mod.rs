use cssparser::Parser;
use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};

use crate::layout_style::{BoxStyle, Units};

mod border_image;
mod paths;
mod shadows;
mod shape_parser;

use border_image::apply_border_image;
pub(crate) use border_image::{border_image_source, border_image_urls};
use paths::filled_paths_difference;
pub(crate) use paths::{
    clip_mesh_to_paths, filled_path, filled_paths, path_clip, rounded_corner_segments,
    rounded_path, rounded_path_with_segments,
};
use shadows::{
    apply_shadow_longhand, color_value, copy_shadows, cross_vec, offset_filled_paths,
    parse_shadows, set_shadows, split_commas,
};
pub(crate) use shadows::{inset_shadow_extent, shadow_shapes};
use shape_parser::{ArcParameters, append_arc, append_cubic, append_quadratic, parse_border_shape};

const MIN_ROUNDED_CORNER_SEGMENTS: usize = 8;
const MAX_ROUNDED_CORNER_SEGMENTS: usize = 128;
const MAX_ROUNDED_CORNER_ERROR: f32 = 0.02;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LengthPercentage {
    Length(f32),
    Percentage(f32),
    Calc(*const ()),
}

impl Default for LengthPercentage {
    fn default() -> Self {
        Self::Length(0.0)
    }
}

impl LengthPercentage {
    pub(crate) fn resolve(self, basis: f32) -> f32 {
        match self {
            Self::Length(value) => value,
            Self::Percentage(value) => value * basis,
            Self::Calc(handle) => crate::math::resolve(handle, basis),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CornerRadius {
    pub(crate) x: LengthPercentage,
    pub(crate) y: LengthPercentage,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum CornerShape {
    #[default]
    Round,
    Squircle,
    Square,
    Bevel,
    Scoop,
    Notch,
    Superellipse(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ShadowPosition {
    #[default]
    Outset,
    Inset,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shadow {
    pub(crate) offset: Option<Vec2>,
    pub(crate) blur: f32,
    pub(crate) spread: f32,
    pub(crate) color: Option<Color32>,
    pub(crate) position: ShadowPosition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ClipPart {
    Length(LengthPercentage),
    Flex(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BorderStripe {
    pub(crate) color: Option<Color32>,
    pub(crate) thickness: ClipPart,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum BorderLimit {
    #[default]
    All,
    Sides(Option<LengthPercentage>),
    Corners(Option<LengthPercentage>),
    Side(usize, Option<LengthPercentage>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ImageSlice {
    Number(f32),
    Percentage(f32),
    PercentageCalc(*const ()),
}

impl Default for ImageSlice {
    fn default() -> Self {
        Self::Percentage(1.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ImageWidth {
    Length(LengthPercentage),
    #[default]
    NumberOne,
    Number(f32),
    Auto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ImageOutset {
    #[default]
    LengthZero,
    Length(f32),
    Number(f32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ImageRepeat {
    #[default]
    Stretch,
    Repeat,
    Round,
    Space,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BorderImage {
    pub(crate) source: crate::css::BackgroundImage,
    pub(crate) slice: [ImageSlice; 4],
    pub(crate) fill: bool,
    pub(crate) width: [ImageWidth; 4],
    pub(crate) outset: [ImageOutset; 4],
    pub(crate) repeat: [ImageRepeat; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GeometryBox {
    #[default]
    Border,
    Padding,
    Content,
    Margin,
    HalfBorder,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShapeRadius {
    LengthPercentage(LengthPercentage),
    ClosestSide,
    FarthestSide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ShapePosition {
    x: LengthPercentage,
    y: LengthPercentage,
    x_from_end: bool,
    y_from_end: bool,
}

impl Default for ShapePosition {
    fn default() -> Self {
        Self {
            x: LengthPercentage::Percentage(0.5),
            y: LengthPercentage::Percentage(0.5),
            x_from_end: false,
            y_from_end: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum BasicShape {
    Circle {
        radius: Option<ShapeRadius>,
        position: ShapePosition,
    },
    Ellipse {
        radii: [Option<ShapeRadius>; 2],
        position: ShapePosition,
    },
    Inset {
        insets: [LengthPercentage; 4],
        radii: Option<[CornerRadius; 4]>,
    },
    Polygon {
        points: Vec<(LengthPercentage, LengthPercentage)>,
        fill_rule: FillRule,
    },
    Path {
        contours: Vec<Vec<Vec2>>,
        fill_rule: FillRule,
    },
    Shape(ShapeFunction),
    Rect([LengthPercentage; 4]),
    Xywh([LengthPercentage; 4]),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum FillRule {
    #[default]
    Nonzero,
    Evenodd,
}

#[derive(Clone, PartialEq)]
pub(crate) struct PathClip {
    quads: Vec<[Pos2; 4]>,
}

impl PathClip {
    pub(crate) fn bounds(&self) -> Rect {
        self.quads
            .iter()
            .flatten()
            .fold(Rect::NOTHING, |bounds, point| {
                bounds.union(Rect::from_min_max(*point, *point))
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ShapePoint {
    x: LengthPercentage,
    y: LengthPercentage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShapeEndpoint {
    To(ShapePosition),
    By(ShapePoint),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShapeControl {
    Absolute(ShapePosition),
    Relative(ShapePoint, ControlOrigin),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ControlOrigin {
    Start,
    End,
    Origin,
}

#[derive(Clone, Debug, PartialEq)]
enum ShapeCommand {
    Move(ShapeEndpoint),
    Line(ShapeEndpoint),
    Horizontal {
        relative: bool,
        value: LengthPercentage,
    },
    Vertical {
        relative: bool,
        value: LengthPercentage,
    },
    Curve {
        endpoint: ShapeEndpoint,
        controls: Vec<ShapeControl>,
    },
    Smooth {
        endpoint: ShapeEndpoint,
        control: Option<ShapeControl>,
    },
    Arc {
        endpoint: ShapeEndpoint,
        radii: [LengthPercentage; 2],
        direction_agnostic_radius: bool,
        clockwise: bool,
        large: bool,
        rotation: f32,
    },
    Close,
}

#[derive(Clone, Debug, PartialEq)]
struct ShapeFunction {
    start: ShapePosition,
    commands: Vec<ShapeCommand>,
    fill_rule: FillRule,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BorderShape {
    shapes: Vec<(BasicShape, GeometryBox)>,
}

impl BorderShape {
    pub(crate) fn outer_paths(
        &self,
        border: Rect,
        stroke: egui::epaint::MarginF32,
        padding: egui::epaint::MarginF32,
        margin: egui::epaint::MarginF32,
        widths: [f32; 4],
    ) -> Vec<Vec<Pos2>> {
        let (shape, geometry) = &self.shapes[0];
        shape.paths(geometry_rect(
            border, *geometry, stroke, padding, margin, widths,
        ))
    }

    fn outer_fill_rule(&self) -> FillRule {
        self.shapes[0].0.fill_rule()
    }

    pub(crate) fn inner_paths(
        &self,
        border: Rect,
        stroke: egui::epaint::MarginF32,
        padding: egui::epaint::MarginF32,
        margin: egui::epaint::MarginF32,
        widths: [f32; 4],
    ) -> Option<Vec<Vec<Pos2>>> {
        let (shape, geometry) = self.shapes.get(1)?;
        Some(shape.paths(geometry_rect(
            border, *geometry, stroke, padding, margin, widths,
        )))
    }

    fn inner_fill_rule(&self) -> Option<FillRule> {
        self.shapes.get(1).map(|(shape, _)| shape.fill_rule())
    }

    pub(crate) fn background_paths(
        &self,
        border: Rect,
        clip: crate::css::BackgroundClip,
        stroke: egui::epaint::MarginF32,
        padding: egui::epaint::MarginF32,
        margin: egui::epaint::MarginF32,
        widths: [f32; 4],
    ) -> (Vec<Vec<Pos2>>, FillRule) {
        if clip == crate::css::BackgroundClip::Border || self.shapes.len() > 1 {
            let index = if clip == crate::css::BackgroundClip::Border {
                0
            } else {
                1
            };
            let (shape, geometry) = &self.shapes[index.min(self.shapes.len() - 1)];
            return (
                shape.paths(geometry_rect(
                    border, *geometry, stroke, padding, margin, widths,
                )),
                shape.fill_rule(),
            );
        }
        let rect = match clip {
            crate::css::BackgroundClip::Border => border,
            crate::css::BackgroundClip::Padding => border - stroke,
            crate::css::BackgroundClip::Content => border - stroke - padding,
        };
        let shape = &self.shapes[0].0;
        (shape.paths(rect), shape.fill_rule())
    }

    pub(crate) fn overflow_paths(
        &self,
        clip: crate::css::BackgroundClip,
        geometry: OverflowGeometry,
    ) -> (Vec<Vec<Pos2>>, FillRule) {
        let (paths, fill_rule) = self.background_paths(
            geometry.border,
            clip,
            geometry.stroke,
            geometry.padding,
            geometry.margin,
            geometry.widths,
        );
        if geometry.outset > 0.0 {
            (
                offset_filled_paths(&paths, fill_rule, geometry.outset),
                fill_rule,
            )
        } else {
            (paths, fill_rule)
        }
    }

    pub(crate) fn background_is_convex(&self, clip: crate::css::BackgroundClip) -> bool {
        let index = if clip == crate::css::BackgroundClip::Border || self.shapes.len() == 1 {
            0
        } else {
            1
        };
        self.shapes[index].0.is_convex()
    }
}

pub(crate) struct OverflowGeometry {
    pub(crate) border: Rect,
    pub(crate) stroke: egui::epaint::MarginF32,
    pub(crate) padding: egui::epaint::MarginF32,
    pub(crate) margin: egui::epaint::MarginF32,
    pub(crate) widths: [f32; 4],
    pub(crate) outset: f32,
}

impl BasicShape {
    fn is_convex(&self) -> bool {
        !matches!(
            self,
            Self::Polygon { .. } | Self::Path { .. } | Self::Shape(_)
        )
    }

    fn fill_rule(&self) -> FillRule {
        match self {
            Self::Polygon { fill_rule, .. }
            | Self::Path { fill_rule, .. }
            | Self::Shape(ShapeFunction { fill_rule, .. }) => *fill_rule,
            _ => FillRule::Nonzero,
        }
    }

    fn paths(&self, rect: Rect) -> Vec<Vec<Pos2>> {
        match self {
            Self::Path { contours, .. } => contours
                .iter()
                .map(|points| points.iter().map(|point| rect.min + *point).collect())
                .collect(),
            Self::Shape(shape) => shape.paths(rect),
            _ => vec![self.points(rect)],
        }
    }

    fn points(&self, rect: Rect) -> Vec<Pos2> {
        match self {
            Self::Circle { radius, position } => {
                let center = rect.min + shape_position_point(*position, rect);
                let horizontal = (center.x - rect.left()).min(rect.right() - center.x);
                let vertical = (center.y - rect.top()).min(rect.bottom() - center.y);
                let farthest = (center.x - rect.left())
                    .max(rect.right() - center.x)
                    .max(center.y - rect.top())
                    .max(rect.bottom() - center.y);
                let radius = resolve_shape_radius(
                    radius.unwrap_or(ShapeRadius::ClosestSide),
                    (rect.width().powi(2) + rect.height().powi(2)).sqrt() / 2.0_f32.sqrt(),
                    horizontal.min(vertical),
                    farthest,
                );
                ellipse_points(center, vec2(radius, radius))
            }
            Self::Ellipse { radii, position } => {
                let center = rect.min + shape_position_point(*position, rect);
                let horizontal = (center.x - rect.left()).min(rect.right() - center.x);
                let vertical = (center.y - rect.top()).min(rect.bottom() - center.y);
                let farthest_x = (center.x - rect.left()).max(rect.right() - center.x);
                let farthest_y = (center.y - rect.top()).max(rect.bottom() - center.y);
                let rx = resolve_shape_radius(
                    radii[0].unwrap_or(ShapeRadius::ClosestSide),
                    rect.width(),
                    horizontal,
                    farthest_x,
                );
                let ry = resolve_shape_radius(
                    radii[1].unwrap_or(ShapeRadius::ClosestSide),
                    rect.height(),
                    vertical,
                    farthest_y,
                );
                ellipse_points(center, vec2(rx, ry))
            }
            Self::Inset { insets, radii } => {
                let inset = [
                    insets[0].resolve(rect.height()),
                    insets[1].resolve(rect.width()),
                    insets[2].resolve(rect.height()),
                    insets[3].resolve(rect.width()),
                ];
                let inner = Rect::from_min_max(
                    pos2(rect.left() + inset[3], rect.top() + inset[0]),
                    pos2(rect.right() - inset[1], rect.bottom() - inset[2]),
                );
                if let Some(radii) = radii {
                    let resolved = std::array::from_fn(|index| {
                        vec2(
                            radii[index].x.resolve(inner.width()),
                            radii[index].y.resolve(inner.height()),
                        )
                    });
                    rounded_path(inner, resolved, [CornerShape::Round; 4])
                } else {
                    vec![
                        inner.left_top(),
                        inner.right_top(),
                        inner.right_bottom(),
                        inner.left_bottom(),
                    ]
                }
            }
            Self::Polygon { points, .. } => points
                .iter()
                .map(|(x, y)| {
                    pos2(
                        rect.left() + x.resolve(rect.width()),
                        rect.top() + y.resolve(rect.height()),
                    )
                })
                .collect(),
            Self::Path { contours, .. } => contours
                .first()
                .map(|points| points.iter().map(|point| rect.min + *point).collect())
                .unwrap_or_default(),
            Self::Shape(_) => Vec::new(),
            Self::Rect(values) => {
                let top = rect.top() + values[0].resolve(rect.height());
                let right = rect.left() + values[1].resolve(rect.width());
                let bottom = rect.top() + values[2].resolve(rect.height());
                let left = rect.left() + values[3].resolve(rect.width());
                vec![
                    pos2(left, top),
                    pos2(right, top),
                    pos2(right, bottom),
                    pos2(left, bottom),
                ]
            }
            Self::Xywh(values) => {
                let left = rect.left() + values[0].resolve(rect.width());
                let top = rect.top() + values[1].resolve(rect.height());
                let width = values[2].resolve(rect.width());
                let height = values[3].resolve(rect.height());
                vec![
                    pos2(left, top),
                    pos2(left + width, top),
                    pos2(left + width, top + height),
                    pos2(left, top + height),
                ]
            }
        }
    }
}

impl ShapeFunction {
    fn paths(&self, rect: Rect) -> Vec<Vec<Pos2>> {
        let mut current = shape_position_point(self.start, rect);
        let mut subpath_start = current;
        let mut points = vec![current];
        let mut contours = Vec::new();
        let mut cubic_control = None;
        let mut quadratic_control = None;
        for command in &self.commands {
            match command {
                ShapeCommand::Move(endpoint) => {
                    if !points.is_empty() {
                        contours.push(std::mem::take(&mut points));
                    }
                    current = resolve_shape_endpoint(*endpoint, current, rect);
                    subpath_start = current;
                    points.push(current);
                    cubic_control = None;
                    quadratic_control = None;
                }
                ShapeCommand::Line(endpoint) => {
                    current = resolve_shape_endpoint(*endpoint, current, rect);
                    points.push(current);
                    cubic_control = None;
                    quadratic_control = None;
                }
                ShapeCommand::Horizontal { relative, value } => {
                    let x = value.resolve(rect.width());
                    current.x = if *relative { current.x + x } else { x };
                    points.push(current);
                    cubic_control = None;
                    quadratic_control = None;
                }
                ShapeCommand::Vertical { relative, value } => {
                    let y = value.resolve(rect.height());
                    current.y = if *relative { current.y + y } else { y };
                    points.push(current);
                    cubic_control = None;
                    quadratic_control = None;
                }
                ShapeCommand::Curve { endpoint, controls } => {
                    let start = current;
                    let end = resolve_shape_endpoint(*endpoint, start, rect);
                    match controls.as_slice() {
                        [control] => {
                            let control = resolve_shape_control(*control, start, end, rect);
                            append_quadratic(&mut points, start, control, end);
                            quadratic_control = Some(control);
                            cubic_control = None;
                        }
                        [first, second] => {
                            let first = resolve_shape_control(*first, start, end, rect);
                            let second = resolve_shape_control(*second, start, end, rect);
                            append_cubic(&mut points, start, first, second, end);
                            cubic_control = Some(second);
                            quadratic_control = None;
                        }
                        _ => {}
                    }
                    current = end;
                }
                ShapeCommand::Smooth { endpoint, control } => {
                    let start = current;
                    let end = resolve_shape_endpoint(*endpoint, start, rect);
                    let previous_control = cubic_control.or(quadratic_control);
                    if let Some(control) = control {
                        let first =
                            previous_control.map_or(start, |previous| start * 2.0 - previous);
                        let second = resolve_shape_control(*control, start, end, rect);
                        append_cubic(&mut points, start, first, second, end);
                        cubic_control = Some(second);
                        quadratic_control = None;
                    } else {
                        let control =
                            previous_control.map_or(start, |previous| start * 2.0 - previous);
                        append_quadratic(&mut points, start, control, end);
                        quadratic_control = Some(control);
                        cubic_control = None;
                    }
                    current = end;
                }
                ShapeCommand::Arc {
                    endpoint,
                    radii,
                    direction_agnostic_radius,
                    clockwise,
                    large,
                    rotation,
                } => {
                    let end = resolve_shape_endpoint(*endpoint, current, rect);
                    let basis =
                        (rect.width().powi(2) + rect.height().powi(2)).sqrt() / 2.0_f32.sqrt();
                    let (rx, ry) = if *direction_agnostic_radius {
                        let radius = radii[0].resolve(basis);
                        (radius, radius)
                    } else {
                        (
                            radii[0].resolve(rect.width()),
                            radii[1].resolve(rect.height()),
                        )
                    };
                    append_arc(
                        &mut points,
                        current,
                        end,
                        ArcParameters {
                            radius_x: rx,
                            radius_y: ry,
                            rotation: *rotation,
                            large_arc: *large,
                            sweep: *clockwise,
                        },
                    );
                    current = end;
                    cubic_control = None;
                    quadratic_control = None;
                }
                ShapeCommand::Close => {
                    if points
                        .last()
                        .is_some_and(|point| (*point - subpath_start).length() > 0.001)
                    {
                        points.push(subpath_start);
                    }
                    current = subpath_start;
                    cubic_control = None;
                    quadratic_control = None;
                }
            }
        }
        if !points.is_empty() {
            contours.push(points);
        }
        contours
            .into_iter()
            .map(|contour| contour.into_iter().map(|point| rect.min + point).collect())
            .collect()
    }
}

fn shape_position_point(position: ShapePosition, rect: Rect) -> Vec2 {
    let x = position.x.resolve(rect.width());
    let y = position.y.resolve(rect.height());
    vec2(
        if position.x_from_end {
            rect.width() - x
        } else {
            x
        },
        if position.y_from_end {
            rect.height() - y
        } else {
            y
        },
    )
}

fn shape_point(point: ShapePoint, rect: Rect) -> Vec2 {
    vec2(
        point.x.resolve(rect.width()),
        point.y.resolve(rect.height()),
    )
}

fn resolve_shape_endpoint(endpoint: ShapeEndpoint, start: Vec2, rect: Rect) -> Vec2 {
    match endpoint {
        ShapeEndpoint::To(position) => shape_position_point(position, rect),
        ShapeEndpoint::By(point) => start + shape_point(point, rect),
    }
}

fn resolve_shape_control(control: ShapeControl, start: Vec2, end: Vec2, rect: Rect) -> Vec2 {
    match control {
        ShapeControl::Absolute(position) => shape_position_point(position, rect),
        ShapeControl::Relative(point, origin) => {
            let origin = match origin {
                ControlOrigin::Start => start,
                ControlOrigin::End => end,
                ControlOrigin::Origin => Vec2::ZERO,
            };
            origin + shape_point(point, rect)
        }
    }
}

fn geometry_rect(
    border: Rect,
    geometry: GeometryBox,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    widths: [f32; 4],
) -> Rect {
    match geometry {
        GeometryBox::Border => border,
        GeometryBox::Padding => border - stroke,
        GeometryBox::Content => border - stroke - padding,
        GeometryBox::Margin => border + margin,
        GeometryBox::HalfBorder => {
            border
                - egui::epaint::MarginF32 {
                    top: widths[0] * 0.5,
                    right: widths[1] * 0.5,
                    bottom: widths[2] * 0.5,
                    left: widths[3] * 0.5,
                }
        }
    }
}

fn resolve_shape_radius(radius: ShapeRadius, basis: f32, closest: f32, farthest: f32) -> f32 {
    match radius {
        ShapeRadius::LengthPercentage(value) => value.resolve(basis),
        ShapeRadius::ClosestSide => closest,
        ShapeRadius::FarthestSide => farthest,
    }
    .max(0.0)
}

fn ellipse_points(center: Pos2, radii: Vec2) -> Vec<Pos2> {
    (0..64)
        .map(|step| {
            let angle = step as f32 * std::f32::consts::TAU / 64.0;
            pos2(
                center.x + radii.x * angle.cos(),
                center.y + radii.y * angle.sin(),
            )
        })
        .collect()
}

impl Default for BorderImage {
    fn default() -> Self {
        Self {
            source: crate::css::BackgroundImage::None,
            slice: [ImageSlice::default(); 4],
            fill: false,
            width: [ImageWidth::default(); 4],
            outset: [ImageOutset::default(); 4],
            repeat: [ImageRepeat::default(); 2],
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DecorationStyle {
    pub(crate) radii: [CornerRadius; 4],
    pub(crate) corner_shapes: [CornerShape; 4],
    pub(crate) shadow_offsets: smallvec::SmallVec<[Option<Vec2>; 1]>,
    pub(crate) shadow_blurs: smallvec::SmallVec<[f32; 1]>,
    pub(crate) shadow_spreads: smallvec::SmallVec<[f32; 1]>,
    pub(crate) shadow_colors: smallvec::SmallVec<[Option<Color32>; 1]>,
    pub(crate) shadow_positions: smallvec::SmallVec<[ShadowPosition; 1]>,
    pub(crate) border_limit: BorderLimit,
    pub(crate) border_clips: [Option<Vec<ClipPart>>; 4],
    pub(crate) border_image: BorderImage,
    pub(crate) border_shape: Option<BorderShape>,
}

impl Default for DecorationStyle {
    fn default() -> Self {
        Self {
            radii: [CornerRadius::default(); 4],
            corner_shapes: [CornerShape::Round; 4],
            shadow_offsets: smallvec::smallvec![None],
            shadow_blurs: smallvec::smallvec![0.0],
            shadow_spreads: smallvec::smallvec![0.0],
            shadow_colors: smallvec::smallvec![None],
            shadow_positions: smallvec::smallvec![ShadowPosition::Outset],
            border_limit: BorderLimit::All,
            border_clips: std::array::from_fn(|_| None),
            border_image: BorderImage::default(),
            border_shape: None,
        }
    }
}

pub(crate) fn supported(name: &str) -> bool {
    matches!(
        name,
        "border-radius"
            | "border-top-left-radius"
            | "border-top-right-radius"
            | "border-bottom-right-radius"
            | "border-bottom-left-radius"
            | "border-start-start-radius"
            | "border-start-end-radius"
            | "border-end-start-radius"
            | "border-end-end-radius"
            | "border-top-radius"
            | "border-right-radius"
            | "border-bottom-radius"
            | "border-left-radius"
            | "border-block-start-radius"
            | "border-block-end-radius"
            | "border-inline-start-radius"
            | "border-inline-end-radius"
            | "corner-shape"
            | "corner-top-left-shape"
            | "corner-top-right-shape"
            | "corner-bottom-right-shape"
            | "corner-bottom-left-shape"
            | "corner-start-start-shape"
            | "corner-start-end-shape"
            | "corner-end-start-shape"
            | "corner-end-end-shape"
            | "corner-top-shape"
            | "corner-right-shape"
            | "corner-bottom-shape"
            | "corner-left-shape"
            | "corner-block-start-shape"
            | "corner-block-end-shape"
            | "corner-inline-start-shape"
            | "corner-inline-end-shape"
            | "corner"
            | "corner-top-left"
            | "corner-top-right"
            | "corner-bottom-left"
            | "corner-bottom-right"
            | "corner-start-start"
            | "corner-start-end"
            | "corner-end-start"
            | "corner-end-end"
            | "corner-top"
            | "corner-right"
            | "corner-bottom"
            | "corner-left"
            | "corner-block-start"
            | "corner-block-end"
            | "corner-inline-start"
            | "corner-inline-end"
            | "box-shadow"
            | "box-shadow-color"
            | "box-shadow-offset"
            | "box-shadow-blur"
            | "box-shadow-spread"
            | "box-shadow-position"
            | "border-limit"
            | "border-clip"
            | "border-top-clip"
            | "border-right-clip"
            | "border-bottom-clip"
            | "border-left-clip"
            | "border-block-clip"
            | "border-inline-clip"
            | "border-block-start-clip"
            | "border-block-end-clip"
            | "border-inline-start-clip"
            | "border-inline-end-clip"
            | "border-image"
            | "border-image-source"
            | "border-image-slice"
            | "border-image-width"
            | "border-image-outset"
            | "border-image-repeat"
            | "border-shape"
    )
}

pub(crate) fn apply(
    name: &str,
    value: &str,
    style: &mut BoxStyle,
    units: Units,
) -> Result<bool, &'static str> {
    if !supported(name) {
        return Ok(false);
    }
    let values = crate::layout_style::components(value)?;
    if name == "border-radius" {
        let (horizontal, vertical) = slash_pair(&values)?;
        let horizontal = radius_quad(horizontal, units)?;
        let vertical = vertical
            .map(|values| radius_quad(values, units))
            .transpose()?
            .unwrap_or(horizontal);
        for index in 0..4 {
            style.decorations.radii[index] = CornerRadius {
                x: horizontal[index],
                y: vertical[index],
            };
        }
        return Ok(true);
    }
    if name.ends_with("-radius") {
        if is_single_corner_radius(name) {
            let (horizontal, vertical) = single_radius(&values, units)?;
            let corner = radius_corners(name, style)[0];
            style.decorations.radii[corner] = CornerRadius {
                x: horizontal,
                y: vertical,
            };
            return Ok(true);
        }
        let (horizontal, vertical) = slash_pair(&values)?;
        let horizontal = radius_pair(horizontal, units)?;
        let vertical = vertical
            .map(|values| radius_pair(values, units))
            .transpose()?
            .unwrap_or(horizontal);
        let indices = radius_corners(name, style);
        for (index, corner) in indices.into_iter().enumerate() {
            style.decorations.radii[corner] = CornerRadius {
                x: horizontal[index],
                y: vertical[index],
            };
        }
        return Ok(true);
    }
    if name == "border-shape" {
        style.decorations.border_shape = parse_border_shape(&values, units)?;
        return Ok(true);
    }
    if name == "corner-shape" {
        let values = quad(&values)?;
        for (index, value) in values.iter().enumerate() {
            style.decorations.corner_shapes[index] = corner_shape(value, units)?;
        }
        return Ok(true);
    }
    if name == "corner" || name.starts_with("corner-") && !name.ends_with("-shape") {
        apply_corner_shorthand(name, &values, style, units)?;
        return Ok(true);
    }
    if name.ends_with("-shape") {
        if !name.starts_with("corner-") {
            return Ok(false);
        }
        let values = if is_corner_side(name) {
            pair(&values)?
                .into_iter()
                .map(|value| corner_shape(&value, units))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![corner_shape(value, units)?]
        };
        let indices = shape_corners(name, style);
        for (position, index) in indices.into_iter().enumerate() {
            style.decorations.corner_shapes[index] = values[position.min(values.len() - 1)];
        }
        return Ok(true);
    }
    if name == "box-shadow" {
        set_shadows(&mut style.decorations, parse_shadows(value, units)?);
        return Ok(true);
    }
    if name.starts_with("box-shadow-") {
        apply_shadow_longhand(name, value, &mut style.decorations, units)?;
        return Ok(true);
    }
    if name == "border-limit" {
        style.decorations.border_limit = parse_border_limit(&values, units, style)?;
        return Ok(true);
    }
    if name.ends_with("-clip") {
        let clip = parse_border_clip(&values, units)?;
        for side in clip_sides(name, style) {
            style.decorations.border_clips[side] = clip.clone();
        }
        return Ok(true);
    }
    if name.starts_with("border-image") {
        apply_border_image(
            name,
            value,
            &values,
            &mut style.decorations.border_image,
            units,
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn apply_corner_shorthand(
    name: &str,
    values: &[String],
    style: &mut BoxStyle,
    units: Units,
) -> Result<(), &'static str> {
    let mut shapes = Vec::new();
    let mut radii = Vec::new();
    for value in values {
        if let Ok(shape) = corner_shape(value, units) {
            shapes.push(shape);
        } else {
            radii.push(value.clone());
        }
    }
    if name == "corner" {
        if shapes.len() > 4 {
            return Err("too many corner shapes");
        }
        style.decorations.corner_shapes = if shapes.is_empty() {
            [CornerShape::Round; 4]
        } else {
            quad(&shapes)?
        };
        if radii.is_empty() {
            style.decorations.radii = [CornerRadius::default(); 4];
        } else {
            let (horizontal, vertical) = slash_pair(&radii)?;
            let horizontal = radius_quad(horizontal, units)?;
            let vertical = vertical
                .map(|values| radius_quad(values, units))
                .transpose()?
                .unwrap_or(horizontal);
            for index in 0..4 {
                style.decorations.radii[index] = CornerRadius {
                    x: horizontal[index],
                    y: vertical[index],
                };
            }
        }
        return Ok(());
    }
    let corners = corner_shortcut_corners(name, style);
    let side = corners.len() == 2;
    if shapes.len() > corners.len() {
        return Err("too many corner shapes");
    }
    for (position, corner) in corners.iter().enumerate() {
        style.decorations.corner_shapes[*corner] = shapes
            .get(position.min(shapes.len().saturating_sub(1)))
            .copied()
            .unwrap_or(CornerShape::Round);
    }
    if radii.is_empty() {
        for corner in corners {
            style.decorations.radii[corner] = CornerRadius::default();
        }
    } else if side {
        let (horizontal, vertical) = slash_pair(&radii)?;
        let horizontal = radius_pair(horizontal, units)?;
        let vertical = vertical
            .map(|values| radius_pair(values, units))
            .transpose()?
            .unwrap_or(horizontal);
        for (position, corner) in corners.into_iter().enumerate() {
            style.decorations.radii[corner] = CornerRadius {
                x: horizontal[position],
                y: vertical[position],
            };
        }
    } else {
        let (horizontal, vertical) = single_radius(&radii, units)?;
        style.decorations.radii[corners[0]] = CornerRadius {
            x: horizontal,
            y: vertical,
        };
    }
    Ok(())
}

fn corner_shortcut_corners(name: &str, style: &BoxStyle) -> Vec<usize> {
    match name {
        "corner-top-left" => vec![0],
        "corner-top-right" => vec![1],
        "corner-bottom-right" => vec![2],
        "corner-bottom-left" => vec![3],
        "corner-start-start" => vec![corner_index("block-start", "inline-start", style)],
        "corner-start-end" => vec![corner_index("block-start", "inline-end", style)],
        "corner-end-start" => vec![corner_index("block-end", "inline-start", style)],
        "corner-end-end" => vec![corner_index("block-end", "inline-end", style)],
        "corner-top" => vec![0, 1],
        "corner-right" => vec![1, 2],
        "corner-bottom" => vec![3, 2],
        "corner-left" => vec![0, 3],
        "corner-block-start" => vec![
            corner_index("block-start", "inline-start", style),
            corner_index("block-start", "inline-end", style),
        ],
        "corner-block-end" => vec![
            corner_index("block-end", "inline-start", style),
            corner_index("block-end", "inline-end", style),
        ],
        "corner-inline-start" => vec![
            corner_index("block-start", "inline-start", style),
            corner_index("block-end", "inline-start", style),
        ],
        _ => vec![
            corner_index("block-start", "inline-end", style),
            corner_index("block-end", "inline-end", style),
        ],
    }
}

pub(crate) fn copy_property(name: &str, out: &mut BoxStyle, source: &BoxStyle) -> bool {
    if !supported(name) {
        return false;
    }
    if name == "border-radius" {
        out.decorations.radii = source.decorations.radii;
    } else if name == "corner" {
        out.decorations.radii = source.decorations.radii;
        out.decorations.corner_shapes = source.decorations.corner_shapes;
    } else if name.starts_with("corner-") && !name.ends_with("-shape") {
        let targets = corner_shortcut_corners(name, out);
        let sources = corner_shortcut_corners(name, source);
        for (to, from) in targets.into_iter().zip(sources) {
            out.decorations.radii[to] = source.decorations.radii[from];
            out.decorations.corner_shapes[to] = source.decorations.corner_shapes[from];
        }
    } else if name == "corner-shape" {
        out.decorations.corner_shapes = source.decorations.corner_shapes;
    } else if name == "box-shadow" {
        copy_shadows(&mut out.decorations, &source.decorations);
    } else if name.starts_with("box-shadow-") {
        match name {
            "box-shadow-offset" => out
                .decorations
                .shadow_offsets
                .clone_from(&source.decorations.shadow_offsets),
            "box-shadow-blur" => out
                .decorations
                .shadow_blurs
                .clone_from(&source.decorations.shadow_blurs),
            "box-shadow-spread" => out
                .decorations
                .shadow_spreads
                .clone_from(&source.decorations.shadow_spreads),
            "box-shadow-color" => out
                .decorations
                .shadow_colors
                .clone_from(&source.decorations.shadow_colors),
            _ => out
                .decorations
                .shadow_positions
                .clone_from(&source.decorations.shadow_positions),
        }
    } else if name == "border-limit" {
        out.decorations.border_limit = source.decorations.border_limit;
    } else if name.starts_with("border-image") {
        match name {
            "border-image-source" => {
                out.decorations.border_image.source = source.decorations.border_image.source.clone()
            }
            "border-image-slice" => {
                out.decorations.border_image.slice = source.decorations.border_image.slice;
                out.decorations.border_image.fill = source.decorations.border_image.fill;
            }
            "border-image-width" => {
                out.decorations.border_image.width = source.decorations.border_image.width
            }
            "border-image-outset" => {
                out.decorations.border_image.outset = source.decorations.border_image.outset
            }
            "border-image-repeat" => {
                out.decorations.border_image.repeat = source.decorations.border_image.repeat
            }
            _ => out.decorations.border_image = source.decorations.border_image.clone(),
        }
    } else if name == "border-shape" {
        out.decorations.border_shape = source.decorations.border_shape.clone();
    } else if name.ends_with("-clip") {
        let sides = clip_sides(name, out);
        let source_sides = clip_sides(name, source);
        for (to, from) in sides.into_iter().zip(source_sides) {
            out.decorations.border_clips[to] = source.decorations.border_clips[from].clone();
        }
    } else if name.starts_with("border-") {
        let indices = radius_corners(name, out);
        let source_indices = radius_corners(name, source);
        for (to, from) in indices.into_iter().zip(source_indices) {
            out.decorations.radii[to] = source.decorations.radii[from];
        }
    } else {
        let indices = shape_corners(name, out);
        let source_indices = shape_corners(name, source);
        for (to, from) in indices.into_iter().zip(source_indices) {
            out.decorations.corner_shapes[to] = source.decorations.corner_shapes[from];
        }
    }
    true
}

pub(crate) fn resolved_radii(style: &BoxStyle, rect: Rect) -> [Vec2; 4] {
    let radii = std::array::from_fn(|index| {
        let radius = style.decorations.radii[index];
        vec2(
            radius.x.resolve(rect.width()).max(0.0),
            radius.y.resolve(rect.height()).max(0.0),
        )
    });
    let sums = [
        radii[0].x + radii[1].x,
        radii[1].y + radii[2].y,
        radii[2].x + radii[3].x,
        radii[3].y + radii[0].y,
    ];
    let lengths = [rect.width(), rect.height(), rect.width(), rect.height()];
    let scale = sums
        .iter()
        .zip(lengths)
        .filter(|(sum, _)| **sum > 0.0)
        .map(|(sum, length)| length / *sum)
        .fold(1.0_f32, f32::min)
        .min(1.0);
    let radii = radii.map(|radius| radius * scale);
    let opposite_scale = if style
        .decorations
        .corner_shapes
        .iter()
        .any(|shape| shape.parameter_is_concave())
    {
        opposite_corner_scale(rect, radii, style.decorations.corner_shapes, [0, 2]).min(
            opposite_corner_scale(rect, radii, style.decorations.corner_shapes, [1, 3]),
        )
    } else {
        1.0
    };
    radii.map(|radius| radius * opposite_scale)
}

pub(crate) fn outset_radii(rect: Rect, radii: [Vec2; 4], outset: [f32; 4]) -> [Vec2; 4] {
    let coverage = radii.map(|radius| {
        (2.0 * (radius.x / rect.width().max(f32::EPSILON))
            .min(radius.y / rect.height().max(f32::EPSILON)))
        .max(0.0)
    });
    std::array::from_fn(|corner| {
        let (horizontal, vertical) = match corner {
            0 => (outset[3], outset[0]),
            1 => (outset[1], outset[0]),
            2 => (outset[1], outset[2]),
            _ => (outset[3], outset[2]),
        };
        let adjusted = |radius: f32, amount: f32| {
            if amount <= 0.0 || radius > amount || coverage[corner] > 1.0 {
                radius + amount
            } else {
                let ratio = radius / amount;
                radius + amount * (1.0 - (1.0 - ratio).powi(3) * (1.0 - coverage[corner].powi(3)))
            }
        };
        vec2(
            adjusted(radii[corner].x, horizontal),
            adjusted(radii[corner].y, vertical),
        )
    })
}

fn opposite_corner_scale(
    rect: Rect,
    radii: [Vec2; 4],
    shapes: [CornerShape; 4],
    corners: [usize; 2],
) -> f32 {
    let first = corner_hull(rect, radii[corners[0]], shapes[corners[0]], corners[0]);
    let second = corner_hull(rect, radii[corners[1]], shapes[corners[1]], corners[1]);
    if !convex_polygons_overlap(&first, &second) {
        return 1.0;
    }

    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..24 {
        let scale = (lower + upper) * 0.5;
        let first = corner_hull(
            rect,
            radii[corners[0]] * scale,
            shapes[corners[0]],
            corners[0],
        );
        let second = corner_hull(
            rect,
            radii[corners[1]] * scale,
            shapes[corners[1]],
            corners[1],
        );
        if convex_polygons_overlap(&first, &second) {
            upper = scale;
        } else {
            lower = scale;
        }
    }
    lower
}

fn corner_hull(rect: Rect, radius: Vec2, shape: CornerShape, corner: usize) -> Vec<Pos2> {
    if radius.x <= 0.0 || radius.y <= 0.0 {
        return Vec::new();
    }
    let k = shape.parameter();
    let mut hull = if k >= 0.0 {
        vec![pos2(1.0, 1.0), pos2(1.0, 0.0), pos2(0.0, 1.0)]
    } else {
        let half = if k == f32::NEG_INFINITY {
            0.0
        } else {
            1.0 - 0.51 / (2.0 * k.abs())
        };
        vec![
            pos2(1.0, 1.0),
            pos2(1.0, 0.0),
            pos2(2.0 * half, 0.0),
            pos2(0.0, 2.0 * half),
            pos2(0.0, 1.0),
        ]
    };
    let (start, end, center) = match corner {
        0 => (
            pos2(rect.left(), rect.top() + radius.y),
            pos2(rect.left() + radius.x, rect.top()),
            pos2(rect.left() + radius.x, rect.top() + radius.y),
        ),
        1 => (
            pos2(rect.right() - radius.x, rect.top()),
            pos2(rect.right(), rect.top() + radius.y),
            pos2(rect.right() - radius.x, rect.top() + radius.y),
        ),
        2 => (
            pos2(rect.right(), rect.bottom() - radius.y),
            pos2(rect.right() - radius.x, rect.bottom()),
            pos2(rect.right() - radius.x, rect.bottom() - radius.y),
        ),
        _ => (
            pos2(rect.left() + radius.x, rect.bottom()),
            pos2(rect.left(), rect.bottom() - radius.y),
            pos2(rect.left() + radius.x, rect.bottom() - radius.y),
        ),
    };
    let along_end = end - center;
    let along_start = start - center;
    for point in &mut hull {
        *point = center + along_end * point.x + along_start * point.y;
    }
    hull
}

fn convex_polygons_overlap(first: &[Pos2], second: &[Pos2]) -> bool {
    if first.len() < 3 || second.len() < 3 {
        return false;
    }
    for polygon in [first, second] {
        for index in 0..polygon.len() {
            let edge = polygon[(index + 1) % polygon.len()] - polygon[index];
            let axis = vec2(-edge.y, edge.x);
            if axis.length_sq() <= f32::EPSILON {
                continue;
            }
            let project = |points: &[Pos2]| {
                points
                    .iter()
                    .map(|point| point.to_vec2().dot(axis))
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), value| {
                        (min.min(value), max.max(value))
                    })
            };
            let (first_min, first_max) = project(first);
            let (second_min, second_max) = project(second);
            let epsilon = axis.length() * 0.0001;
            if first_max <= second_min + epsilon || second_max <= first_min + epsilon {
                return false;
            }
        }
    }
    true
}

pub(crate) fn simple_visual_radius(value: &str) -> Option<u8> {
    let values = crate::layout_style::components(value).ok()?;
    if values.len() != 1 || values[0].contains('%') || values[0].contains('(') {
        return None;
    }
    let value = values[0]
        .strip_suffix("px")
        .unwrap_or(&values[0])
        .parse::<f32>()
        .ok()?;
    (value >= 0.0 && value <= f32::from(u8::MAX)).then_some(value.round() as u8)
}

pub(crate) fn limit_intervals(
    limit: BorderLimit,
    side: usize,
    length: f32,
    percentage_basis: f32,
    in_corner: bool,
) -> Vec<(f32, f32)> {
    match limit {
        BorderLimit::All => vec![(0.0, length)],
        BorderLimit::Sides(extent) => {
            if in_corner {
                return Vec::new();
            }
            let width = extent
                .map_or(length, |value| value.resolve(percentage_basis))
                .clamp(0.0, length);
            vec![((length - width) * 0.5, (length + width) * 0.5)]
        }
        BorderLimit::Corners(extent) => {
            if in_corner {
                return vec![(0.0, length)];
            }
            let Some(extent) = extent else {
                return Vec::new();
            };
            let extent = extent.resolve(percentage_basis).clamp(0.0, length);
            vec![(0.0, extent), (length - extent, length)]
        }
        BorderLimit::Side(selected, extent) => {
            if side == selected {
                return vec![(0.0, length)];
            }
            if in_corner {
                return Vec::new();
            }
            let Some(extent) = extent else {
                return Vec::new();
            };
            let extent = extent.resolve(percentage_basis).clamp(0.0, length);
            match (selected, side) {
                (3, 0) | (1, 2) | (0, 1) | (2, 3) => vec![(0.0, extent)],
                (3, 2) | (1, 0) | (0, 3) | (2, 1) => vec![(length - extent, length)],
                _ => Vec::new(),
            }
        }
    }
}

pub(crate) fn clip_intervals(clips: &[ClipPart], length: f32) -> Vec<(f32, f32)> {
    let mut fixed = 0.0;
    let mut flex = 0.0;
    let mut has_flex = false;
    for part in clips {
        match part {
            ClipPart::Length(value) => fixed += value.resolve(length).max(0.0),
            ClipPart::Flex(weight) => {
                flex += weight.max(0.0);
                has_flex = true;
            }
        }
    }
    let use_default_flex = !has_flex && fixed < length;
    let flex_total = if use_default_flex { 1.0 } else { flex };
    let flex_unit = if fixed >= length || flex_total == 0.0 {
        0.0
    } else {
        (length - fixed) / flex_total
    };
    let mut cursor = 0.0;
    let mut visible = true;
    let mut intervals = Vec::new();
    for part in clips {
        let part_length = match part {
            ClipPart::Length(value) => value.resolve(length).max(0.0),
            ClipPart::Flex(weight) => weight.max(0.0) * flex_unit,
        };
        let end = (cursor + part_length).min(length);
        if visible && end > cursor {
            intervals.push((cursor, end));
        }
        cursor = end;
        visible = !visible;
        if cursor >= length {
            return intervals;
        }
    }
    if use_default_flex && visible && cursor < length {
        intervals.push((cursor, length));
    }
    intervals
}

pub(crate) fn intersect_intervals(left: &[(f32, f32)], right: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut result = Vec::new();
    for &(left_start, left_end) in left {
        for &(right_start, right_end) in right {
            let start = left_start.max(right_start);
            let end = left_end.min(right_end);
            if end > start {
                result.push((start, end));
            }
        }
    }
    result
}

impl CornerShape {
    fn parameter(self) -> f32 {
        match self {
            Self::Round => 1.0,
            Self::Squircle => 2.0,
            Self::Square => f32::INFINITY,
            Self::Bevel => 0.0,
            Self::Scoop => -1.0,
            Self::Notch => f32::NEG_INFINITY,
            Self::Superellipse(value) => value,
        }
    }

    pub(crate) fn parameter_is_concave(self) -> bool {
        self.parameter() < 0.0
    }
}

fn radius_quad(values: &[String], units: Units) -> Result<[LengthPercentage; 4], &'static str> {
    let values = quad(values)?;
    Ok([
        length_percentage(&values[0], units, false)?,
        length_percentage(&values[1], units, false)?,
        length_percentage(&values[2], units, false)?,
        length_percentage(&values[3], units, false)?,
    ])
}

fn radius_pair(values: &[String], units: Units) -> Result<[LengthPercentage; 2], &'static str> {
    if values.is_empty() || values.len() > 2 {
        return Err("expected one or two radius values");
    }
    let first = length_percentage(&values[0], units, false)?;
    Ok([
        first,
        values
            .get(1)
            .map(|value| length_percentage(value, units, false))
            .transpose()?
            .unwrap_or(first),
    ])
}

pub(crate) fn length_percentage(
    value: &str,
    units: Units,
    negative: bool,
) -> Result<LengthPercentage, &'static str> {
    if let Some(handle) = crate::math::length(value, units, negative, true)? {
        return Ok(LengthPercentage::Calc(handle));
    }
    let (value, is_percentage) = crate::layout_style::length_value(value, units, negative)?;
    Ok(if is_percentage {
        LengthPercentage::Percentage(value)
    } else {
        LengthPercentage::Length(value)
    })
}

fn slash_pair(values: &[String]) -> Result<(&[String], Option<&[String]>), &'static str> {
    let Some(index) = values.iter().position(|value| value == "/") else {
        return Ok((values, None));
    };
    if index == 0 || index + 1 == values.len() || values[index + 1..].contains(&"/".to_owned()) {
        return Err("invalid slash-separated value");
    }
    Ok((&values[..index], Some(&values[index + 1..])))
}

fn quad<T: Clone>(values: &[T]) -> Result<[T; 4], &'static str> {
    if values.is_empty() || values.len() > 4 {
        return Err("expected one to four values");
    }
    Ok([
        values[0].clone(),
        values.get(1).unwrap_or(&values[0]).clone(),
        values.get(2).unwrap_or(&values[0]).clone(),
        values
            .get(3)
            .unwrap_or(values.get(1).unwrap_or(&values[0]))
            .clone(),
    ])
}

fn pair<T: Clone>(values: &[T]) -> Result<[T; 2], &'static str> {
    if values.is_empty() || values.len() > 2 {
        return Err("expected one or two values");
    }
    Ok([
        values[0].clone(),
        values.get(1).unwrap_or(&values[0]).clone(),
    ])
}

fn radius_corners(name: &str, style: &BoxStyle) -> Vec<usize> {
    match name {
        "border-top-left-radius" => vec![0],
        "border-top-right-radius" => vec![1],
        "border-bottom-right-radius" => vec![2],
        "border-bottom-left-radius" => vec![3],
        "border-start-start-radius" => vec![corner_index("block-start", "inline-start", style)],
        "border-start-end-radius" => vec![corner_index("block-start", "inline-end", style)],
        "border-end-start-radius" => vec![corner_index("block-end", "inline-start", style)],
        "border-end-end-radius" => vec![corner_index("block-end", "inline-end", style)],
        "corner-top-left-radius" => vec![0],
        "corner-top-right-radius" => vec![1],
        "corner-bottom-right-radius" => vec![2],
        "corner-bottom-left-radius" => vec![3],
        "corner-start-start-radius" => vec![corner_index("block-start", "inline-start", style)],
        "corner-start-end-radius" => vec![corner_index("block-start", "inline-end", style)],
        "corner-end-start-radius" => vec![corner_index("block-end", "inline-start", style)],
        "corner-end-end-radius" => vec![corner_index("block-end", "inline-end", style)],
        _ => side_corner_indices(name, style),
    }
}

fn shape_corners(name: &str, style: &BoxStyle) -> Vec<usize> {
    let corner = name
        .strip_prefix("corner-")
        .unwrap_or(name)
        .strip_suffix("-shape")
        .unwrap_or("");
    let radius_name = if matches!(
        corner,
        "top"
            | "right"
            | "bottom"
            | "left"
            | "block-start"
            | "block-end"
            | "inline-start"
            | "inline-end"
    ) {
        format!("border-{corner}-radius")
    } else {
        format!("corner-{corner}-radius")
    };
    radius_corners(&radius_name, style)
}

fn side_corner_indices(name: &str, style: &BoxStyle) -> Vec<usize> {
    let side_name = name
        .strip_prefix("border-")
        .or_else(|| name.strip_prefix("corner-"))
        .and_then(|value| value.rsplit_once('-').map(|(side, _)| side))
        .unwrap_or("");
    if matches!(
        side_name,
        "block-start" | "block-end" | "inline-start" | "inline-end"
    ) {
        return match side_name {
            "block-start" => vec![
                corner_index("block-start", "inline-start", style),
                corner_index("block-start", "inline-end", style),
            ],
            "block-end" => vec![
                corner_index("block-end", "inline-start", style),
                corner_index("block-end", "inline-end", style),
            ],
            "inline-start" => vec![
                corner_index("block-start", "inline-start", style),
                corner_index("block-end", "inline-start", style),
            ],
            _ => vec![
                corner_index("block-start", "inline-end", style),
                corner_index("block-end", "inline-end", style),
            ],
        };
    }
    let side = match side_name {
        "top" => 0,
        "right" => 1,
        "bottom" => 2,
        "left" => 3,
        _ => 0,
    };
    match side {
        0 => vec![0, 1],
        1 => vec![1, 2],
        2 => vec![3, 2],
        _ => vec![0, 3],
    }
}

fn corner_index(block: &str, inline: &str, style: &BoxStyle) -> usize {
    let vertical = crate::layout_style::logical_side(
        block,
        style.layout.direction,
        style.format.writing,
        &style.format.text_orientation,
    )
    .unwrap_or(0);
    let horizontal = crate::layout_style::logical_side(
        inline,
        style.layout.direction,
        style.format.writing,
        &style.format.text_orientation,
    )
    .unwrap_or(3);
    match (vertical, horizontal) {
        (0, 3) | (3, 0) => 0,
        (0, 1) | (1, 0) => 1,
        (2, 1) | (1, 2) => 2,
        _ => 3,
    }
}

fn is_single_corner_radius(name: &str) -> bool {
    matches!(
        name,
        "border-top-left-radius"
            | "border-top-right-radius"
            | "border-bottom-right-radius"
            | "border-bottom-left-radius"
            | "border-start-start-radius"
            | "border-start-end-radius"
            | "border-end-start-radius"
            | "border-end-end-radius"
    )
}

fn single_radius(
    values: &[String],
    units: Units,
) -> Result<(LengthPercentage, LengthPercentage), &'static str> {
    let (horizontal, vertical) = slash_pair(values)?;
    match vertical {
        Some(vertical) if horizontal.len() == 1 && vertical.len() == 1 => Ok((
            length_percentage(&horizontal[0], units, false)?,
            length_percentage(&vertical[0], units, false)?,
        )),
        Some(_) => Err("invalid corner radius"),
        None if horizontal.len() == 1 => {
            let value = length_percentage(&horizontal[0], units, false)?;
            Ok((value, value))
        }
        None if horizontal.len() == 2 => Ok((
            length_percentage(&horizontal[0], units, false)?,
            length_percentage(&horizontal[1], units, false)?,
        )),
        _ => Err("invalid corner radius"),
    }
}

fn is_corner_side(name: &str) -> bool {
    name == "corner-top-shape"
        || name == "corner-right-shape"
        || name == "corner-bottom-shape"
        || name == "corner-left-shape"
        || name == "corner-block-start-shape"
        || name == "corner-block-end-shape"
        || name == "corner-inline-start-shape"
        || name == "corner-inline-end-shape"
}

fn corner_shape(value: &str, units: Units) -> Result<CornerShape, &'static str> {
    Ok(match value.to_ascii_lowercase().as_str() {
        "round" => CornerShape::Round,
        "squircle" => CornerShape::Squircle,
        "square" => CornerShape::Square,
        "bevel" => CornerShape::Bevel,
        "scoop" => CornerShape::Scoop,
        "notch" => CornerShape::Notch,
        value if value.starts_with("superellipse(") && value.ends_with(')') => {
            let parameter = &value[13..value.len() - 1];
            let number = match parameter {
                "infinity" => f32::INFINITY,
                "-infinity" => f32::NEG_INFINITY,
                _ => {
                    if let Some(number) = crate::math::number(parameter, units)
                        .map_err(|_| "invalid superellipse parameter")?
                    {
                        number
                    } else {
                        let mut parser = Parser::new(parameter);
                        let number = match parser
                            .next()
                            .map_err(|_| "invalid superellipse parameter")?
                        {
                            cssparser::Token::Number { value, .. } => *value,
                            _ => return Err("invalid superellipse parameter"),
                        };
                        parser
                            .expect_exhausted()
                            .map_err(|_| "invalid superellipse parameter")?;
                        number
                    }
                }
            };
            if number.is_nan() {
                return Err("invalid superellipse parameter");
            }
            CornerShape::Superellipse(number)
        }
        _ => return Err("invalid corner shape"),
    })
}

fn parse_border_limit(
    values: &[String],
    units: Units,
    style: &BoxStyle,
) -> Result<BorderLimit, &'static str> {
    match values {
        [value] if value.eq_ignore_ascii_case("all") => Ok(BorderLimit::All),
        [kind] if kind.eq_ignore_ascii_case("sides") => Ok(BorderLimit::Sides(None)),
        [kind, extent] if kind.eq_ignore_ascii_case("sides") => Ok(BorderLimit::Sides(Some(
            length_percentage(extent, units, false)?,
        ))),
        [kind] if kind.eq_ignore_ascii_case("corners") => Ok(BorderLimit::Corners(None)),
        [kind, extent] if kind.eq_ignore_ascii_case("corners") => Ok(BorderLimit::Corners(Some(
            length_percentage(extent, units, false)?,
        ))),
        [side] if border_side(side).is_some() => {
            Ok(BorderLimit::Side(border_side(side).unwrap(), None))
        }
        [side, extent] if border_side(side).is_some() => Ok(BorderLimit::Side(
            border_side(side).unwrap(),
            Some(length_percentage(extent, units, false)?),
        )),
        [side]
            if matches!(
                side.to_ascii_lowercase().as_str(),
                "block-start" | "block-end" | "inline-start" | "inline-end"
            ) =>
        {
            let side = crate::layout_style::logical_side(
                side,
                style.layout.direction,
                style.format.writing,
                &style.format.text_orientation,
            )
            .ok_or("invalid border-limit side")?;
            Ok(BorderLimit::Side(side, None))
        }
        [side, extent]
            if matches!(
                side.to_ascii_lowercase().as_str(),
                "block-start" | "block-end" | "inline-start" | "inline-end"
            ) =>
        {
            let side = crate::layout_style::logical_side(
                side,
                style.layout.direction,
                style.format.writing,
                &style.format.text_orientation,
            )
            .ok_or("invalid border-limit side")?;
            Ok(BorderLimit::Side(
                side,
                Some(length_percentage(extent, units, false)?),
            ))
        }
        _ => Err("invalid border-limit"),
    }
}

fn parse_border_clip(
    values: &[String],
    units: Units,
) -> Result<Option<Vec<ClipPart>>, &'static str> {
    if values.len() == 1 && values[0].eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    if values.is_empty() {
        return Err("invalid border-clip");
    }
    let parts = values
        .iter()
        .map(|value| {
            if let Some(weight) = value.to_ascii_lowercase().strip_suffix("fr") {
                let weight = weight
                    .parse::<f32>()
                    .map_err(|_| "invalid border-clip flex")?;
                if !weight.is_finite() || weight < 0.0 {
                    return Err("invalid border-clip flex");
                }
                Ok(ClipPart::Flex(weight))
            } else {
                Ok(ClipPart::Length(length_percentage(value, units, false)?))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(parts))
}

pub(crate) fn parse_border_stripes(
    value: &str,
    units: Units,
) -> Result<Vec<BorderStripe>, &'static str> {
    let start = value
        .find('(')
        .filter(|index| value[..*index].eq_ignore_ascii_case("stripes"))
        .ok_or("invalid stripes() border color")?;
    let inner = value[start + 1..]
        .strip_suffix(')')
        .ok_or("invalid stripes() border color")?;
    let mut stripes = Vec::new();
    for stripe in split_commas(inner) {
        let components = crate::layout_style::components(stripe)?;
        if components.is_empty() {
            return Err("empty stripes() entry");
        }
        let mut color = None;
        let mut color_seen = false;
        let mut thickness = None;
        for component in &components {
            if let Ok(value) = color_value(component) {
                if color_seen {
                    return Err("duplicate stripes() color");
                }
                color = value;
                color_seen = true;
                continue;
            }
            if thickness.is_some() {
                return Err("duplicate stripes() thickness");
            }
            if let Some(weight) = component.to_ascii_lowercase().strip_suffix("fr") {
                let weight = weight
                    .parse::<f32>()
                    .map_err(|_| "invalid stripes() flex")?;
                if !weight.is_finite() || weight < 0.0 {
                    return Err("invalid stripes() flex");
                }
                thickness = Some(ClipPart::Flex(weight));
            } else {
                let value = length_percentage(component, units, false)?;
                if let LengthPercentage::Percentage(value) = value
                    && !(0.0..=1.0).contains(&value)
                {
                    return Err("stripes() percentage must be between zero and 100%");
                }
                thickness = Some(ClipPart::Length(value));
            }
        }
        if !color_seen {
            return Err("stripes() entry requires a color");
        }
        stripes.push(BorderStripe {
            color,
            thickness: thickness.unwrap_or(ClipPart::Flex(1.0)),
        });
    }
    if stripes.is_empty() {
        return Err("stripes() requires at least one entry");
    }
    Ok(stripes)
}

fn clip_sides(name: &str, style: &BoxStyle) -> Vec<usize> {
    match name {
        "border-clip" => vec![0, 1, 2, 3],
        "border-block-clip" => ["block-start", "block-end"]
            .into_iter()
            .filter_map(|side| {
                crate::layout_style::logical_side(
                    side,
                    style.layout.direction,
                    style.format.writing,
                    &style.format.text_orientation,
                )
            })
            .collect(),
        "border-inline-clip" => ["inline-start", "inline-end"]
            .into_iter()
            .filter_map(|side| {
                crate::layout_style::logical_side(
                    side,
                    style.layout.direction,
                    style.format.writing,
                    &style.format.text_orientation,
                )
            })
            .collect(),
        _ => {
            let side = name.trim_start_matches("border-").trim_end_matches("-clip");
            vec![
                border_side(side)
                    .or_else(|| {
                        crate::layout_style::logical_side(
                            side,
                            style.layout.direction,
                            style.format.writing,
                            &style.format.text_orientation,
                        )
                    })
                    .unwrap_or(0),
            ]
        }
    }
}

fn border_side(side: &str) -> Option<usize> {
    match side.to_ascii_lowercase().as_str() {
        "top" => Some(0),
        "right" => Some(1),
        "bottom" => Some(2),
        "left" => Some(3),
        _ => None,
    }
}
