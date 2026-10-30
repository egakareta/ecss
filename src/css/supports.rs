use super::{
    CssParser, ParseError, ParseIssue, Token, parse_declarations, parse_selectors, strip_important,
    variables,
};

pub(super) fn parse(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    let condition = parse_condition(input)?;
    input.expect_exhausted().map_err(|_| invalid())?;
    Ok(condition)
}

fn invalid() -> ParseError<ParseIssue> {
    ParseError::custom(ParseIssue::new("invalid supports condition"))
}

fn parse_condition(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    if input
        .try_parse(|input| input.expect_ident_matching("not"))
        .is_ok()
    {
        let condition = parse_in_parens(input)?;
        input.expect_exhausted().map_err(|_| invalid())?;
        return Ok(!condition);
    }

    let first = parse_in_parens(input)?;
    if input.is_exhausted() {
        return Ok(first);
    }

    let operator = input.expect_ident_cloned().map_err(|_| invalid())?;
    let operator = operator.to_ascii_lowercase();
    if operator != "and" && operator != "or" {
        return Err(invalid());
    }
    let mut conditions = vec![first, parse_in_parens(input)?];
    while !input.is_exhausted() {
        input
            .expect_ident_matching(&operator)
            .map_err(|_| invalid())?;
        conditions.push(parse_in_parens(input)?);
    }
    Ok(if operator == "and" {
        conditions.into_iter().all(|condition| condition)
    } else {
        conditions.into_iter().any(|condition| condition)
    })
}

fn parse_in_parens(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    match input.next().map_err(|_| invalid())?.clone() {
        Token::ParenthesisBlock => input.parse_nested_block(|input| {
            let start = input.state();
            let first = input.next().map_err(|_| invalid())?.clone();
            input.reset(&start);
            let grouped = match &first {
                Token::ParenthesisBlock => true,
                Token::Function(_) => true,
                Token::Ident(name) if name.eq_ignore_ascii_case("not") => {
                    input.expect_ident_matching("not").map_err(|_| invalid())?;
                    let declaration = input.try_parse(|input| input.expect_colon()).is_ok();
                    input.reset(&start);
                    !declaration
                }
                _ => false,
            };
            let condition = if grouped {
                parse_condition(input)?
            } else {
                parse_declaration(input)?
            };
            input.expect_exhausted().map_err(|_| invalid())?;
            Ok(condition)
        }),
        Token::Function(name) => parse_function(input, name.as_ref()),
        _ => Err(invalid()),
    }
}

fn parse_function(input: &mut CssParser<'_>, name: &str) -> Result<bool, ParseError<ParseIssue>> {
    if matches!(name.to_ascii_lowercase().as_str(), "not" | "and" | "or") {
        return Err(invalid());
    }
    if name.eq_ignore_ascii_case("selector") {
        return input.parse_nested_block(|selector| {
            let start = selector.position();
            while selector.next().is_ok() {}
            let source = selector.slice_from(start).trim();
            let mut parser = CssParser::new(source);
            let supported = parse_selectors(&mut parser)
                .ok()
                .is_some_and(|selectors| selectors.len() == 1 && parser.is_exhausted());
            Ok(supported)
        });
    }
    input.parse_nested_block(|input| {
        while input.next().is_ok() {}
        Ok(false)
    })
}

fn parse_declaration(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    let start = input.position();
    let mut contains_bang = false;
    while let Ok(token) = input.next().cloned() {
        match token {
            Token::Semicolon => return Err(invalid()),
            Token::Delim('!') => contains_bang = true,
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                input
                    .parse_nested_block(|input| {
                        while input.next().is_ok() {}
                        Ok::<_, ParseError<()>>(())
                    })
                    .map_err(|_| invalid())?;
            }
            _ => {}
        }
    }
    let source = input.slice_from(start).trim();
    if contains_bang && strip_important(source).is_none() {
        return Ok(false);
    }
    let mut syntax = CssParser::new(source);
    let name = syntax.expect_ident_cloned().map_err(|_| invalid())?;
    syntax.expect_colon().map_err(|_| invalid())?;
    let name = name.as_ref();
    let contains_var = variables::contains_var(source);
    if !property_supported(name)
        || !valid_var_functions(source)
        || (contains_var && !name.starts_with("--") && !valid_supported_functions(source))
    {
        return Ok(false);
    }
    let mut declarations = CssParser::new(source);
    Ok(parse_declarations(&mut declarations, true)
        .map(|declarations| declarations.len() == 1)
        .unwrap_or(false))
}

fn property_supported(name: &str) -> bool {
    if name.starts_with("--") {
        return true;
    }
    let source = format!("{name}: initial");
    let mut input = CssParser::new(&source);
    parse_declarations(&mut input, true)
        .map(|declarations| declarations.len() == 1)
        .unwrap_or(false)
}

fn valid_var_functions(source: &str) -> bool {
    let mut input = CssParser::new(source);
    valid_tokens(&mut input, false)
}

fn valid_supported_functions(source: &str) -> bool {
    let mut input = CssParser::new(source);
    valid_tokens(&mut input, true)
}

fn valid_tokens(input: &mut CssParser<'_>, reject_unknown: bool) -> bool {
    while let Ok(token) = input.next().cloned() {
        let valid = match token {
            Token::Function(name) => {
                if reject_unknown && !supported_function(name.as_ref()) {
                    return false;
                }
                input
                    .parse_nested_block(|input| {
                        let valid = if name.eq_ignore_ascii_case("var") {
                            valid_var_contents(input, reject_unknown)
                        } else {
                            valid_tokens(input, reject_unknown)
                        };
                        Ok::<_, ParseError<()>>(valid)
                    })
                    .ok()
                    .unwrap_or(false)
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => input
                .parse_nested_block(|input| {
                    Ok::<_, ParseError<()>>(valid_tokens(input, reject_unknown))
                })
                .ok()
                .unwrap_or(false),
            _ => true,
        };
        if !valid {
            return false;
        }
    }
    true
}

fn valid_var_contents(input: &mut CssParser<'_>, reject_unknown: bool) -> bool {
    let Ok(name) = input.expect_ident_cloned() else {
        return false;
    };
    if !name.starts_with("--") {
        return false;
    }
    if input.is_exhausted() {
        return true;
    }
    if input.try_parse(|input| input.expect_comma()).is_err() {
        return false;
    }
    valid_tokens(input, reject_unknown)
}

fn supported_function(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "var"
            | "calc"
            | "min"
            | "max"
            | "clamp"
            | "round"
            | "mod"
            | "rem"
            | "hypot"
            | "sign"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "sqrt"
            | "exp"
            | "pow"
            | "log"
            | "fit-content"
            | "stripes"
            | "superellipse"
            | "url"
            | "element"
            | "linear-gradient"
            | "repeating-linear-gradient"
            | "radial-gradient"
            | "repeating-radial-gradient"
            | "conic-gradient"
            | "repeating-conic-gradient"
            | "cross-fade"
            | "image"
            | "image-set"
            | "type"
            | "rgb"
            | "rgba"
            | "hsl"
            | "hsla"
            | "steps"
            | "cubic-bezier"
            | "linear"
    )
}
