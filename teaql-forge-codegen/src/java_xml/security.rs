//! The scanner's UTF-16 name, attribute-count and element-depth limits.
use super::{failure, Failure};
use crate::xml_factory::{Limit, ParserFactory, SecurityLimits};

pub(super) fn number(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i != 0 && (text.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub(super) fn name_message(limits: &SecurityLimits, name: &str, entity: &str) -> Option<String> {
    let length = name.encode_utf16().count() as u64;
    limits.over(Limit::Name, length).then(|| format!("JAXP00010005: The length of entity \"{entity}\" is \"{}\" that exceeds the \"{}\" limit set by \"{}\".", number(length), number(limits.get(Limit::Name)), limits.state(Limit::Name)))
}

fn name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(ch, '_' | ':' | '-' | '.' | '\u{b7}')
        || matches!(ch as u32, 0xc0..=0xd6 | 0xd8..=0xf6 | 0xf8..=0x2ff | 0x300..=0x37d | 0x37f..=0x1fff | 0x200c..=0x200d | 0x203f..=0x2040 | 0x2070..=0x218f | 0x2c00..=0x2fef | 0x3001..=0xd7ff | 0xf900..=0xfdcf | 0xfdf0..=0xfffd | 0x10000..=0xeffff)
}

fn scan_name<'a>(text: &'a str, pos: &mut usize) -> &'a str {
    let start = *pos;
    *pos += text[start..]
        .char_indices()
        .take_while(|(_, ch)| name_char(*ch))
        .map(|(_, ch)| ch.len_utf8())
        .sum::<usize>();
    &text[start..*pos]
}

pub(super) struct Consumption {
    pub end: usize,
    pub size: u64,
    pub node: bool,
    pub scanner: bool,
}

/// Preserve scanner call boundaries: limits are checked after each consumed
/// name, delimiter or literal run, before the next token is scanned.
pub(super) fn consumptions(text: &str, attribute: bool, external: bool) -> Vec<Consumption> {
    fn event(out: &mut Vec<Consumption>, text: &str, start: usize, end: usize, node: bool) {
        out.push(Consumption {
            end,
            size: text[start..end].encode_utf16().count() as u64,
            node,
            scanner: true,
        });
    }
    fn reference(out: &mut Vec<Consumption>, text: &str, pos: &mut usize, attribute: bool) {
        let start = *pos;
        let Some(end) = text[start..].find(';').map(|n| start + n + 1) else {
            *pos = text.len();
            return;
        };
        let name = &text[start + 1..end - 1];
        if attribute && !name.starts_with('#') {
            event(out, text, start + 1, end - 1, false);
        }
        let decoded = super::dtd::character_references(&text[start..end]);
        let builtin = matches!(name, "amp" | "lt" | "gt" | "quot" | "apos");
        if builtin || name.starts_with('#') {
            let size = if builtin {
                1
            } else {
                decoded.encode_utf16().count() as u64
            };
            out.push(Consumption {
                end,
                size,
                node: false,
                scanner: false,
            });
        }
        *pos = end;
    }
    fn literal(out: &mut Vec<Consumption>, text: &str, pos: &mut usize, end: usize) {
        while *pos < end {
            if text[*pos..].starts_with('&') {
                reference(out, text, pos, true);
                continue;
            }
            let stop = text[*pos..end]
                .find(['&', '<', ']', '\r', '\n'])
                .map(|n| *pos + n);
            let next = stop
                .map(|n| {
                    if text[n..].starts_with('&') {
                        n
                    } else {
                        n + text[n..].chars().next().unwrap().len_utf8()
                    }
                })
                .unwrap_or(end);
            event(out, text, *pos, next, false);
            *pos = next;
        }
    }
    let mut out = Vec::new();
    let mut pos = 0;
    if attribute {
        literal(&mut out, text, &mut pos, text.len());
        return out;
    }
    if external && !text.contains(['<', '&', '\r', '\n', ']']) {
        // The UTF-8 external stream initially reads 32 characters for text
        // declaration detection, followed by the JDK's 8192-character buffer.
        // The noncoalescing content driver consumes two runs per callback.
        let mut run = 0;
        while pos < text.len() {
            let start = pos;
            let count = if run == 0 { 32 } else { 8192 };
            let mut units = 0;
            for ch in text[pos..].chars() {
                if units + ch.len_utf16() > count {
                    break;
                }
                units += ch.len_utf16();
                pos += ch.len_utf8();
            }
            event(&mut out, text, start, pos, run % 2 == 1);
            run += 1;
        }
        return out;
    }
    while pos < text.len() {
        if text[pos..].starts_with('&') {
            reference(&mut out, text, &mut pos, false);
            continue;
        }
        if !text[pos..].starts_with('<') {
            let end = text[pos..]
                .find(['<', '&'])
                .map(|n| pos + n)
                .unwrap_or(text.len());
            event(&mut out, text, pos, end, false);
            pos = end;
            if pos < text.len() && text[pos..].starts_with('<') {
                // skipChar('<') is attempted before the character callback.
                event(&mut out, text, pos, pos + 1, true);
                pos += 1;
            } else {
                continue;
            }
        } else {
            event(&mut out, text, pos, pos + 1, false);
            pos += 1;
        }
        if text[pos..].starts_with('!') {
            event(&mut out, text, pos, pos + 1, false);
            pos += 1;
            if text[pos..].starts_with("--") {
                event(&mut out, text, pos, pos + 2, false);
                pos += 2;
                let end = text[pos..]
                    .find("--")
                    .map(|n| pos + n + 2)
                    .unwrap_or(text.len());
                event(&mut out, text, pos, end, false);
                pos = end;
                if text[pos..].starts_with('>') {
                    event(&mut out, text, pos, pos + 1, true);
                    pos += 1;
                }
            } else if text[pos..].starts_with("[CDATA[") {
                event(&mut out, text, pos, pos + 7, false);
                pos += 7;
                let end = text[pos..]
                    .find("]]")
                    .map(|n| pos + n + 2)
                    .unwrap_or(text.len());
                event(&mut out, text, pos, end, false);
                pos = end;
                if text[pos..].starts_with('>') {
                    event(&mut out, text, pos, pos + 1, true);
                    pos += 1;
                }
            }
            continue;
        }
        if text[pos..].starts_with('?') {
            event(&mut out, text, pos, pos + 1, false);
            pos += 1;
            let start = pos;
            scan_name(text, &mut pos);
            event(&mut out, text, start, pos, false);
            let start = pos;
            spaces(text, &mut pos);
            event(&mut out, text, start, pos, false);
            let end = text[pos..]
                .find("?>")
                .map(|n| pos + n + 2)
                .unwrap_or(text.len());
            event(&mut out, text, pos, end, true);
            pos = end;
            continue;
        }
        let closing = text[pos..].starts_with('/');
        if closing {
            event(&mut out, text, pos, pos + 1, false);
            pos += 1;
        }
        let start = pos;
        let name = scan_name(text, &mut pos);
        if name.is_empty() {
            pos = text.len();
            continue;
        }
        event(&mut out, text, start, pos, !closing);
        loop {
            let start = pos;
            spaces(text, &mut pos);
            if start != pos {
                event(&mut out, text, start, pos, false);
            }
            if pos == text.len() {
                break;
            }
            if text[pos..].starts_with('/') {
                event(&mut out, text, pos, pos + 1, false);
                pos += 1;
            }
            if text[pos..].starts_with('>') {
                event(&mut out, text, pos, pos + 1, false);
                pos += 1;
                break;
            }
            let start = pos;
            let name = scan_name(text, &mut pos);
            if name.is_empty() {
                break;
            }
            event(&mut out, text, start, pos, true);
            let start = pos;
            spaces(text, &mut pos);
            if start != pos {
                event(&mut out, text, start, pos, false);
            }
            if !text[pos..].starts_with('=') {
                break;
            }
            event(&mut out, text, pos, pos + 1, false);
            pos += 1;
            let start = pos;
            spaces(text, &mut pos);
            if start != pos {
                event(&mut out, text, start, pos, false);
            }
            let Some(quote) = text[pos..]
                .chars()
                .next()
                .filter(|c| matches!(c, '\'' | '"'))
            else {
                break;
            };
            event(&mut out, text, pos, pos + 1, false);
            pos += 1;
            let end = text[pos..]
                .find(quote)
                .map(|n| pos + n)
                .unwrap_or(text.len());
            literal(&mut out, text, &mut pos, end);
            if pos < text.len() {
                event(&mut out, text, pos, pos + 1, false);
                pos += 1;
            }
        }
    }
    out
}

fn spaces(text: &str, pos: &mut usize) {
    *pos += text[*pos..].len()
        - text[*pos..]
            .trim_start_matches([' ', '\t', '\r', '\n'])
            .len();
}

fn check_name(text: &str, pos: usize, name: &str, limits: &SecurityLimits) -> Result<(), Failure> {
    // Namespace-aware QName scanning checks prefix and local part separately.
    for part in name.split(':') {
        if let Some(message) = name_message(limits, part, "[xml]") {
            return Err(failure(text, pos, message));
        }
    }
    Ok(())
}

pub(super) fn scan(text: &str) -> Result<(), Failure> {
    scan_for(text, ParserFactory::Sax)
}

pub(super) fn scan_for(text: &str, factory: ParserFactory) -> Result<(), Failure> {
    let limits = SecurityLimits::load(factory)?;
    let mut pos = 0;
    let mut depth = 0_u64;
    while let Some(start) = text[pos..].find('<').map(|i| pos + i) {
        pos = start;
        let rest = &text[pos..];
        if rest.starts_with("<!--") || rest.starts_with("<![CDATA[") {
            let close = if rest.starts_with("<!--") {
                "-->"
            } else {
                "]]>"
            };
            pos += rest
                .find(close)
                .map(|i| i + close.len())
                .unwrap_or(rest.len());
            continue;
        }
        if rest.starts_with("<?") {
            pos += 2;
            let name = scan_name(text, &mut pos);
            check_name(text, pos, name, &limits)?;
            pos += text[pos..]
                .find("?>")
                .map(|i| i + 2)
                .unwrap_or(text.len() - pos);
            continue;
        }
        if rest.starts_with("<!") {
            if rest.starts_with("<!DOCTYPE") {
                pos += 9;
                spaces(text, &mut pos);
                let name = scan_name(text, &mut pos);
                check_name(text, pos, name, &limits)?;
            } else {
                pos += 2;
            }
            let mut quote = None;
            let mut brackets = 0_usize;
            while pos < text.len() {
                let ch = text[pos..].chars().next().unwrap();
                pos += ch.len_utf8();
                if let Some(delimiter) = quote {
                    if ch == delimiter {
                        quote = None;
                    }
                } else if matches!(ch, '\'' | '"') {
                    quote = Some(ch);
                } else if ch == '[' {
                    brackets += 1;
                } else if ch == ']' {
                    brackets = brackets.saturating_sub(1);
                } else if ch == '>' && brackets == 0 {
                    break;
                }
            }
            continue;
        }
        pos += 1;
        let closing = text[pos..].starts_with('/');
        if closing {
            pos += 1;
        }
        let name = scan_name(text, &mut pos);
        check_name(text, pos, name, &limits)?;
        if name.is_empty() {
            continue;
        }
        if closing {
            depth = depth.saturating_sub(1);
            pos += text[pos..]
                .find('>')
                .map(|i| i + 1)
                .unwrap_or(text.len() - pos);
            continue;
        }
        depth += 1;
        if limits.over(Limit::Depth, depth) {
            return Err(failure(text, pos, format!("JAXP00010006: The element \"{name}\" has a depth of \"{}\" that exceeds the limit \"{}\" set by \"jdk.xml.maxElementDepth\".", number(depth), number(limits.get(Limit::Depth)))));
        }
        let mut count = 0_u64;
        loop {
            spaces(text, &mut pos);
            if text[pos..].starts_with("/>") {
                depth = depth.saturating_sub(1);
                pos += 2;
                break;
            }
            if text[pos..].starts_with('>') {
                pos += 1;
                break;
            }
            if pos == text.len() {
                break;
            }
            let attr = scan_name(text, &mut pos);
            check_name(text, pos, attr, &limits)?;
            if attr.is_empty() {
                break;
            }
            spaces(text, &mut pos);
            if !text[pos..].starts_with('=') {
                break;
            }
            pos += 1;
            spaces(text, &mut pos);
            let Some(quote) = text[pos..]
                .chars()
                .next()
                .filter(|ch| matches!(ch, '\'' | '"'))
            else {
                break;
            };
            pos += 1;
            let Some(end) = text[pos..].find(quote).map(|i| pos + i) else {
                pos = text.len();
                break;
            };
            let value = &text[pos..end];
            pos = end + 1;
            if attr == "xmlns" || attr.starts_with("xmlns:") {
                let xml = format!("<value a={quote}{value}{quote}/>");
                if let Ok(parsed) = roxmltree::Document::parse(&xml) {
                    let value = parsed.root_element().attribute("a").unwrap();
                    if let Some(message) = name_message(&limits, value, value) {
                        return Err(failure(text, pos, message));
                    }
                }
            }
            count += 1;
            if limits.over(Limit::Attributes, count) {
                let limit = number(limits.get(Limit::Attributes));
                return Err(failure(text, pos, format!("JAXP00010002:  Element \"{name}\" has more than \"{limit}\" attributes, \"{limit}\" is the limit set by \"jdk.xml.elementAttributeLimit\".")));
            }
        }
    }
    Ok(())
}
