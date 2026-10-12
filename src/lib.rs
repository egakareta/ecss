#![deny(missing_docs)]

//! Apply CSS to egui widgets.
//!
//! ```no_run
//! use trcss::{SelectorExt as _, Stylesheet};
//!
//! # fn ui(ui: &mut egui::Ui, context: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
//! let css = Stylesheet::from_file("theme.css")?;
//! css.install(context);
//! ui.add(egui::Button::new("Save").selector(".primary"));
//! # Ok(())
//! # }
//! ```

use std::{error::Error, fmt, path::Path, sync::Arc};

use egui::{Color32, CornerRadius, Id, Response, Ui, Widget, style::WidgetVisuals};

const STYLESHEET_ID: &str = "trcss.stylesheet";
const BACKGROUND_ID: &str = "trcss.background";

/// A parsed stylesheet containing supported class rules.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
}

impl Stylesheet {
    /// Reads and parses a stylesheet from a UTF-8 CSS file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CssFileError> {
        let source = std::fs::read_to_string(path).map_err(CssFileError::Read)?;
        Self::parse(&source).map_err(CssFileError::Parse)
    }

    /// Parses class rules from CSS text.
    ///
    /// Supported selectors are `.class`, `.class:hover`, and `.class:active`.
    /// Supported declarations are `color`, `background-color`, `padding`, and
    /// `border-radius`. Lengths are interpreted as egui points. Unknown
    /// declarations are ignored.
    pub fn parse(source: &str) -> Result<Self, CssError> {
        let source = remove_comments(source)?;
        let mut rules = Vec::new();
        let mut offset = 0;

        while offset < source.len() {
            let remaining = &source[offset..];
            let leading_whitespace = remaining.len() - remaining.trim_start().len();
            offset += leading_whitespace;
            if offset == source.len() {
                break;
            }

            let remaining = &source[offset..];
            let Some(open_brace) = remaining.find('{') else {
                return Err(CssError::new(
                    &source,
                    offset,
                    "expected `{` after selector",
                ));
            };
            let Some(close_relative) = remaining[open_brace + 1..].find('}') else {
                return Err(CssError::new(
                    &source,
                    offset + open_brace,
                    "expected `}` to close rule",
                ));
            };

            let selector_offset = offset;
            let selector = remaining[..open_brace].trim();
            let body_offset = offset + open_brace + 1;
            let body = &remaining[open_brace + 1..open_brace + 1 + close_relative];
            if body.contains('{') {
                return Err(CssError::new(
                    &source,
                    body_offset,
                    "nested rules are unsupported",
                ));
            }

            let declarations = parse_declarations(&source, body, body_offset)?;
            for selector in selector.split(',') {
                let (class, state) = parse_selector(selector.trim()).ok_or_else(|| {
                    CssError::new(&source, selector_offset, "unsupported class selector")
                })?;
                rules.push(Rule {
                    class: class.to_owned(),
                    state,
                    declarations: declarations.clone(),
                });
            }

            offset += open_brace + 1 + close_relative + 1;
        }

        Ok(Self { rules })
    }

    fn apply_w3c_defaults(&self, context: &egui::Context) {
        context.all_styles_mut(apply_w3c_defaults);

        let background_registered = context
            .data(|data| data.get_temp::<bool>(Id::new(BACKGROUND_ID)))
            .unwrap_or(false);
        if !background_registered {
            context.on_begin_pass(
                BACKGROUND_ID,
                Arc::new(|ui| {
                    ui.painter().rect_filled(
                        ui.max_rect(),
                        CornerRadius::same(0),
                        ui.visuals().panel_fill,
                    );
                }),
            );
            context.data_mut(|data| {
                data.insert_temp(Id::new(BACKGROUND_ID), true);
            });
        }
    }

    /// Installs this stylesheet on an egui context for selector-wrapped widgets to use.
    pub fn install(&self, context: &egui::Context) {
        self.apply_w3c_defaults(context);
        let stylesheet = Arc::new(self.clone());
        context.data_mut(|data| {
            data.insert_temp(Id::new(STYLESHEET_ID), stylesheet);
        });
    }
}

/// Extension trait that adds `.selector(...)` to every egui widget.
pub trait SelectorExt: Widget + Sized {
    /// Wraps this widget and applies the installed stylesheet rule for `selector`.
    ///
    /// The selector should be a supported class selector, such as `.primary`.
    /// If no stylesheet is installed on the widget's context, the widget is
    /// passed through unchanged.
    fn selector(self, selector: impl Into<String>) -> StyledWidget<Self> {
        StyledWidget {
            widget: self,
            selector: selector.into(),
        }
    }
}

impl<W: Widget> SelectorExt for W {}

/// A widget decorated with a CSS selector.
#[must_use = "add the styled widget to an egui Ui with `ui.add(widget)`"]
pub struct StyledWidget<W> {
    widget: W,
    selector: String,
}

impl<W: Widget> Widget for StyledWidget<W> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { widget, selector } = self;
        let Some(class) = selector_class(&selector) else {
            return widget.ui(ui);
        };

        let stylesheet = ui
            .ctx()
            .data(|data| data.get_temp::<Arc<Stylesheet>>(Id::new(STYLESHEET_ID)));
        let Some(stylesheet) = stylesheet else {
            return widget.ui(ui);
        };

        let original_style = ui.style().as_ref().clone();
        for state in [RuleState::Base, RuleState::Hovered, RuleState::Active] {
            for rule in stylesheet
                .rules
                .iter()
                .filter(|rule| rule.class == class && rule.state == state)
            {
                apply_declarations(ui, state, &rule.declarations);
            }
        }

        // Delegating on the same Ui preserves egui's autogenerated widget IDs.
        let response = widget.ui(ui);
        *ui.style_mut() = original_style;
        response
    }
}

/// An error encountered while parsing a stylesheet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CssError {
    line: usize,
    message: &'static str,
}

impl CssError {
    fn new(source: &str, offset: usize, message: &'static str) -> Self {
        Self {
            line: source[..offset]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1,
            message,
        }
    }

    /// Returns the one-based source line containing the error.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns a short description of the parse error.
    pub fn message(&self) -> &'static str {
        self.message
    }
}

impl fmt::Display for CssError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CSS error on line {}: {}",
            self.line, self.message
        )
    }
}

impl Error for CssError {}

/// An error encountered while reading or parsing a CSS file.
#[derive(Debug)]
pub enum CssFileError {
    /// The file could not be read as UTF-8 text.
    Read(std::io::Error),
    /// The file contained unsupported or malformed CSS.
    Parse(CssError),
}

impl fmt::Display for CssFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "could not read CSS file: {error}"),
            Self::Parse(error) => error.fmt(formatter),
        }
    }
}

impl Error for CssFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Parse(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug)]
struct Rule {
    class: String,
    state: RuleState,
    declarations: Vec<Declaration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuleState {
    Base,
    Hovered,
    Active,
}

#[derive(Clone, Copy, Debug)]
enum Declaration {
    Color(Color32),
    Background(Color32),
    Padding(egui::Vec2),
    BorderRadius(u8),
}

fn remove_comments(source: &str) -> Result<String, CssError> {
    let mut bytes = source.as_bytes().to_vec();
    let mut offset = 0;
    while let Some(start_relative) = bytes[offset..].windows(2).position(|pair| pair == b"/*") {
        let start = offset + start_relative;
        let Some(end_relative) = bytes[start + 2..].windows(2).position(|pair| pair == b"*/")
        else {
            return Err(CssError::new(source, start, "unterminated comment"));
        };
        let end = start + 2 + end_relative + 2;
        for byte in &mut bytes[start..end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        offset = end;
    }
    Ok(String::from_utf8(bytes).expect("replacing comment bytes preserves UTF-8"))
}

fn parse_selector(selector: &str) -> Option<(&str, RuleState)> {
    let selector = selector.strip_prefix('.')?;
    let (class, state) = match selector.split_once(':') {
        Some((class, "hover")) => (class, RuleState::Hovered),
        Some((class, "active")) => (class, RuleState::Active),
        Some(_) => return None,
        None => (selector, RuleState::Base),
    };
    if class.is_empty()
        || !class
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return None;
    }
    Some((class, state))
}

fn selector_class(selector: &str) -> Option<&str> {
    let (class, state) = parse_selector(selector.trim())?;
    (state == RuleState::Base).then_some(class)
}

fn parse_declarations(
    source: &str,
    body: &str,
    body_offset: usize,
) -> Result<Vec<Declaration>, CssError> {
    let mut declarations = Vec::new();
    let mut offset = body_offset;

    for raw_declaration in body.split(';') {
        let declaration = raw_declaration.trim();
        if declaration.is_empty() {
            offset += raw_declaration.len() + 1;
            continue;
        }

        let declaration_offset =
            offset + raw_declaration.len() - raw_declaration.trim_start().len();
        let Some((property, value)) = declaration.split_once(':') else {
            return Err(CssError::new(
                source,
                declaration_offset,
                "expected `:` in declaration",
            ));
        };
        let property = property.trim();
        let value = value.trim();
        let parsed = match property {
            "color" => Some(Declaration::Color(parse_color(value).ok_or_else(|| {
                CssError::new(source, declaration_offset, "unsupported color value")
            })?)),
            "background-color" => Some(Declaration::Background(parse_color(value).ok_or_else(
                || CssError::new(source, declaration_offset, "unsupported color value"),
            )?)),
            "padding" => Some(Declaration::Padding(parse_padding(value).ok_or_else(
                || {
                    CssError::new(
                        source,
                        declaration_offset,
                        "padding must have one or two nonnegative lengths",
                    )
                },
            )?)),
            "border-radius" => Some(Declaration::BorderRadius(
                parse_length(value)
                    .ok_or_else(|| {
                        CssError::new(source, declaration_offset, "unsupported border radius")
                    })?
                    .round()
                    .min(f32::from(u8::MAX)) as u8,
            )),
            _ => None,
        };
        if let Some(parsed) = parsed {
            declarations.push(parsed);
        }
        offset += raw_declaration.len() + 1;
    }

    Ok(declarations)
}

fn parse_color(value: &str) -> Option<Color32> {
    if value.eq_ignore_ascii_case("transparent") {
        return Some(Color32::TRANSPARENT);
    }

    let hex = value.strip_prefix('#')?;
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let nibble = |index: usize| u8::from_str_radix(&hex[index..index + 1], 16).ok();
    let byte = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    match hex.len() {
        3 => Some(Color32::from_rgb(
            nibble(0)?.saturating_mul(17),
            nibble(1)?.saturating_mul(17),
            nibble(2)?.saturating_mul(17),
        )),
        4 => Some(Color32::from_rgba_unmultiplied(
            nibble(0)?.saturating_mul(17),
            nibble(1)?.saturating_mul(17),
            nibble(2)?.saturating_mul(17),
            nibble(3)?.saturating_mul(17),
        )),
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)?,
        )),
        _ => None,
    }
}

fn parse_length(value: &str) -> Option<f32> {
    let value = value.strip_suffix("px").unwrap_or(value).trim();
    let length = value.parse::<f32>().ok()?;
    (length.is_finite() && length >= 0.0).then_some(length)
}

fn parse_padding(value: &str) -> Option<egui::Vec2> {
    let lengths = value
        .split_whitespace()
        .map(parse_length)
        .collect::<Option<Vec<_>>>()?;
    match lengths.as_slice() {
        [all] => Some(egui::Vec2::splat(*all)),
        [vertical, horizontal] => Some(egui::vec2(*horizontal, *vertical)),
        _ => None,
    }
}

fn apply_w3c_defaults(style: &mut egui::Style) {
    *style = egui::Style::default();

    let black = Color32::BLACK;
    let white = Color32::WHITE;
    let button = Color32::from_gray(239);
    let button_hovered = Color32::from_gray(225);
    let button_active = Color32::from_gray(210);
    let border = Color32::from_gray(118);

    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8);
    style.spacing.menu_margin = egui::Margin::same(4);
    style.spacing.button_padding = egui::vec2(6.0, 2.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));

    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::light();
    visuals.override_text_color = Some(black);
    visuals.weak_text_color = Some(black);
    visuals.weak_text_alpha = 1.0;
    visuals.hyperlink_color = Color32::from_rgb(0, 0, 238);
    visuals.faint_bg_color = white;
    visuals.extreme_bg_color = white;
    visuals.text_edit_bg_color = Some(white);
    visuals.code_bg_color = white;
    visuals.window_fill = white;
    visuals.window_stroke = egui::Stroke::new(1.0, border);
    visuals.window_corner_radius = CornerRadius::same(0);
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.menu_corner_radius = CornerRadius::same(0);
    visuals.panel_fill = white;
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.window_highlight_topmost = false;

    set_widget_visuals(
        &mut visuals.widgets.noninteractive,
        white,
        Color32::TRANSPARENT,
        Color32::TRANSPARENT,
        black,
        0,
    );
    set_widget_visuals(
        &mut visuals.widgets.inactive,
        white,
        button,
        border,
        black,
        2,
    );
    set_widget_visuals(
        &mut visuals.widgets.hovered,
        white,
        button_hovered,
        Color32::from_gray(80),
        black,
        2,
    );
    set_widget_visuals(
        &mut visuals.widgets.active,
        white,
        button_active,
        Color32::from_gray(64),
        black,
        2,
    );
    set_widget_visuals(&mut visuals.widgets.open, white, button, border, black, 2);
}

fn set_widget_visuals(
    visuals: &mut WidgetVisuals,
    background: Color32,
    weak_background: Color32,
    border: Color32,
    text: Color32,
    radius: u8,
) {
    visuals.bg_fill = background;
    visuals.weak_bg_fill = weak_background;
    visuals.bg_stroke = egui::Stroke::new(
        if border == Color32::TRANSPARENT {
            0.0
        } else {
            1.0
        },
        border,
    );
    visuals.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.corner_radius = CornerRadius::same(radius);
    visuals.expansion = 0.0;
}

fn apply_declarations(ui: &mut Ui, state: RuleState, declarations: &[Declaration]) {
    for declaration in declarations {
        if let Declaration::Padding(padding) = declaration {
            if state == RuleState::Base {
                ui.spacing_mut().button_padding = *padding;
            }
            continue;
        }

        if matches!(declaration, Declaration::Color(_)) {
            ui.style_mut().visuals.override_text_color = None;
        }

        let visuals = &mut ui.style_mut().visuals.widgets;
        match state {
            RuleState::Base => {
                apply_visual_declaration(&mut visuals.noninteractive, declaration);
                apply_visual_declaration(&mut visuals.inactive, declaration);
                apply_visual_declaration(&mut visuals.hovered, declaration);
                apply_visual_declaration(&mut visuals.active, declaration);
                apply_visual_declaration(&mut visuals.open, declaration);
            }
            RuleState::Hovered => apply_visual_declaration(&mut visuals.hovered, declaration),
            RuleState::Active => apply_visual_declaration(&mut visuals.active, declaration),
        }
    }
}

fn apply_visual_declaration(visuals: &mut WidgetVisuals, declaration: &Declaration) {
    match declaration {
        Declaration::Color(color) => visuals.fg_stroke.color = *color,
        Declaration::Background(color) => {
            visuals.bg_fill = *color;
            visuals.weak_bg_fill = *color;
        }
        Declaration::BorderRadius(radius) => {
            visuals.corner_radius = CornerRadius::same(*radius);
        }
        Declaration::Padding(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_styles_apply_to_wrapped_widgets_without_changing_the_parent_ui() {
        let stylesheet = Stylesheet::from_file(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/theme.css"),
        )
        .unwrap();
        let context = egui::Context::default();
        let expected_text = Color32::WHITE;
        let expected_background = Color32::from_rgb(0x2d, 0x6c, 0xdf);
        let mut observed = false;

        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 200.0),
                )),
                ..Default::default()
            },
            |ui| {
                stylesheet.install(ui.ctx());
                let original_padding = ui.spacing().button_padding;
                let inspect_style = |ui: &mut Ui| {
                    let style = ui.style();
                    assert_eq!(
                        style.visuals.widgets.inactive.fg_stroke.color,
                        expected_text
                    );
                    assert_eq!(
                        style.visuals.widgets.inactive.weak_bg_fill,
                        expected_background
                    );
                    assert_eq!(
                        style.visuals.widgets.hovered.weak_bg_fill,
                        Color32::from_rgb(0x24, 0x58, 0xb5)
                    );
                    assert_eq!(style.spacing.button_padding, egui::vec2(12.0, 8.0));
                    assert_eq!(
                        style.visuals.widgets.inactive.corner_radius,
                        CornerRadius::same(6)
                    );
                    observed = true;
                    ui.label("styled")
                };
                ui.add(inspect_style.selector(".primary"));
                assert_eq!(ui.spacing().button_padding, original_padding);
            },
        );
        output.textures_delta.clear();

        assert!(observed);
    }

    #[test]
    fn malformed_supported_declarations_report_their_source_line() {
        let error = Stylesheet::parse(".primary {\n  color: not-a-color;\n}").unwrap_err();
        assert_eq!(error.line(), 2);
    }

    #[test]
    fn selector_wrapper_preserves_the_wrapped_widgets_id() {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 200.0),
            )),
            ..Default::default()
        };
        let direct_context = egui::Context::default();
        let mut direct_id = None;
        let mut direct_output = direct_context.run_ui(input.clone(), |ui| {
            direct_id = Some(ui.add(egui::Button::new("Save")).id);
        });
        direct_output.textures_delta.clear();

        let stylesheet = Stylesheet::parse(".primary {}").unwrap();
        let styled_context = egui::Context::default();
        let mut styled_id = None;
        let mut styled_output = styled_context.run_ui(input, |ui| {
            stylesheet.install(ui.ctx());
            styled_id = Some(ui.add(egui::Button::new("Save").selector(".primary")).id);
        });
        styled_output.textures_delta.clear();

        assert_eq!(styled_id, direct_id);
    }
}
