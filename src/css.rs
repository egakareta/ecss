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
    pub(crate) pseudo: Option<PseudoElementKind>,
    pub(crate) declarations: Vec<Declaration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuleState {
    Base,
    Hovered,
    Active,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PseudoElementKind {
    FirstLetter,
    FirstLine,
}

#[derive(Clone, Debug)]
pub(crate) enum Declaration {
    Color(Color32),
    Background(Color32),
    BackgroundShorthand(BackgroundShorthand),
    BackgroundImage(Vec<BackgroundImage>),
    BackgroundAttachment(Vec<BackgroundAttachment>),
    BackgroundClip(Vec<BackgroundClip>),
    Layout(crate::layout_style::Declaration),
    BorderRadius(u8),
    FontFamily(Vec<String>),
    FontWeight(u16),
    FontStyle(bool),
}

#[derive(Clone, Debug)]
pub(crate) struct BackgroundShorthand {
    pub(crate) color: Color32,
    pub(crate) images: Vec<BackgroundImage>,
    pub(crate) attachments: Vec<BackgroundAttachment>,
    pub(crate) clips: Vec<BackgroundClip>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BackgroundImage {
    None,
    Url(String),
    Element(String),
    SolidColor(Color32),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    ConicGradient(ConicGradient),
    CrossFade(CrossFade),
    ImageSet(Vec<ImageSetCandidate>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CrossFade {
    pub(crate) items: Vec<CrossFadeItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CrossFadeItem {
    pub(crate) image: Box<BackgroundImage>,
    pub(crate) weight: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImageSetCandidate {
    image: Box<BackgroundImage>,
    resolution: f32,
    type_hint: Option<String>,
}

impl BackgroundImage {
    fn contains_image_set(&self) -> bool {
        match self {
            Self::ImageSet(_) => true,
            Self::CrossFade(cross_fade) => cross_fade
                .items
                .iter()
                .any(|item| item.image.contains_image_set()),
            Self::None
            | Self::Url(_)
            | Self::Element(_)
            | Self::SolidColor(_)
            | Self::LinearGradient(_)
            | Self::RadialGradient(_)
            | Self::ConicGradient(_) => false,
        }
    }

    pub(crate) fn for_scale(&self, scale: f32) -> &Self {
        self.image_set_candidate(scale)
            .map_or(self, |candidate| candidate.image.for_scale(scale))
    }

    pub(crate) fn resolution_for_scale(&self, scale: f32) -> f32 {
        self.image_set_candidate(scale).map_or(1.0, |candidate| {
            if matches!(candidate.image.as_ref(), Self::Url(_)) {
                candidate.resolution
            } else {
                candidate.image.resolution_for_scale(scale)
            }
        })
    }

    fn image_set_candidate(&self, scale: f32) -> Option<&ImageSetCandidate> {
        let Self::ImageSet(candidates) = self else {
            return None;
        };
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        candidates
            .iter()
            .filter(|candidate| {
                candidate
                    .type_hint
                    .as_deref()
                    .is_none_or(supported_image_type)
            })
            .filter(|candidate| candidate.resolution >= scale)
            .min_by(|first, second| first.resolution.total_cmp(&second.resolution))
            .or_else(|| {
                candidates
                    .iter()
                    .filter(|candidate| {
                        candidate
                            .type_hint
                            .as_deref()
                            .is_none_or(supported_image_type)
                    })
                    .max_by(|first, second| first.resolution.total_cmp(&second.resolution))
            })
    }

    pub(crate) fn collect_urls(&self, urls: &mut Vec<String>) {
        match self {
            Self::Url(url) => urls.push(url.clone()),
            Self::CrossFade(cross_fade) => {
                for item in &cross_fade.items {
                    item.image.collect_urls(urls);
                }
            }
            Self::ImageSet(candidates) => {
                for candidate in candidates {
                    if candidate
                        .type_hint
                        .as_deref()
                        .is_none_or(supported_image_type)
                    {
                        candidate.image.collect_urls(urls);
                    }
                }
            }
            Self::Element(_) => {}
            Self::None
            | Self::SolidColor(_)
            | Self::LinearGradient(_)
            | Self::RadialGradient(_)
            | Self::ConicGradient(_) => {}
        }
    }

    pub(crate) fn collect_element_ids(&self, ids: &mut Vec<String>) {
        match self {
            Self::Element(id) => ids.push(id.clone()),
            Self::CrossFade(cross_fade) => {
                for item in &cross_fade.items {
                    item.image.collect_element_ids(ids);
                }
            }
            Self::ImageSet(candidates) => {
                for candidate in candidates {
                    if candidate
                        .type_hint
                        .as_deref()
                        .is_none_or(supported_image_type)
                    {
                        candidate.image.collect_element_ids(ids);
                    }
                }
            }
            Self::None
            | Self::Url(_)
            | Self::SolidColor(_)
            | Self::LinearGradient(_)
            | Self::RadialGradient(_)
            | Self::ConicGradient(_) => {}
        }
    }
}

fn supported_image_type(mime_type: &str) -> bool {
    mime_type.eq_ignore_ascii_case("image/svg+xml")
        || image::ImageFormat::from_mime_type(mime_type)
            .is_some_and(|format| format.reading_enabled())
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LinearGradient {
    pub(crate) angle: f32,
    pub(crate) stops: Vec<GradientStop>,
    pub(crate) repeating: bool,
    pub(crate) color_space: GradientColorSpace,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConicGradient {
    pub(crate) from_angle: f32,
    pub(crate) center: [GradientPosition; 2],
    pub(crate) stops: Vec<GradientStop>,
    pub(crate) repeating: bool,
    pub(crate) color_space: GradientColorSpace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RadialShape {
    Circle,
    Ellipse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RadialExtent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RadialRadius {
    Length(f32),
    Percentage(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RadialSize {
    Extent(RadialExtent),
    Explicit([RadialRadius; 2]),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RadialGradient {
    pub(crate) shape: RadialShape,
    pub(crate) size: RadialSize,
    pub(crate) center: [GradientPosition; 2],
    pub(crate) stops: Vec<GradientStop>,
    pub(crate) repeating: bool,
    pub(crate) color_space: GradientColorSpace,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GradientColorSpace {
    Srgb,
    SrgbLinear,
    #[default]
    Oklab,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GradientPosition {
    percentage: f32,
    length: f32,
    from_end: bool,
}

impl GradientPosition {
    fn center() -> Self {
        Self {
            percentage: 0.5,
            length: 0.0,
            from_end: false,
        }
    }

    pub(crate) fn resolve(self, extent: f32) -> f32 {
        let offset = self.percentage * extent + self.length;
        if self.from_end {
            extent - offset
        } else {
            offset
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GradientStop {
    pub(crate) color: Color32,
    pub(crate) position: Option<f32>,
    pub(crate) hint_after: Option<f32>,
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
            let (class, state, pseudo) = selectors.pop()?;
            (state == RuleState::Base && pseudo.is_none())
                .then_some(class)
                .flatten()
        })
}

struct StylesheetRuleParser;

impl<'i> QualifiedRuleParser<'i> for StylesheetRuleParser {
    type Prelude = Vec<(Option<String>, RuleState, Option<PseudoElementKind>)>;
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
                .map(|(class, state, pseudo)| Rule {
                    class,
                    state,
                    pseudo,
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
) -> Result<Vec<(Option<String>, RuleState, Option<PseudoElementKind>)>, ParseError<ParseIssue>> {
    let selector_parser = SimpleSelectorParser;
    let selectors = SelectorList::parse(&selector_parser, input, ParseRelative::No)
        .map_err(|_| ParseError::custom(ParseIssue::new("unsupported class selector")))?;

    let mut result = Vec::with_capacity(selectors.len());
    for selector in selectors.slice() {
        let mut class = None;
        let mut universal = false;
        let mut state = RuleState::Base;
        let mut pseudo = None;
        for component in selector.iter_raw_parse_order_from(0) {
            match component {
                Component::Class(identifier) if class.is_none() && !universal => {
                    class = Some(identifier.0.clone());
                }
                Component::ID(identifier) if class.is_none() && !universal => {
                    class = Some(format!("@id:{}", identifier.0));
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
                Component::NonTSPseudoClass(StatePseudo::FirstLetter) if pseudo.is_none() => {
                    pseudo = Some(PseudoElementKind::FirstLetter);
                }
                Component::NonTSPseudoClass(StatePseudo::FirstLine) if pseudo.is_none() => {
                    pseudo = Some(PseudoElementKind::FirstLine);
                }
                Component::PseudoElement(value) if pseudo.is_none() => pseudo = Some(*value),
                Component::Combinator(combinator) if combinator.is_pseudo_element() => {}
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
        } else if pseudo.is_some() {
            None
        } else {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported class selector",
            )));
        };
        result.push((selector, state, pseudo));
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
            Some(Declaration::BackgroundImage(parse_background_images_value(
                input,
            )?))
        } else if name.eq_ignore_ascii_case("background-attachment") {
            Some(Declaration::BackgroundAttachment(
                parse_background_attachments(input)?,
            ))
        } else if name.eq_ignore_ascii_case("background-clip") {
            Some(Declaration::BackgroundClip(parse_background_clips(input)?))
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
    let layers = split_top_level_commas(&value);
    if layers.is_empty() {
        return Err(ParseError::custom("empty background shorthand"));
    }
    let mut background = BackgroundShorthand {
        color: Color32::TRANSPARENT,
        images: Vec::with_capacity(layers.len()),
        attachments: Vec::with_capacity(layers.len()),
        clips: Vec::with_capacity(layers.len()),
    };

    for (layer_index, layer) in layers.iter().enumerate() {
        let components = crate::layout_style::components(layer).map_err(ParseError::custom)?;
        if components.is_empty() {
            return Err(ParseError::custom("empty background layer"));
        }
        let mut image = None;
        let mut attachment = None;
        let mut clip = None;
        for component in components {
            if let Ok(color) = parse_color(&mut CssParser::new(&component)) {
                if layer_index + 1 != layers.len() {
                    return Err(ParseError::custom(
                        "background color must appear in the final layer",
                    ));
                }
                background.color = color;
            } else if let Ok(parsed) = parse_background_image(&component) {
                if image.replace(parsed).is_some() {
                    return Err(ParseError::custom("duplicate image in background layer"));
                }
            } else if let Ok(parsed) = parse_background_attachment_value(&component) {
                if attachment.replace(parsed).is_some() {
                    return Err(ParseError::custom(
                        "duplicate attachment in background layer",
                    ));
                }
            } else if let Ok(parsed) = parse_background_clip_value(&component) {
                if clip.replace(parsed).is_some() {
                    return Err(ParseError::custom("duplicate clip in background layer"));
                }
            } else if !is_background_extra(&component) {
                return Err(ParseError::custom("unsupported background value"));
            }
        }
        background
            .images
            .push(image.unwrap_or(BackgroundImage::None));
        background
            .attachments
            .push(attachment.unwrap_or(BackgroundAttachment::Scroll));
        background
            .clips
            .push(clip.unwrap_or(BackgroundClip::Border));
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

fn parse_background_images_value<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<BackgroundImage>, ParseError<&'static str>> {
    split_top_level_commas(&consume_value(input)?)
        .into_iter()
        .map(parse_background_image)
        .collect()
}

pub(crate) fn parse_background_image(
    value: &str,
) -> Result<BackgroundImage, ParseError<&'static str>> {
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
        Token::Function(name) if name.eq_ignore_ascii_case("element") => {
            let id = input.parse_nested_block(|args| {
                let id = match args.next()? {
                    Token::IDHash(id) => id.to_string(),
                    _ => return Err(ParseError::custom("element() requires an ID selector")),
                };
                args.expect_exhausted()?;
                Ok(id)
            })?;
            BackgroundImage::Element(id)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("linear-gradient")
                || name.eq_ignore_ascii_case("repeating-linear-gradient") =>
        {
            let repeating = name.eq_ignore_ascii_case("repeating-linear-gradient");
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::LinearGradient(parse_linear_gradient(&args, repeating)?)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("conic-gradient")
                || name.eq_ignore_ascii_case("repeating-conic-gradient") =>
        {
            let repeating = name.eq_ignore_ascii_case("repeating-conic-gradient");
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::ConicGradient(parse_conic_gradient(&args, repeating)?)
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("radial-gradient")
                || name.eq_ignore_ascii_case("repeating-radial-gradient") =>
        {
            let repeating = name.eq_ignore_ascii_case("repeating-radial-gradient");
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::RadialGradient(parse_radial_gradient(&args, repeating)?)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("cross-fade") => {
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            parse_cross_fade(&args)?
        }
        Token::Function(name) if name.eq_ignore_ascii_case("image") => {
            let color = input.parse_nested_block(parse_color)?;
            BackgroundImage::SolidColor(color)
        }
        Token::Function(name) if name.eq_ignore_ascii_case("image-set") => {
            let args = input.parse_nested_block(|args| {
                let start = args.position();
                while args.next().is_ok() {}
                Ok::<_, ParseError<&'static str>>(args.slice_from(start).trim().to_owned())
            })?;
            BackgroundImage::ImageSet(parse_image_set(&args)?)
        }
        _ => return Err(ParseError::custom("unsupported background image")),
    };
    input.expect_exhausted()?;
    Ok(image)
}

fn parse_cross_fade(value: &str) -> Result<BackgroundImage, ParseError<&'static str>> {
    let arguments = split_top_level_commas(value);
    if arguments.is_empty() {
        return Err(ParseError::custom(
            "cross-fade() requires at least one argument",
        ));
    }
    let mut parsed = Vec::new();
    let mut specified = 0.0;
    let mut omitted = 0;
    for argument in arguments {
        let components = crate::layout_style::components(argument).map_err(ParseError::custom)?;
        if components.is_empty() {
            return Err(ParseError::custom("empty cross-fade() argument"));
        }
        let mut weight = None;
        let mut image_components = Vec::with_capacity(components.len());
        for component in components {
            if let Ok(parsed_weight) = parse_percentage(&component) {
                if weight.replace(parsed_weight).is_some() {
                    return Err(ParseError::custom("duplicate cross-fade percentage"));
                }
                if !(0.0..=1.0).contains(&parsed_weight) {
                    return Err(ParseError::custom(
                        "cross-fade percentages must be between 0% and 100%",
                    ));
                }
            } else {
                image_components.push(component);
            }
        }
        if let Some(weight) = weight {
            specified += weight;
        } else {
            omitted += 1;
        }
        if image_components.is_empty() {
            return Err(ParseError::custom(
                "cross-fade() argument is missing an image or color",
            ));
        }
        let source = image_components.join(" ");
        let image = parse_background_image(&source).or_else(|_| {
            parse_color(&mut CssParser::new(&source)).map(BackgroundImage::SolidColor)
        })?;
        if image == BackgroundImage::None {
            return Err(ParseError::custom("cross-fade() arguments cannot be none"));
        }
        if image.contains_image_set() {
            return Err(ParseError::custom(
                "image-set() cannot be nested inside another image function",
            ));
        }
        parsed.push((image, weight));
    }
    let omitted_weight = if omitted == 0 {
        0.0
    } else {
        (1.0 - specified).max(0.0) / omitted as f32
    };
    Ok(BackgroundImage::CrossFade(CrossFade {
        items: parsed
            .into_iter()
            .map(|(image, weight)| CrossFadeItem {
                image: Box::new(image),
                weight: weight.unwrap_or(omitted_weight),
            })
            .collect(),
    }))
}

fn parse_percentage(value: &str) -> Result<f32, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let percentage = match input.next()? {
        Token::Percentage { unit_value, .. } => *unit_value,
        _ => return Err(ParseError::custom("expected a percentage")),
    };
    input.expect_exhausted()?;
    Ok(percentage)
}

fn parse_image_set(value: &str) -> Result<Vec<ImageSetCandidate>, ParseError<&'static str>> {
    let mut candidates = Vec::new();
    for candidate in split_top_level_commas(value) {
        let parts = crate::layout_style::components(candidate).map_err(ParseError::custom)?;
        let Some(source) = parts.first() else {
            return Err(ParseError::custom("empty image-set candidate"));
        };
        let image = parse_image_set_source(source)?;
        if image == BackgroundImage::None {
            return Err(ParseError::custom("image-set candidates cannot be none"));
        }
        if image.contains_image_set() {
            return Err(ParseError::custom(
                "image-set() cannot be nested inside another image function",
            ));
        }
        let mut resolution = None;
        let mut type_hint = None;
        for descriptor in &parts[1..] {
            if descriptor
                .split_once('(')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("type"))
            {
                if type_hint.is_some() {
                    return Err(ParseError::custom("duplicate image-set type hint"));
                }
                type_hint = Some(parse_image_type_hint(descriptor)?);
            } else if resolution
                .replace(parse_image_resolution(descriptor)?)
                .is_some()
            {
                return Err(ParseError::custom("duplicate image-set resolution"));
            }
        }
        candidates.push(ImageSetCandidate {
            image: Box::new(image),
            resolution: resolution.unwrap_or(1.0),
            type_hint,
        });
    }
    if candidates.is_empty() {
        return Err(ParseError::custom(
            "image-set() requires at least one image",
        ));
    }
    Ok(candidates)
}

fn parse_image_set_source(value: &str) -> Result<BackgroundImage, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    if let Token::QuotedString(url) = input.next()?.clone() {
        input.expect_exhausted()?;
        return Ok(BackgroundImage::Url(url.to_string()));
    }
    parse_background_image(value)
}

fn parse_image_resolution(value: &str) -> Result<f32, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let resolution = match input.next()? {
        Token::Dimension { value, unit, .. } => match unit.to_ascii_lowercase().as_str() {
            "x" | "dppx" => *value,
            "dpi" => *value / 96.0,
            "dpcm" => *value * 2.54 / 96.0,
            _ => return Err(ParseError::custom("invalid image-set resolution unit")),
        },
        _ => return Err(ParseError::custom("invalid image-set resolution")),
    };
    input.expect_exhausted()?;
    if !resolution.is_finite() || resolution <= 0.0 {
        return Err(ParseError::custom("image-set resolution must be positive"));
    }
    Ok(resolution)
}

fn parse_image_type_hint(value: &str) -> Result<String, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let mime = match input.next()? {
        Token::Function(name) if name.eq_ignore_ascii_case("type") => {
            input.parse_nested_block(|args| {
                let mime = args.expect_string()?.to_string();
                if mime.is_empty() {
                    return Err(ParseError::custom("empty image-set type hint"));
                }
                args.expect_exhausted()?;
                Ok::<_, ParseError<&'static str>>(mime)
            })?
        }
        _ => return Err(ParseError::custom("invalid image-set type hint")),
    };
    input.expect_exhausted()?;
    Ok(mime.to_ascii_lowercase())
}

fn parse_background_attachments<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<BackgroundAttachment>, ParseError<&'static str>> {
    split_top_level_commas(&consume_value(input)?)
        .into_iter()
        .map(parse_background_attachment_value)
        .collect()
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

fn parse_background_clips<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<BackgroundClip>, ParseError<&'static str>> {
    split_top_level_commas(&consume_value(input)?)
        .into_iter()
        .map(parse_background_clip_value)
        .collect()
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

fn parse_linear_gradient(
    value: &str,
    repeating: bool,
) -> Result<LinearGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid linear gradient"));
    }
    let mut prelude = crate::layout_style::components(parts[0]).map_err(ParseError::custom)?;
    let (color_space, has_color_space) = parse_gradient_color_space(&mut prelude)?;
    let mut angle = 180.0;
    if !prelude.is_empty()
        && let Some(parsed) = parse_gradient_angle(&prelude.join(" "))
    {
        angle = parsed;
        parts.remove(0);
    } else if prelude.is_empty() && has_color_space {
        parts.remove(0);
    } else if prelude
        .first()
        .is_some_and(|value| parse_gradient_angle(value).is_some())
    {
        angle = parse_gradient_angle(&prelude[0]).unwrap_or(angle);
        if prelude.len() != 1 {
            return Err(ParseError::custom("invalid linear gradient prelude"));
        }
        parts.remove(0);
    }
    let stops = parse_gradient_stops(parts.iter().copied(), false)?;
    if stops.len() < 2 {
        return Err(ParseError::custom(
            "a linear gradient needs two color stops",
        ));
    }
    Ok(LinearGradient {
        angle,
        stops,
        repeating,
        color_space,
    })
}

fn parse_conic_gradient(
    value: &str,
    repeating: bool,
) -> Result<ConicGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid conic gradient"));
    }
    let mut from_angle = 0.0;
    let mut center = [GradientPosition::center(); 2];
    let mut prelude = crate::layout_style::components(parts[0]).map_err(ParseError::custom)?;
    let (color_space, has_color_space) = parse_gradient_color_space(&mut prelude)?;
    if has_color_space
        || prelude.first().is_some_and(|value| {
            value.eq_ignore_ascii_case("from") || value.eq_ignore_ascii_case("at")
        })
    {
        let mut index = 0;
        let mut from_seen = false;
        let mut at_seen = false;
        while index < prelude.len() {
            if prelude[index].eq_ignore_ascii_case("from") && !from_seen {
                let value = prelude
                    .get(index + 1)
                    .ok_or_else(|| ParseError::custom("missing conic gradient angle"))?;
                from_angle = parse_gradient_angle(value)
                    .ok_or_else(|| ParseError::custom("invalid conic gradient angle"))?;
                from_seen = true;
                index += 2;
            } else if prelude[index].eq_ignore_ascii_case("at") && !at_seen {
                center = parse_gradient_position(&prelude[index + 1..])?;
                at_seen = true;
                index = prelude.len();
            } else {
                return Err(ParseError::custom("invalid conic gradient prelude"));
            }
        }
        parts.remove(0);
    }
    let stops = parse_gradient_stops(parts.iter().copied(), true)?;
    if stops.len() < 2 {
        return Err(ParseError::custom("a conic gradient needs two color stops"));
    }
    Ok(ConicGradient {
        from_angle,
        center,
        stops,
        repeating,
        color_space,
    })
}

fn parse_radial_gradient(
    value: &str,
    repeating: bool,
) -> Result<RadialGradient, ParseError<&'static str>> {
    let mut parts = split_top_level_commas(value);
    if parts.len() < 2 {
        return Err(ParseError::custom("invalid radial gradient"));
    }
    let mut shape = RadialShape::Ellipse;
    let mut size = RadialSize::Extent(RadialExtent::FarthestCorner);
    let mut center = [GradientPosition::center(); 2];
    let mut prelude = crate::layout_style::components(parts[0]).map_err(ParseError::custom)?;
    let (color_space, has_color_space) = parse_gradient_color_space(&mut prelude)?;
    let mut index = 0;
    let mut has_prelude = has_color_space;
    if has_color_space
        || prelude.first().is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "circle"
                    | "ellipse"
                    | "closest-side"
                    | "farthest-side"
                    | "closest-corner"
                    | "farthest-corner"
                    | "at"
            )
        })
        || prelude
            .first()
            .is_some_and(|value| parse_radial_radius(value).is_ok())
    {
        has_prelude = true;
        if prelude[index].eq_ignore_ascii_case("circle") {
            shape = RadialShape::Circle;
            index += 1;
        } else if prelude[index].eq_ignore_ascii_case("ellipse") {
            shape = RadialShape::Ellipse;
            index += 1;
        }

        let mut explicit_radii = Vec::new();
        while index < prelude.len() && !prelude[index].eq_ignore_ascii_case("at") {
            let extent = match prelude[index].to_ascii_lowercase().as_str() {
                "closest-side" => Some(RadialExtent::ClosestSide),
                "farthest-side" => Some(RadialExtent::FarthestSide),
                "closest-corner" => Some(RadialExtent::ClosestCorner),
                "farthest-corner" => Some(RadialExtent::FarthestCorner),
                _ => None,
            };
            if let Some(extent) = extent {
                if !explicit_radii.is_empty() {
                    return Err(ParseError::custom("conflicting radial gradient sizes"));
                }
                size = RadialSize::Extent(extent);
                index += 1;
                break;
            }
            explicit_radii.push(parse_radial_radius(&prelude[index])?);
            if explicit_radii.len() > 2 {
                return Err(ParseError::custom("too many radial gradient radii"));
            }
            index += 1;
        }
        if !explicit_radii.is_empty() {
            if shape == RadialShape::Circle && explicit_radii.len() > 1 {
                return Err(ParseError::custom("circle gradients accept one radius"));
            }
            let radii = if explicit_radii.len() == 1 {
                [explicit_radii[0]; 2]
            } else {
                [explicit_radii[0], explicit_radii[1]]
            };
            size = RadialSize::Explicit(radii);
        }
        if index < prelude.len() {
            if !prelude[index].eq_ignore_ascii_case("at") || index + 1 >= prelude.len() {
                return Err(ParseError::custom("invalid radial gradient prelude"));
            }
            center = parse_gradient_position(&prelude[index + 1..])?;
        }
        parts.remove(0);
    }
    if !has_prelude {
        center = [GradientPosition::center(); 2];
    }
    let stops = parse_gradient_stops(parts.iter().copied(), false)?;
    if stops.len() < 2 {
        return Err(ParseError::custom(
            "a radial gradient needs two color stops",
        ));
    }
    Ok(RadialGradient {
        shape,
        size,
        center,
        stops,
        repeating,
        color_space,
    })
}

fn parse_gradient_color_space(
    prelude: &mut Vec<String>,
) -> Result<(GradientColorSpace, bool), ParseError<&'static str>> {
    let Some(index) = prelude
        .iter()
        .position(|value| value.eq_ignore_ascii_case("in"))
    else {
        return Ok((GradientColorSpace::default(), false));
    };
    let space = prelude
        .get(index + 1)
        .ok_or_else(|| ParseError::custom("missing gradient interpolation color space"))?;
    let color_space = if space.eq_ignore_ascii_case("srgb") {
        GradientColorSpace::Srgb
    } else if space.eq_ignore_ascii_case("srgb-linear") {
        GradientColorSpace::SrgbLinear
    } else if space.eq_ignore_ascii_case("oklab") {
        GradientColorSpace::Oklab
    } else {
        return Err(ParseError::custom(
            "unsupported gradient interpolation color space",
        ));
    };
    prelude.drain(index..=index + 1);
    Ok((color_space, true))
}

fn parse_radial_radius(value: &str) -> Result<RadialRadius, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let radius = match input.next()? {
        Token::Number { value, .. } if *value == 0.0 => RadialRadius::Length(0.0),
        Token::Percentage { unit_value, .. } if *unit_value >= 0.0 => {
            RadialRadius::Percentage(*unit_value)
        }
        Token::Dimension { value, unit, .. } => {
            let factor = match unit.to_ascii_lowercase().as_str() {
                "px" => 1.0,
                "pt" => 96.0 / 72.0,
                "pc" => 16.0,
                "in" => 96.0,
                "cm" => 96.0 / 2.54,
                "mm" => 96.0 / 25.4,
                "q" => 96.0 / 101.6,
                _ => return Err(ParseError::custom("unsupported radial gradient length")),
            };
            if *value < 0.0 {
                return Err(ParseError::custom(
                    "radial gradient radii cannot be negative",
                ));
            }
            RadialRadius::Length(*value * factor)
        }
        _ => return Err(ParseError::custom("invalid radial gradient radius")),
    };
    input.expect_exhausted()?;
    Ok(radius)
}

fn parse_gradient_position(
    values: &[String],
) -> Result<[GradientPosition; 2], ParseError<&'static str>> {
    if values.is_empty() || values.len() > 4 {
        return Err(ParseError::custom("invalid conic gradient position"));
    }
    let mut position = [None, None];
    let mut index = 0;
    while index < values.len() {
        let lower = values[index].to_ascii_lowercase();
        let edge = match lower.as_str() {
            "left" => Some((0, false, 0.0)),
            "right" => Some((0, true, 1.0)),
            "top" => Some((1, false, 0.0)),
            "bottom" => Some((1, true, 1.0)),
            _ => None,
        };
        if let Some((axis, from_end, percentage)) = edge {
            let has_offset = values.len() > 2
                && values.get(index + 1).is_some_and(|next| {
                    !matches!(
                        next.to_ascii_lowercase().as_str(),
                        "left" | "right" | "top" | "bottom" | "center"
                    )
                });
            let coordinate = if has_offset {
                let mut coordinate = parse_gradient_coordinate(&values[index + 1])?;
                coordinate.from_end = from_end;
                index += 2;
                coordinate
            } else {
                index += 1;
                GradientPosition {
                    percentage,
                    length: 0.0,
                    from_end: false,
                }
            };
            if position[axis].replace(coordinate).is_some() {
                return Err(ParseError::custom("duplicate gradient position axis"));
            }
        } else if lower == "center" {
            let axis = if position[0].is_none() {
                0
            } else if position[1].is_none() {
                1
            } else {
                return Err(ParseError::custom("too many gradient position coordinates"));
            };
            position[axis] = Some(GradientPosition::center());
            index += 1;
        } else {
            let axis = if position[0].is_none() {
                0
            } else if position[1].is_none() {
                1
            } else {
                return Err(ParseError::custom("too many gradient position coordinates"));
            };
            position[axis] = Some(parse_gradient_coordinate(&values[index])?);
            index += 1;
        }
    }
    Ok([
        position[0].unwrap_or_else(GradientPosition::center),
        position[1].unwrap_or_else(GradientPosition::center),
    ])
}

fn parse_gradient_coordinate(value: &str) -> Result<GradientPosition, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let (percentage, length) = match input.next()? {
        Token::Percentage { unit_value, .. } => (*unit_value, 0.0),
        Token::Number { value, .. } if *value == 0.0 => (0.0, 0.0),
        Token::Dimension { value, unit, .. } => {
            let factor = match unit.to_ascii_lowercase().as_str() {
                "px" => 1.0,
                "pt" => 96.0 / 72.0,
                "pc" => 16.0,
                "in" => 96.0,
                "cm" => 96.0 / 2.54,
                "mm" => 96.0 / 25.4,
                "q" => 96.0 / 101.6,
                _ => return Err(ParseError::custom("unsupported gradient position length")),
            };
            (0.0, *value * factor)
        }
        _ => return Err(ParseError::custom("invalid gradient position coordinate")),
    };
    input.expect_exhausted()?;
    if !percentage.is_finite() || !length.is_finite() {
        return Err(ParseError::custom("invalid gradient position coordinate"));
    }
    Ok(GradientPosition {
        percentage,
        length,
        from_end: false,
    })
}

fn parse_gradient_stops<'a>(
    parts: impl IntoIterator<Item = &'a str>,
    conic: bool,
) -> Result<Vec<GradientStop>, ParseError<&'static str>> {
    let mut stops: Vec<GradientStop> = Vec::new();
    let mut pending_hint = None;
    for part in parts {
        let parsed = if conic {
            parse_conic_gradient_stop(part)
        } else {
            parse_gradient_stop(part)
        };
        match parsed {
            Ok(mut next) => {
                if let Some(hint) = pending_hint.take() {
                    let Some(previous) = stops.last_mut() else {
                        return Err(ParseError::custom(
                            "gradient hint requires a preceding stop",
                        ));
                    };
                    previous.hint_after = Some(hint);
                }
                stops.append(&mut next);
            }
            Err(error) => {
                let hint = parse_gradient_hint(part, conic);
                if let Ok(hint) = hint {
                    if stops.is_empty() || pending_hint.replace(hint).is_some() {
                        return Err(ParseError::custom("invalid gradient hint position"));
                    }
                } else {
                    return Err(error);
                }
            }
        }
    }
    if pending_hint.is_some() {
        return Err(ParseError::custom(
            "gradient hint requires a following stop",
        ));
    }
    Ok(stops)
}

fn parse_gradient_hint(value: &str, conic: bool) -> Result<f32, ParseError<&'static str>> {
    let mut input = CssParser::new(value);
    let position = match input.next()? {
        Token::Percentage { unit_value, .. } => *unit_value,
        Token::Number { value, .. } if *value == 0.0 => 0.0,
        Token::Dimension { .. } if conic => {
            parse_gradient_angle(value)
                .ok_or_else(|| ParseError::custom("invalid conic gradient hint"))?
                / 360.0
        }
        _ => return Err(ParseError::custom("invalid gradient hint")),
    };
    input.expect_exhausted()?;
    Ok(position)
}

fn parse_conic_gradient_stop(value: &str) -> Result<Vec<GradientStop>, ParseError<&'static str>> {
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
    let mut positions = Vec::new();
    while !input.is_exhausted() {
        let start = input.position();
        let token = input.next()?.clone();
        let position = match token {
            Token::Percentage { unit_value, .. } => unit_value,
            Token::Number { value, .. } if value == 0.0 => 0.0,
            Token::Dimension { .. } => {
                parse_gradient_angle(input.slice_from(start).trim())
                    .ok_or_else(|| ParseError::custom("invalid conic gradient stop"))?
                    / 360.0
            }
            _ => return Err(ParseError::custom("invalid conic gradient stop")),
        };
        positions.push(position);
        if positions.len() > 2 {
            return Err(ParseError::custom("too many conic gradient stop positions"));
        }
        input.skip_whitespace();
    }
    Ok(gradient_stops(color, positions))
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
    Some(angle)
}

fn parse_gradient_stop(value: &str) -> Result<Vec<GradientStop>, ParseError<&'static str>> {
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
    let mut positions = Vec::new();
    while !input.is_exhausted() {
        let position = match input.next()? {
            Token::Percentage { unit_value, .. } => *unit_value,
            _ => return Err(ParseError::custom("unsupported gradient stop")),
        };
        positions.push(position);
        if positions.len() > 2 {
            return Err(ParseError::custom("too many gradient stop positions"));
        }
        input.skip_whitespace();
    }
    Ok(gradient_stops(color, positions))
}

fn gradient_stops(color: Color32, positions: Vec<f32>) -> Vec<GradientStop> {
    if positions.is_empty() {
        vec![GradientStop {
            color,
            position: None,
            hint_after: None,
        }]
    } else {
        positions
            .into_iter()
            .map(|position| GradientStop {
                color,
                position: Some(position),
                hint_after: None,
            })
            .collect()
    }
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
    FirstLetter,
    FirstLine,
}

impl parser::NonTSPseudoClass for StatePseudo {
    fn is_active_or_hover(&self) -> bool {
        matches!(self, Self::Hover | Self::Active)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(self, Self::Hover | Self::Active)
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
            Self::FirstLetter => ":first-letter",
            Self::FirstLine => ":first-line",
        })
    }
}

impl parser::PseudoElement for PseudoElementKind {}

impl ToCss for PseudoElementKind {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        _dest.write_str(match self {
            Self::FirstLetter => "::first-letter",
            Self::FirstLine => "::first-line",
        })
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
    type PseudoElement = PseudoElementKind;
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
        } else if name.eq_ignore_ascii_case("first-letter") {
            Ok(StatePseudo::FirstLetter)
        } else if name.eq_ignore_ascii_case("first-line") {
            Ok(StatePseudo::FirstLine)
        } else {
            Err(ParseError::custom(
                parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }

    fn parse_pseudo_element(
        &self,
        name: CowRcStr<'i>,
    ) -> Result<PseudoElementKind, ParseError<Self::Error>> {
        if name.eq_ignore_ascii_case("first-letter") {
            Ok(PseudoElementKind::FirstLetter)
        } else if name.eq_ignore_ascii_case("first-line") {
            Ok(PseudoElementKind::FirstLine)
        } else {
            Err(ParseError::custom(
                parser::SelectorParseErrorKind::UnsupportedPseudoClassOrElement,
            ))
        }
    }
}
