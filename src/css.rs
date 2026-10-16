use std::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
};

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, ParseErrorKind, Parser as CssParser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, SourceLocation,
    StyleSheetParser, ToCss, Token, serialize_identifier,
};
use egui::Color32;
use precomputed_hash::PrecomputedHash;
use selectors::parser::{self, Component, ParseRelative, SelectorList};

use crate::CssError;

#[derive(Clone, Debug)]
pub(crate) struct Rule {
    pub(crate) class: Option<String>,
    pub(crate) state: RuleState,
    pub(crate) declarations: Vec<Declaration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuleState {
    Base,
    Hovered,
    Active,
}

#[derive(Clone, Debug)]
pub(crate) enum Declaration {
    Color(Color32),
    Background(Color32),
    BackgroundShorthand(BackgroundShorthand),
    BackgroundImage(BackgroundImage),
    BackgroundAttachment(BackgroundAttachment),
    BackgroundClip(BackgroundClip),
    Layout(crate::layout_style::Declaration),
    BorderRadius(u8),
    FontFamily(Vec<String>),
    FontWeight(u16),
    FontStyle(bool),
}

#[derive(Clone, Debug)]
pub(crate) struct BackgroundShorthand {
    pub(crate) color: Color32,
    pub(crate) image: BackgroundImage,
    pub(crate) attachment: BackgroundAttachment,
    pub(crate) clip: BackgroundClip,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BackgroundImage {
    None,
    Url(String),
    LinearGradient(LinearGradient),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LinearGradient {
    pub(crate) angle: f32,
    pub(crate) stops: Vec<GradientStop>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GradientStop {
    pub(crate) color: Color32,
    pub(crate) position: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BackgroundAttachment {
    #[default]
    Scroll,
    Fixed,
    Local,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BackgroundClip {
    #[default]
    Border,
    Padding,
    Content,
}

enum ParsedRule {
    Import(String),
    Style(Vec<Rule>),
}

enum AtRulePrelude {
    Import(String),
}

#[derive(Clone, Copy, Debug)]
struct ParseIssue {
    message: &'static str,
    line: Option<u32>,
}

impl ParseIssue {
    fn new(message: &'static str) -> Self {
        Self {
            message,
            line: None,
        }
    }
}

pub(crate) fn parse(source: &str) -> Result<(Vec<Rule>, Vec<String>), CssError> {
    let mut parser = CssParser::new(source);
    let mut rule_parser = StylesheetRuleParser;
    let mut rules = Vec::new();
    let mut imports = Vec::new();

    for parsed in StyleSheetParser::new(&mut parser, &mut rule_parser) {
        match parsed {
            Ok(ParsedRule::Import(import)) => imports.push(import),
            Ok(ParsedRule::Style(mut parsed_rules)) => rules.append(&mut parsed_rules),
            Err((error, _, location)) => return Err(to_css_error(error, location)),
        }
    }

    Ok((rules, imports))
}

pub(crate) fn selector_class(selector: &str) -> Option<String> {
    let mut parser = CssParser::new(selector);
    parse_selectors(&mut parser)
        .ok()
        .filter(|selectors| selectors.len() == 1)
        .and_then(|mut selectors| {
            let (class, state) = selectors.pop()?;
            (state == RuleState::Base).then_some(class).flatten()
        })
}

struct StylesheetRuleParser;

impl<'i> QualifiedRuleParser<'i> for StylesheetRuleParser {
    type Prelude = Vec<(Option<String>, RuleState)>;
    type QualifiedRule = ParsedRule;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        parse_selectors(input)
    }

    fn parse_block(
        &mut self,
        selectors: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Self::QualifiedRule, ParseError<Self::Error>> {
        let declarations = parse_declarations(input).map_err(ParseError::custom)?;
        Ok(ParsedRule::Style(
            selectors
                .into_iter()
                .map(|(class, state)| Rule {
                    class,
                    state,
                    declarations: declarations.clone(),
                })
                .collect(),
        ))
    }
}

impl<'i> AtRuleParser<'i> for StylesheetRuleParser {
    type Prelude = AtRulePrelude;
    type AtRule = ParsedRule;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        if !name.eq_ignore_ascii_case("import") {
            return Err(ParseError::custom(ParseIssue::new("unsupported at-rule")));
        }
        parse_import(input).map(AtRulePrelude::Import)
    }

    fn rule_without_block(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
    ) -> Result<Self::AtRule, ()> {
        match prelude {
            AtRulePrelude::Import(import) => Ok(ParsedRule::Import(import)),
        }
    }
}

fn parse_import<'i>(input: &mut CssParser<'i>) -> Result<String, ParseError<ParseIssue>> {
    let token = input
        .next()
        .map_err(|_| ParseError::custom(ParseIssue::new("unsupported import rule")))?
        .clone();
    let url = match token {
        Token::QuotedString(value) | Token::UnquotedUrl(value) => value.to_string(),
        Token::Function(name) if name.eq_ignore_ascii_case("url") => {
            input.parse_nested_block(|url| {
                url.expect_string()
                    .map(|value| value.to_string())
                    .map_err(Into::into)
            })?
        }
        _ => {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported import rule",
            )));
        }
    };
    if url.trim().is_empty() {
        return Err(ParseError::custom(ParseIssue::new(
            "unsupported import rule",
        )));
    }
    Ok(url)
}

fn parse_selectors<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<(Option<String>, RuleState)>, ParseError<ParseIssue>> {
    let selector_parser = SimpleSelectorParser;
    let selectors = SelectorList::parse(&selector_parser, input, ParseRelative::No)
        .map_err(|_| ParseError::custom(ParseIssue::new("unsupported class selector")))?;

    let mut result = Vec::with_capacity(selectors.len());
    for selector in selectors.slice() {
        let mut class = None;
        let mut universal = false;
        let mut state = RuleState::Base;
        for component in selector.iter_raw_parse_order_from(0) {
            match component {
                Component::Class(identifier) if class.is_none() && !universal => {
                    class = Some(identifier.0.clone());
                }
                Component::ExplicitUniversalType if class.is_none() && !universal => {
                    universal = true;
                }
                Component::NonTSPseudoClass(StatePseudo::Hover) if state == RuleState::Base => {
                    state = RuleState::Hovered;
                }
                Component::NonTSPseudoClass(StatePseudo::Active) if state == RuleState::Base => {
                    state = RuleState::Active;
                }
                _ => {
                    return Err(ParseError::custom(ParseIssue::new(
                        "unsupported class selector",
                    )));
                }
            }
        }
        if universal && class.is_some() {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported class selector",
            )));
        }
        let selector = if universal {
            None
        } else if let Some(class) = class {
            Some(class)
        } else {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported class selector",
            )));
        };
        result.push((selector, state));
    }
    if result.is_empty() {
        return Err(ParseError::custom(ParseIssue::new(
            "unsupported class selector",
        )));
    }
    Ok(result)
}

struct Declarations;

impl<'i> DeclarationParser<'i> for Declarations {
    type Declaration = Option<Declaration>;
    type Error = &'static str;

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
        _declaration_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<Self::Error>> {
        if let Some(declaration) =
            crate::layout_style::parse(&name, input).map_err(ParseError::custom)?
        {
            return Ok(Some(Declaration::Layout(declaration)));
        }
        let declaration = if name.eq_ignore_ascii_case("color") {
            Some(Declaration::Color(parse_color(input)?))
        } else if name.eq_ignore_ascii_case("background-color") {
            Some(Declaration::Background(parse_color(input)?))
        } else if name.eq_ignore_ascii_case("background") {
            Some(Declaration::BackgroundShorthand(parse_background(input)?))
        } else if name.eq_ignore_ascii_case("background-image") {
            Some(Declaration::BackgroundImage(parse_background_image_value(
                input,
            )?))
        } else if name.eq_ignore_ascii_case("background-attachment") {
            Some(Declaration::BackgroundAttachment(
                parse_background_attachment(input)?,
            ))
        } else if name.eq_ignore_ascii_case("background-clip") {
            Some(Declaration::BackgroundClip(parse_background_clip(input)?))
        } else if name.eq_ignore_ascii_case("border-radius") {
            let radius = parse_length(input, "unsupported border radius")?;
            Some(Declaration::BorderRadius(
                radius.round().min(f32::from(u8::MAX)) as u8,
            ))
        } else if name.eq_ignore_ascii_case("font-family") {
            Some(Declaration::FontFamily(parse_font_family(input)?))
        } else if name.eq_ignore_ascii_case("font-weight") {
            Some(Declaration::FontWeight(parse_font_weight(input)?))
        } else if name.eq_ignore_ascii_case("font-style") {
            Some(Declaration::FontStyle(parse_font_style(input)?))
        } else {
            while input.next().is_ok() {}
            None
        };
        Ok(declaration)
    }
}

fn parse_background<'i>(
    input: &mut CssParser<'i>,
) -> Result<BackgroundShorthand, ParseError<&'static str>> {
    let value = consume_value(input)?;
    let components = crate::layout_style::components(&value).map_err(ParseError::custom)?;
    let mut background = BackgroundShorthand {
        color: Color32::TRANSPARENT,
        image: BackgroundImage::None,
        attachment: BackgroundAttachment::Scroll,
        clip: BackgroundClip::Border,
    };

    for component in components {
        if let Ok(color) = parse_color(&mut CssParser::new(&component)) {
            background.color = color;
        } else if let Ok(image) = parse_background_image(&component) {
            background.image = image;
        } else if let Ok(attachment) = parse_background_attachment_value(&component) {
            background.attachment = attachment;
        } else if let Ok(clip) = parse_background_clip_value(&component) {
            background.clip = clip;
        } else if !is_background_extra(&component) {
            return Err(ParseError::custom("unsupported background value"));
        }
    }
    Ok(background)
}

fn is_background_extra(value: &str) -> bool {
    if matches!(
        value.to_ascii_lowercase().as_str(),
        "/" | "repeat"
            | "no-repeat"
            | "repeat-x"
            | "repeat-y"
            | "space"
            | "round"
            | "left"
            | "right"
            | "top"
            | "bottom"
            | "center"
            | "cover"
            | "contain"
    ) {
        return true;
    }
    let mut input = CssParser::new(value);
    matches!(
        input.next(),
        Ok(Token::Number { .. } | Token::Percentage { .. } | Token::Dimension { .. })
    ) && input.expect_exhausted().is_ok()
}

fn parse_background_image_value<'i>(
    input: &mut CssParser<'i>,
) -> Result<BackgroundImage, ParseError<&'static str>> {
    parse_background_image(&consume_value(input)?)
}

fn parse_background_image(value: &str) -> Result<BackgroundImage, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let image = match input.next()? {
        Token::Ident(name) if name.eq_ignore_ascii_case("none") => BackgroundImage::None,
        Token::UnquotedUrl(url) => BackgroundImage::Url(url.to_string()),
        Token::Function(name) if name.eq_ignore_ascii_case("url") => {
            let url = input.parse_nested_block(|url| {
                let value = match url.next()? {
                    Token::QuotedString(value) | Token::UnquotedUrl(value) => value.to_string(),
                    _ => return Err(ParseError::custom("unsupported background image")),
                };
                url.expect_exhausted()?;
                Ok(value)
            })?;
            BackgroundImage::Url(url)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("linear-gradient") => {
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::LinearGradient(parse_linear_gradient(&args)?)
        }
        _ => return Err(ParseError::custom("unsupported background image")),
    };
    input.expect_exhausted()?;
    Ok(image)
}

fn parse_background_attachment<'i>(
    input: &mut CssParser<'i>,
) -> Result<BackgroundAttachment, ParseError<&'static str>> {
    parse_background_attachment_value(&consume_value(input)?)
}

fn parse_background_attachment_value(
    value: &str,
) -> Result<BackgroundAttachment, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let attachment = match input.expect_ident()?.to_ascii_lowercase().as_str() {
        "scroll" => BackgroundAttachment::Scroll,
        "fixed" => BackgroundAttachment::Fixed,
        "local" => BackgroundAttachment::Local,
        _ => return Err(ParseError::custom("unsupported background attachment")),
    };
    input.expect_exhausted()?;
    Ok(attachment)
}

fn parse_background_clip<'i>(
    input: &mut CssParser<'i>,
) -> Result<BackgroundClip, ParseError<&'static str>> {
    parse_background_clip_value(&consume_value(input)?)
}

fn parse_background_clip_value(value: &str) -> Result<BackgroundClip, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let clip = match input.expect_ident()?.to_ascii_lowercase().as_str() {
        "border-box" => BackgroundClip::Border,
        "padding-box" => BackgroundClip::Padding,
        "content-box" => BackgroundClip::Content,
        _ => return Err(ParseError::custom("unsupported background clip")),
    };
    input.expect_exhausted()?;
    Ok(clip)
}

fn consume_value<'i>(input: &mut CssParser<'i>) -> Result<String, ParseError<&'static str>> {
    let start = input.position();
    while input.next().is_ok() {}
    Ok(input.slice_from(start).trim().to_owned())
}

fn parse_linear_gradient(value: &str) -> Result<LinearGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid linear gradient"));
    }
    let mut angle = 180.0;
    if let Some(first) = parts.first()
        && let Some(parsed) = parse_gradient_angle(first)
    {
        angle = parsed;
        parts.remove(0);
    }
    let stops = parts
        .iter()
        .map(|part| parse_gradient_stop(part))
        .collect::<Result<Vec<_>, _>>()?;
    if stops.len() < 2 {
        return Err(ParseError::custom(
            "a linear gradient needs two color stops",
        ));
    }
    Ok(LinearGradient { angle, stops })
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_u32;
    let mut quote = None;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
        } else {
            match character {
                '\'' | '"' => quote = Some(character),
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    parts.push(value[start..index].trim());
                    start = index + character.len_utf8();
                }
                _ => {}
            }
        }
    }
    parts.push(value[start..].trim());
    parts
}

fn parse_gradient_angle(value: &str) -> Option<f32> {
    let lower = value.to_ascii_lowercase();
    if let Some(direction) = lower.strip_prefix("to ") {
        return Some(match direction.trim() {
            "top" => 0.0,
            "right" => 90.0,
            "bottom" => 180.0,
            "left" => 270.0,
            "top right" | "right top" => 45.0,
            "bottom right" | "right bottom" => 135.0,
            "bottom left" | "left bottom" => 225.0,
            "top left" | "left top" => 315.0,
            _ => return None,
        });
    }
    let mut input = CssParser::new(value);
    let angle = match input.next().ok()? {
        Token::Number { value, .. } if *value == 0.0 => 0.0,
        Token::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str() {
            "deg" => *value,
            "grad" => *value * 0.9,
            "rad" => value.to_degrees(),
            "turn" => *value * 360.0,
            _ => return None,
        },
        _ => return None,
    };
    input.expect_exhausted().ok()?;
    Some(angle.rem_euclid(360.0))
}

fn parse_gradient_stop(value: &str) -> Result<GradientStop, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let start = input.position();
    let token = input.next()?.clone();
    if matches!(token, Token::Function(_) | Token::ParenthesisBlock) {
        input.parse_nested_block(|args| {
            while args.next().is_ok() {}
            Ok::<_, ParseError<&'static str>>(())
        })?;
    }
    let color_text = input.slice_from(start).trim().to_owned();
    let color = parse_color(&mut CssParser::new(&color_text))?;
    input.skip_whitespace();
    let position = if input.is_exhausted() {
        None
    } else {
        let position = match input.next()? {
            Token::Percentage { unit_value, .. } => *unit_value,
            _ => return Err(ParseError::custom("unsupported gradient stop")),
        };
        input.expect_exhausted()?;
        Some(position)
    };
    Ok(GradientStop { color, position })
}

impl<'i> AtRuleParser<'i> for Declarations {
    type Prelude = ();
    type AtRule = Option<Declaration>;
    type Error = &'static str;
}

impl<'i> QualifiedRuleParser<'i> for Declarations {
    type Prelude = ();
    type QualifiedRule = Option<Declaration>;
    type Error = &'static str;
}

impl<'i> RuleBodyItemParser<'i, Option<Declaration>, &'static str> for Declarations {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        false
    }
}

fn parse_declarations<'i>(input: &mut CssParser<'i>) -> Result<Vec<Declaration>, ParseIssue> {
    let mut parser = Declarations;
    let mut declarations = Vec::new();
    for item in RuleBodyParser::new(input, &mut parser) {
        match item {
            Ok(Some(declaration)) => declarations.push(declaration),
            Ok(None) => {}
            Err((error, _, location)) => {
                let message = match error.kind {
                    ParseErrorKind::Custom(message) => message,
                    ParseErrorKind::Basic(_) => "invalid CSS syntax",
                };
                return Err(ParseIssue {
                    message,
                    line: Some(location.line),
                });
            }
        }
    }
    Ok(declarations)
}

pub(crate) fn parse_color<'i>(
    input: &mut CssParser<'i>,
) -> Result<Color32, ParseError<&'static str>> {
    let color = match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("transparent") => {
            Ok(Color32::TRANSPARENT)
        }
        Token::Hash(hex) | Token::IDHash(hex) => {
            parse_hex_color(hex).ok_or_else(|| ParseError::custom("unsupported color value"))
        }
        Token::Ident(value) => cssparser::color::parse_named_color(value)
            .map(|(r, g, b)| Color32::from_rgb(r, g, b))
            .map_err(|_| ParseError::custom("unsupported color value")),
        Token::Function(name)
            if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") =>
        {
            parse_functional_color(input, FunctionalColorKind::Rgb)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("hsl") || name.eq_ignore_ascii_case("hsla") =>
        {
            parse_functional_color(input, FunctionalColorKind::Hsl)
        }
        _ => Err(ParseError::custom("unsupported color value")),
    }?;
    input.expect_exhausted()?;
    Ok(color)
}

#[derive(Clone, Copy)]
enum FunctionalColorKind {
    Rgb,
    Hsl,
}

enum FunctionalColorToken {
    Number(f32),
    Percentage(f32),
    Dimension(f32, String),
    Comma,
    Slash,
}

fn parse_functional_color<'i>(
    input: &mut CssParser<'i>,
    kind: FunctionalColorKind,
) -> Result<Color32, ParseError<&'static str>> {
    let values = input.parse_nested_block(|args| {
        let mut values = Vec::new();
        while !args.is_exhausted() {
            let value = match args.next()? {
                Token::Number { value, .. } => FunctionalColorToken::Number(*value),
                Token::Percentage { unit_value, .. } => {
                    FunctionalColorToken::Percentage(*unit_value)
                }
                Token::Dimension { value, unit, .. } => {
                    FunctionalColorToken::Dimension(*value, unit.to_string())
                }
                Token::Comma => FunctionalColorToken::Comma,
                Token::Delim('/') => FunctionalColorToken::Slash,
                _ => return Err(ParseError::custom("unsupported color value")),
            };
            values.push(value);
        }
        Ok(values)
    })?;

    let (first, second, third, alpha) = functional_components(&values)
        .ok_or_else(|| ParseError::custom("unsupported color value"))?;
    let alpha = match alpha {
        Some(alpha) => {
            color_alpha(alpha).ok_or_else(|| ParseError::custom("unsupported color value"))?
        }
        None => 1.0,
    };

    let (red, green, blue) = match kind {
        FunctionalColorKind::Rgb => (
            rgb_component(first),
            rgb_component(second),
            rgb_component(third),
        ),
        FunctionalColorKind::Hsl => hsl_components(first, second, third),
    };
    let (Some(red), Some(green), Some(blue)) = (red, green, blue) else {
        return Err(ParseError::custom("unsupported color value"));
    };

    Ok(Color32::from_rgba_unmultiplied(
        color_byte(red),
        color_byte(green),
        color_byte(blue),
        color_byte(alpha),
    ))
}

fn functional_components(
    values: &[FunctionalColorToken],
) -> Option<(
    &FunctionalColorToken,
    &FunctionalColorToken,
    &FunctionalColorToken,
    Option<&FunctionalColorToken>,
)> {
    match values {
        [
            first,
            FunctionalColorToken::Comma,
            second,
            FunctionalColorToken::Comma,
            third,
        ] => Some((first, second, third, None)),
        [
            first,
            FunctionalColorToken::Comma,
            second,
            FunctionalColorToken::Comma,
            third,
            FunctionalColorToken::Comma,
            alpha,
        ] => Some((first, second, third, Some(alpha))),
        [first, second, third] => Some((first, second, third, None)),
        [first, second, third, FunctionalColorToken::Slash, alpha] => {
            Some((first, second, third, Some(alpha)))
        }
        _ => None,
    }
}

fn rgb_component(value: &FunctionalColorToken) -> Option<f32> {
    let channel = match value {
        FunctionalColorToken::Number(value) => value / 255.0,
        FunctionalColorToken::Percentage(value) => *value,
        _ => return None,
    };
    channel.is_finite().then(|| channel.clamp(0.0, 1.0))
}

fn color_alpha(value: &FunctionalColorToken) -> Option<f32> {
    let alpha = match value {
        FunctionalColorToken::Number(value) => *value,
        FunctionalColorToken::Percentage(value) => *value,
        _ => return None,
    };
    alpha.is_finite().then(|| alpha.clamp(0.0, 1.0))
}

fn hsl_components(
    hue: &FunctionalColorToken,
    saturation: &FunctionalColorToken,
    lightness: &FunctionalColorToken,
) -> (Option<f32>, Option<f32>, Option<f32>) {
    let hue = match hue {
        FunctionalColorToken::Number(value) => Some(*value),
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("deg") => {
            Some(*value)
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("grad") => {
            Some(*value * 0.9)
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("rad") => {
            Some(value.to_degrees())
        }
        FunctionalColorToken::Dimension(value, unit) if unit.eq_ignore_ascii_case("turn") => {
            Some(*value * 360.0)
        }
        _ => None,
    };
    let saturation = hsl_percentage(saturation);
    let lightness = hsl_percentage(lightness);
    let (Some(hue), Some(saturation), Some(lightness)) = (hue, saturation, lightness) else {
        return (None, None, None);
    };
    if !hue.is_finite() {
        return (None, None, None);
    }

    let hue = hue.rem_euclid(360.0) / 60.0;
    let saturation = saturation.clamp(0.0, 1.0);
    let lightness = lightness.clamp(0.0, 1.0);
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let secondary = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let offset = lightness - chroma / 2.0;
    (
        Some(red + offset),
        Some(green + offset),
        Some(blue + offset),
    )
}

fn hsl_percentage(value: &FunctionalColorToken) -> Option<f32> {
    match value {
        FunctionalColorToken::Percentage(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn color_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn parse_hex_color(hex: &str) -> Option<Color32> {
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

fn parse_length<'i>(
    input: &mut CssParser<'i>,
    message: &'static str,
) -> Result<f32, ParseError<&'static str>> {
    let length = match input.next()? {
        Token::Number { value, .. } => *value,
        Token::Dimension { value, unit, .. } if unit.eq_ignore_ascii_case("px") => *value,
        _ => return Err(ParseError::custom(message)),
    };
    if length.is_finite() && length >= 0.0 {
        Ok(length)
    } else {
        Err(ParseError::custom(message))
    }
}

fn parse_font_family<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<String>, ParseError<&'static str>> {
    let families = input.parse_comma_separated(|family| {
        let first = family.next()?.clone();
        match first {
            Token::QuotedString(name) if !name.trim().is_empty() => {
                family.expect_exhausted()?;
                Ok(name.to_string())
            }
            Token::Ident(name) => {
                let mut names = vec![name.to_string()];
                while !family.is_exhausted() {
                    match family.next()? {
                        Token::Ident(name) => names.push(name.to_string()),
                        _ => return Err(ParseError::custom("unsupported font family")),
                    }
                }
                Ok(names.join(" "))
            }
            _ => Err(ParseError::custom("unsupported font family")),
        }
    })?;
    if families.is_empty() {
        Err(ParseError::custom("unsupported font family"))
    } else {
        Ok(families)
    }
}

fn parse_font_weight<'i>(input: &mut CssParser<'i>) -> Result<u16, ParseError<&'static str>> {
    match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("normal") => Ok(400),
        Token::Ident(value) if value.eq_ignore_ascii_case("bold") => Ok(700),
        Token::Number {
            int_value: Some(weight),
            ..
        } if (1..=1000).contains(weight) => Ok(*weight as u16),
        _ => Err(ParseError::custom("unsupported font weight")),
    }
}

fn parse_font_style<'i>(input: &mut CssParser<'i>) -> Result<bool, ParseError<&'static str>> {
    match input.next()? {
        Token::Ident(value) if value.eq_ignore_ascii_case("normal") => Ok(false),
        Token::Ident(value)
            if value.eq_ignore_ascii_case("italic") || value.eq_ignore_ascii_case("oblique") =>
        {
            Ok(true)
        }
        _ => Err(ParseError::custom("unsupported font style")),
    }
}

fn to_css_error(error: ParseError<ParseIssue>, location: SourceLocation) -> CssError {
    let (line, message) = match error.kind {
        ParseErrorKind::Custom(issue) => (issue.line.unwrap_or(location.line), issue.message),
        ParseErrorKind::Basic(_) => (location.line, "invalid CSS syntax"),
    };
    CssError::at_line(line, message)
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
struct Atom(String);

impl ToCss for Atom {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        serialize_identifier(&self.0, dest)
    }
}

impl<'i> From<CowRcStr<'i>> for Atom {
    fn from(value: CowRcStr<'i>) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for Atom {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Atom {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl Borrow<str> for Atom {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PrecomputedHash for Atom {
    fn precomputed_hash(&self) -> u32 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish() as u32
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum StatePseudo {
    Hover,
    Active,
}

impl parser::NonTSPseudoClass for StatePseudo {
    fn is_active_or_hover(&self) -> bool {
        true
    }

    fn is_user_action_state(&self) -> bool {
        true
    }
}

impl ToCss for StatePseudo {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(match self {
            Self::Hover => ":hover",
            Self::Active => ":active",
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum NoPseudoElement {}

impl parser::PseudoElement for NoPseudoElement {}

impl ToCss for NoPseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match *self {}
    }
}

#[derive(Clone, Debug)]
struct SimpleSelectorImpl;

impl parser::SelectorImpl for SimpleSelectorImpl {
    type ExtraMatchingData<'a> = ();
    type AttrValue = Atom;
    type Identifier = Atom;
    type LocalName = Atom;
    type NamespaceUrl = Atom;
    type NamespacePrefix = Atom;
    type BorrowedLocalName = str;
    type BorrowedNamespaceUrl = str;
    type NonTSPseudoClass = StatePseudo;
    type PseudoElement = NoPseudoElement;
}

struct SimpleSelectorParser;

impl<'i> parser::Parser<'i> for SimpleSelectorParser {
    type Impl = SimpleSelectorImpl;
    type Error = parser::SelectorParseErrorKind;

    fn parse_non_ts_pseudo_class(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<StatePseudo, ParseError<Self::Error>> {
        if name.eq_ignore_ascii_case("hover") {
            Ok(StatePseudo::Hover)
        } else if name.eq_ignore_ascii_case("active") {
            Ok(StatePseudo::Active)
        } else {
            Err(ParseError::custom(
                parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }
}
