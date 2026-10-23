//! SAX parameter declarations and nonvalidating DTD attribute processing.
use super::Failure;
use std::collections::BTreeSet;
mod entities;

/// Delimit the uploaded declaration without interpreting comment/PI contents
/// or brackets in quoted identifiers as DTD structure.
pub(super) fn doctype_span(text: &str, start: usize) -> Option<(usize, Option<std::ops::Range<usize>>)> {
    let mut pos = start + 9;
    let mut quote = None;
    let mut depth = 0_usize;
    let mut begin = None;
    let mut finish = None;
    while pos < text.len() {
        let rest = &text[pos..];
        if quote.is_none() {
            let close = if rest.starts_with("<!--") { Some("-->") } else if rest.starts_with("<?") { Some("?>") } else { None };
            if let Some(close) = close { pos += rest.find(close)? + close.len(); continue; }
        }
        let ch = rest.chars().next()?;
        if let Some(q) = quote { if ch == q { quote = None; } }
        else if matches!(ch, '\'' | '"') { quote = Some(ch); }
        else if ch == '[' { if depth == 0 { begin = Some(pos + 1); } depth += 1; }
        else if ch == ']' && depth > 0 { depth -= 1; if depth == 0 { finish = Some(pos); } }
        else if ch == '>' && depth == 0 {
            return Some((pos + 1, begin.zip(finish).map(|(a,b)| a..b)));
        }
        pos += ch.len_utf8();
    }
    None
}

pub(super) fn subsets(
    internal: &str,
    external: Option<&str>,
    body: &str,
    source: &str,
    internal_start: usize,
    body_start: usize,
) -> Result<(String, String, Vec<(String, usize)>), Failure> {
    entities::subsets(internal, external, body, source, internal_start, body_start)
}

fn space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\n')
}

struct Cursor<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }
    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }
    fn spaces(&mut self) {
        self.pos += self.rest().len() - self.rest().trim_start_matches(space).len();
    }
    fn word(&mut self) -> &'a str {
        self.spaces();
        let end = self
            .rest()
            .find(|ch: char| space(ch) || matches!(ch, '=' | '>' | '/' | '\'' | '"' | '(' | ')'))
            .unwrap_or(self.rest().len());
        let start = self.pos;
        self.pos += end;
        &self.text[start..self.pos]
    }
    fn quoted(&mut self) -> Option<(char, &'a str)> {
        self.spaces();
        let quote = self
            .rest()
            .chars()
            .next()
            .filter(|ch| matches!(ch, '\'' | '"'))?;
        self.pos += 1;
        let start = self.pos;
        self.pos += self.rest().find(quote)?;
        let value = &self.text[start..self.pos];
        self.pos += 1;
        Some((quote, value))
    }
}

fn declaration_end(text: &str) -> Option<usize> {
    let mut quote = None;
    for (pos, ch) in text.char_indices() {
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == '>' {
            return Some(pos + 1);
        }
    }
    None
}

pub(super) fn character_references(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("&#") {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find(';') {
            let number = &rest[2..end];
            let parsed = number
                .strip_prefix('x')
                .map(|s| u32::from_str_radix(s, 16))
                .unwrap_or_else(|| number.parse());
            if let Some(ch) = parsed.ok().and_then(char::from_u32) {
                out.push(ch);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn parameter_reference(text: &str) -> Option<(&str, usize)> {
    let body = text.strip_prefix('%')?;
    let end = body.find(';')?;
    let name = &body[..end];
    (!name.is_empty() && !name.chars().any(space)).then_some((name, end + 2))
}

#[derive(Clone)]
struct Attribute {
    element: String,
    name: String,
    token: bool,
    default: Option<(char, String)>,
}

fn attributes(subset: &str) -> Vec<Attribute> {
    let mut result = Vec::new();
    let mut rest = subset;
    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            rest = rest.find("-->").map(|end| &rest[end + 3..]).unwrap_or("");
            continue;
        }
        if rest.starts_with("<!") {
            let end = declaration_end(rest).unwrap_or(rest.len());
            if let Some(body) = rest[..end].strip_prefix("<!ATTLIST") {
                let mut cursor = Cursor::new(body);
                let element = cursor.word().to_owned();
                loop {
                    let name = cursor.word();
                    if name.is_empty() {
                        break;
                    }
                    cursor.spaces();
                    let kind = if cursor.rest().starts_with('(') {
                        cursor.pos += cursor
                            .rest()
                            .find(')')
                            .map(|n| n + 1)
                            .unwrap_or(cursor.rest().len());
                        "ENUMERATION"
                    } else {
                        cursor.word()
                    };
                    if kind == "NOTATION" {
                        cursor.spaces();
                        cursor.pos += cursor
                            .rest()
                            .find(')')
                            .map(|n| n + 1)
                            .unwrap_or(cursor.rest().len());
                    }
                    cursor.spaces();
                    if cursor.rest().starts_with("#FIXED") {
                        cursor.pos += 6;
                    }
                    let default = cursor.quoted().map(|(quote, value)| (quote, value.into()));
                    if default.is_none() {
                        cursor.word();
                    }
                    if !result
                        .iter()
                        .any(|attr: &Attribute| attr.element == element && attr.name == name)
                    {
                        result.push(Attribute {
                            element: element.clone(),
                            name: name.into(),
                            token: kind != "CDATA",
                            default,
                        });
                    }
                }
            }
            rest = &rest[end..];
        } else {
            let ch = rest.chars().next().unwrap();
            rest = &rest[ch.len_utf8()..];
        }
    }
    result
}

fn normalized(value: &str, quote: char, subset: &str) -> Option<String> {
    let xml = format!("<!DOCTYPE parity [{subset}]><parity value={quote}{value}{quote}/>");
    let doc = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .ok()?;
    let value = doc.root_element().attribute("value")?;
    Some(
        value
            .split(' ')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('\t', "&#9;")
            .replace('\r', "&#13;")
            .replace('\n', "&#10;"),
    )
}

fn apply(text: &str, subset: &str, attributes: &[Attribute]) -> String {
    let mut result = String::new();
    let mut pos = 0;
    while let Some(start) = text[pos..].find('<').map(|n| n + pos) {
        result.push_str(&text[pos..start]);
        let rest = &text[start..];
        let end = if rest.starts_with("<!--") {
            rest.find("-->").map(|n| n + 3)
        } else if rest.starts_with("<![CDATA[") {
            rest.find("]]>").map(|n| n + 3)
        } else if rest.starts_with("<?") {
            rest.find("?>").map(|n| n + 2)
        } else {
            declaration_end(rest)
        }
        .unwrap_or(rest.len());
        let tag = &rest[..end];
        if rest.starts_with("<!") || rest.starts_with("<?") || rest.starts_with("</") {
            result.push_str(tag);
            pos = start + end;
            continue;
        }
        let mut cursor = Cursor::new(&tag[1..]);
        let element = cursor.word();
        let declared = attributes
            .iter()
            .filter(|attr| attr.element == element)
            .collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        let mut edits = Vec::new();
        loop {
            let name = cursor.word();
            if name.is_empty() {
                break;
            }
            seen.insert(name);
            cursor.spaces();
            if !cursor.rest().starts_with('=') {
                break;
            }
            cursor.pos += 1;
            cursor.spaces();
            let value_start = cursor.pos;
            let Some((quote, value)) = cursor.quoted() else {
                break;
            };
            if declared.iter().any(|attr| attr.name == name && attr.token) {
                if let Some(value) = normalized(value, quote, subset) {
                    edits.push((value_start + 1, cursor.pos + 1, format!("\"{value}\"")));
                }
            }
        }
        let mut rendered = tag.to_owned();
        for (start, end, value) in edits.into_iter().rev() {
            rendered.replace_range(start..end, &value);
        }
        let insertion = rendered
            .rfind("/>")
            .or_else(|| rendered.rfind('>'))
            .unwrap_or(rendered.len());
        let mut defaults = String::new();
        for attribute in declared {
            if seen.contains(attribute.name.as_str()) {
                continue;
            }
            if let Some((quote, value)) = &attribute.default {
                let value = if attribute.token {
                    normalized(value, *quote, subset).map(|value| format!("\"{value}\""))
                } else {
                    None
                }
                .unwrap_or_else(|| format!("{quote}{value}{quote}"));
                defaults.push_str(&format!(" {}={value}", attribute.name));
            }
        }
        rendered.insert_str(insertion, &defaults);
        result.push_str(&rendered);
        pos = start + end;
    }
    result.push_str(&text[pos..]);
    result
}

pub(super) fn expand(xml: &str) -> Result<String, Failure> {
    let Some(start) = xml
        .match_indices("<!DOCTYPE")
        .find(|(pos, _)| {
            !matches!(
                super::context(xml, *pos).area,
                "comment" | "cdata" | "instruction" | "attribute"
            )
        })
        .map(|(pos, _)| pos)
    else {
        return Ok(xml.into());
    };
    let Some((finish, Some(subset_range))) = doctype_span(xml, start) else {
        return Ok(xml.into());
    };
    let (begin, end, pos) = (subset_range.start - 1, subset_range.end, finish - 1);
    let subset = &xml[begin + 1..end];
    if !subset.contains("<!ATTLIST") && !subset.contains('%') {
        return Ok(xml.into());
    }
    let expanded = subset.to_owned();
    let attrs = attributes(&expanded);
    Ok(format!(
        "{}{}{}{}",
        &xml[..begin + 1],
        expanded,
        &xml[end..pos + 1],
        apply(&xml[pos + 1..], &expanded, &attrs)
    ))
}
