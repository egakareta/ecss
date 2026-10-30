use super::{
    CssParser, ParseError, ParseErrorKind, ParseIssue, Token, parse_declarations, parse_selectors,
    strip_important,
};

pub(super) fn parse(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    let start = input.state();
    general_tokens(input, 0)?;
    input.reset(&start);
    let condition = parse_condition(input).map_err(|error| with_source_line(error, input))?;
    if input.expect_exhausted().is_err() {
        return Err(invalid_at(input));
    }
    Ok(condition)
}

fn invalid_at_line(line: u32) -> ParseError<ParseIssue> {
    let mut issue = ParseIssue::new("invalid supports condition");
    issue.line = Some(line);
    ParseError::custom(issue)
}

fn invalid_at(input: &CssParser<'_>) -> ParseError<ParseIssue> {
    invalid_at_line(input.current_source_location().line)
}

fn with_source_line(
    error: ParseError<ParseIssue>,
    input: &CssParser<'_>,
) -> ParseError<ParseIssue> {
    match error.kind {
        ParseErrorKind::Custom(mut issue) => {
            issue
                .line
                .get_or_insert(input.current_source_location().line);
            ParseError::custom(issue)
        }
        ParseErrorKind::Basic(kind) => ParseError {
            kind: ParseErrorKind::Basic(kind),
        },
    }
}

fn parse_condition(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    if input
        .try_parse(|input| input.expect_ident_matching("not"))
        .is_ok()
    {
        let condition = parse_in_parens(input)?;
        if input.expect_exhausted().is_err() {
            return Err(invalid_at(input));
        }
        return Ok(!condition);
    }

    let first = parse_in_parens(input)?;
    if input.is_exhausted() {
        return Ok(first);
    }

    let operator = match input.expect_ident_cloned() {
        Ok(operator) => operator,
        Err(_) => return Err(invalid_at(input)),
    };
    let operator = operator.to_ascii_lowercase();
    if operator != "and" && operator != "or" {
        return Err(invalid_at(input));
    }
    let mut conditions = vec![first, parse_in_parens(input)?];
    while !input.is_exhausted() {
        if input.expect_ident_matching(&operator).is_err() {
            return Err(invalid_at(input));
        }
        conditions.push(parse_in_parens(input)?);
    }
    Ok(if operator == "and" {
        conditions.into_iter().all(|condition| condition)
    } else {
        conditions.into_iter().any(|condition| condition)
    })
}

fn parse_in_parens(input: &mut CssParser<'_>) -> Result<bool, ParseError<ParseIssue>> {
    let token = match input.next() {
        Ok(token) => token.clone(),
        Err(_) => return Err(invalid_at(input)),
    };
    match token {
        Token::ParenthesisBlock => input.parse_nested_block(|input| {
            if let Ok(condition) = input.try_parse(parse_condition) {
                return Ok(condition);
            }
            if let Ok(condition) = input.try_parse(parse_declaration) {
                return Ok(condition);
            }
            general_tokens(input, 0)?;
            Ok(false)
        }),
        Token::Function(name) => parse_function(input, name.as_ref()),
        _ => Err(invalid_at(input)),
    }
}

fn general_tokens(input: &mut CssParser<'_>, depth: usize) -> Result<(), ParseError<ParseIssue>> {
    if depth >= 128 {
        return Err(invalid_at(input));
    }
    while !input.is_exhausted() {
        let token = match input.next() {
            Ok(token) => token.clone(),
            Err(_) => return Err(invalid_at(input)),
        };
        if token.is_parse_error() {
            return Err(invalid_at(input));
        }
        if matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        ) {
            input.parse_nested_block(|input| general_tokens(input, depth + 1))?;
        }
    }
    Ok(())
}

fn parse_function(input: &mut CssParser<'_>, name: &str) -> Result<bool, ParseError<ParseIssue>> {
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
    let declaration_line = input.current_source_location().line;
    let mut contains_bang = false;
    while let Ok(token) = input.next().cloned() {
        match token {
            Token::Semicolon => return Err(invalid_at(input)),
            Token::Delim('!') => contains_bang = true,
            Token::Function(_)
            | Token::ParenthesisBlock
            | Token::SquareBracketBlock
            | Token::CurlyBracketBlock => {
                let line = input.current_source_location().line;
                input
                    .parse_nested_block(|input| {
                        while input.next().is_ok() {}
                        Ok::<_, ParseError<()>>(())
                    })
                    .map_err(|_| invalid_at_line(line))?;
            }
            _ => {}
        }
    }
    let source = input.slice_from(start).trim();
    if contains_bang && strip_important(source).is_none() {
        return Ok(false);
    }
    let mut syntax = CssParser::new(source);
    let syntax_line =
        declaration_line.saturating_add(syntax.current_source_location().line.saturating_sub(1));
    let name = syntax
        .expect_ident_cloned()
        .map_err(|_| invalid_at_line(syntax_line))?;
    let syntax_line =
        declaration_line.saturating_add(syntax.current_source_location().line.saturating_sub(1));
    syntax
        .expect_colon()
        .map_err(|_| invalid_at_line(syntax_line))?;
    let name = name.as_ref();
    if !property_supported(name) || !valid_var_functions(source) {
        return Ok(false);
    }
    let mut declarations = CssParser::new(source);
    Ok(parse_declarations(&mut declarations, true)
        .map(|declarations| declarations.len() == 1)
        .unwrap_or(false))
}

fn property_supported(name: &str) -> bool {
    if name.starts_with("--") {
        return name != "--";
    }
    let source = format!("{name}: initial");
    let mut input = CssParser::new(&source);
    parse_declarations(&mut input, true)
        .map(|declarations| declarations.len() == 1)
        .unwrap_or(false)
}

fn valid_var_functions(source: &str) -> bool {
    let mut input = CssParser::new(source);
    valid_tokens(&mut input)
}

fn valid_tokens(input: &mut CssParser<'_>) -> bool {
    while let Ok(token) = input.next().cloned() {
        let valid = match token {
            Token::Function(name) => input
                .parse_nested_block(|input| {
                    let valid = if name.eq_ignore_ascii_case("var") {
                        valid_var_contents(input)
                    } else {
                        valid_tokens(input)
                    };
                    Ok::<_, ParseError<()>>(valid)
                })
                .ok()
                .unwrap_or(false),
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => input
                .parse_nested_block(|input| Ok::<_, ParseError<()>>(valid_tokens(input)))
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

fn valid_var_contents(input: &mut CssParser<'_>) -> bool {
    let Ok(name) = input.expect_ident_cloned() else {
        return false;
    };
    if !name.starts_with("--") || name.as_ref() == "--" {
        return false;
    }
    if input.is_exhausted() {
        return true;
    }
    if input.try_parse(|input| input.expect_comma()).is_err() {
        return false;
    }
    valid_tokens(input)
}
