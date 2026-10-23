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

mod background;
mod color;
mod media;
pub(crate) mod variables;

pub(crate) use background::parse_background_image;
use background::{
    consume_value, parse_background_attachment_value, parse_background_attachments,
    parse_background_clip_value, parse_background_clips, parse_background_origins,
    parse_background_position_value, parse_background_positions, parse_background_repeat_atom,
    parse_background_repeats, parse_background_size_components, parse_background_sizes,
    split_top_level_commas,
};
pub(crate) use color::parse_color;
use media::MediaQueryList;

#[derive(Clone, Debug)]
pub(crate) struct Rule {
    pub(crate) target: SelectorTarget,
    pub(crate) state: RuleState,
    pub(crate) pseudo: Option<PseudoElementKind>,
    pub(crate) specificity: u32,
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) media: Vec<MediaQueryList>,
}

impl Rule {
    pub(crate) fn matches_media(&self, viewport: egui::Vec2) -> bool {
        self.media.iter().all(|query| query.matches(viewport))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SelectorTarget {
    pub(crate) classes: Vec<String>,
    pub(crate) id: Option<String>,
}

impl SelectorTarget {
    pub(crate) fn is_universal(&self) -> bool {
        self.classes.is_empty() && self.id.is_none()
    }

    pub(crate) fn matches(&self, target: &Self) -> bool {
        self.id
            .as_ref()
            .is_none_or(|id| target.id.as_ref() == Some(id))
            && self
                .classes
                .iter()
                .all(|class| target.classes.contains(class))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Keyframe {
    pub(crate) offset: f32,
    pub(crate) timing_function: Option<String>,
    pub(crate) declarations: Vec<Declaration>,
}

#[derive(Clone, Debug)]
pub(crate) struct Keyframes {
    pub(crate) name: String,
    pub(crate) frames: Vec<Keyframe>,
    pub(crate) media: Vec<MediaQueryList>,
}

#[derive(Debug)]
pub(crate) struct ParsedStylesheet {
    pub(crate) rules: Vec<Rule>,
    pub(crate) imports: Vec<String>,
    pub(crate) keyframes: Vec<Keyframes>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuleState(u8);

impl RuleState {
    pub(crate) const BASE: Self = Self(0);
    pub(crate) const HOVERED: Self = Self(1);
    pub(crate) const ACTIVE: Self = Self(2);
    pub(crate) const FOCUS: Self = Self(4);
    pub(crate) const FOCUS_VISIBLE: Self = Self(8);
    pub(crate) const ENABLED: Self = Self(16);
    pub(crate) const DISABLED: Self = Self(32);

    pub(crate) fn bits(self) -> u8 {
        self.0
    }

    pub(crate) fn matches(self, state: u8) -> bool {
        state & self.0 == self.0
    }

    fn insert(&mut self, state: Self) {
        self.0 |= state.0;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PseudoElementKind {
    FirstLetter,
    FirstLine,
}

#[derive(Clone, Debug)]
pub(crate) enum Declaration {
    All(AllKeyword),
    Color(Color32),
    Background(Color32),
    BackgroundShorthand(BackgroundShorthand),
    BackgroundImage(Vec<BackgroundImage>),
    BackgroundAttachment(Vec<BackgroundAttachment>),
    BackgroundClip(Vec<BackgroundClip>),
    BackgroundOrigin(Vec<BackgroundOrigin>),
    BackgroundPosition(Vec<BackgroundPosition>),
    BackgroundRepeat(Vec<BackgroundRepeatPair>),
    BackgroundSize(Vec<BackgroundSize>),
    Layout(crate::layout_style::Declaration),
    BorderRadius(u8),
    FontFamily(Vec<String>),
    FontWeight(u16),
    FontStyle(bool),
    Animation(crate::animation::Property),
    Transition(crate::transition::Property),
    CustomProperty {
        name: String,
        value: String,
        important: bool,
    },
    Deferred {
        name: String,
        value: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AllKeyword {
    Initial,
    Inherit,
    Unset,
    Revert,
}

#[derive(Clone, Debug)]
pub(crate) struct BackgroundShorthand {
    pub(crate) color: Color32,
    pub(crate) images: Vec<BackgroundImage>,
    pub(crate) attachments: Vec<BackgroundAttachment>,
    pub(crate) clips: Vec<BackgroundClip>,
    pub(crate) origins: Vec<BackgroundOrigin>,
    pub(crate) positions: Vec<BackgroundPosition>,
    pub(crate) repeats: Vec<BackgroundRepeatPair>,
    pub(crate) sizes: Vec<BackgroundSize>,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BackgroundOrigin {
    Border,
    #[default]
    Padding,
    Content,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BackgroundRepeat {
    #[default]
    Repeat,
    NoRepeat,
    Space,
    Round,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BackgroundRepeatPair {
    pub(crate) x: BackgroundRepeat,
    pub(crate) y: BackgroundRepeat,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BackgroundPosition {
    pub(crate) x: BackgroundPositionAxis,
    pub(crate) y: BackgroundPositionAxis,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BackgroundPositionAxis {
    FreeSpace(String),
    Start(String),
    Center,
    End(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BackgroundSize {
    Cover,
    Contain,
    Explicit([BackgroundSizeComponent; 2]),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BackgroundSizeComponent {
    Auto,
    Value(String),
}

enum ParsedRule {
    Import(String),
    Style(Vec<Rule>),
    Keyframes(Keyframes),
    Media(ParsedStylesheet),
}

enum AtRulePrelude {
    Import(String),
    Keyframes(String),
    Media(MediaQueryList),
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

pub(crate) fn parse(source: &str) -> Result<ParsedStylesheet, CssError> {
    let mut parser = CssParser::new(source);
    parse_rules(&mut parser).map_err(|error| to_css_error(error, parser.current_source_location()))
}

fn parse_rules(input: &mut CssParser<'_>) -> Result<ParsedStylesheet, ParseError<ParseIssue>> {
    let mut rule_parser = StylesheetRuleParser;
    let mut rules = Vec::new();
    let mut imports = Vec::new();
    let mut keyframes = Vec::new();

    for parsed in StyleSheetParser::new(input, &mut rule_parser) {
        match parsed {
            Ok(ParsedRule::Import(import)) => imports.push(import),
            Ok(ParsedRule::Style(mut parsed_rules)) => rules.append(&mut parsed_rules),
            Ok(ParsedRule::Keyframes(parsed)) => {
                keyframes.push(parsed);
            }
            Ok(ParsedRule::Media(mut parsed)) => {
                rules.append(&mut parsed.rules);
                keyframes.append(&mut parsed.keyframes);
            }
            Err((error, _, location)) => {
                let mut issue = match error.kind {
                    ParseErrorKind::Custom(issue) => issue,
                    ParseErrorKind::Basic(_) => ParseIssue::new("invalid CSS syntax"),
                };
                issue.line.get_or_insert(location.line);
                return Err(ParseError::custom(issue));
            }
        }
    }

    Ok(ParsedStylesheet {
        rules,
        imports,
        keyframes,
    })
}

pub(crate) fn parse_style_attribute(source: &str) -> Result<Vec<Declaration>, CssError> {
    let mut parser = CssParser::new(source);
    parse_declarations(&mut parser, true)
        .map_err(|issue| CssError::at_line(issue.line.unwrap_or(0), issue.message))
}

pub(crate) fn selector_target(selector: &str) -> Option<SelectorTarget> {
    if let Some(class) = selector.strip_prefix('.')
        && is_simple_class_name(class)
    {
        return Some(SelectorTarget {
            classes: vec![class.to_owned()],
            id: None,
        });
    }
    let mut parser = CssParser::new(selector);
    parse_selectors(&mut parser)
        .ok()
        .filter(|selectors| selectors.len() == 1)
        .and_then(|mut selectors| {
            let (target, state, pseudo, _) = selectors.pop()?;
            (state == RuleState::BASE && pseudo.is_none() && !target.is_universal()).then(|| {
                let mut target = target;
                target.classes.sort();
                target.classes.dedup();
                target
            })
        })
}

fn is_simple_class_name(class: &str) -> bool {
    let bytes = class.as_bytes();
    let Some(first) = bytes.first() else {
        return false;
    };
    let valid_start = first.is_ascii_alphabetic()
        || *first == b'_'
        || *first == b'-'
            && bytes
                .get(1)
                .is_some_and(|next| next.is_ascii_alphabetic() || matches!(*next, b'_' | b'-'));
    valid_start
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
}

struct StylesheetRuleParser;

impl<'i> QualifiedRuleParser<'i> for StylesheetRuleParser {
    type Prelude = Vec<ParsedSelector>;
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
        let declarations = parse_declarations(input, false).map_err(ParseError::custom)?;
        Ok(ParsedRule::Style(
            selectors
                .into_iter()
                .map(|(target, state, pseudo, specificity)| Rule {
                    target,
                    state,
                    pseudo,
                    specificity,
                    declarations: declarations.clone(),
                    media: Vec::new(),
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
        if name.eq_ignore_ascii_case("import") {
            return parse_import(input).map(AtRulePrelude::Import);
        }
        if name.eq_ignore_ascii_case("media") {
            return MediaQueryList::parse(input).map(AtRulePrelude::Media);
        }
        if name.eq_ignore_ascii_case("keyframes") {
            let name = input
                .expect_ident_cloned()
                .map_err(|_| ParseError::custom(ParseIssue::new("invalid keyframes name")))?
                .to_string();
            input
                .expect_exhausted()
                .map_err(|_| ParseError::custom(ParseIssue::new("invalid keyframes name")))?;
            return Ok(AtRulePrelude::Keyframes(name));
        }
        Err(ParseError::custom(ParseIssue::new("unsupported at-rule")))
    }

    fn rule_without_block(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
    ) -> Result<Self::AtRule, ()> {
        match prelude {
            AtRulePrelude::Import(import) => Ok(ParsedRule::Import(import)),
            AtRulePrelude::Keyframes(_) | AtRulePrelude::Media(_) => Err(()),
        }
    }

    fn parse_block(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Self::AtRule, ParseError<Self::Error>> {
        match prelude {
            AtRulePrelude::Import(_) => {
                Err(ParseError::custom(ParseIssue::new("invalid import rule")))
            }
            AtRulePrelude::Keyframes(name) => parse_keyframes(input).map(|frames| {
                ParsedRule::Keyframes(Keyframes {
                    name,
                    frames,
                    media: Vec::new(),
                })
            }),
            AtRulePrelude::Media(query) => {
                let mut parsed = parse_rules(input)?;
                if !parsed.imports.is_empty() {
                    return Err(ParseError::custom(ParseIssue::new(
                        "invalid nested import rule",
                    )));
                }
                for rule in &mut parsed.rules {
                    rule.media.push(query.clone());
                }
                for keyframes in &mut parsed.keyframes {
                    keyframes.media.push(query.clone());
                }
                Ok(ParsedRule::Media(parsed))
            }
        }
    }
}

struct KeyframeRuleParser;

impl<'i> QualifiedRuleParser<'i> for KeyframeRuleParser {
    type Prelude = Vec<f32>;
    type QualifiedRule = Vec<Keyframe>;
    type Error = ParseIssue;

    fn parse_prelude(
        &mut self,
        input: &mut CssParser<'i>,
    ) -> Result<Self::Prelude, ParseError<Self::Error>> {
        let start = input.position();
        while input.next().is_ok() {}
        let selectors = split_top_level_commas(input.slice_from(start));
        if selectors.is_empty() {
            return Err(ParseError::custom(ParseIssue::new(
                "invalid keyframe selector",
            )));
        }
        selectors
            .into_iter()
            .map(|selector| {
                let offset = if selector.eq_ignore_ascii_case("from") {
                    0.0
                } else if selector.eq_ignore_ascii_case("to") {
                    1.0
                } else {
                    let value = selector
                        .strip_suffix('%')
                        .ok_or_else(|| {
                            ParseError::custom(ParseIssue::new("invalid keyframe selector"))
                        })?
                        .trim()
                        .parse::<f32>()
                        .map_err(|_| {
                            ParseError::custom(ParseIssue::new("invalid keyframe selector"))
                        })?;
                    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                        return Err(ParseError::custom(ParseIssue::new(
                            "invalid keyframe selector",
                        )));
                    }
                    value / 100.0
                };
                Ok(offset)
            })
            .collect()
    }

    fn parse_block(
        &mut self,
        offsets: Self::Prelude,
        _start: &ParserState,
        input: &mut CssParser<'i>,
    ) -> Result<Self::QualifiedRule, ParseError<Self::Error>> {
        let declarations = parse_declarations(input, false).map_err(ParseError::custom)?;
        let timing_function = declarations.iter().rev().find_map(|declaration| {
            let Declaration::Animation(property) = declaration else {
                return None;
            };
            (property.name == "animation-timing-function").then(|| property.value.clone())
        });
        Ok(offsets
            .into_iter()
            .map(|offset| Keyframe {
                offset,
                timing_function: timing_function.clone(),
                declarations: declarations.clone(),
            })
            .collect())
    }
}

impl<'i> AtRuleParser<'i> for KeyframeRuleParser {
    type Prelude = ();
    type AtRule = Vec<Keyframe>;
    type Error = ParseIssue;
}

fn parse_keyframes(input: &mut CssParser<'_>) -> Result<Vec<Keyframe>, ParseError<ParseIssue>> {
    let mut parser = KeyframeRuleParser;
    let mut frames = Vec::new();
    for parsed in StyleSheetParser::new(input, &mut parser) {
        match parsed {
            Ok(mut parsed) => frames.append(&mut parsed),
            Err((error, _, location)) => {
                let issue = match error.kind {
                    ParseErrorKind::Custom(issue) => issue,
                    ParseErrorKind::Basic(_) => ParseIssue {
                        message: "invalid keyframes rule",
                        line: Some(location.line),
                    },
                };
                return Err(ParseError::custom(issue));
            }
        }
    }
    frames.sort_by(|first, second| first.offset.total_cmp(&second.offset));
    Ok(frames)
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

type ParsedSelector = (SelectorTarget, RuleState, Option<PseudoElementKind>, u32);

fn parse_selectors<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<ParsedSelector>, ParseError<ParseIssue>> {
    let selector_parser = SimpleSelectorParser;
    let selectors = SelectorList::parse(&selector_parser, input, ParseRelative::No)
        .map_err(|_| ParseError::custom(ParseIssue::new("unsupported class selector")))?;

    let mut result = Vec::with_capacity(selectors.len());
    for selector in selectors.slice() {
        let mut target = SelectorTarget::default();
        let mut universal = false;
        let mut state = RuleState::BASE;
        let mut pseudo = None;
        for component in selector.iter_raw_parse_order_from(0) {
            match component {
                Component::Class(identifier) => {
                    target.classes.push(identifier.0.clone());
                }
                Component::ID(identifier) if target.id.is_none() => {
                    target.id = Some(identifier.0.clone());
                }
                Component::ExplicitUniversalType if !universal => {
                    universal = true;
                }
                Component::NonTSPseudoClass(StatePseudo::FirstLetter) if pseudo.is_none() => {
                    pseudo = Some(PseudoElementKind::FirstLetter);
                }
                Component::NonTSPseudoClass(StatePseudo::FirstLine) if pseudo.is_none() => {
                    pseudo = Some(PseudoElementKind::FirstLine);
                }
                Component::NonTSPseudoClass(value) => {
                    state.insert(match value {
                        StatePseudo::Hover => RuleState::HOVERED,
                        StatePseudo::Active => RuleState::ACTIVE,
                        StatePseudo::Focus => RuleState::FOCUS,
                        StatePseudo::FocusVisible => RuleState::FOCUS_VISIBLE,
                        StatePseudo::Enabled => RuleState::ENABLED,
                        StatePseudo::Disabled => RuleState::DISABLED,
                        StatePseudo::FirstLetter | StatePseudo::FirstLine => {
                            return Err(ParseError::custom(ParseIssue::new(
                                "unsupported class selector",
                            )));
                        }
                    });
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
        if !universal && target.is_universal() && state == RuleState::BASE && pseudo.is_none() {
            return Err(ParseError::custom(ParseIssue::new(
                "unsupported class selector",
            )));
        }
        result.push((target, state, pseudo, selector.specificity()));
    }
    if result.is_empty() {
        return Err(ParseError::custom(ParseIssue::new(
            "unsupported class selector",
        )));
    }
    Ok(result)
}

struct Declarations {
    allow_important: bool,
}

impl<'i> DeclarationParser<'i> for Declarations {
    type Declaration = Option<Declaration>;
    type Error = &'static str;

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
        _declaration_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<Self::Error>> {
        if self.allow_important {
            let start = input.state();
            while input.next().is_ok() {}
            let source = input.slice_from(start.position()).trim();
            let stripped = strip_important(source);
            let important = stripped.is_some();
            let source = stripped.unwrap_or(source).trim();
            let mut input = CssParser::new(source);
            return self.parse_value_content(name, &mut input, important);
        }
        self.parse_value_content(name, input, false)
    }
}

impl Declarations {
    fn parse_value_content<'i>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut CssParser<'i>,
        important: bool,
    ) -> Result<Option<Declaration>, ParseError<&'static str>> {
        let start = input.state();
        while input.next().is_ok() {}
        let value = input.slice_from(start.position()).trim().to_owned();

        if name.starts_with("--") {
            let stripped = strip_important(&value);
            return Ok(Some(Declaration::CustomProperty {
                name: name.to_string(),
                value: stripped.unwrap_or(&value).trim().to_owned(),
                important: important || stripped.is_some(),
            }));
        }
        if variables::contains_var(&value) {
            return Ok(Some(Declaration::Deferred {
                name: name.to_ascii_lowercase(),
                value,
            }));
        }
        input.reset(&start);
        if name.eq_ignore_ascii_case("all") {
            let keyword = match value.trim().to_ascii_lowercase().as_str() {
                "initial" => AllKeyword::Initial,
                "inherit" => AllKeyword::Inherit,
                "unset" => AllKeyword::Unset,
                "revert" => AllKeyword::Revert,
                "revert-layer" => return Err(ParseError::custom("cascade layers are unsupported")),
                _ => return Err(ParseError::custom("invalid all value")),
            };
            while input.next().is_ok() {}
            return Ok(Some(Declaration::All(keyword)));
        }
        if crate::animation::is_property(&name) {
            let value = consume_value(input)?;
            return crate::animation::parse_property(&name, &value)
                .map(Declaration::Animation)
                .map(Some)
                .map_err(ParseError::custom);
        }
        if crate::transition::is_property(&name) {
            let value = consume_value(input)?;
            return crate::transition::parse_property(&name, &value)
                .map(Declaration::Transition)
                .map(Some)
                .map_err(ParseError::custom);
        }
        if (name.eq_ignore_ascii_case("color") || name.eq_ignore_ascii_case("background-color"))
            && ["inherit", "initial", "unset", "revert", "currentcolor"]
                .iter()
                .any(|keyword| value.eq_ignore_ascii_case(keyword))
        {
            while input.next().is_ok() {}
            return Ok(Some(Declaration::Layout(
                crate::layout_style::Declaration {
                    name: name.to_ascii_lowercase(),
                    value,
                },
            )));
        }
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
        } else if name.eq_ignore_ascii_case("background-origin") {
            Some(Declaration::BackgroundOrigin(parse_background_origins(
                input,
            )?))
        } else if name.eq_ignore_ascii_case("background-position") {
            Some(Declaration::BackgroundPosition(parse_background_positions(
                input,
            )?))
        } else if name.eq_ignore_ascii_case("background-repeat") {
            Some(Declaration::BackgroundRepeat(parse_background_repeats(
                input,
            )?))
        } else if name.eq_ignore_ascii_case("background-size") {
            Some(Declaration::BackgroundSize(parse_background_sizes(input)?))
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

pub(crate) fn resolve_deferred(
    declaration: &Declaration,
    properties: &variables::CustomProperties,
) -> Option<Declaration> {
    let Declaration::Deferred { name, value } = declaration else {
        return Some(declaration.clone());
    };
    let Some(value) = variables::substitute(value, properties) else {
        return Some(invalid_computed_value(name));
    };
    let source = format!("{name}: {value};");
    let mut input = CssParser::new(&source);
    let mut declarations = parse_declarations(&mut input, false).unwrap_or_default();
    declarations
        .pop()
        .or_else(|| Some(invalid_computed_value(name)))
}

fn invalid_computed_value(name: &str) -> Declaration {
    if crate::transition::is_property(name) {
        return Declaration::Transition(
            crate::transition::parse_property(name, "unset").expect("valid transition reset"),
        );
    }
    Declaration::Layout(crate::layout_style::Declaration {
        name: name.to_owned(),
        value: "unset".to_owned(),
    })
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
        origins: Vec::with_capacity(layers.len()),
        positions: Vec::with_capacity(layers.len()),
        repeats: Vec::with_capacity(layers.len()),
        sizes: Vec::with_capacity(layers.len()),
    };

    for (layer_index, layer) in layers.iter().enumerate() {
        let components = crate::layout_style::components(layer).map_err(ParseError::custom)?;
        if components.is_empty() {
            return Err(ParseError::custom("empty background layer"));
        }
        let slash = components.iter().position(|component| component == "/");
        if slash.is_some_and(|slash| {
            slash == 0
                || slash + 1 == components.len()
                || components[slash + 1..].contains(&"/".to_owned())
        }) {
            return Err(ParseError::custom("invalid background position and size"));
        }
        let split = slash.unwrap_or(components.len());
        let mut image = None;
        let mut attachment = None;
        let mut repeat_tokens = Vec::new();
        let mut box_tokens = Vec::new();
        let mut position_tokens = Vec::new();
        let mut size_tokens = Vec::new();
        let mut color_seen = false;
        for component in &components[..split] {
            if let Ok(color) = parse_color(&mut CssParser::new(component)) {
                if layer_index + 1 != layers.len() || color_seen {
                    return Err(ParseError::custom(
                        "background color must appear once in the final layer",
                    ));
                }
                background.color = color;
                color_seen = true;
            } else if let Ok(parsed) = parse_background_image(component) {
                if image.replace(parsed).is_some() {
                    return Err(ParseError::custom("duplicate image in background layer"));
                }
            } else if let Ok(parsed) = parse_background_attachment_value(component) {
                if attachment.replace(parsed).is_some() {
                    return Err(ParseError::custom(
                        "duplicate attachment in background layer",
                    ));
                }
            } else if let Ok(parsed) = parse_background_clip_value(component) {
                box_tokens.push(parsed);
            } else if background::parse_background_repeat_component(component).is_ok() {
                repeat_tokens.push(component.clone());
            } else {
                position_tokens.push(component.clone());
            }
        }
        let position = if position_tokens.is_empty() {
            parse_background_position_value("0% 0%")
        } else {
            parse_background_position_value(&position_tokens.join(" "))
        }?;

        let size = if let Some(slash) = slash {
            let mut after_size = slash + 1;
            while after_size < components.len()
                && size_tokens.len() < 3
                && is_background_size_component(&components[after_size])
            {
                size_tokens.push(components[after_size].clone());
                after_size += 1;
            }
            if size_tokens.is_empty() {
                return Err(ParseError::custom("background slash requires a size"));
            }
            for component in &components[after_size..] {
                if let Ok(parsed) = parse_background_attachment_value(component) {
                    if attachment.replace(parsed).is_some() {
                        return Err(ParseError::custom(
                            "duplicate attachment in background layer",
                        ));
                    }
                } else if let Ok(parsed) = parse_background_clip_value(component) {
                    box_tokens.push(parsed);
                } else if background::parse_background_repeat_component(component).is_ok() {
                    repeat_tokens.push(component.clone());
                } else if let Ok(color) = parse_color(&mut CssParser::new(component)) {
                    if layer_index + 1 != layers.len() || color_seen {
                        return Err(ParseError::custom(
                            "background color must appear once in the final layer",
                        ));
                    }
                    background.color = color;
                    color_seen = true;
                } else {
                    return Err(ParseError::custom("unsupported background value"));
                }
            }
            parse_background_size_components(&size_tokens)?
        } else {
            if components.iter().any(|component| {
                matches!(component.to_ascii_lowercase().as_str(), "cover" | "contain")
            }) {
                return Err(ParseError::custom(
                    "background cover and contain require a position-size slash",
                ));
            }
            BackgroundSize::Explicit([BackgroundSizeComponent::Auto, BackgroundSizeComponent::Auto])
        };

        if box_tokens.len() > 2 {
            return Err(ParseError::custom("too many background box values"));
        }
        let (origin, clip) = match box_tokens.as_slice() {
            [] => (BackgroundOrigin::Padding, None),
            [box_value] => (origin_for_clip(*box_value), Some(*box_value)),
            [origin, clip] => (origin_for_clip(*origin), Some(*clip)),
            _ => unreachable!(),
        };
        let repeat = parse_background_repeat_tokens(&repeat_tokens).map_err(ParseError::custom)?;
        background
            .images
            .push(image.unwrap_or(BackgroundImage::None));
        background
            .attachments
            .push(attachment.unwrap_or(BackgroundAttachment::Scroll));
        background
            .clips
            .push(clip.unwrap_or(BackgroundClip::Border));
        background.origins.push(origin);
        background.positions.push(position);
        background.repeats.push(repeat);
        background.sizes.push(size);
    }
    Ok(background)
}

fn origin_for_clip(clip: BackgroundClip) -> BackgroundOrigin {
    match clip {
        BackgroundClip::Border => BackgroundOrigin::Border,
        BackgroundClip::Padding => BackgroundOrigin::Padding,
        BackgroundClip::Content => BackgroundOrigin::Content,
    }
}

fn is_background_size_component(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "auto" | "cover" | "contain"
    ) || background::is_background_length_percentage(value, false)
}

fn parse_background_repeat_tokens(values: &[String]) -> Result<BackgroundRepeatPair, &'static str> {
    match values {
        [] => Ok(BackgroundRepeatPair::default()),
        [value] => background::parse_background_repeat_component(value)
            .map_err(|_| "invalid background repeat"),
        [first, second] => {
            let first = parse_background_repeat_atom(first).ok_or("invalid background repeat")?;
            let second = parse_background_repeat_atom(second).ok_or("invalid background repeat")?;
            Ok(BackgroundRepeatPair {
                x: first,
                y: second,
            })
        }
        _ => Err("too many background repeat values"),
    }
}

fn parse_background_images_value<'i>(
    input: &mut CssParser<'i>,
) -> Result<Vec<BackgroundImage>, ParseError<&'static str>> {
    split_top_level_commas(&consume_value(input)?)
        .into_iter()
        .map(parse_background_image)
        .collect()
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

fn parse_declarations<'i>(
    input: &mut CssParser<'i>,
    allow_important: bool,
) -> Result<Vec<Declaration>, ParseIssue> {
    let mut parser = Declarations { allow_important };
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

fn strip_important(source: &str) -> Option<&str> {
    let mut input = CssParser::new(source);
    while !input.is_exhausted() {
        input.skip_whitespace();
        if input.is_exhausted() {
            break;
        }
        let state = input.state();
        let token = input.next_including_whitespace_and_comments().ok()?.clone();
        match token {
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                input
                    .parse_nested_block(|nested| {
                        while nested.next().is_ok() {}
                        Ok::<_, ParseError<()>>(())
                    })
                    .ok()?;
            }
            Token::Delim('!') => {
                input.reset(&state);
                if input.try_parse(cssparser::parse_important).is_ok() {
                    input.skip_whitespace();
                    if input.is_exhausted() {
                        return Some(&source[..state.position().byte_index()]);
                    }
                }
            }
            _ => {}
        }
    }
    None
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
    Focus,
    FocusVisible,
    Enabled,
    Disabled,
    FirstLetter,
    FirstLine,
}

impl parser::NonTSPseudoClass for StatePseudo {
    fn is_active_or_hover(&self) -> bool {
        matches!(self, Self::Hover | Self::Active)
    }

    fn is_user_action_state(&self) -> bool {
        matches!(
            self,
            Self::Hover | Self::Active | Self::Focus | Self::FocusVisible
        )
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
            Self::Focus => ":focus",
            Self::FocusVisible => ":focus-visible",
            Self::Enabled => ":enabled",
            Self::Disabled => ":disabled",
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
        } else if name.eq_ignore_ascii_case("focus") {
            Ok(StatePseudo::Focus)
        } else if name.eq_ignore_ascii_case("focus-visible") {
            Ok(StatePseudo::FocusVisible)
        } else if name.eq_ignore_ascii_case("enabled") {
            Ok(StatePseudo::Enabled)
        } else if name.eq_ignore_ascii_case("disabled") {
            Ok(StatePseudo::Disabled)
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
