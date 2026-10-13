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
use fonts::{FontAssets, FontRequest};

mod fonts;

const STYLESHEET_ID: &str = "trcss.stylesheet";
const BACKGROUND_ID: &str = "trcss.background";

/// A parsed stylesheet containing supported class rules.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
    imports: Vec<String>,
    fonts: FontAssets,
}

impl Stylesheet {
    /// Reads and parses a stylesheet from a UTF-8 CSS file without fetching imports.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CssFileError> {
        let source = std::fs::read_to_string(path).map_err(CssFileError::Read)?;
        Self::parse(&source).map_err(CssFileError::Parse)
    }

    /// Parses a stylesheet and asynchronously loads fonts referenced by `@import`.
    ///
    /// Imported stylesheets must contain supported `@font-face` rules. TTF, OTF,
    /// WOFF, and WOFF2 font files are supported. Network access is required for
    /// remote imports.
    pub async fn load(source: &str) -> Result<Self, StylesheetLoadError> {
        let mut stylesheet = Self::parse(source).map_err(StylesheetLoadError::Parse)?;
        let imports = stylesheet.imports.clone();
        let requests = stylesheet.font_requests();
        stylesheet.fonts = FontAssets::load(&imports, &requests).await?;
        Ok(stylesheet)
    }

    /// Reads a UTF-8 CSS file and asynchronously loads fonts referenced by `@import`.
    pub async fn load_file(path: impl AsRef<Path>) -> Result<Self, StylesheetLoadError> {
        let source = std::fs::read_to_string(path).map_err(StylesheetLoadError::Read)?;
        Self::load(&source).await
    }

    /// Parses class rules from CSS text.
    ///
    /// Supported selectors are `.class`, `.class:hover`, and `.class:active`.
    /// Supported declarations are `color`, `background-color`, `padding`,
    /// `border-radius`, `font-family`, `font-weight`, and `font-style`. Lengths
    /// are interpreted as egui points. Unknown declarations are ignored.
    pub fn parse(source: &str) -> Result<Self, CssError> {
        let source = remove_comments(source)?;
        let mut rules = Vec::new();
        let mut imports = Vec::new();
        let mut offset = 0;

        while offset < source.len() {
            let remaining = &source[offset..];
            let leading_whitespace = remaining.len() - remaining.trim_start().len();
            offset += leading_whitespace;
            if offset == source.len() {
                break;
            }

            let remaining = &source[offset..];
            if remaining.starts_with("@import") {
                let Some(end) = find_statement_end(remaining) else {
                    return Err(CssError::new(&source, offset, "expected `;` after import"));
                };
                let import = parse_import_url(&remaining[..end])
                    .ok_or_else(|| CssError::new(&source, offset, "unsupported import rule"))?;
                imports.push(import);
                offset += end + 1;
                continue;
            }

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

        Ok(Self {
            rules,
            imports,
            fonts: FontAssets::default(),
        })
    }

    fn font_requests(&self) -> Vec<FontRequest> {
        let mut classes = Vec::new();
        for rule in self
            .rules
            .iter()
            .filter(|rule| rule.state == RuleState::Base)
        {
            if !classes.contains(&rule.class) {
                classes.push(rule.class.clone());
            }
        }

        classes
            .into_iter()
            .filter_map(|class| self.font_request_for_class(&class))
            .collect()
    }

    fn font_request_for_class(&self, class: &str) -> Option<FontRequest> {
        let mut families = None;
        let mut weight = 400;
        let mut italic = false;
        for declaration in self
            .rules
            .iter()
            .filter(|rule| rule.class == class && rule.state == RuleState::Base)
            .flat_map(|rule| &rule.declarations)
        {
            match declaration {
                Declaration::FontFamily(value) => families = Some(value.clone()),
                Declaration::FontWeight(value) => weight = *value,
                Declaration::FontStyle(value) => italic = *value,
                _ => {}
            }
        }
        Some(FontRequest {
            families: families?,
            weight,
            italic,
        })
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
        self.fonts.install(context);
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

        if let Some(request) = stylesheet.font_request_for_class(class)
            && let Some(family) =
                stylesheet
                    .fonts
                    .family(ui.ctx(), &request.families, request.weight, request.italic)
        {
            for font_id in ui.style_mut().text_styles.values_mut() {
                font_id.family = family.clone();
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

/// An error encountered while parsing a stylesheet or loading its imported fonts.
#[derive(Debug)]
pub enum StylesheetLoadError {
    /// A CSS file could not be read as UTF-8 text.
    Read(std::io::Error),
    /// The stylesheet contained unsupported or malformed CSS.
    Parse(CssError),
    /// An imported font stylesheet or font file could not be fetched or decoded.
    Font {
        /// The URL that could not be loaded.
        url: String,
        /// A short description of the loading failure.
        message: String,
    },
}

impl fmt::Display for StylesheetLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "could not read CSS file: {error}"),
            Self::Parse(error) => error.fmt(formatter),
            Self::Font { url, message } => {
                write!(formatter, "could not load font from {url}: {message}")
            }
        }
    }
}

impl Error for StylesheetLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Parse(error) => Some(error),
            Self::Font { .. } => None,
        }
    }
}

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

#[derive(Clone, Debug)]
enum Declaration {
    Color(Color32),
    Background(Color32),
    Padding(egui::Vec2),
    BorderRadius(u8),
    FontFamily(Vec<String>),
    FontWeight(u16),
    FontStyle(bool),
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

fn parse_import_url(rule: &str) -> Option<String> {
    let value = rule.strip_prefix("@import")?.trim();
    let value = if let Some(value) = value.strip_prefix("url(") {
        let end = value.find(')')?;
        value[..end].trim()
    } else {
        value.split_whitespace().next()?
    };
    let value = parse_css_string(value)?;
    (!value.is_empty()).then_some(value)
}

fn find_statement_end(source: &str) -> Option<usize> {
    let mut quote = None;
    let mut escaped = false;
    let mut parentheses = 0_usize;
    for (offset, character) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active_quote) = quote {
            if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => parentheses += 1,
            ')' => parentheses = parentheses.saturating_sub(1),
            ';' if parentheses == 0 => return Some(offset),
            _ => {}
        }
    }
    None
}

fn parse_css_string(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.as_bytes().first().copied()?;
    if (quote == b'\'' || quote == b'"') && value.as_bytes().last() == Some(&quote) {
        Some(value[1..value.len() - 1].to_owned())
    } else if !value.is_empty() {
        Some(value.to_owned())
    } else {
        None
    }
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
            "font-family" => Some(Declaration::FontFamily(
                parse_font_family(value).ok_or_else(|| {
                    CssError::new(source, declaration_offset, "unsupported font family")
                })?,
            )),
            "font-weight" => Some(Declaration::FontWeight(
                parse_font_weight(value).ok_or_else(|| {
                    CssError::new(source, declaration_offset, "unsupported font weight")
                })?,
            )),
            "font-style" => Some(Declaration::FontStyle(parse_font_style(value).ok_or_else(
                || CssError::new(source, declaration_offset, "unsupported font style"),
            )?)),
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

fn parse_font_family(value: &str) -> Option<Vec<String>> {
    let families = value
        .split(',')
        .map(parse_css_string)
        .collect::<Option<Vec<_>>>()?;
    (!families.is_empty() && families.iter().all(|family| !family.trim().is_empty()))
        .then_some(families)
}

fn parse_font_weight(value: &str) -> Option<u16> {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" => Some(400),
        "bold" => Some(700),
        value => value
            .parse::<u16>()
            .ok()
            .filter(|weight| (1..=1000).contains(weight)),
    }
}

fn parse_font_style(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" => Some(false),
        "italic" | "oblique" => Some(true),
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
        Declaration::Padding(_)
        | Declaration::FontFamily(_)
        | Declaration::FontWeight(_)
        | Declaration::FontStyle(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_font_import_and_requested_font_weight_are_parsed() {
        let stylesheet = Stylesheet::parse(include_str!("../tests/fixtures/theme.css")).unwrap();

        assert_eq!(
            stylesheet.imports,
            [
                "https://fonts.googleapis.com/css2?family=Outfit:wght@100..900&family=SUSE+Mono:ital,wght@0,100..800;1,100..800&display=swap"
            ]
        );
        let request = stylesheet.font_request_for_class("outfit-text").unwrap();
        assert_eq!(request.families, ["Outfit", "sans-serif"]);
        assert_eq!(request.weight, 500);
        assert!(!request.italic);
    }

    #[test]
    fn selector_applies_loaded_font_and_restores_parent_style() {
        let context = egui::Context::default();
        let font_bytes = egui::FontDefinitions::default().font_data["Ubuntu-Light"]
            .as_ref()
            .as_ref()
            .to_vec();
        let mut stylesheet = Stylesheet::parse(
            ".outfit-text { font-family: \"Outfit\", sans-serif; font-weight: 500; }",
        )
        .unwrap();
        stylesheet.fonts = FontAssets::with_test_font("Outfit", 500, font_bytes);

        let mut setup_output = context.run_ui(egui::RawInput::default(), |_ui| {});
        setup_output.textures_delta.clear();
        stylesheet.install(&context);
        let mut font_output = context.run_ui(egui::RawInput::default(), |_ui| {});
        font_output.textures_delta.clear();

        let mut observed_font = None;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 200.0),
                )),
                ..Default::default()
            },
            |ui| {
                let parent_font = ui.style().text_styles[&egui::TextStyle::Body]
                    .family
                    .clone();
                let inspect_font = |ui: &mut Ui| {
                    observed_font = Some(
                        ui.style().text_styles[&egui::TextStyle::Body]
                            .family
                            .clone(),
                    );
                    ui.label("styled text")
                };
                ui.add(inspect_font.selector(".outfit-text"));
                assert_eq!(
                    ui.style().text_styles[&egui::TextStyle::Body].family,
                    parent_font
                );
            },
        );
        output.textures_delta.clear();

        assert_eq!(
            observed_font,
            Some(egui::FontFamily::Name("trcss:outfit:500:normal".into()))
        );
    }

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
