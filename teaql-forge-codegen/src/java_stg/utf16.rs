//! UTF-16 positions and template scanning following ST4 4.3.3 STLexer.java.
// Copyright (c) 2011 Terence Parr. All rights reserved.
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
// 1. Redistributions of source code must retain the above copyright notice,
//    this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
// 3. The name of the author may not be used to endorse or promote products
//    derived from this software without specific prior written permission.
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR IMPLIED
// WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
// IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
// OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
// OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
// ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
use super::*;

pub(super) fn byte_index(input: &str, units: usize) -> Option<usize> {
    let mut count = 0;
    for (position, ch) in input.char_indices() {
        if count == units {
            return Some(position);
        }
        count += ch.len_utf16();
    }
    (count == units).then_some(input.len())
}
fn find(input: &[u16], pattern: &[u16]) -> Option<usize> {
    input.windows(pattern.len()).position(|s| s == pattern)
}
fn blank(input: &[u16]) -> bool {
    input.iter().all(|c| matches!(c, 9 | 32))
}
fn string(input: &[u16]) -> String {
    Text(input.to_vec()).utf8()
}

pub(super) fn anonymous_end(
    input: &[u16],
    start: usize,
    d: delimiters::Delimiters,
) -> Result<usize, String> {
    let (mut p, mut depth, mut inside, mut quoted) = (start + 1, 1usize, false, false);
    while p < input.len() {
        let c = input[p];
        if c == 92 {
            p = (p + 2).min(input.len());
            continue;
        }
        if !inside && input[p..].starts_with(&[d.start_unit(), 33]) {
            let tail = &input[p + 2..];
            p += 2 + find(tail, &[33, d.stop_unit()]).ok_or("unterminated singleton comment")? + 2;
            continue;
        }
        if inside {
            if c == 34 {
                quoted = !quoted;
            } else if !quoted && c == 123 {
                depth += 1;
                inside = false;
            } else if !quoted && c == d.stop_unit() {
                inside = false;
            }
        } else if c == d.start_unit() {
            inside = true;
        } else if c == 125 {
            depth -= 1;
            if depth == 0 {
                return Ok(p + 1);
            }
            inside = true;
        }
        p += 1;
    }
    Err("unterminated anonymous template".into())
}

fn flush(tokens: &mut Vec<Token>, literal: &mut Vec<u16>, initial: bool) {
    if literal.is_empty() {
        return;
    }
    let value = std::mem::take(literal);
    let mut p = 0;
    let mut line_start = initial;
    while p < value.len() {
        if matches!(value[p], 10 | 13) {
            // STLexer.outside consumes both CR and the following code unit.
            p += if value[p] == 13 && p + 1 < value.len() {
                2
            } else {
                1
            };
            tokens.push(Token::Newline);
            line_start = true;
            continue;
        }
        let start = p;
        while p < value.len() && !matches!(value[p], 10 | 13) {
            p += 1;
        }
        let mut text_start = start;
        if line_start {
            while text_start < p && matches!(value[text_start], 9 | 32) {
                text_start += 1;
            }
        }
        if text_start > start && text_start < p {
            tokens.push(Token::IndentedText {
                indent: string(&value[start..text_start]),
                text: Text(value[text_start..p].to_vec()),
            });
        } else {
            tokens.push(Token::Text(Text(value[start..p].to_vec())));
        }
        line_start = false;
    }
}

pub(super) fn tokenize(
    source: &str,
    initial: bool,
    d: delimiters::Delimiters,
) -> Result<Vec<Token>, String> {
    let all = source.encode_utf16().collect::<Vec<_>>();
    // STLexer stores lookahead in a char: U+FFFF is its EOF sentinel.
    let input = &all[..all.iter().position(|c| *c == 0xffff).unwrap_or(all.len())];
    let (open, close) = (d.start_unit(), d.stop_unit());
    let mut tokens = Vec::new();
    let mut literal = Vec::new();
    let (mut p, mut literal_initial, mut suppressed) = (0, initial, 0);
    let mut conditions = Vec::new();
    while p < input.len() {
        if input[p..].starts_with(&[open, 92, 92]) {
            // LINEBREAK uses match(), which reports but consumes a mismatched
            // closing/newline code unit as well. EOF consumes nothing.
            let mut end = (p + 4).min(input.len());
            while matches!(input.get(end), Some(9 | 32)) {
                end += 1;
            }
            if input.get(end) == Some(&13) {
                end += 1;
            }
            end = (end + 1).min(input.len());
            while matches!(input.get(end), Some(9 | 32)) {
                end += 1;
            }
            p = end;
            suppressed = end;
            continue;
        }
        if input[p] == 92
            && input
                .get(p + 1)
                .is_some_and(|c| *c == open || matches!(c, 92 | 125))
        {
            if literal.is_empty() {
                literal_initial = p == 0 && initial || p > 0 && input[p - 1] == 10;
            }
            literal.push(input[p + 1]);
            p += 2;
            continue;
        }
        if input[p] != open {
            if literal.is_empty() {
                literal_initial = p == 0 && initial || p > 0 && input[p - 1] == 10;
            }
            literal.push(input[p]);
            if input[p] == 13 && p + 1 < input.len() {
                literal.push(input[p + 1]);
                p += 1;
            }
            p += 1;
            continue;
        }
        let tag_start = p;
        let is_comment = input.get(p + 1) == Some(&33);
        if is_comment && find(&input[p + 2..], &[33, close]).is_none() {
            let start = input[..p]
                .iter()
                .rposition(|c| *c == 10)
                .map(|i| i + 1)
                .unwrap_or(0);
            if blank(&input[start..p]) {
                literal.truncate(literal.len().saturating_sub(p - start));
            }
            break;
        }
        let escaped = input.get(p + 1) == Some(&92);
        if d.stop_is_syntax() && !is_comment && !escaped {
            return Err(format!("{COMPILE_ERROR}expression has no RDELIM token"));
        }
        let unicode = input.get(p + 1..p + 3) == Some(&[92, 117][..])
            && input.get(p + 3..p + 7).is_some_and(|digits| {
                digits
                    .iter()
                    .all(|c| matches!(c, 48..=57 | 65..=70 | 97..=102))
            });
        let end = if unicode {
            p + 7
        } else if is_comment {
            p + 3 + find(&input[p + 2..], &[33, close]).unwrap()
        } else {
            let (mut position, mut braces, mut quoted, mut tags) = (p + 1, 0usize, false, 1usize);
            loop {
                let c = *input.get(position).ok_or_else(|| {
                    format!("{COMPILE_ERROR}unterminated STG tag at UTF-16 unit {tag_start}")
                })?;
                let expression = tags > braces;
                if c == 92 && !(expression && !quoted && c == close) {
                    position = (position + 2).min(input.len());
                    continue;
                }
                match c {
                    34 if expression => quoted = !quoted,
                    123 if expression && !quoted => braces += 1,
                    125 if !expression => braces = braces.saturating_sub(1),
                    c if c == open && !expression => tags += 1,
                    c if c == close && expression && !quoted => {
                        if tags == 1 && braces == 0 {
                            break position;
                        }
                        tags = tags.saturating_sub(1);
                    }
                    _ => {}
                }
                position += 1;
            }
        };
        let after_end = (end + 1).min(input.len());
        let raw = &input[p + 1..end];
        let expr = if !escaped && !is_comment {
            syntax::tag_units(raw, d)?
        } else {
            string(raw).trim().to_owned()
        };
        let multiline_end = if expr == "endif" {
            conditions
                .pop()
                .is_some_and(|start| input[start..p].contains(&10))
        } else {
            if expr.starts_with("if(") {
                conditions.push(p);
            }
            false
        };
        let line_start = input[..p]
            .iter()
            .rposition(|c| *c == 10)
            .map(|i| i + 1)
            .unwrap_or(0);
        let before = &input[line_start..p];
        let after = &input[after_end..];
        let line_end = after.iter().position(|c| *c == 10).unwrap_or(after.len());
        let after_line = &after[..line_end];
        let leading = line_start >= suppressed && (line_start > 0 || initial) && blank(before);
        let standalone = leading && blank(after_line) && line_end < after.len();
        let control = expr.starts_with("if(") || expr == "else" || expr == "endif";
        let branch_indent =
            (leading && !standalone && expr.starts_with("if(")).then(|| string(before));
        let indent = (standalone || leading && (!control || branch_indent.is_some()))
            .then(|| string(before));
        if indent.is_some() {
            literal.truncate(literal.len().saturating_sub(before.len()));
        }
        p = if standalone || multiline_end && blank(after_line) && line_end < after.len() {
            after_end + line_end + 1
        } else {
            after_end
        };
        flush(&mut tokens, &mut literal, literal_initial);
        if escaped {
            let text = match raw {
                [92, 110] => Some(Text(vec![10])),
                [92, 116] => Some(Text(vec![9])),
                [92, 32] => Some(Text(vec![32])),
                [92, 117, ..] if unicode => Some(Text(vec![
                    u16::from_str_radix(&string(&raw[2..]), 16).unwrap(),
                ])),
                _ => None,
            };
            if let Some(text) = text {
                tokens.push(if let Some(indent) = indent.filter(|s| !s.is_empty()) {
                    Token::IndentedText { indent, text }
                } else {
                    Token::Text(text)
                });
                if standalone {
                    tokens.push(Token::Newline);
                }
            }
        } else if !is_comment {
            let closing = expr == "endif" || expr == "@end";
            tokens.push(Token::Tag {
                expr,
                line_indent: indent,
                inline_branch_indent: branch_indent,
            });
            if standalone && !closing {
                tokens.push(Token::Newline);
            }
        }
    }
    flush(&mut tokens, &mut literal, literal_initial);
    syntax::structure(&tokens)?;
    Ok(tokens)
}
