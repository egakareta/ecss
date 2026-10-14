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

use std::{
    error::Error,
    fmt,
    future::Future,
    path::Path,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use css::{Declaration, Rule, RuleState};
use egui::{Color32, CornerRadius, Id, Response, Ui, Widget, style::WidgetVisuals};
use fonts::{FontAssets, FontRequest};

mod css;
mod fonts;

const STYLESHEET_ID: &str = "trcss.stylesheet";
const BACKGROUND_ID: &str = "trcss.background";

#[cfg(not(target_arch = "wasm32"))]
fn spawn_background<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let _ = std::thread::Builder::new()
        .name("trcss-font-loader".to_owned())
        .spawn(move || pollster::block_on(future))
        .expect("failed to start font loader");
}

#[cfg(target_arch = "wasm32")]
fn spawn_background<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    ehttp::spawn_future(future);
}

/// A parsed stylesheet containing supported universal and class rules.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
    imports: Vec<String>,
    fonts: Arc<RwLock<FontAssets>>,
    font_revision: Arc<AtomicU64>,
}

impl Stylesheet {
    /// Reads and parses a UTF-8 CSS file, then loads imported fonts in the background.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, CssFileError> {
        let source = std::fs::read_to_string(path).map_err(CssFileError::Read)?;
        Self::parse(&source).map_err(CssFileError::Parse)
    }

    /// Parses CSS synchronously and loads fonts referenced by `@import` in the background.
    ///
    /// Imported stylesheets must contain supported `@font-face` rules. TTF, OTF,
    /// WOFF, and WOFF2 font files are supported. Network access is required for
    /// remote imports. Loaded fonts update contexts where this stylesheet is
    /// installed. Font loading errors are ignored; use [`Stylesheet::load`] to
    /// wait for fonts and receive loading errors.
    pub async fn load(source: &str) -> Result<Self, StylesheetLoadError> {
        let stylesheet = Self::parse_unloaded(source).map_err(StylesheetLoadError::Parse)?;
        let imports = stylesheet.imports.clone();
        let requests = stylesheet.font_requests();
        let fonts = FontAssets::load(&imports, &requests).await?;
        stylesheet.replace_fonts(fonts);
        Ok(stylesheet)
    }

    /// Reads a UTF-8 CSS file and asynchronously loads fonts referenced by `@import`.
    pub async fn load_file(path: impl AsRef<Path>) -> Result<Self, StylesheetLoadError> {
        let source = std::fs::read_to_string(path).map_err(StylesheetLoadError::Read)?;
        Self::load(&source).await
    }

    /// Parses universal and class rules from CSS text and starts background font loading.
    ///
    /// Supported selectors are `*`, `.class`, `.class:hover`, and `.class:active`.
    /// Supported declarations are `color`, `background-color`, `padding`,
    /// `border-radius`, `font-family`, `font-weight`, and `font-style`. Lengths
    /// are interpreted as egui points. Unknown declarations are ignored.
    pub fn parse(source: &str) -> Result<Self, CssError> {
        let stylesheet = Self::parse_unloaded(source)?;
        stylesheet.start_font_load();
        Ok(stylesheet)
    }

    fn parse_unloaded(source: &str) -> Result<Self, CssError> {
        let (rules, imports) = css::parse(source)?;
        Ok(Self {
            rules,
            imports,
            fonts: Arc::default(),
            font_revision: Arc::default(),
        })
    }

    fn start_font_load(&self) {
        let imports = self.imports.clone();
        let requests = self.font_requests();
        if imports.is_empty() || requests.is_empty() {
            return;
        }

        let fonts = self.fonts.clone();
        let font_revision = self.font_revision.clone();
        spawn_background(async move {
            if let Ok(loaded_fonts) = FontAssets::load(&imports, &requests).await
                && let Ok(mut fonts) = fonts.write()
            {
                *fonts = loaded_fonts;
                font_revision.fetch_add(1, Ordering::Release);
            }
        });
    }

    fn replace_fonts(&self, loaded_fonts: FontAssets) {
        if let Ok(mut fonts) = self.fonts.write() {
            *fonts = loaded_fonts;
            self.font_revision.fetch_add(1, Ordering::Release);
        }
    }

    fn font_requests(&self) -> Vec<FontRequest> {
        let mut classes = Vec::new();
        for rule in self
            .rules
            .iter()
            .filter(|rule| rule.state == RuleState::Base)
        {
            if let Some(class) = &rule.class
                && !classes.contains(class)
            {
                classes.push(class.clone());
            }
        }

        self.font_request_for_target(None)
            .into_iter()
            .chain(
                classes
                    .into_iter()
                    .filter_map(|class| self.font_request_for_class(&class)),
            )
            .collect()
    }

    fn font_request_for_class(&self, class: &str) -> Option<FontRequest> {
        self.font_request_for_target(Some(class))
    }

    fn font_request_for_target(&self, class: Option<&str>) -> Option<FontRequest> {
        let mut families = None;
        let mut weight = 400;
        let mut italic = false;
        let global_rules = self
            .rules
            .iter()
            .filter(|rule| rule.class.is_none() && rule.state == RuleState::Base);
        let class_rules = self.rules.iter().filter(|rule| {
            rule.class.is_some() && rule.class.as_deref() == class && rule.state == RuleState::Base
        });
        for declaration in global_rules
            .chain(class_rules)
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
        context.all_styles_mut(|style| {
            apply_w3c_defaults(style);
            for state in [RuleState::Base, RuleState::Hovered, RuleState::Active] {
                for rule in self
                    .rules
                    .iter()
                    .filter(|rule| rule.class.is_none() && rule.state == state)
                {
                    apply_style_declarations(style, state, &rule.declarations);
                }
            }
        });

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
        FontAssets::install_shared(
            self.fonts.clone(),
            self.font_revision.clone(),
            self.font_request_for_target(None),
            context,
        );
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
        let Some(class) = css::selector_class(&selector) else {
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
                .filter(|rule| rule.class.as_deref() == Some(class.as_str()) && rule.state == state)
            {
                apply_declarations(ui, state, &rule.declarations);
            }
        }

        if let Some(request) = stylesheet.font_request_for_class(&class)
            && let Some(family) = stylesheet.fonts.read().ok().and_then(|fonts| {
                fonts.family(ui.ctx(), &request.families, request.weight, request.italic)
            })
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
    pub(crate) fn at_line(line: u32, message: &'static str) -> Self {
        Self {
            line: line as usize + 1,
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
    apply_style_declarations(ui.style_mut(), state, declarations);
}

fn apply_style_declarations(
    style: &mut egui::Style,
    state: RuleState,
    declarations: &[Declaration],
) {
    for declaration in declarations {
        if let Declaration::Padding(padding) = declaration {
            if state == RuleState::Base {
                style.spacing.button_padding = *padding;
            }
            continue;
        }

        if matches!(declaration, Declaration::Color(_)) {
            style.visuals.override_text_color = None;
        }

        let visuals = &mut style.visuals.widgets;
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
        let stylesheet = Stylesheet::parse(
            ".outfit-text { font-family: \"Outfit\", sans-serif; font-weight: 500; }",
        )
        .unwrap();
        stylesheet.replace_fonts(FontAssets::with_test_font("Outfit", 500, font_bytes));

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
