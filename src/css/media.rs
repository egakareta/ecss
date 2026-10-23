use super::{CssParser, ParseError, ParseIssue, Token};
use egui::Vec2;

#[derive(Clone, Debug)]
pub(crate) struct MediaQueryList(Vec<Condition>);

impl MediaQueryList {
    pub(super) fn parse(input: &mut CssParser<'_>) -> Result<Self, ParseError<ParseIssue>> {
        input.parse_comma_separated(parse_query).map(Self)
    }

    pub(crate) fn matches(&self, viewport: Vec2) -> bool {
        self.0.iter().any(|query| query.matches(viewport))
    }
}

#[derive(Clone, Debug)]
enum Condition {
    Constant(bool),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    Compare(Feature, Comparison, f32),
    Portrait(bool),
}

impl Condition {
    fn matches(&self, viewport: Vec2) -> bool {
        match self {
            Self::Constant(value) => *value,
            Self::Not(condition) => !condition.matches(viewport),
            Self::And(conditions) => conditions
                .iter()
                .all(|condition| condition.matches(viewport)),
            Self::Or(conditions) => conditions
                .iter()
                .any(|condition| condition.matches(viewport)),
            Self::Compare(feature, comparison, expected) => {
                let actual = match feature {
                    Feature::Width => viewport.x,
                    Feature::Height => viewport.y,
                    Feature::AspectRatio => viewport.x / viewport.y,
                };
                comparison.matches(actual, *expected)
            }
            Self::Portrait(portrait) => (viewport.y >= viewport.x) == *portrait,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Feature {
    Width,
    Height,
    AspectRatio,
}

impl Feature {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "width" => Some(Self::Width),
            "height" => Some(Self::Height),
            "aspect-ratio" => Some(Self::AspectRatio),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Comparison {
    Equal,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl Comparison {
    fn matches(self, actual: f32, expected: f32) -> bool {
        match self {
            Self::Equal => actual == expected,
            Self::Less => actual < expected,
            Self::LessEqual => actual <= expected,
            Self::Greater => actual > expected,
            Self::GreaterEqual => actual >= expected,
        }
    }

    fn reversed(self) -> Self {
        match self {
            Self::Equal => Self::Equal,
            Self::Less => Self::Greater,
            Self::LessEqual => Self::GreaterEqual,
            Self::Greater => Self::Less,
            Self::GreaterEqual => Self::LessEqual,
        }
    }
}

fn invalid() -> ParseError<ParseIssue> {
    ParseError::custom(ParseIssue::new("invalid media query"))
}

fn parse_query(input: &mut CssParser<'_>) -> Result<Condition, ParseError<ParseIssue>> {
    let start = input.state();
    let modifier = input
        .try_parse(|input| {
            let name = input.expect_ident_cloned()?;
            if name.eq_ignore_ascii_case("not") || name.eq_ignore_ascii_case("only") {
                Ok(name.to_ascii_lowercase())
            } else {
                Err(invalid())
            }
        })
        .ok();
    if let Ok(name) = input.try_parse(|input| input.expect_ident_cloned()) {
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "not" | "only" | "and" | "or"
        ) {
            return Err(invalid());
        }
        let media_type = Condition::Constant(
            name.eq_ignore_ascii_case("screen") || name.eq_ignore_ascii_case("all"),
        );
        let condition = if input.is_exhausted() {
            media_type
        } else {
            input.expect_ident_matching("and")?;
            Condition::And(vec![media_type, parse_condition(input, false)?])
        };
        return Ok(if modifier.as_deref() == Some("not") {
            Condition::Not(Box::new(condition))
        } else {
            condition
        });
    }
    if modifier.as_deref() == Some("only") {
        return Err(invalid());
    }
    input.reset(&start);
    parse_condition(input, true)
}

fn parse_condition(
    input: &mut CssParser<'_>,
    allow_or: bool,
) -> Result<Condition, ParseError<ParseIssue>> {
    if input
        .try_parse(|input| input.expect_ident_matching("not"))
        .is_ok()
    {
        let condition = parse_in_parens(input)?;
        input.expect_exhausted()?;
        return Ok(Condition::Not(Box::new(condition)));
    }
    let first = parse_in_parens(input)?;
    if input.is_exhausted() {
        return Ok(first);
    }
    let operator = input.expect_ident_cloned()?.to_ascii_lowercase();
    if operator != "and" && !(allow_or && operator == "or") {
        return Err(invalid());
    }
    let mut conditions = vec![first, parse_in_parens(input)?];
    while !input.is_exhausted() {
        input.expect_ident_matching(&operator)?;
        conditions.push(parse_in_parens(input)?);
    }
    Ok(if operator == "and" {
        Condition::And(conditions)
    } else {
        Condition::Or(conditions)
    })
}

fn parse_in_parens(input: &mut CssParser<'_>) -> Result<Condition, ParseError<ParseIssue>> {
    match input.next()?.clone() {
        Token::ParenthesisBlock => input.parse_nested_block(|input| {
            let start = input.state();
            let grouped = match input.next()? {
                Token::ParenthesisBlock | Token::Function(_) => true,
                Token::Ident(name) => name.eq_ignore_ascii_case("not"),
                _ => false,
            };
            input.reset(&start);
            if grouped {
                parse_condition(input, true)
            } else {
                parse_feature(input)
            }
        }),
        Token::Function(_) => input.parse_nested_block(|input| {
            while input.next().is_ok() {}
            Ok(Condition::Constant(false))
        }),
        _ => Err(invalid()),
    }
}

fn parse_feature(input: &mut CssParser<'_>) -> Result<Condition, ParseError<ParseIssue>> {
    if let Ok(name) = input.try_parse(|input| input.expect_ident_cloned()) {
        let name = name.to_ascii_lowercase();
        let (name, comparison) = if let Some(name) = name.strip_prefix("min-") {
            (name, Comparison::GreaterEqual)
        } else if let Some(name) = name.strip_prefix("max-") {
            (name, Comparison::LessEqual)
        } else {
            (name.as_str(), Comparison::Equal)
        };
        if name == "orientation" && matches!(comparison, Comparison::Equal) {
            if input.is_exhausted() {
                return Ok(Condition::Constant(true));
            }
            input.expect_colon()?;
            let orientation = input.expect_ident_cloned()?;
            input.expect_exhausted()?;
            return match orientation.to_ascii_lowercase().as_str() {
                "portrait" => Ok(Condition::Portrait(true)),
                "landscape" => Ok(Condition::Portrait(false)),
                _ => Err(invalid()),
            };
        }
        let Some(feature) = Feature::parse(name) else {
            while input.next().is_ok() {}
            return Ok(Condition::Constant(false));
        };
        if input.is_exhausted() && matches!(comparison, Comparison::Equal) {
            return Ok(Condition::Compare(feature, Comparison::Greater, 0.0));
        }
        if input.try_parse(|input| input.expect_colon()).is_ok() {
            let value = parse_value(input, feature)?;
            input.expect_exhausted()?;
            return Ok(Condition::Compare(feature, comparison, value));
        }
        if !matches!(comparison, Comparison::Equal) {
            return Err(invalid());
        }
        let comparison = parse_comparison(input)?;
        let value = parse_value(input, feature)?;
        input.expect_exhausted()?;
        return Ok(Condition::Compare(feature, comparison, value));
    }
    let value_start = input.state();
    let (_, length) = parse_number_or_length(input)?;
    if !length && input.try_parse(|input| input.expect_delim('/')).is_ok() {
        input.expect_number()?;
    }
    parse_comparison(input)?;
    let name = input.expect_ident_cloned()?.to_ascii_lowercase();
    let Some(feature) = Feature::parse(&name) else {
        while input.next().is_ok() {}
        return Ok(Condition::Constant(false));
    };
    input.reset(&value_start);
    let value = parse_value(input, feature)?;
    let comparison = parse_comparison(input)?;
    input.expect_ident_matching(&name)?;
    let first = Condition::Compare(feature, comparison.reversed(), value);
    if input.is_exhausted() {
        return Ok(first);
    }
    let second_comparison = parse_comparison(input)?;
    if !matches!(
        (comparison, second_comparison),
        (
            Comparison::Less | Comparison::LessEqual,
            Comparison::Less | Comparison::LessEqual
        ) | (
            Comparison::Greater | Comparison::GreaterEqual,
            Comparison::Greater | Comparison::GreaterEqual
        )
    ) {
        return Err(invalid());
    }
    let value = parse_value(input, feature)?;
    input.expect_exhausted()?;
    Ok(Condition::And(vec![
        first,
        Condition::Compare(feature, second_comparison, value),
    ]))
}

fn parse_comparison(input: &mut CssParser<'_>) -> Result<Comparison, ParseError<ParseIssue>> {
    match input.next()? {
        Token::Delim('=') => Ok(Comparison::Equal),
        Token::Delim('<') => Ok(
            if input.try_parse(|input| input.expect_delim('=')).is_ok() {
                Comparison::LessEqual
            } else {
                Comparison::Less
            },
        ),
        Token::Delim('>') => Ok(
            if input.try_parse(|input| input.expect_delim('=')).is_ok() {
                Comparison::GreaterEqual
            } else {
                Comparison::Greater
            },
        ),
        _ => Err(invalid()),
    }
}

fn parse_value(input: &mut CssParser<'_>, feature: Feature) -> Result<f32, ParseError<ParseIssue>> {
    if matches!(feature, Feature::AspectRatio) {
        let numerator = input.expect_number()?;
        let denominator = if input.try_parse(|input| input.expect_delim('/')).is_ok() {
            input.expect_number()?
        } else {
            1.0
        };
        if numerator < 0.0 || denominator <= 0.0 {
            return Err(invalid());
        }
        return Ok(numerator / denominator);
    }
    let (value, length) = parse_number_or_length(input)?;
    if !length && value != 0.0 {
        return Err(invalid());
    }
    Ok(value)
}

fn parse_number_or_length(
    input: &mut CssParser<'_>,
) -> Result<(f32, bool), ParseError<ParseIssue>> {
    let (value, length) = match input.next()? {
        Token::Number { value, .. } => (*value, false),
        Token::Dimension { value, unit, .. } => {
            let scale = match unit.to_ascii_lowercase().as_str() {
                "px" => 1.0,
                "em" | "rem" => 16.0,
                "in" => 96.0,
                "cm" => 96.0 / 2.54,
                "mm" => 96.0 / 25.4,
                "q" => 96.0 / 101.6,
                "pt" => 96.0 / 72.0,
                "pc" => 16.0,
                _ => return Err(invalid()),
            };
            (*value * scale, true)
        }
        _ => return Err(invalid()),
    };
    if !value.is_finite() {
        return Err(invalid());
    }
    Ok((value, length))
}
