use cssparser::{ParseError, Parser, Token};

#[derive(Clone, Debug)]
pub(crate) enum FontShorthand {
    Values(FontValues),
    Wide(FontWideKeyword),
}

#[derive(Clone, Debug)]
pub(crate) struct FontValues {
    pub(crate) style: bool,
    pub(crate) weight: u16,
    pub(crate) size: String,
    pub(crate) line_height: Option<String>,
    pub(crate) families: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FontWideKeyword {
    Initial,
    Inherit,
    Unset,
    Revert,
}

pub(crate) fn parse(value: &str) -> Result<FontShorthand, ParseError<&'static str>> {
    let trimmed = value.trim();
    if let Some(keyword) = parse_wide_keyword(trimmed) {
        return Ok(FontShorthand::Wide(keyword));
    }
    if trimmed.eq_ignore_ascii_case("revert-layer") {
        return Err(ParseError::custom("cascade layers are unsupported"));
    }

    let mut input = Parser::new(trimmed);
    let mut style = false;
    let mut style_seen = false;
    let mut weight = 400;
    let mut weight_seen = false;
    let size = loop {
        input.skip_whitespace();
        let start = input.position();
        let token = input
            .next()
            .map_err(|_| ParseError::custom("font shorthand requires a font size"))?
            .clone();
        match token {
            Token::Ident(name) => {
                if name.eq_ignore_ascii_case("normal") && !style_seen && !weight_seen {
                    if next_token_is_style(&mut input) {
                        weight_seen = true;
                    } else {
                        style_seen = true;
                    }
                    continue;
                }
                if !style_seen
                    && (name.eq_ignore_ascii_case("italic") || name.eq_ignore_ascii_case("oblique"))
                {
                    style = !name.eq_ignore_ascii_case("normal");
                    style_seen = true;
                    continue;
                }
                if !weight_seen
                    && (name.eq_ignore_ascii_case("normal") || name.eq_ignore_ascii_case("bold"))
                {
                    weight = if name.eq_ignore_ascii_case("bold") {
                        700
                    } else {
                        400
                    };
                    weight_seen = true;
                    continue;
                }
                if is_font_size_keyword(&name) {
                    break input.slice_from(start).trim().to_owned();
                }
                return Err(ParseError::custom(
                    "font shorthand supports only style, weight, size, line-height, and family",
                ));
            }
            Token::Number {
                int_value: Some(value),
                ..
            } if !weight_seen && (1..=1000).contains(&value) => {
                weight = value as u16;
                weight_seen = true;
                continue;
            }
            Token::Dimension { .. }
            | Token::Percentage { .. }
            | Token::Number { .. }
            | Token::Function(_)
            | Token::ParenthesisBlock => {
                consume_nested(&mut input, &token)?;
                break input.slice_from(start).trim().to_owned();
            }
            _ => {
                return Err(ParseError::custom(
                    "font shorthand requires a font size and family",
                ));
            }
        }
    };

    validate_component("font-size", &size)?;

    input.skip_whitespace();
    let line_height = if input.try_parse(|input| input.expect_delim('/')).is_ok() {
        Some(parse_component(
            &mut input,
            "font shorthand requires a line height",
        )?)
    } else {
        None
    };
    if let Some(line_height) = &line_height {
        validate_component("line-height", line_height)?;
    }

    input.skip_whitespace();
    let families = input.parse_comma_separated(|family| {
        let first = family
            .next()
            .map_err(|_| ParseError::custom("unsupported font family"))?
            .clone();
        match first {
            Token::QuotedString(name) if !name.trim().is_empty() => {
                family.expect_exhausted()?;
                Ok(name.to_string())
            }
            Token::Ident(name) => {
                let mut names = vec![name.to_string()];
                while !family.is_exhausted() {
                    match family
                        .next()
                        .map_err(|_| ParseError::custom("unsupported font family"))?
                    {
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
        return Err(ParseError::custom("font shorthand requires a font family"));
    }

    Ok(FontShorthand::Values(FontValues {
        style,
        weight,
        size,
        line_height,
        families,
    }))
}

fn parse_wide_keyword(value: &str) -> Option<FontWideKeyword> {
    Some(match value.to_ascii_lowercase().as_str() {
        "initial" => FontWideKeyword::Initial,
        "inherit" => FontWideKeyword::Inherit,
        "unset" => FontWideKeyword::Unset,
        "revert" => FontWideKeyword::Revert,
        _ => return None,
    })
}

fn is_font_size_keyword(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "xx-small"
            | "x-small"
            | "small"
            | "medium"
            | "large"
            | "x-large"
            | "xx-large"
            | "xxx-large"
            | "smaller"
            | "larger"
    )
}

fn next_token_is_style(input: &mut Parser<'_>) -> bool {
    let state = input.state();
    input.skip_whitespace();
    let is_style = matches!(
        input.next().ok(),
        Some(Token::Ident(name))
            if name.eq_ignore_ascii_case("italic") || name.eq_ignore_ascii_case("oblique")
    );
    input.reset(&state);
    is_style
}

fn parse_component(
    input: &mut Parser<'_>,
    message: &'static str,
) -> Result<String, ParseError<&'static str>> {
    input.skip_whitespace();
    let start = input.position();
    let token = input
        .next()
        .map_err(|_| ParseError::custom(message))?
        .clone();
    consume_nested(input, &token)?;
    Ok(input.slice_from(start).trim().to_owned())
}

fn consume_nested(
    input: &mut Parser<'_>,
    token: &Token<'_>,
) -> Result<(), ParseError<&'static str>> {
    if matches!(
        token,
        Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock
    ) {
        input.parse_nested_block(|nested| {
            while nested.next().is_ok() {}
            Ok::<_, ParseError<&'static str>>(())
        })?;
    }
    Ok(())
}

fn validate_component(name: &str, value: &str) -> Result<(), ParseError<&'static str>> {
    let mut input = Parser::new(value);
    crate::layout_style::parse(name, &mut input)
        .map_err(ParseError::custom)?
        .ok_or_else(|| ParseError::custom("unsupported font shorthand component"))?;
    Ok(())
}
