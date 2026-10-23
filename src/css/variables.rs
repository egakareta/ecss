use std::collections::{HashMap, HashSet};

use cssparser::{Parser, Token};

use super::Declaration;

pub(crate) type CustomProperties = HashMap<String, String>;

pub(crate) fn contains_var(value: &str) -> bool {
    find_var(value, 0).is_some()
}

pub(crate) fn compute(
    parent: &CustomProperties,
    declarations: &[&Declaration],
) -> CustomProperties {
    let mut values = parent.clone();
    let mut priorities = HashMap::new();

    for declaration in declarations {
        let Declaration::CustomProperty {
            name,
            value,
            important,
        } = declaration
        else {
            continue;
        };
        if priorities
            .get(name)
            .is_some_and(|previous_important| *previous_important && !important)
        {
            continue;
        }
        priorities.insert(name.clone(), *important);
        match value.trim().to_ascii_lowercase().as_str() {
            "inherit" | "unset" => {
                if let Some(inherited) = parent.get(name) {
                    values.insert(name.clone(), inherited.clone());
                } else {
                    values.remove(name);
                }
            }
            "initial" | "revert" | "revert-layer" => {
                values.remove(name);
            }
            _ => {
                values.insert(name.clone(), value.clone());
            }
        }
    }

    let cyclic = cyclic_properties(&values);
    let mut resolver = Resolver {
        values: &values,
        cyclic,
        cache: HashMap::new(),
        visiting: HashSet::new(),
    };
    let names = values.keys().cloned().collect::<Vec<_>>();
    names
        .into_iter()
        .filter_map(|name| resolver.resolve(&name).map(|value| (name, value)))
        .collect()
}

pub(crate) fn substitute(value: &str, properties: &CustomProperties) -> Option<String> {
    substitute_inner(value, 0, &mut |name| properties.get(name).cloned())
}

fn cyclic_properties(values: &CustomProperties) -> HashSet<String> {
    let dependencies = values
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                references(value)
                    .into_iter()
                    .filter(|dependency| values.contains_key(dependency))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut cyclic = HashSet::new();

    for name in values.keys() {
        let mut visited = HashSet::new();
        let mut pending = dependencies.get(name).cloned().unwrap_or_default();
        while let Some(dependency) = pending.pop() {
            if dependency == *name {
                cyclic.insert(name.clone());
                break;
            }
            if visited.insert(dependency.clone()) {
                pending.extend(dependencies.get(&dependency).into_iter().flatten().cloned());
            }
        }
    }

    cyclic
}

fn references(value: &str) -> Vec<String> {
    let mut references = Vec::new();
    let mut cursor = 0;
    while let Some(call) = find_var(value, cursor) {
        if let Some(name) = call.name.as_deref() {
            references.push(name.to_owned());
        }
        cursor = call.start + 1;
    }
    references
}

struct Resolver<'a> {
    values: &'a CustomProperties,
    cyclic: HashSet<String>,
    cache: HashMap<String, Option<String>>,
    visiting: HashSet<String>,
}

impl Resolver<'_> {
    fn resolve(&mut self, name: &str) -> Option<String> {
        if let Some(value) = self.cache.get(name) {
            return value.clone();
        }
        let raw = self.values.get(name)?.clone();
        if self.cyclic.contains(name) || !self.visiting.insert(name.to_owned()) {
            return None;
        }
        let resolved = substitute_inner(&raw, 0, &mut |dependency| self.resolve(dependency));
        self.visiting.remove(name);
        self.cache.insert(name.to_owned(), resolved.clone());
        resolved
    }
}

struct VarCall<'a> {
    start: usize,
    end: usize,
    name: Option<String>,
    fallback: Option<&'a str>,
}

fn find_var(source: &str, from: usize) -> Option<VarCall<'_>> {
    let bytes = source.as_bytes();
    let mut cursor = from;
    while cursor + 4 <= bytes.len() {
        if bytes[cursor..].starts_with(b"/*") {
            cursor = comment_end(bytes, cursor).unwrap_or(bytes.len());
            continue;
        }
        match bytes[cursor] {
            b'\'' | b'"' => {
                cursor = string_end(bytes, cursor).unwrap_or(bytes.len());
                continue;
            }
            _ => {}
        }
        if matches!(bytes[cursor], b'v' | b'V' | b'\\')
            && (cursor == 0 || !is_name_byte(bytes[cursor - 1]))
        {
            let mut parser = Parser::new(&source[cursor..]);
            if let Ok(Token::Function(name)) = parser.next()
                && name.eq_ignore_ascii_case("var")
            {
                let open = cursor + parser.position().byte_index() - 1;
                let close = matching_close(source, open)?;
                let contents = &source[open + 1..close];
                let comma = top_level_comma(contents);
                let name_end = comma.unwrap_or(contents.len());
                let name = parse_name(contents[..name_end].trim());
                let fallback = comma.map(|comma| contents[comma + 1..].trim());
                return Some(VarCall {
                    start: cursor,
                    end: close + 1,
                    name,
                    fallback,
                });
            }
        }
        cursor = next_char(source, cursor);
    }
    None
}

fn parse_name(source: &str) -> Option<String> {
    let mut parser = Parser::new(source);
    let name = parser.expect_ident().ok()?.to_string();
    parser.expect_exhausted().ok()?;
    name.starts_with("--").then_some(name)
}

fn substitute_inner(
    source: &str,
    depth: usize,
    resolve: &mut impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    if depth > 64 {
        return None;
    }
    let mut output = String::new();
    let mut copied_until = 0;
    let mut cursor = 0;
    while let Some(call) = find_var(source, cursor) {
        let prefix = &source[copied_until..call.start];
        output.push_str(prefix);
        let name = call.name.as_deref()?;
        let replacement = resolve(name).or_else(|| {
            call.fallback
                .and_then(|fallback| substitute_inner(fallback, depth + 1, resolve))
        })?;
        let left = output.chars().next_back();
        let first = replacement.chars().next();
        if needs_separator(left, first) {
            output.push(' ');
        }
        output.push_str(&replacement);
        let last = replacement.chars().next_back();
        let right = source[call.end..].chars().next();
        if needs_separator(last, right) {
            output.push(' ');
        }
        copied_until = call.end;
        cursor = call.end;
    }
    output.push_str(&source[copied_until..]);
    Some(output)
}

fn top_level_comma(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut stack = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor..].starts_with(b"/*") {
            cursor = comment_end(bytes, cursor).unwrap_or(bytes.len());
            continue;
        }
        match bytes[cursor] {
            b'\\' => {
                cursor = skip_escape(source, cursor);
                continue;
            }
            b'\'' | b'"' => {
                cursor = string_end(bytes, cursor).unwrap_or(bytes.len());
                continue;
            }
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b'{' => stack.push(b'}'),
            b',' if stack.is_empty() => return Some(cursor),
            close if matches!(close, b')' | b']' | b'}') && stack.last() == Some(&close) => {
                stack.pop();
            }
            _ => {}
        }
        cursor = next_char(source, cursor);
    }
    None
}

fn matching_close(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut stack = vec![b')'];
    let mut cursor = open + 1;
    while cursor < bytes.len() {
        if bytes[cursor..].starts_with(b"/*") {
            cursor = comment_end(bytes, cursor)?;
            continue;
        }
        match bytes[cursor] {
            b'\\' => {
                cursor = skip_escape(source, cursor);
                continue;
            }
            b'\'' | b'"' => {
                cursor = string_end(bytes, cursor)?;
                continue;
            }
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b'{' => stack.push(b'}'),
            close if matches!(close, b')' | b']' | b'}') && stack.last() == Some(&close) => {
                stack.pop();
                if stack.is_empty() {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn comment_end(bytes: &[u8], start: usize) -> Option<usize> {
    bytes[start + 2..]
        .windows(2)
        .position(|window| window == b"*/")
        .map(|offset| start + offset + 4)
}

fn string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 1,
            value if value == quote => return Some(cursor + 1),
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'\\') || !byte.is_ascii()
}

fn is_name_char(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(character, '-' | '_' | '\\')
        || !character.is_ascii()
}

fn needs_separator(left: Option<char>, right: Option<char>) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    if left.is_whitespace() || right.is_whitespace() {
        return false;
    }
    (is_name_char(left) && is_name_char(right))
        || is_name_char(left) && right == '('
        || matches!(left, '#' | '@') && is_name_char(right)
        || left == '.' && right.is_ascii_digit()
        || matches!(left, '+' | '-') && (right == '.' || right.is_ascii_digit())
}

fn next_char(source: &str, cursor: usize) -> usize {
    cursor + source[cursor..].chars().next().unwrap().len_utf8()
}

fn skip_escape(source: &str, cursor: usize) -> usize {
    let escaped = cursor + 1;
    if escaped < source.len() {
        next_char(source, escaped)
    } else {
        source.len()
    }
}
