//! Group lexer boundaries and deferred singleton values in ST4 dictionaries.
use super::*;

pub(super) fn contains(ranges: &[(usize, usize)], position: usize) -> bool {
    ranges
        .iter()
        .any(|(start, end)| position >= *start && position < *end)
}

fn next(input: &str, position: usize) -> usize {
    position + input[position..].chars().next().unwrap().len_utf8()
}
pub(super) fn comment_end(input: &str, position: usize) -> Option<usize> {
    if input[position..].starts_with("/*") {
        Some(
            input[position + 2..]
                .find("*/")
                .map(|p| position + 2 + p + 2)
                .unwrap_or(input.len()),
        )
    } else if input[position..].starts_with("//") {
        Some(
            input[position..]
                .find('\n')
                .map(|p| position + p + 1)
                .unwrap_or(input.len()),
        )
    } else {
        None
    }
}
pub(super) fn whitespace(input: &str, mut position: usize) -> usize {
    while position < input.len() {
        if let Some(end) = comment_end(input, position) {
            position = end;
        } else if input[position..].chars().next().unwrap().is_whitespace() {
            position = next(input, position);
        } else {
            break;
        }
    }
    position
}

pub(super) fn value_end(input: &str, start: usize) -> Result<Option<usize>, String> {
    value_end_with(input, start, delimiters::Delimiters::default())
}

pub(super) fn value_end_with(
    input: &str,
    start: usize,
    delimiters: delimiters::Delimiters,
) -> Result<Option<usize>, String> {
    let rest = &input[start..];
    if rest.starts_with('"') {
        let mut p = start + 1;
        while p < input.len() {
            if input.as_bytes()[p] == b'\\' {
                p = next(input, p);
                if p < input.len() {
                    p = next(input, p);
                }
            } else if input.as_bytes()[p] == b'"' {
                return Ok(Some(p + 1));
            } else {
                p = next(input, p);
            }
        }
    } else if rest.starts_with("<%") {
        return input[start + 2..]
            .find("%>")
            .map(|p| Some(start + 2 + p + 2))
            .ok_or("unterminated no-newline group template".into());
    } else if rest.starts_with("<<") {
        let mut p = start + 2;
        while p < input.len() {
            if input[p..].starts_with(">>") {
                return Ok(Some(p + 2));
            }
            if input.as_bytes()[p] == b'\\' {
                p = next(input, p);
                if p < input.len() {
                    p = next(input, p);
                }
            } else {
                p = next(input, p);
            }
        }
    } else if rest.starts_with('{') {
        let units = input.encode_utf16().collect::<Vec<_>>();
        let begin = input[..start].encode_utf16().count();
        // Group.g's ANONYMOUS_TEMPLATE returns the token through EOF after
        // reporting a missing brace; it does not abort the group lexer.
        let end = utf16::anonymous_end(&units, begin, delimiters).unwrap_or(units.len());
        return utf16::byte_index(input, end)
            .map(Some)
            .ok_or("group value ended inside a surrogate pair".into());
    } else if rest.starts_with('[') {
        let mut p = start + 1;
        while p < input.len() {
            if input.as_bytes()[p] == b']' {
                return Ok(Some(p + 1));
            }
            if let Some(end) = comment_end(input, p) {
                p = end;
            } else if let Some(end) = value_end_with(input, p, delimiters)? {
                if end == input.len() && input[p..].starts_with('{') {
                    let units = input.encode_utf16().collect::<Vec<_>>();
                    let begin = input[..p].encode_utf16().count();
                    if utf16::anonymous_end(&units, begin, delimiters).is_err() {
                        // This anonymous token consumed the remaining group,
                        // including any apparent dictionary closing bracket.
                        return Ok(Some(end));
                    }
                }
                p = end;
            } else {
                p = next(input, p);
            }
        }
    } else {
        return Ok(None);
    }
    Err(format!("unterminated group value at byte {start}"))
}

pub(super) fn group_values(input: &str) -> Result<Vec<(usize, usize)>, String> {
    let mut result = Vec::new();
    let mut p = 0;
    while p < input.len() {
        if let Some(end) = comment_end(input, p) {
            result.push((p, end));
            p = end;
        } else if let Some(end) = value_end(input, p)? {
            result.push((p, end));
            p = end;
        } else {
            p = next(input, p);
        }
    }
    Ok(result)
}

pub(super) fn entries(
    input: &str,
    delimiters: delimiters::Delimiters,
) -> Result<Vec<(&str, &str)>, String> {
    let mut result = Vec::new();
    let mut p = whitespace(input, 0);
    while p < input.len() {
        let start = p;
        p = if let Some(end) = value_end_with(input, p, delimiters)? {
            end
        } else {
            let mut end = p;
            while end < input.len()
                && (input.as_bytes()[end].is_ascii_alphanumeric()
                    || matches!(input.as_bytes()[end], b'_' | b'-'))
            {
                end += 1;
            }
            if end == p {
                return Err("missing dictionary key".into());
            }
            end
        };
        let key = input[start..p].trim();
        p = whitespace(input, p);
        if input.as_bytes().get(p) != Some(&b':') {
            return Err("missing dictionary colon".into());
        }
        p = whitespace(input, p + 1);
        let start = p;
        p = if let Some(end) = value_end_with(input, p, delimiters)? {
            end
        } else {
            let mut end = p;
            while end < input.len()
                && !input[end..].starts_with(',')
                && comment_end(input, end).is_none()
                && !input[end..].chars().next().unwrap().is_whitespace()
            {
                end = next(input, end);
            }
            end
        };
        result.push((key, input[start..p].trim()));
        p = whitespace(input, p);
        if p < input.len() {
            if input.as_bytes()[p] != b',' {
                return Err("missing dictionary comma".into());
            }
            p = whitespace(input, p + 1);
        }
    }
    Ok(result)
}

pub(super) fn right_angles(input: &str) -> String {
    let mut output = String::new();
    let mut p = 0;
    while p < input.len() {
        if input[p..].starts_with("<\\\\>") {
            output.push_str("<\\\\>");
            p += "<\\\\>".len();
        } else if input[p..].starts_with(">\\>") {
            output.push_str(">>");
            p += 3;
        } else if input[p..].starts_with("\\>") && !input[p..].starts_with("\\>>>") {
            output.push('>');
            p += 2;
        } else {
            let ch = input[p..].chars().next().unwrap();
            output.push(ch);
            p += ch.len_utf8();
        }
    }
    output
}

pub(super) fn literal(input: &str) -> Result<Value, String> {
    let (body, no_newlines) = if input.starts_with("<<") && input.ends_with(">>") {
        (right_angles(&input[2..input.len() - 2]), false)
    } else if input.starts_with("<%") && input.ends_with("%>") {
        (input[2..input.len() - 2].replace("%\\>", "%>"), true)
    } else if input.starts_with('{') && input.ends_with('}') {
        (input[1..input.len() - 1].to_owned(), false)
    } else {
        return Ok(properties::dictionary_literal(input));
    };
    Ok(
        serde_json::json!({"__stg_inline_template":body,"bindings":{},"__stg_singleton":true,"__stg_no_newlines":no_newlines}),
    )
}

pub(super) fn tokens(
    body: &str,
    no_newlines: bool,
    initial_line_start: bool,
    delimiters: delimiters::Delimiters,
) -> Result<Vec<Token>, String> {
    let tokens = tokenize_at(body, true, initial_line_start, delimiters)?;
    if !no_newlines {
        return Ok(tokens);
    }
    let mut result = Vec::new();
    let mut start = initial_line_start;
    for token in tokens {
        match token {
            Token::Newline => {
                start = true;
            }
            Token::IndentedText { text, .. } => {
                result.push(Token::Text(text));
                start = false;
            }
            Token::Tag { expr, .. } => {
                result.push(Token::Tag {
                    expr,
                    line_indent: None,
                    inline_branch_indent: None,
                });
                start = false;
            }
            Token::Text(text) => {
                let text = if start {
                    text.trim_start_indent()
                } else {
                    text
                };
                if !text.is_empty() {
                    result.push(Token::Text(text));
                    start = false;
                }
            }
        }
    }
    Ok(result)
}
