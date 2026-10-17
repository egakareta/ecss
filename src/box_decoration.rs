use cssparser::Parser;
use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};

use crate::layout_style::{BoxStyle, Units};

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

#[derive(Clone)]
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
        border: Rect,
        clip: crate::css::BackgroundClip,
        stroke: egui::epaint::MarginF32,
        padding: egui::epaint::MarginF32,
        margin: egui::epaint::MarginF32,
        widths: [f32; 4],
        outset: f32,
    ) -> (Vec<Vec<Pos2>>, FillRule) {
        let (paths, fill_rule) =
            self.background_paths(border, clip, stroke, padding, margin, widths);
        if outset > 0.0 {
            (offset_filled_paths(&paths, fill_rule, outset), fill_rule)
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
                        rx,
                        ry,
                        *rotation,
                        *large,
                        *clockwise,
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
    pub(crate) shadow_offsets: Vec<Option<Vec2>>,
    pub(crate) shadow_blurs: Vec<f32>,
    pub(crate) shadow_spreads: Vec<f32>,
    pub(crate) shadow_colors: Vec<Option<Color32>>,
    pub(crate) shadow_positions: Vec<ShadowPosition>,
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
            shadow_offsets: vec![None],
            shadow_blurs: vec![0.0],
            shadow_spreads: vec![0.0],
            shadow_colors: vec![None],
            shadow_positions: vec![ShadowPosition::Outset],
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

fn parse_border_shape(
    values: &[String],
    units: Units,
) -> Result<Option<BorderShape>, &'static str> {
    if values.len() == 1 && values[0].eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let mut shapes = Vec::new();
    let mut geometry_seen = false;
    for value in values {
        if let Some(geometry) = geometry_box(value) {
            if geometry_seen {
                return Err("duplicate border-shape geometry box");
            }
            let Some((_, existing)) = shapes.last_mut() else {
                return Err("border-shape geometry box needs a preceding shape");
            };
            *existing = geometry;
            geometry_seen = true;
            continue;
        }
        if shapes.len() == 2 {
            return Err("border-shape accepts at most two shapes");
        }
        let shape = parse_basic_shape(value, units)?;
        let default_box = match shapes.len() {
            0 if values.iter().filter(|value| value.contains('(')).count() == 1 => {
                GeometryBox::HalfBorder
            }
            0 => GeometryBox::Border,
            _ => GeometryBox::Padding,
        };
        shapes.push((shape, default_box));
        geometry_seen = false;
    }
    if shapes.is_empty() {
        return Err("border-shape requires a shape or none");
    }
    Ok(Some(BorderShape { shapes }))
}

fn geometry_box(value: &str) -> Option<GeometryBox> {
    match value.to_ascii_lowercase().as_str() {
        "border-box" => Some(GeometryBox::Border),
        "padding-box" => Some(GeometryBox::Padding),
        "content-box" => Some(GeometryBox::Content),
        "margin-box" => Some(GeometryBox::Margin),
        "half-border-box" => Some(GeometryBox::HalfBorder),
        _ => None,
    }
}

fn parse_basic_shape(value: &str, units: Units) -> Result<BasicShape, &'static str> {
    let open = value.find('(').ok_or("expected a basic shape function")?;
    if !value.ends_with(')') {
        return Err("invalid basic shape function");
    }
    let name = value[..open].to_ascii_lowercase();
    let inner = &value[open + 1..value.len() - 1];
    let components = crate::layout_style::components(inner)?;
    match name.as_str() {
        "circle" => {
            let at = components
                .iter()
                .position(|value| value.eq_ignore_ascii_case("at"));
            let radius_values = &components[..at.unwrap_or(components.len())];
            if radius_values.len() > 1 {
                return Err("circle() takes one radius");
            }
            let radius = radius_values
                .first()
                .map(|value| shape_radius(value, units, false))
                .transpose()?;
            let position = at
                .map(|at| shape_position(&components[at + 1..], units))
                .transpose()?
                .unwrap_or_default();
            Ok(BasicShape::Circle { radius, position })
        }
        "ellipse" => {
            let at = components
                .iter()
                .position(|value| value.eq_ignore_ascii_case("at"));
            let radius_values = &components[..at.unwrap_or(components.len())];
            if radius_values.len() > 2 {
                return Err("ellipse() takes two radii");
            }
            let radii = match radius_values {
                [] => [None, None],
                [value] => {
                    let radius = Some(shape_radius(value, units, false)?);
                    [radius, radius]
                }
                [x, y] => [
                    Some(shape_radius(x, units, false)?),
                    Some(shape_radius(y, units, false)?),
                ],
                _ => unreachable!(),
            };
            let position = at
                .map(|at| shape_position(&components[at + 1..], units))
                .transpose()?
                .unwrap_or_default();
            Ok(BasicShape::Ellipse { radii, position })
        }
        "inset" => {
            let round = components
                .iter()
                .position(|value| value.eq_ignore_ascii_case("round"));
            let inset_values = &components[..round.unwrap_or(components.len())];
            let inset_values = quad(inset_values)?;
            let insets = [
                length_percentage(&inset_values[0], units, true)?,
                length_percentage(&inset_values[1], units, true)?,
                length_percentage(&inset_values[2], units, true)?,
                length_percentage(&inset_values[3], units, true)?,
            ];
            let radii = if let Some(round) = round {
                let (horizontal, vertical) = slash_pair(&components[round + 1..])?;
                let horizontal = radius_quad(horizontal, units)?;
                let vertical = vertical
                    .map(|values| radius_quad(values, units))
                    .transpose()?
                    .unwrap_or(horizontal);
                Some(std::array::from_fn(|index| CornerRadius {
                    x: horizontal[index],
                    y: vertical[index],
                }))
            } else {
                None
            };
            Ok(BasicShape::Inset { insets, radii })
        }
        "polygon" => {
            let mut points = Vec::new();
            let mut fill_rule = FillRule::Nonzero;
            for (index, pair) in split_commas(inner).into_iter().enumerate() {
                let pair = crate::layout_style::components(pair)?;
                if index == 0
                    && pair.len() == 1
                    && (pair[0].eq_ignore_ascii_case("evenodd")
                        || pair[0].eq_ignore_ascii_case("nonzero"))
                {
                    fill_rule = if pair[0].eq_ignore_ascii_case("evenodd") {
                        FillRule::Evenodd
                    } else {
                        FillRule::Nonzero
                    };
                    continue;
                }
                if pair.len() != 2 {
                    return Err("polygon() requires coordinate pairs");
                }
                points.push((
                    length_percentage(&pair[0], units, true)?,
                    length_percentage(&pair[1], units, true)?,
                ));
            }
            if points.len() < 3 {
                return Err("polygon() requires at least three points");
            }
            Ok(BasicShape::Polygon { points, fill_rule })
        }
        "rect" => {
            if components.len() != 4 {
                return Err("rect() requires four edges");
            }
            let values = [
                rect_edge(&components[0], 0, units)?,
                rect_edge(&components[1], 1, units)?,
                rect_edge(&components[2], 2, units)?,
                rect_edge(&components[3], 3, units)?,
            ];
            Ok(BasicShape::Rect(values))
        }
        "xywh" => {
            if components.len() != 4 {
                return Err("xywh() requires four values");
            }
            let values = [
                length_percentage(&components[0], units, true)?,
                length_percentage(&components[1], units, true)?,
                length_percentage(&components[2], units, false)?,
                length_percentage(&components[3], units, false)?,
            ];
            Ok(BasicShape::Xywh(values))
        }
        "shape" => Ok(BasicShape::Shape(parse_shape(inner, units)?)),
        "path" => {
            let (contours, fill_rule) = parse_path(inner)?;
            Ok(BasicShape::Path {
                contours,
                fill_rule,
            })
        }
        _ => Err("unsupported border-shape basic shape"),
    }
}

fn parse_shape(value: &str, units: Units) -> Result<ShapeFunction, &'static str> {
    let commands = split_commas(value);
    if commands.len() < 2 {
        return Err("shape() requires a starting point and a command");
    }
    let mut header = crate::layout_style::components(commands[0])?;
    let fill_rule = if header
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("evenodd"))
    {
        header.remove(0);
        FillRule::Evenodd
    } else if header
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("nonzero"))
    {
        header.remove(0);
        FillRule::Nonzero
    } else {
        FillRule::Nonzero
    };
    if !header
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("from"))
    {
        return Err("shape() requires a from position");
    }
    let start = shape_position(&header[1..], units)?;
    let mut parsed = Vec::with_capacity(commands.len() - 1);
    for command in commands.iter().skip(1) {
        let values = crate::layout_style::components(command)?;
        let Some(name) = values.first().map(|value| value.to_ascii_lowercase()) else {
            return Err("empty shape() command");
        };
        match name.as_str() {
            "close" if values.len() == 1 => parsed.push(ShapeCommand::Close),
            "line" | "move" => {
                let endpoint = parse_shape_endpoint(&values[1..], units)?;
                parsed.push(if name == "line" {
                    ShapeCommand::Line(endpoint)
                } else {
                    ShapeCommand::Move(endpoint)
                });
            }
            "hline" | "vline" => {
                let [mode, value] = &values[1..] else {
                    return Err("hline() and vline() require one coordinate");
                };
                let mode = mode.to_ascii_lowercase();
                let relative = match mode.as_str() {
                    "to" => false,
                    "by" => true,
                    _ => return Err("hline() and vline() require to or by"),
                };
                let value = shape_axis_value(value, name == "hline", units, relative)?;
                parsed.push(if name == "hline" {
                    ShapeCommand::Horizontal { relative, value }
                } else {
                    ShapeCommand::Vertical { relative, value }
                });
            }
            "curve" => {
                let with = values
                    .iter()
                    .position(|value| value.eq_ignore_ascii_case("with"))
                    .ok_or("curve command requires with control points")?;
                let endpoint = parse_shape_endpoint(&values[1..with], units)?;
                let control_groups = split_shape_controls(&values[with + 1..])?;
                let relative = matches!(endpoint, ShapeEndpoint::By(_));
                let controls = control_groups
                    .into_iter()
                    .map(|control| parse_shape_control(control, units, relative))
                    .collect::<Result<Vec<_>, _>>()?;
                if controls.is_empty() || controls.len() > 2 {
                    return Err("curve command requires one or two control points");
                }
                parsed.push(ShapeCommand::Curve { endpoint, controls });
            }
            "smooth" => {
                let with = values
                    .iter()
                    .position(|value| value.eq_ignore_ascii_case("with"));
                let endpoint_end = with.unwrap_or(values.len());
                let endpoint = parse_shape_endpoint(&values[1..endpoint_end], units)?;
                let control = with
                    .map(|index| {
                        parse_shape_control(
                            &values[index + 1..],
                            units,
                            matches!(endpoint, ShapeEndpoint::By(_)),
                        )
                    })
                    .transpose()?;
                parsed.push(ShapeCommand::Smooth { endpoint, control });
            }
            "arc" => parsed.push(parse_shape_arc(&values[1..], units)?),
            _ => return Err("unsupported shape() command"),
        }
    }
    Ok(ShapeFunction {
        start,
        commands: parsed,
        fill_rule,
    })
}

fn parse_shape_endpoint(values: &[String], units: Units) -> Result<ShapeEndpoint, &'static str> {
    let Some(mode) = values.first().map(|value| value.to_ascii_lowercase()) else {
        return Err("shape command requires to or by");
    };
    match mode.as_str() {
        "to" => Ok(ShapeEndpoint::To(shape_position(&values[1..], units)?)),
        "by" => {
            let [x, y] = &values[1..] else {
                return Err("relative shape position requires two coordinates");
            };
            Ok(ShapeEndpoint::By(ShapePoint {
                x: length_percentage(x, units, true)?,
                y: length_percentage(y, units, true)?,
            }))
        }
        _ => Err("shape command requires to or by"),
    }
}

fn parse_shape_control(
    values: &[String],
    units: Units,
    relative_only: bool,
) -> Result<ShapeControl, &'static str> {
    let from = values
        .iter()
        .position(|value| value.eq_ignore_ascii_case("from"));
    let (coordinates, origin) = if let Some(from) = from {
        let origin = match values.get(from + 1).map(|value| value.to_ascii_lowercase()) {
            Some(value) if value == "start" => ControlOrigin::Start,
            Some(value) if value == "end" => ControlOrigin::End,
            Some(value) if value == "origin" => ControlOrigin::Origin,
            _ => return Err("control point from requires start, end, or origin"),
        };
        if from + 2 != values.len() {
            return Err("invalid control point reference");
        }
        (&values[..from], origin)
    } else {
        (values, ControlOrigin::Start)
    };
    if from.is_some() || relative_only {
        let [x, y] = coordinates else {
            return Err("relative control point requires two coordinates");
        };
        Ok(ShapeControl::Relative(
            ShapePoint {
                x: length_percentage(x, units, true)?,
                y: length_percentage(y, units, true)?,
            },
            origin,
        ))
    } else {
        Ok(ShapeControl::Absolute(shape_position(coordinates, units)?))
    }
}

fn split_shape_controls(values: &[String]) -> Result<Vec<&[String]>, &'static str> {
    let slash = values.iter().position(|value| value == "/");
    match slash {
        Some(index) if index > 0 && index + 1 < values.len() => {
            Ok(vec![&values[..index], &values[index + 1..]])
        }
        Some(_) => Err("invalid curve control point separator"),
        None if !values.is_empty() => Ok(vec![values]),
        None => Err("curve command requires control points"),
    }
}

fn shape_axis_value(
    value: &str,
    horizontal: bool,
    units: Units,
    relative: bool,
) -> Result<LengthPercentage, &'static str> {
    let keyword = match (horizontal, value.to_ascii_lowercase().as_str()) {
        (true, "left" | "x-start") | (false, "top" | "y-start") => Some(0.0),
        (_, "center") => Some(0.5),
        (true, "right" | "x-end") | (false, "bottom" | "y-end") => Some(1.0),
        _ => None,
    };
    if let Some(value) = keyword {
        if relative {
            return Err("relative hline/vline coordinates cannot use position keywords");
        }
        return Ok(LengthPercentage::Percentage(value));
    }
    length_percentage(value, units, true)
}

fn parse_shape_arc(values: &[String], units: Units) -> Result<ShapeCommand, &'static str> {
    let parameter = values
        .iter()
        .position(|value| is_shape_arc_parameter(value))
        .unwrap_or(values.len());
    let endpoint = parse_shape_endpoint(&values[..parameter], units)?;
    let mut radii = None;
    let mut direction_agnostic_radius = false;
    let mut clockwise = false;
    let mut sweep_seen = false;
    let mut large = false;
    let mut size_seen = false;
    let mut rotation = 0.0;
    let mut rotation_seen = false;
    let mut index = parameter;
    while index < values.len() {
        let value = values[index].to_ascii_lowercase();
        match value.as_str() {
            "of" => {
                if radii.is_some() {
                    return Err("duplicate arc radius");
                }
                index += 1;
                let start = index;
                while index < values.len() && !is_shape_arc_parameter(&values[index]) {
                    index += 1;
                }
                match &values[start..index] {
                    [radius] => {
                        let radius = length_percentage(radius, units, true)?;
                        radii = Some([radius, radius]);
                        direction_agnostic_radius = true;
                    }
                    [x, y] => {
                        radii = Some([
                            length_percentage(x, units, true)?,
                            length_percentage(y, units, true)?,
                        ]);
                    }
                    _ => return Err("arc of requires one or two radii"),
                }
            }
            "cw" | "ccw" => {
                if sweep_seen {
                    return Err("duplicate arc sweep direction");
                }
                clockwise = value == "cw";
                sweep_seen = true;
                index += 1;
            }
            "large" | "small" => {
                if size_seen {
                    return Err("duplicate arc size");
                }
                large = value == "large";
                size_seen = true;
                index += 1;
            }
            "rotate" => {
                if rotation_seen || index + 1 >= values.len() {
                    return Err("rotate requires one angle");
                }
                rotation = parse_shape_angle(&values[index + 1])?;
                rotation_seen = true;
                index += 2;
            }
            _ => return Err("invalid arc parameter"),
        }
    }
    Ok(ShapeCommand::Arc {
        endpoint,
        radii: radii.ok_or("arc command requires an of radius")?,
        direction_agnostic_radius,
        clockwise,
        large,
        rotation,
    })
}

fn is_shape_arc_parameter(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "of" | "cw" | "ccw" | "large" | "small" | "rotate"
    )
}

fn parse_shape_angle(value: &str) -> Result<f32, &'static str> {
    let mut parser = Parser::new(value);
    let angle = match parser.next().map_err(|_| "invalid shape angle")? {
        cssparser::Token::Number { value, .. } if *value == 0.0 => 0.0,
        cssparser::Token::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str()
        {
            "deg" => *value,
            "grad" => *value * 0.9,
            "rad" => value.to_degrees(),
            "turn" => *value * 360.0,
            _ => return Err("invalid shape angle unit"),
        },
        _ => return Err("invalid shape angle"),
    };
    parser
        .expect_exhausted()
        .map_err(|_| "invalid shape angle")?;
    angle
        .is_finite()
        .then_some(angle)
        .ok_or("invalid shape angle")
}

#[derive(Clone, Copy)]
enum PathToken {
    Command(char),
    Number(f32),
}

fn parse_path(value: &str) -> Result<(Vec<Vec<Vec2>>, FillRule), &'static str> {
    let mut values = crate::layout_style::components(value)?;
    values.retain(|value| value != ",");
    let fill_rule = if values
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("evenodd"))
    {
        values.remove(0);
        FillRule::Evenodd
    } else if values
        .first()
        .is_some_and(|value| value.eq_ignore_ascii_case("nonzero"))
    {
        values.remove(0);
        FillRule::Nonzero
    } else {
        FillRule::Nonzero
    };
    if values.len() != 1 {
        return Err("path() requires one quoted path string");
    }
    let mut parser = Parser::new(&values[0]);
    let data = match parser.next().map_err(|_| "invalid path() string")? {
        cssparser::Token::QuotedString(value) => value.to_string(),
        _ => return Err("path() requires one quoted path string"),
    };
    parser
        .expect_exhausted()
        .map_err(|_| "invalid path() string")?;
    Ok((parse_svg_path(&data)?, fill_rule))
}

fn parse_svg_path(data: &str) -> Result<Vec<Vec<Vec2>>, &'static str> {
    let tokens = path_tokens(data)?;
    let mut index = 0;
    let mut command = None;
    let mut current = Vec2::ZERO;
    let mut start = Vec2::ZERO;
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut cubic_control = None;
    let mut quadratic_control = None;
    let mut moved = false;
    while index < tokens.len() {
        if let PathToken::Command(next) = tokens[index] {
            command = Some(next);
            index += 1;
        }
        let command_value = command.ok_or("path data must begin with a command")?;
        if matches!(command_value, 'Z' | 'z') {
            if !moved {
                return Err("path close command has no subpath");
            }
            if points
                .last()
                .is_some_and(|point: &Vec2| (*point - start).length() > 0.001)
            {
                points.push(start);
            }
            current = start;
            command = None;
            cubic_control = None;
            quadratic_control = None;
            continue;
        }
        let arity = match command_value.to_ascii_lowercase() {
            'm' | 'l' | 't' => 2,
            'h' | 'v' => 1,
            'c' => 6,
            's' | 'q' => 4,
            'a' => 7,
            _ => return Err("unsupported SVG path command"),
        };
        if index + arity > tokens.len()
            || tokens[index..index + arity]
                .iter()
                .any(|token| matches!(token, PathToken::Command(_)))
        {
            return Err("incomplete SVG path command");
        }
        let mut numbers = Vec::with_capacity(arity);
        for token in &tokens[index..index + arity] {
            let PathToken::Number(value) = token else {
                unreachable!()
            };
            numbers.push(*value);
        }
        index += arity;
        let relative = command_value.is_ascii_lowercase();
        let point = |x: f32, y: f32| {
            if relative {
                current + vec2(x, y)
            } else {
                vec2(x, y)
            }
        };
        match command_value.to_ascii_lowercase() {
            'm' => {
                if !points.is_empty() {
                    contours.push(std::mem::take(&mut points));
                }
                current = point(numbers[0], numbers[1]);
                start = current;
                points.push(current);
                moved = true;
                command = Some(if relative { 'l' } else { 'L' });
                cubic_control = None;
                quadratic_control = None;
            }
            'l' => {
                current = point(numbers[0], numbers[1]);
                points.push(current);
                cubic_control = None;
                quadratic_control = None;
            }
            'h' => {
                current.x = if relative {
                    current.x + numbers[0]
                } else {
                    numbers[0]
                };
                points.push(current);
                cubic_control = None;
                quadratic_control = None;
            }
            'v' => {
                current.y = if relative {
                    current.y + numbers[0]
                } else {
                    numbers[0]
                };
                points.push(current);
                cubic_control = None;
                quadratic_control = None;
            }
            'c' => {
                let first = point(numbers[0], numbers[1]);
                let second = point(numbers[2], numbers[3]);
                let end = point(numbers[4], numbers[5]);
                append_cubic(&mut points, current, first, second, end);
                current = end;
                cubic_control = Some(second);
                quadratic_control = None;
            }
            's' => {
                let first = cubic_control.map_or(current, |control: Vec2| current * 2.0 - control);
                let second = point(numbers[0], numbers[1]);
                let end = point(numbers[2], numbers[3]);
                append_cubic(&mut points, current, first, second, end);
                current = end;
                cubic_control = Some(second);
                quadratic_control = None;
            }
            'q' => {
                let control = point(numbers[0], numbers[1]);
                let end = point(numbers[2], numbers[3]);
                append_quadratic(&mut points, current, control, end);
                current = end;
                quadratic_control = Some(control);
                cubic_control = None;
            }
            't' => {
                let control =
                    quadratic_control.map_or(current, |control: Vec2| current * 2.0 - control);
                let end = point(numbers[0], numbers[1]);
                append_quadratic(&mut points, current, control, end);
                current = end;
                quadratic_control = Some(control);
                cubic_control = None;
            }
            'a' => {
                let end = point(numbers[5], numbers[6]);
                append_arc(
                    &mut points,
                    current,
                    end,
                    numbers[0],
                    numbers[1],
                    numbers[2],
                    numbers[3] != 0.0,
                    numbers[4] != 0.0,
                );
                current = end;
                cubic_control = None;
                quadratic_control = None;
            }
            _ => unreachable!(),
        }
        if !points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite())
        {
            return Err("path coordinates must be finite");
        }
    }
    if !points.is_empty() {
        contours.push(points);
    }
    if contours.is_empty() || contours.iter().any(|points| points.len() < 3) {
        return Err("path() requires a subpath with at least three points");
    }
    for points in &mut contours {
        if points
            .last()
            .is_some_and(|point| (*point - points[0]).length() <= 0.001)
        {
            points.pop();
        }
    }
    Ok(contours)
}

fn path_tokens(data: &str) -> Result<Vec<PathToken>, &'static str> {
    let bytes = data.as_bytes();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() || bytes[index] == b',' {
            index += 1;
            continue;
        }
        if bytes[index].is_ascii_alphabetic() {
            tokens.push(PathToken::Command(bytes[index] as char));
            index += 1;
            continue;
        }
        let start = index;
        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let mut digits = 0;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            digits += 1;
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                digits += 1;
                index += 1;
            }
        }
        if digits == 0 {
            return Err("invalid SVG path number");
        }
        if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
            let exponent = index;
            index += 1;
            if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            let exponent_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if exponent_start == index {
                index = exponent;
            }
        }
        let value = data[start..index]
            .parse::<f32>()
            .map_err(|_| "invalid SVG path number")?;
        if !value.is_finite() {
            return Err("path coordinates must be finite");
        }
        tokens.push(PathToken::Number(value));
    }
    Ok(tokens)
}

fn append_cubic(points: &mut Vec<Vec2>, start: Vec2, first: Vec2, second: Vec2, end: Vec2) {
    for step in 1..=12 {
        let t = step as f32 / 12.0;
        let inverse = 1.0 - t;
        points.push(
            start * inverse.powi(3)
                + first * (3.0 * inverse.powi(2) * t)
                + second * (3.0 * inverse * t.powi(2))
                + end * t.powi(3),
        );
    }
}

fn append_quadratic(points: &mut Vec<Vec2>, start: Vec2, control: Vec2, end: Vec2) {
    for step in 1..=12 {
        let t = step as f32 / 12.0;
        let inverse = 1.0 - t;
        points.push(start * inverse.powi(2) + control * (2.0 * inverse * t) + end * t.powi(2));
    }
}

fn append_arc(
    points: &mut Vec<Vec2>,
    start: Vec2,
    end: Vec2,
    radius_x: f32,
    radius_y: f32,
    rotation: f32,
    large_arc: bool,
    sweep: bool,
) {
    let (mut rx, mut ry) = (radius_x.abs(), radius_y.abs());
    if rx == 0.0 || ry == 0.0 || (start - end).length() <= f32::EPSILON {
        points.push(end);
        return;
    }
    let phi = rotation.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let half = (start - end) * 0.5;
    let x_prime = cos_phi * half.x + sin_phi * half.y;
    let y_prime = -sin_phi * half.x + cos_phi * half.y;
    let scale = (x_prime.powi(2) / rx.powi(2) + y_prime.powi(2) / ry.powi(2)).sqrt();
    if scale > 1.0 {
        rx *= scale;
        ry *= scale;
    }
    let numerator =
        (rx.powi(2) * ry.powi(2) - rx.powi(2) * y_prime.powi(2) - ry.powi(2) * x_prime.powi(2))
            .max(0.0);
    let denominator = rx.powi(2) * y_prime.powi(2) + ry.powi(2) * x_prime.powi(2);
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coefficient = sign * (numerator / denominator.max(f32::EPSILON)).sqrt();
    let center_x_prime = coefficient * rx * y_prime / ry;
    let center_y_prime = coefficient * -ry * x_prime / rx;
    let center = vec2(
        cos_phi * center_x_prime - sin_phi * center_y_prime + (start.x + end.x) * 0.5,
        sin_phi * center_x_prime + cos_phi * center_y_prime + (start.y + end.y) * 0.5,
    );
    let first = vec2(
        (x_prime - center_x_prime) / rx,
        (y_prime - center_y_prime) / ry,
    );
    let second = vec2(
        (-x_prime - center_x_prime) / rx,
        (-y_prime - center_y_prime) / ry,
    );
    let start_angle = first.y.atan2(first.x);
    let mut sweep_angle = cross_vec(first, second).atan2(first.dot(second));
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= std::f32::consts::TAU;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += std::f32::consts::TAU;
    }
    let steps = (sweep_angle.abs() / (std::f32::consts::PI / 24.0))
        .ceil()
        .clamp(1.0, 96.0) as usize;
    for step in 1..=steps {
        let angle = start_angle + sweep_angle * step as f32 / steps as f32;
        let (sin, cos) = angle.sin_cos();
        points.push(vec2(
            center.x + cos_phi * rx * cos - sin_phi * ry * sin,
            center.y + sin_phi * rx * cos + cos_phi * ry * sin,
        ));
    }
}

fn shape_radius(value: &str, units: Units, negative: bool) -> Result<ShapeRadius, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "closest-side" => Ok(ShapeRadius::ClosestSide),
        "farthest-side" => Ok(ShapeRadius::FarthestSide),
        _ => Ok(ShapeRadius::LengthPercentage(length_percentage(
            value, units, negative,
        )?)),
    }
}

fn shape_position(values: &[String], units: Units) -> Result<ShapePosition, &'static str> {
    if values.is_empty() {
        return Err("position after at cannot be empty");
    }
    if values.len() > 4 {
        return Err("unsupported basic shape position");
    }
    if values.len() == 1 {
        return Ok(match values[0].to_ascii_lowercase().as_str() {
            "left" => ShapePosition {
                x: LengthPercentage::Percentage(0.0),
                ..ShapePosition::default()
            },
            "right" => ShapePosition {
                x: LengthPercentage::Percentage(1.0),
                ..ShapePosition::default()
            },
            "top" => ShapePosition {
                y: LengthPercentage::Percentage(0.0),
                ..ShapePosition::default()
            },
            "bottom" => ShapePosition {
                y: LengthPercentage::Percentage(1.0),
                ..ShapePosition::default()
            },
            "center" => ShapePosition::default(),
            _ => ShapePosition {
                x: length_percentage(&values[0], units, true)?,
                ..ShapePosition::default()
            },
        });
    }

    let mut x = None;
    let mut y = None;
    let mut x_from_end = false;
    let mut y_from_end = false;
    let mut index = 0;
    while index < values.len() {
        let value = values[index].to_ascii_lowercase();
        let edge_axis = match value.as_str() {
            "left" => Some((true, false)),
            "right" => Some((true, true)),
            "top" => Some((false, false)),
            "bottom" => Some((false, true)),
            _ => None,
        };
        if values.len() > 2
            && let Some((horizontal, from_end)) = edge_axis
            && values.get(index + 1).is_some_and(|next| {
                !matches!(
                    next.to_ascii_lowercase().as_str(),
                    "left" | "right" | "top" | "bottom" | "center"
                )
            })
        {
            let offset = length_percentage(&values[index + 1], units, true)?;
            let axis = if horizontal { &mut x } else { &mut y };
            if axis.replace(offset).is_some() {
                return Err("duplicate axis in shape position");
            }
            if horizontal {
                x_from_end = from_end;
            } else {
                y_from_end = from_end;
            }
            index += 2;
            continue;
        }
        match value.as_str() {
            "left" | "right" => {
                let axis = if value == "left" { 0.0 } else { 1.0 };
                if x.replace(LengthPercentage::Percentage(axis)).is_some() {
                    return Err("duplicate horizontal axis in shape position");
                }
            }
            "top" | "bottom" => {
                let axis = if value == "top" { 0.0 } else { 1.0 };
                if y.replace(LengthPercentage::Percentage(axis)).is_some() {
                    return Err("duplicate vertical axis in shape position");
                }
            }
            "center" => {
                if x.is_none() {
                    x = Some(LengthPercentage::Percentage(0.5));
                } else if y.is_none() {
                    y = Some(LengthPercentage::Percentage(0.5));
                } else {
                    return Err("too many coordinates in shape position");
                }
            }
            _ => {
                let coordinate = length_percentage(&values[index], units, true)?;
                if x.is_none() {
                    x = Some(coordinate);
                } else if y.is_none() {
                    y = Some(coordinate);
                } else {
                    return Err("too many coordinates in shape position");
                }
            }
        }
        index += 1;
    }
    Ok(ShapePosition {
        x: x.unwrap_or(LengthPercentage::Percentage(0.5)),
        y: y.unwrap_or(LengthPercentage::Percentage(0.5)),
        x_from_end,
        y_from_end,
    })
}

fn rect_edge(value: &str, edge: usize, units: Units) -> Result<LengthPercentage, &'static str> {
    if value.eq_ignore_ascii_case("auto") {
        return Ok(LengthPercentage::Percentage(if matches!(edge, 1 | 2) {
            1.0
        } else {
            0.0
        }));
    }
    length_percentage(value, units, true)
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
            let intervals = match (selected, side) {
                (3, 0) | (1, 2) | (0, 1) | (2, 3) => vec![(0.0, extent)],
                (3, 2) | (1, 0) | (0, 3) | (2, 1) => vec![(length - extent, length)],
                _ => Vec::new(),
            };
            intervals
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

pub(crate) fn rounded_path(rect: Rect, radii: [Vec2; 4], shapes: [CornerShape; 4]) -> Vec<Pos2> {
    let mut points = Vec::with_capacity(40);
    for corner in 0..4 {
        let radius = radii[corner];
        if radius.x <= 0.0 || radius.y <= 0.0 {
            let point = match corner {
                0 => pos2(rect.left(), rect.top()),
                1 => pos2(rect.right(), rect.top()),
                2 => pos2(rect.right(), rect.bottom()),
                _ => pos2(rect.left(), rect.bottom()),
            };
            points.extend(std::iter::repeat_n(point, 9));
            continue;
        }
        for step in 0..=8 {
            let t = step as f32 / 8.0;
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

fn filled_paths_difference(
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
    for triangle in mesh.indices.chunks_exact(3) {
        let vertices = triangle
            .iter()
            .map(|index| mesh.vertices[*index as usize].clone())
            .collect::<Vec<_>>();
        for quad in &clip.quads {
            let polygon = clip_vertex_polygon(vertices.clone(), quad);
            if polygon.len() < 3 {
                continue;
            }
            for index in 1..polygon.len() - 1 {
                let first = output.vertices.len() as u32;
                output.vertices.extend([
                    polygon[0].clone(),
                    polygon[index].clone(),
                    polygon[index + 1].clone(),
                ]);
                output.add_triangle(first, first + 1, first + 2);
            }
        }
    }
    output
}

fn clip_vertex_polygon(
    mut polygon: Vec<egui::epaint::Vertex>,
    clip: &[Pos2; 4],
) -> Vec<egui::epaint::Vertex> {
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
        let input = std::mem::take(&mut polygon);
        let mut previous = input.last().unwrap().clone();
        let mut previous_distance = cross_vec(direction, previous.pos - start) * orientation;
        for current in input {
            let current_distance = cross_vec(direction, current.pos - start) * orientation;
            let previous_inside = previous_distance >= -0.001;
            let current_inside = current_distance >= -0.001;
            if previous_inside != current_inside {
                let amount = previous_distance / (previous_distance - current_distance);
                polygon.push(interpolate_vertex(&previous, &current, amount));
            }
            if current_inside {
                polygon.push(current.clone());
            }
            previous = current;
            previous_distance = current_distance;
        }
    }
    polygon
}

fn interpolate_vertex(
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

fn path_fill_quads(paths: Vec<Vec<Pos2>>, fill_rule: FillRule) -> Vec<[Pos2; 4]> {
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
            .filter_map(|(a, b)| {
                ((a.y < middle && b.y > middle) || (b.y < middle && a.y > middle)).then(|| {
                    let direction = if b.y > a.y { 1_i32 } else { -1_i32 };
                    let x = x_at_y(a, b, middle);
                    (x, direction, (a, b))
                })
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
            } else if was_inside && !is_inside {
                if let (Some((left_a, left_b)), Some((right_a, right_b))) =
                    (left.take(), group.first().map(|crossing| crossing.2))
                {
                    let quad = vec![
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
    }
    quads
}

fn path_difference_quads(
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
            .filter_map(|(a, b, shape)| {
                ((a.y < middle && b.y > middle) || (b.y < middle && a.y > middle)).then(|| {
                    let direction = if b.y > a.y { 1_i32 } else { -1_i32 };
                    (x_at_y(a, b, middle), direction, (a, b), shape)
                })
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

fn x_at_y(a: Pos2, b: Pos2, y: f32) -> f32 {
    if (b.y - a.y).abs() <= f32::EPSILON {
        a.x.min(b.x)
    } else {
        a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y)
    }
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

fn radius_quad<'a>(
    values: &'a [String],
    units: Units,
) -> Result<[LengthPercentage; 4], &'static str> {
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

fn length_percentage(
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

pub(crate) fn border_image_urls(value: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for part in crate::layout_style::components(value).unwrap_or_default() {
        if let Ok(image) = crate::css::parse_background_image(&part) {
            image.collect_urls(&mut urls);
        }
    }
    urls
}

pub(crate) fn border_image_source(name: &str, value: &str) -> Option<crate::css::BackgroundImage> {
    match name {
        "border-image-source" => crate::css::parse_background_image(value).ok(),
        "border" => Some(crate::css::BackgroundImage::None),
        "border-image" => {
            let values = crate::layout_style::components(value).ok()?;
            let source = values
                .iter()
                .find(|value| is_border_image_source_token(value));
            Some(source.map_or(crate::css::BackgroundImage::None, |value| {
                crate::css::parse_background_image(value).expect("validated border image source")
            }))
        }
        _ => None,
    }
}

fn is_border_image_source_token(token: &str) -> bool {
    token.eq_ignore_ascii_case("none")
        || token
            .get(..token.find('(').unwrap_or(0))
            .is_some_and(|function| {
                [
                    "url",
                    "element",
                    "image",
                    "linear-gradient",
                    "repeating-linear-gradient",
                    "radial-gradient",
                    "repeating-radial-gradient",
                    "conic-gradient",
                    "repeating-conic-gradient",
                    "cross-fade",
                    "image-set",
                ]
                .iter()
                .any(|name| function.eq_ignore_ascii_case(name))
            })
}

fn apply_border_image(
    name: &str,
    value: &str,
    values: &[String],
    image: &mut BorderImage,
    units: Units,
) -> Result<(), &'static str> {
    match name {
        "border-image" => *image = parse_border_image(values, units)?,
        "border-image-source" => {
            image.source = crate::css::parse_background_image(value)
                .map_err(|_| "invalid border image source")?;
        }
        "border-image-slice" => {
            (image.slice, image.fill) = parse_image_slice(values, units)?;
        }
        "border-image-width" => image.width = parse_image_width(values, units)?,
        "border-image-outset" => image.outset = parse_image_outset(values, units)?,
        "border-image-repeat" => image.repeat = parse_image_repeat(values)?,
        _ => return Err("unsupported border-image property"),
    }
    Ok(())
}

fn parse_border_image(values: &[String], units: Units) -> Result<BorderImage, &'static str> {
    let mut sections = vec![Vec::new()];
    for value in values {
        if value == "/" {
            if sections.len() == 3 {
                return Err("too many border-image slashes");
            }
            sections.push(Vec::new());
        } else {
            sections.last_mut().unwrap().push(value.clone());
        }
    }
    let mut image = BorderImage::default();
    let mut initial = Vec::new();
    let mut source_seen = false;
    for token in &sections[0] {
        if let Some(repeat) = image_repeat(token) {
            if image.repeat[0] != ImageRepeat::Stretch {
                image.repeat[1] = repeat;
            } else {
                image.repeat[0] = repeat;
                image.repeat[1] = repeat;
            }
        } else if is_border_image_source_token(token) {
            if source_seen {
                return Err("duplicate border image source");
            }
            image.source = crate::css::parse_background_image(token)
                .map_err(|_| "invalid border image source")?;
            source_seen = true;
        } else {
            initial.push(token.clone());
        }
    }
    if initial.is_empty() && sections.len() > 1 {
        return Err("border-image slash values require a slice");
    }
    if !initial.is_empty() {
        (image.slice, image.fill) = parse_image_slice(&initial, units)?;
    }
    if sections.len() == 2 && sections[1].is_empty() {
        return Err("border-image width requires a value");
    }
    if sections.len() == 3 && sections[2].is_empty() {
        return Err("border-image outset requires a value");
    }
    if sections.len() > 1 {
        let width_values = sections[1].clone();
        if !width_values.is_empty() {
            image.width = parse_image_width(&width_values, units)?;
        }
    }
    if sections.len() > 2 {
        let outset_values = sections[2].clone();
        if !outset_values.is_empty() {
            image.outset = parse_image_outset(&outset_values, units)?;
        }
    }
    let repeat_values = sections[0]
        .iter()
        .filter_map(|value| image_repeat(value))
        .collect::<Vec<_>>();
    if repeat_values.len() > 2 {
        return Err("invalid border-image-repeat");
    }
    if let Some(first) = repeat_values.first() {
        image.repeat = [*first, *repeat_values.get(1).unwrap_or(first)];
    }
    Ok(image)
}

fn parse_image_slice(
    values: &[String],
    units: Units,
) -> Result<([ImageSlice; 4], bool), &'static str> {
    let mut fill = false;
    let mut slices = Vec::with_capacity(values.len());
    for value in values {
        if value.eq_ignore_ascii_case("fill") {
            if fill {
                return Err("duplicate border-image-slice fill");
            }
            fill = true;
            continue;
        }
        let value = value.trim();
        let slice = if let Ok(Some(number)) = crate::math::number(value, units) {
            if number < 0.0 || !number.is_finite() {
                return Err("invalid border-image-slice");
            }
            ImageSlice::Number(number)
        } else if let Ok(Some(percentage)) = crate::math::percentage(value, units, false) {
            ImageSlice::PercentageCalc(percentage)
        } else if let Some(percent) = value.strip_suffix('%') {
            let percent = percent
                .parse::<f32>()
                .map_err(|_| "invalid border-image-slice")?;
            if percent < 0.0 || !percent.is_finite() {
                return Err("invalid border-image-slice");
            }
            ImageSlice::Percentage(percent / 100.0)
        } else {
            let number = value
                .parse::<f32>()
                .map_err(|_| "invalid border-image-slice")?;
            if number < 0.0 || !number.is_finite() {
                return Err("invalid border-image-slice");
            }
            ImageSlice::Number(number)
        };
        slices.push(slice);
    }
    if slices.is_empty() {
        return Err("border-image-slice requires a value");
    }
    let expanded = quad(&slices)?;
    Ok((expanded, fill))
}

fn parse_image_width(values: &[String], units: Units) -> Result<[ImageWidth; 4], &'static str> {
    let values = values
        .iter()
        .map(|value| {
            if value.eq_ignore_ascii_case("auto") {
                return Ok(ImageWidth::Auto);
            }
            if let Ok(Some(number)) = crate::math::number(value, units) {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageWidth::Number(number));
                }
                return Err("invalid border-image-width");
            }
            if let Ok(number) = value.parse::<f32>() {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageWidth::Number(number));
                }
                return Err("invalid border-image-width");
            }
            Ok(ImageWidth::Length(length_percentage(value, units, false)?))
        })
        .collect::<Result<Vec<_>, _>>()?;
    quad(&values)
}

fn parse_image_outset(values: &[String], units: Units) -> Result<[ImageOutset; 4], &'static str> {
    let values = values
        .iter()
        .map(|value| {
            if let Ok(Some(number)) = crate::math::number(value, units) {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageOutset::Number(number));
                }
                return Err("invalid border-image-outset");
            }
            if let Ok(number) = value.parse::<f32>() {
                if number >= 0.0 && number.is_finite() {
                    return Ok(ImageOutset::Number(number));
                }
                return Err("invalid border-image-outset");
            }
            let length = if let Some(handle) = crate::math::length(value, units, false, false)? {
                crate::math::resolve(handle, 0.0)
            } else {
                let (length, percentage) = crate::layout_style::length_value(value, units, false)?;
                if percentage {
                    return Err("border-image-outset cannot be a percentage");
                }
                length
            };
            Ok(ImageOutset::Length(length))
        })
        .collect::<Result<Vec<_>, _>>()?;
    quad(&values)
}

fn image_repeat(value: &str) -> Option<ImageRepeat> {
    Some(match value.to_ascii_lowercase().as_str() {
        "stretch" => ImageRepeat::Stretch,
        "repeat" => ImageRepeat::Repeat,
        "round" => ImageRepeat::Round,
        "space" => ImageRepeat::Space,
        _ => return None,
    })
}

fn parse_image_repeat(values: &[String]) -> Result<[ImageRepeat; 2], &'static str> {
    if values.is_empty() || values.len() > 2 {
        return Err("invalid border-image-repeat");
    }
    let first = image_repeat(&values[0]).ok_or("invalid border-image-repeat")?;
    let second = values
        .get(1)
        .map(|value| image_repeat(value).ok_or("invalid border-image-repeat"))
        .transpose()?
        .unwrap_or(first);
    Ok([first, second])
}

fn parse_shadows(value: &str, units: Units) -> Result<Vec<Shadow>, &'static str> {
    if value.trim().eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    split_commas(value)
        .into_iter()
        .map(|value| parse_shadow(value, units))
        .collect()
}

fn set_shadows(style: &mut DecorationStyle, shadows: Vec<Shadow>) {
    if shadows.is_empty() {
        style.shadow_offsets = vec![None];
        style.shadow_blurs = vec![0.0];
        style.shadow_spreads = vec![0.0];
        style.shadow_colors = vec![None];
        style.shadow_positions = vec![ShadowPosition::Outset];
    } else {
        style.shadow_offsets = shadows.iter().map(|shadow| shadow.offset).collect();
        style.shadow_blurs = shadows.iter().map(|shadow| shadow.blur).collect();
        style.shadow_spreads = shadows.iter().map(|shadow| shadow.spread).collect();
        style.shadow_colors = shadows.iter().map(|shadow| shadow.color).collect();
        style.shadow_positions = shadows.iter().map(|shadow| shadow.position).collect();
    }
}

fn copy_shadows(out: &mut DecorationStyle, source: &DecorationStyle) {
    out.shadow_offsets.clone_from(&source.shadow_offsets);
    out.shadow_blurs.clone_from(&source.shadow_blurs);
    out.shadow_spreads.clone_from(&source.shadow_spreads);
    out.shadow_colors.clone_from(&source.shadow_colors);
    out.shadow_positions.clone_from(&source.shadow_positions);
}

fn apply_shadow_longhand(
    name: &str,
    value: &str,
    style: &mut DecorationStyle,
    units: Units,
) -> Result<(), &'static str> {
    let values = split_commas(value);
    if values.is_empty() || values.iter().any(|value| value.is_empty()) {
        return Err("empty shadow list");
    }
    match name {
        "box-shadow-color" => {
            style.shadow_colors = values
                .iter()
                .map(|value| color_value(value))
                .collect::<Result<_, _>>()?;
        }
        "box-shadow-offset" => {
            style.shadow_offsets = values
                .iter()
                .map(|value| {
                    let parts = crate::layout_style::components(value)?;
                    if parts.len() == 1 && parts[0].eq_ignore_ascii_case("none") {
                        return Ok(None);
                    }
                    if parts.is_empty() || parts.len() > 2 {
                        return Err("invalid shadow offset");
                    }
                    let x = shadow_length(&parts[0], units, true)?;
                    let y = if let Some(second) = parts.get(1) {
                        shadow_length(second, units, true)?
                    } else {
                        x
                    };
                    Ok(Some(vec2(x, y)))
                })
                .collect::<Result<_, _>>()?;
        }
        "box-shadow-blur" => {
            style.shadow_blurs = values
                .iter()
                .map(|value| shadow_length(value, units, false))
                .collect::<Result<_, _>>()?;
        }
        "box-shadow-spread" => {
            style.shadow_spreads = values
                .iter()
                .map(|value| shadow_length(value, units, true))
                .collect::<Result<_, _>>()?;
        }
        _ => {
            style.shadow_positions = values
                .iter()
                .map(|value| match value.to_ascii_lowercase().as_str() {
                    "inset" => Ok(ShadowPosition::Inset),
                    "outset" => Ok(ShadowPosition::Outset),
                    _ => Err("invalid shadow position"),
                })
                .collect::<Result<_, _>>()?;
        }
    }
    Ok(())
}

fn shadow_length(value: &str, units: Units, negative: bool) -> Result<f32, &'static str> {
    if let Some(handle) = crate::math::length(value, units, negative, false)? {
        return Ok(crate::math::resolve(handle, 0.0));
    }
    let (value, percentage) = crate::layout_style::length_value(value, units, negative)?;
    if percentage {
        return Err("box-shadow lengths cannot be percentages");
    }
    Ok(value)
}

fn shadows(style: &DecorationStyle) -> Vec<Shadow> {
    let count = style.shadow_offsets.len();
    (0..count)
        .map(|index| Shadow {
            offset: style.shadow_offsets[index],
            blur: coordinated(&style.shadow_blurs, index),
            spread: coordinated(&style.shadow_spreads, index),
            color: coordinated(&style.shadow_colors, index),
            position: coordinated(&style.shadow_positions, index),
        })
        .collect()
}

fn coordinated<T: Copy>(values: &[T], index: usize) -> T {
    values[index % values.len()]
}

fn parse_shadow(value: &str, units: Units) -> Result<Shadow, &'static str> {
    let values = crate::layout_style::components(value)?;
    let mut color = None;
    let mut color_seen = false;
    let mut position = ShadowPosition::Outset;
    let mut position_seen = false;
    let mut offset: Option<Option<Vec2>> = None;
    let mut lengths = Vec::new();
    for value in &values {
        match value.to_ascii_lowercase().as_str() {
            "inset" | "outset" => {
                if position_seen {
                    return Err("duplicate shadow position");
                }
                position = if value.eq_ignore_ascii_case("inset") {
                    ShadowPosition::Inset
                } else {
                    ShadowPosition::Outset
                };
                position_seen = true;
            }
            "none" => {
                if offset.is_some() || !lengths.is_empty() {
                    return Err("invalid none shadow offset");
                }
                offset = Some(None);
            }
            _ => {
                if let Ok(parsed) = color_value(value) {
                    if color_seen {
                        return Err("duplicate shadow color");
                    }
                    color = parsed;
                    color_seen = true;
                } else {
                    let length = shadow_length(value, units, true)?;
                    lengths.push(length);
                }
            }
        }
    }
    let no_offset = offset == Some(None);
    if lengths.len() > if no_offset { 2 } else { 4 } {
        return Err("invalid box-shadow");
    }
    let shadow_offset = if no_offset {
        None
    } else if lengths.len() >= 2 {
        Some(vec2(lengths[0], lengths[1]))
    } else {
        return Err("box-shadow requires two offsets");
    };
    let blur = lengths
        .get(if no_offset { 0 } else { 2 })
        .copied()
        .unwrap_or(0.0);
    if blur < 0.0 {
        return Err("negative shadow blur");
    }
    let spread = lengths
        .get(if no_offset { 1 } else { 3 })
        .copied()
        .unwrap_or(0.0);
    Ok(Shadow {
        offset: shadow_offset,
        blur,
        spread,
        color: if !color_seen && no_offset {
            Some(Color32::TRANSPARENT)
        } else {
            color
        },
        position,
    })
}

fn color_value(value: &str) -> Result<Option<Color32>, &'static str> {
    if value.eq_ignore_ascii_case("currentcolor") {
        return Ok(None);
    }
    crate::css::parse_color(&mut Parser::new(value))
        .map(Some)
        .map_err(|_| "invalid shadow color")
}

fn split_commas(value: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    let mut quote = None;
    for (index, character) in value.char_indices() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(value[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    result.push(value[start..].trim());
    result
}

pub(crate) fn shadow_shapes(
    rect: Rect,
    style: &BoxStyle,
    current: Color32,
    position: ShadowPosition,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
) -> Vec<Shape> {
    let radii = resolved_radii(style, rect);
    let border_widths = style.layout.border.map(|value| {
        let value = value.into_raw();
        match value.tag() {
            taffy::CompactLength::CALC_TAG => crate::math::resolve(value.calc_value(), 0.0),
            taffy::CompactLength::LENGTH_TAG => value.value(),
            _ => 0.0,
        }
    });
    let widths = [
        border_widths.top,
        border_widths.right,
        border_widths.bottom,
        border_widths.left,
    ];
    let mut shapes = Vec::new();
    for shadow in shadows(&style.decorations).into_iter().rev() {
        if shadow.position != position {
            continue;
        }
        let Some(offset) = shadow.offset else {
            continue;
        };
        let color = shadow.color.unwrap_or(current);
        if position == ShadowPosition::Inset {
            shapes.extend(inset_shadow_shapes(
                rect, style, radii, &shadow, color, stroke, padding, margin, widths,
            ));
            continue;
        }

        let blur = shadow.blur;
        let count = if blur > 0.0 { 96 } else { 1 };
        let sigma = blur * 0.5;
        for step in 0..count {
            let (distance, alpha) = if count == 1 {
                (shadow.spread, f32::from(color.a()))
            } else {
                let outer_distance = shadow.spread + sigma * 3.0;
                let inner_distance = shadow.spread - sigma * 3.0;
                let previous_distance = outer_distance
                    + (inner_distance - outer_distance) * ((step as f32 - 0.5) / count as f32);
                let distance = outer_distance
                    + (inner_distance - outer_distance) * ((step as f32 + 0.5) / count as f32);
                let previous_opacity = if step == 0 {
                    0.0
                } else {
                    shadow_opacity(previous_distance, sigma)
                };
                let opacity = shadow_opacity(distance, sigma);
                (
                    distance,
                    (opacity - previous_opacity) / (1.0 - previous_opacity) * f32::from(color.a()),
                )
            };
            let target = rect.translate(offset).expand(distance);
            if !target.is_positive() || alpha <= 0.0 {
                continue;
            }
            let target_radii =
                outset_radii(rect, radii, [distance; 4]).map(|radius| radius.max(Vec2::ZERO));
            let paint = shadow_color(color, alpha);
            if let Some(border_shape) = &style.decorations.border_shape {
                let fill_rule = border_shape.outer_fill_rule();
                let source_paths = border_shape.outer_paths(rect, stroke, padding, margin, widths);
                let paths = offset_filled_paths(&source_paths, fill_rule, distance)
                    .into_iter()
                    .map(|path| path.into_iter().map(|point| point + offset).collect())
                    .collect();
                shapes.push(filled_paths_difference(
                    paths,
                    fill_rule,
                    source_paths,
                    fill_rule,
                    paint,
                ));
            } else {
                let points = rounded_path(target, target_radii, style.decorations.corner_shapes);
                let source = rounded_path(rect, radii, style.decorations.corner_shapes);
                shapes.push(filled_paths_difference(
                    vec![points],
                    FillRule::Nonzero,
                    vec![source],
                    FillRule::Nonzero,
                    paint,
                ));
            }
        }
    }
    shapes
}

pub(crate) fn inset_shadow_extent(rect: Rect, style: &BoxStyle) -> f32 {
    shadows(&style.decorations)
        .into_iter()
        .filter(|shadow| shadow.position == ShadowPosition::Inset)
        .filter_map(|shadow| {
            let offset = shadow.offset?;
            Some(
                (shadow.spread + shadow.blur * 1.5 + offset.x.abs().max(offset.y.abs()))
                    .max(0.0)
                    .min(rect.width().min(rect.height()) * 0.5),
            )
        })
        .fold(0.0, f32::max)
}

fn inset_shadow_shapes(
    rect: Rect,
    style: &BoxStyle,
    outer_radii: [Vec2; 4],
    shadow: &Shadow,
    color: Color32,
    stroke: egui::epaint::MarginF32,
    padding: egui::epaint::MarginF32,
    margin: egui::epaint::MarginF32,
    widths: [f32; 4],
) -> Vec<Shape> {
    use egui::epaint::{Mesh, Vertex};

    let Some(offset) = shadow.offset else {
        return Vec::new();
    };
    let blur = shadow.blur;
    let thickness = (shadow.spread + blur * 1.5 + offset.x.abs().max(offset.y.abs()))
        .max(0.0)
        .min(rect.width().min(rect.height()) * 0.5);
    if thickness <= 0.0 {
        return Vec::new();
    }
    let count = if blur > 0.0 {
        64
    } else {
        thickness.ceil().max(1.0) as usize
    };
    let (base_paths, fill_rule) = if let Some(border_shape) = &style.decorations.border_shape {
        match border_shape.inner_paths(rect, stroke, padding, margin, widths) {
            Some(paths) => (
                paths,
                border_shape.inner_fill_rule().unwrap_or(FillRule::Nonzero),
            ),
            None => (
                border_shape.outer_paths(rect, stroke, padding, margin, widths),
                border_shape.outer_fill_rule(),
            ),
        }
    } else {
        let padding_box = crate::nonnegative_rect(rect - stroke);
        let radii = [
            vec2(
                (outer_radii[0].x - stroke.left).max(0.0),
                (outer_radii[0].y - stroke.top).max(0.0),
            ),
            vec2(
                (outer_radii[1].x - stroke.right).max(0.0),
                (outer_radii[1].y - stroke.top).max(0.0),
            ),
            vec2(
                (outer_radii[2].x - stroke.right).max(0.0),
                (outer_radii[2].y - stroke.bottom).max(0.0),
            ),
            vec2(
                (outer_radii[3].x - stroke.left).max(0.0),
                (outer_radii[3].y - stroke.bottom).max(0.0),
            ),
        ];
        (
            vec![rounded_path(
                padding_box,
                radii,
                style.decorations.corner_shapes,
            )],
            FillRule::Nonzero,
        )
    };
    let base_paths = base_paths
        .into_iter()
        .filter(|path| path.len() >= 3)
        .collect::<Vec<_>>();
    if base_paths.is_empty() {
        return Vec::new();
    }
    let directions = (0..base_paths.len())
        .map(|index| contour_expansion_direction(&base_paths, index, fill_rule))
        .collect::<Vec<_>>();
    let sigma = blur * 0.5;
    let mut mesh = Mesh::default();
    for (base, contour_direction) in base_paths.iter().zip(directions) {
        for ring in 0..count {
            let outer_inset = thickness * ring as f32 / count as f32;
            let inner_inset = thickness * (ring + 1) as f32 / count as f32;
            let outer_points = offset_polygon(base, -contour_direction * outer_inset);
            let inner_points = offset_polygon(base, -contour_direction * inner_inset);
            if outer_points.len() != inner_points.len() || outer_points.len() < 3 {
                break;
            }
            for index in 0..outer_points.len() {
                let next = (index + 1) % outer_points.len();
                let direction = base[next] - base[index];
                let normal = outward_normal(base, index, direction) * contour_direction;
                let offset = offset.dot(normal) - shadow.spread;
                let outer_alpha = if blur == 0.0 {
                    if outer_inset + offset < 0.0 {
                        f32::from(color.a())
                    } else {
                        0.0
                    }
                } else {
                    shadow_opacity(outer_inset + offset, sigma) * f32::from(color.a())
                };
                let inner_alpha = if blur == 0.0 {
                    if inner_inset + offset < 0.0 {
                        f32::from(color.a())
                    } else {
                        0.0
                    }
                } else {
                    shadow_opacity(inner_inset + offset, sigma) * f32::from(color.a())
                };
                let first = mesh.vertices.len() as u32;
                mesh.vertices.extend([
                    Vertex {
                        pos: outer_points[index],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, outer_alpha),
                    },
                    Vertex {
                        pos: outer_points[next],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, outer_alpha),
                    },
                    Vertex {
                        pos: inner_points[next],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, inner_alpha),
                    },
                    Vertex {
                        pos: inner_points[index],
                        uv: Pos2::ZERO,
                        color: shadow_color(color, inner_alpha),
                    },
                ]);
                mesh.add_triangle(first, first + 1, first + 2);
                mesh.add_triangle(first, first + 2, first + 3);
            }
        }
    }
    vec![Shape::Mesh(mesh.into())]
}

fn offset_filled_paths(paths: &[Vec<Pos2>], fill_rule: FillRule, distance: f32) -> Vec<Vec<Pos2>> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let direction = contour_expansion_direction(paths, index, fill_rule);
            offset_polygon(path, distance * direction)
        })
        .collect()
}

fn contour_expansion_direction(paths: &[Vec<Pos2>], index: usize, fill_rule: FillRule) -> f32 {
    let path = &paths[index];
    if path.len() < 3 {
        return 1.0;
    }
    let Some((edge_index, direction)) = (0..path.len()).find_map(|edge_index| {
        let direction = path[(edge_index + 1) % path.len()] - path[edge_index];
        (direction.length_sq() > 0.000_001).then_some((edge_index, direction))
    }) else {
        return 1.0;
    };
    let extent = path.iter().map(|point| point.to_vec2()).fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(min, max), point| (min.min(point), max.max(point)),
    );
    let epsilon = (extent.1 - extent.0).max_elem() * 0.0001;
    let sample = path[edge_index].lerp(path[(edge_index + 1) % path.len()], 0.5)
        - outward_normal(path, edge_index, direction) * epsilon.max(0.0001);
    let own_winding = path_winding_number(path, sample);
    let other_winding = paths
        .iter()
        .enumerate()
        .filter(|(other_index, _)| *other_index != index)
        .map(|(_, path)| path_winding_number(path, sample));
    match fill_rule {
        FillRule::Evenodd => {
            if other_winding.map(i32::unsigned_abs).sum::<u32>() % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        }
        FillRule::Nonzero => {
            let other_winding = other_winding.sum::<i32>();
            match (other_winding == 0, other_winding + own_winding == 0) {
                (true, false) => 1.0,
                (false, true) => -1.0,
                _ => 0.0,
            }
        }
    }
}

fn path_winding_number(path: &[Pos2], point: Pos2) -> i32 {
    let mut winding = 0;
    for index in 0..path.len() {
        let start = path[index];
        let end = path[(index + 1) % path.len()];
        let cross = cross_vec(end - start, point - start);
        if start.y <= point.y {
            if end.y > point.y && cross > 0.0 {
                winding += 1;
            }
        } else if end.y <= point.y && cross < 0.0 {
            winding -= 1;
        }
    }
    winding
}

fn offset_polygon(points: &[Pos2], amount: f32) -> Vec<Pos2> {
    if points.len() < 3 || amount.abs() < 0.0001 {
        return points.to_vec();
    }
    (0..points.len())
        .map(|index| {
            let previous = points[(index + points.len() - 1) % points.len()];
            let point = points[index];
            let next = points[(index + 1) % points.len()];
            let before = (point - previous).normalized();
            let after = (next - point).normalized();
            let n_before = outward_normal(points, index, before);
            let n_after = outward_normal(points, (index + 1) % points.len(), after);
            let a = point + n_before * amount;
            let b = point + n_after * amount;
            let denominator = cross_vec(before, after);
            if denominator.abs() < 0.0001 {
                point + (n_before + n_after).normalized() * amount
            } else {
                a + before * (cross_vec(b - a, after) / denominator)
            }
        })
        .collect()
}

fn outward_normal(points: &[Pos2], _index: usize, direction: Vec2) -> Vec2 {
    let area = (0..points.len())
        .map(|index| {
            let a = points[index];
            let b = points[(index + 1) % points.len()];
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>();
    if area >= 0.0 {
        vec2(direction.y, -direction.x).normalized()
    } else {
        vec2(-direction.y, direction.x).normalized()
    }
}

fn cross_vec(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn shadow_opacity(distance: f32, sigma: f32) -> f32 {
    if sigma <= 0.0 {
        return if distance < 0.0 { 1.0 } else { 0.0 };
    }
    0.5 * (1.0 - erf(distance / (sigma * std::f32::consts::SQRT_2)))
}

fn erf(value: f32) -> f32 {
    let sign = value.signum();
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let polynomial = (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_8) * t - 0.284_496_72)
        * t
        + 0.254_829_6)
        * t;
    sign * (1.0 - polynomial * (-x * x).exp())
}

fn shadow_color(color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        color.r(),
        color.g(),
        color.b(),
        alpha.round().clamp(0.0, 255.0) as u8,
    )
}
