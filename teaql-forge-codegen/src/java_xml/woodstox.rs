//! Woodstox 6.2.6's byte stream and namespace-aware SAX path.
//!
//! Cursor positions below refer to consumed UTF-16 characters, as in
//! StreamScanner.getLastCharLocation. DOM and SourceIndex use separate paths.
use super::{declaration, dtd, external, Failure};
use std::collections::{BTreeMap, BTreeSet};
mod entities;

fn plain(message: impl Into<String>) -> Failure {
    Failure { message: format!("SAXParseException: {}", message.into()), line: None, offset: None, factory: None }
}

fn at(text: &str, pos: usize, message: impl Into<String>) -> Failure {
    let mut row = 1;
    let mut column = 0;
    let mut cr = false;
    for ch in text[..pos.min(text.len())].chars() {
        if ch == '\r' || ch == '\n' && !cr {
            row += 1;
            column = 0;
        } else if ch != '\n' || !cr {
            column += ch.len_utf16();
        }
        cr = ch == '\r';
    }
    let mut error = plain(format!("{}\n at [row,col {{unknown-source}}]: [{row},{column}]", message.into()));
    error.line = Some(row);
    error.offset = Some(pos);
    error
}

pub(super) fn description(ch: char) -> String {
    let n = ch as u32;
    if ch.is_control() { format!("(CTRL-CHAR, code {n})") }
    else if n > 255 { format!("'{ch}' (code {n} / 0x{n:x})") }
    else { format!("'{ch}' (code {n})") }
}

fn name_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | ':' | '-' | '.' | '\u{b7}') || ch as u32 >= 0x300
}

// XmlChars handles non-ASCII code units; Woodstox handles ASCII and surrogate
// pairs in its scanner. Keep these rules separate from the JDK SAX/DOM path.
const WOODSTOX_NAME_FLAGS: &[u8] = include_bytes!("../data/woodstox-xml-name-6.2.6.bin");

pub(super) fn woodstox_name_character(ch: char, first: bool, version11: bool) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphabetic()
            || matches!(ch, '_' | ':')
            || !first && (ch.is_ascii_digit() || matches!(ch, '-' | '.'));
    }
    let code = ch as u32;
    // Woodstox accepts surrogate pairs as XML 1.0 and XML 1.1 names.
    if code >= 0x10000 {
        return true;
    }
    let mask = if version11 {
        if first { 4 } else { 8 }
    } else if first {
        1
    } else {
        2
    };
    WOODSTOX_NAME_FLAGS[code as usize] & mask != 0
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    version11: bool,
    stack: Vec<String>,
    namespaces: Vec<BTreeMap<String, String>>,
    seen_root: bool,
    entities: BTreeSet<String>,
    stream: Option<&'a entities::Stream>,
    nodes: Vec<(String, usize)>,
}

impl<'a> Scanner<'a> {
    fn rest(&self) -> &'a str { &self.text[self.pos..] }
    fn at(&self, pos: usize, message: impl Into<String>) -> Failure {
        if let Some(stream) = self.stream { stream.at(pos, message) } else { at(self.text, pos, message) }
    }
    fn error(&self, message: impl Into<String>) -> Failure { self.at(self.pos, message) }
    fn frame(&self, pos: usize) -> usize { self.stream.map(|s| s.frame(pos)).unwrap_or(0) }
    fn limit_error(&self, message: impl Into<String>) -> Failure {
        let mut error = plain(message); error.offset = Some(self.pos); error
    }
    fn next(&mut self) -> Option<char> {
        let ch = self.rest().chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
    fn unexpected(&self, ch: char, suffix: &str) -> Failure {
        self.error(format!("Unexpected character {}{suffix}", description(ch)))
    }
    fn spaces(&mut self) -> bool {
        let start = self.pos;
        while self.rest().starts_with([' ', '\t', '\r', '\n']) { self.next(); }
        self.pos != start
    }
    fn name(&mut self) -> Result<&'a str, Failure> {
        let start = self.pos;
        let Some(first) = self.rest().chars().next() else {
            return Err(self.error("Unexpected end of input block; expected an identifier"));
        };
        if !woodstox_name_character(first, true, self.version11) {
            self.next();
            return Err(self.unexpected(first, " (expected a name start character)"));
        }
        self.next();
        while self.rest().chars().next().is_some_and(|ch| woodstox_name_character(ch, false, self.version11)) { self.next(); }
        Ok(&self.text[start..self.pos])
    }
    fn character(&mut self) -> Result<(), Failure> {
        let ch = self.next().unwrap();
        if ch == '\0' {
            return Err(self.error("Illegal character (NULL, unicode 0) encountered: not valid in any content"));
        }
        if (ch as u32) < 32 && !matches!(ch, '\t' | '\r' | '\n') {
            let note = if self.version11 { " [note: in XML 1.1, it could be included via entity expansion]" } else { "" };
            return Err(self.error(format!("Illegal character ({}){note}", description(ch))));
        }
        Ok(())
    }
    fn reference(&mut self) -> Result<(), Failure> {
        self.pos += 1; // &
        let reference_cursor = self.pos;
        if self.rest().starts_with('#') {
            self.pos += 1;
            let hex = self.rest().starts_with('x');
            if hex { self.pos += 1; }
            let mut value = 0_u32;
            loop {
                let ch = self.next().ok_or_else(|| self.error("Unexpected EOF in entity reference"))?;
                if ch == ';' { break; }
                let Some(digit) = ch.to_digit(if hex { 16 } else { 10 }) else {
                    return Err(self.unexpected(ch, if hex { "; expected a hex digit (0-9a-fA-F)." } else { "; expected a decimal number." }));
                };
                value = value.saturating_mul(if hex { 16 } else { 10 }).saturating_add(digit);
                if value > 0x10ffff { return Err(self.at(reference_cursor, "Illegal character entity: value higher than max allowed (0x10ffff)")); }
            }
            if value == 0 { return Err(self.error("Invalid character reference: null character not allowed in XML content.")); }
            if (0xd800..=0xdfff).contains(&value) || value == 0xfffe || value == 0xffff
                || value < 32 && !matches!(value, 9 | 10 | 13) && !self.version11 {
                return Err(self.error(format!("Illegal character entity: expansion character (code 0x{value:x}")));
            }
        } else {
            let name = self.name()?;
            let ch = self.next().ok_or_else(|| self.error(format!("Missing semicolon after reference for entity \"{name}\"")))?;
            if ch != ';' { return Err(self.unexpected(ch, &format!("; expected a semi-colon after the reference for entity '{name}'"))); }
            if !matches!(name, "lt" | "gt" | "amp" | "apos" | "quot") && !self.entities.contains(name) {
                return Err(self.error(format!("Undeclared general entity \"{name}\"")));
            }
        }
        Ok(())
    }
    fn quoted(&mut self) -> Result<&'a str, Failure> {
        let quote_frame = self.frame(self.pos);
        let quote = self.next().ok_or_else(|| self.error("Unexpected EOF in start tag"))?;
        if !matches!(quote, '\'' | '"') { return Err(self.unexpected(quote, " in start tag Expected a quote")); }
        let start = self.pos;
        while !self.rest().starts_with(quote) || self.frame(self.pos) != quote_frame {
            if self.rest().is_empty() { return Err(self.error("Unexpected EOF in attribute value")); }
            if self.rest().starts_with('<') {
                self.pos += 1;
                return Err(self.unexpected('<', " in attribute value"));
            }
            if self.rest().starts_with('&') { self.reference()?; } else { self.character()?; }
        }
        let end = self.pos;
        self.pos += 1;
        if self.text[start..end].encode_utf16().count() > 524288 { return Err(self.limit_error("Maximum attribute size limit (524288) exceeded")); }
        Ok(&self.text[start..end])
    }
    fn delimited(&mut self, open: usize, close: &str, comment: bool) -> Result<(), Failure> {
        self.pos += open;
        let primary_cursor = self.pos + self.rest().chars().next().map(char::len_utf8).unwrap_or(0);
        loop {
            if self.rest().starts_with(close) { self.pos += close.len(); return Ok(()); }
            if self.rest().is_empty() { return Err(self.error("Unexpected EOF in comment")); }
            if comment && self.rest().starts_with("--") {
                self.pos += 3;
                return Err(self.error("String '--' not allowed in comment (missing '>'?)"));
            }
            if let Err(error) = self.character() {
                // readCDataPrimary uses a local pointer. Its diagnostic
                // location still points at the first character of that run.
                if open == 9 {
                    let message = error.message.strip_prefix("SAXParseException: ").unwrap().split("\n at ").next().unwrap();
                    return Err(at(self.text, primary_cursor, message));
                }
                return Err(error);
            }
        }
    }
    fn doctype(&mut self) -> Result<(), Failure> {
        self.pos += 9;
        let mut brackets = 0_usize;
        let mut quote = None;
        while !self.rest().is_empty() {
            if quote.is_none() && self.rest().starts_with("<!--") {
                self.delimited(4, "-->", true)?;
                continue;
            }
            if quote.is_none() && self.rest().starts_with("<?") {
                self.delimited(2, "?>", false)?;
                continue;
            }
            if quote.is_none() && brackets > 0 && self.rest().starts_with("<!ENTITY") {
                self.pos += "<!ENTITY".len();
                self.spaces();
                if self.rest().starts_with('%') {
                    self.next();
                    self.spaces();
                }
                let Some(first) = self.rest().chars().next() else {
                    return Err(self.error("Unexpected EOF in internal DTD subset"));
                };
                if !woodstox_name_character(first, true, self.version11) {
                    self.next();
                    return Err(self.unexpected(first, " in internal DTD subset; expected an identifier"));
                }
                self.next();
                while self.rest().chars().next().is_some_and(|ch| woodstox_name_character(ch, false, self.version11)) {
                    self.next();
                }
                if let Some(ch) = self.rest().chars().next().filter(|ch| !matches!(ch, ' ' | '\t' | '\r' | '\n')) {
                    self.next();
                    return Err(self.unexpected(ch, " in internal DTD subset; expected a separating white space"));
                }
                continue;
            }
            let ch = self.next().unwrap();
            if ch == '\0' || (ch as u32) < 32 && !matches!(ch, '\t' | '\r' | '\n') {
                self.pos -= ch.len_utf8();self.character()?;
            }
            if let Some(q) = quote {
                if ch == q { quote = None; }
                else if ch == '&' && self.rest().starts_with('#') { self.pos -= 1; self.reference()?; }
            } else if matches!(ch, '\'' | '"') { quote = Some(ch); }
            else if ch == '[' { brackets += 1; }
            else if ch == ']' { brackets = brackets.saturating_sub(1); }
            else if ch == '>' && brackets == 0 { return Ok(()); }
        }
        Err(self.error("Unexpected EOF in DOCTYPE declaration"))
    }
    fn tag(&mut self) -> Result<(), Failure> {
        self.pos += 1;
        let source_line = self.at(self.pos, "").line.unwrap_or(1);
        if self.rest().starts_with('<') {
            self.pos += 1;
            return Err(self.unexpected('<', " in content after '<' (malformed start element?)."));
        }
        let closing = self.rest().starts_with('/');
        if closing { self.pos += 1; }
        if !closing {
            if let Some(first) = self.rest().chars().next().filter(|ch| !woodstox_name_character(*ch, true, self.version11)) {
                self.next();
                return Err(self.unexpected(first, " in content after '<' (malformed start element?)."));
            }
        }
        let name = self.name()?.to_owned();
        if self.rest().is_empty() {
            return Err(self.error(if closing { "Unexpected end of input block in end tag" } else { "Unexpected end of input block; expected an identifier" }));
        }
        if closing {
            if self.stack.is_empty() {
                return Err(self.at(self.pos - name.len(), if self.seen_root {
                    "Unexpected character combination '</' in epilog (extra close tag?)."
                } else { "Unexpected character combination '</' in prolog." }));
            }
            let expected = self.stack.last().cloned().unwrap_or_default();
            if expected != name {
                return Err(self.error(format!("Unexpected close tag </{name}>; expected </{expected}>.")));
            }
            self.spaces();
            self.next();
            self.stack.pop();
            self.namespaces.pop();
            return Ok(());
        }
        if self.stack.is_empty() && self.seen_root {
            // Prolog dispatch has consumed '<' and the first name character.
            let p = self.pos - name.len() + name.chars().next().unwrap().len_utf8();
            return Err(self.at(p, "Illegal to have multiple roots (start tag in epilog?)."));
        }
        self.seen_root = true;
        if self.stack.len() >= 1000 { return Err(self.limit_error("Maximum Element Depth limit (1000) Exceeded")); }
        let mut attrs = BTreeMap::new();
        let mut duplicate = None;
        let mut ns = self.namespaces.last().cloned().unwrap_or_default();
        ns.insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
        let empty;
        loop {
            let spaced = self.spaces();
            if self.rest().starts_with("/>") { self.pos += 2; empty = true; break; }
            if self.rest().starts_with('>') { self.pos += 1; empty = false; break; }
            if self.rest().is_empty() { return Err(self.error("Unexpected EOF in start tag")); }
            if !spaced {
                let ch = self.next().unwrap();
                return Err(self.unexpected(ch, " excepted space, or '>' or \"/>\""));
            }
            let attr = self.name()?.to_owned();
            if let Some(prefix) = attr.strip_prefix("xmlns:") {
                if attrs.contains_key(&attr) {
                    // Namespace declarations are checked when their names
                    // are read, before the second value is scanned.
                    return Err(self.at(self.pos + 2, format!("Duplicate declaration for namespace prefix '{prefix}'.")));
                }
            }
            self.spaces();
            let ch = self.next().ok_or_else(|| self.error("Unexpected EOF in start tag"))?;
            if ch != '=' { return Err(self.unexpected(ch, " expected '='")); }
            self.spaces();
            let value = self.quoted()?.to_owned();
            if attrs.insert(attr.clone(), value.clone()).is_some() { duplicate = Some(attr.clone()); }
            if attr == "xmlns" { ns.insert(String::new(), value); }
            else if let Some(prefix) = attr.strip_prefix("xmlns:") { ns.insert(prefix.into(), value); }
            if attrs.len() > 1000 { return Err(self.limit_error("Attribute limit (1000) exceeded")); }
        }
        if let Some(attr) = duplicate { return Err(self.error(format!("Duplicate attribute '{attr}'."))); }
        if let Some((prefix, _)) = name.split_once(':') {
            if !ns.contains_key(prefix) { return Err(self.error(format!("Undeclared namespace prefix \"{prefix}\""))); }
        }
        let mut expanded_names = BTreeSet::new();
        for attr in attrs.keys().filter(|s| s.as_str() != "xmlns" && !s.starts_with("xmlns:")) {
            let (uri, local) = if let Some((prefix, local)) = attr.split_once(':') {
                let Some(uri) = ns.get(prefix) else { return Err(self.error(format!("Undeclared namespace prefix \"{prefix}\" (for attribute \"{local}\")"))); };
                (uri.as_str(), local)
            } else { ("", attr.as_str()) };
            if !expanded_names.insert((uri, local)) {
                return Err(self.error(format!("Duplicate attribute '{{{uri}}}{local}'.")));
            }
        }
        self.nodes.push((name.rsplit(':').next().unwrap().to_owned(), source_line));
        if !empty { self.stack.push(name); self.namespaces.push(ns); }
        Ok(())
    }
    fn scan(&mut self) -> Result<(), Failure> {
        if self.rest().starts_with("<?xml") {
            let end = self.rest().find("?>").map(|p| p + 2).unwrap_or(self.text.len());
            let attrs = declaration(&self.text[..end]);
            if let Some(version) = attrs.get("version").filter(|s| !matches!(s.as_str(), "1.0" | "1.1")) {
                let quote = self.text[..end].find(version).unwrap_or(0) + version.len();
                return Err(plain(format!("Invalid XML pseudo-attribute 'version' value '{version}'; expected \"1.0\" or \"1.1\"\n at [row,col,system-id]: [1,{},\"N/A\"]", self.text[..quote].encode_utf16().count() + 1)));
            }
            self.pos = end;
        }
        while !self.rest().is_empty() {
            if self.rest().starts_with("<!--") { self.delimited(4, "-->", true)?; }
            else if self.rest().starts_with("<![CDATA[") {
                if self.stack.is_empty() {
                    self.pos += 3;
                    return Err(self.unexpected('C', " (CDATA not allowed in prolog/epilog)"));
                }
                self.delimited(9, "]]>", false)?;
            } else if self.rest().starts_with("<?") {
                self.pos += 2;
                let target = self.name()?;
                if target.eq_ignore_ascii_case("xml") {
                    return Err(self.error("Illegal processing instruction target (\"xml\"); xml (case insensitive) is reserved by the specs."));
                }
                if !self.rest().starts_with("?>") {
                    if let Some(ch) = self.rest().chars().next().filter(|ch| !matches!(ch, ' ' | '\t' | '\r' | '\n')) {
                        self.next();
                        return Err(self.unexpected(ch, "excepted either space or \"?>\" after PI target"));
                    }
                }
                self.delimited(0, "?>", false)?;
            } else if self.rest().starts_with("<!DOCTYPE") { self.doctype()?; }
            else if self.rest().starts_with('<') { self.tag()?; }
            else if self.rest().starts_with('&') && !self.stack.is_empty() { self.reference()?; }
            else {
                if self.stack.is_empty() && !self.rest().starts_with([' ', '\t', '\r', '\n']) {
                    let start = self.pos;
                    self.character()?;
                    let ch = self.text[start..].chars().next().unwrap();
                    return Err(self.unexpected(ch, if self.seen_root { " in epilog; expected '<'" } else { " in prolog; expected '<'" }));
                }
                self.character()?;
            }
        }
        if let Some(name) = self.stack.last() {
            return Err(self.error(format!("Unexpected EOF; was expecting a close tag for element <{name}>")));
        }
        if !self.seen_root { return Err(self.error("Unexpected EOF in prolog")); }
        Ok(())
    }
}

pub(super) fn decode(bytes: &[u8]) -> Result<String, Failure> {
    let utf32 = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) { Some((true, &bytes[4..])) }
        else if bytes.starts_with(&[0, 0, 0xfe, 0xff]) { Some((false, &bytes[4..])) }
        else if bytes.starts_with(&[b'<', 0, 0, 0]) { Some((true, bytes)) }
        else if bytes.starts_with(&[0, 0, 0, b'<']) { Some((false, bytes)) } else { None };
    if let Some((little, data)) = utf32 {
        let mut out = String::new();
        for word in data.chunks_exact(4) {
            let n = if little { u32::from_le_bytes(word.try_into().unwrap()) } else { u32::from_be_bytes(word.try_into().unwrap()) };
            out.push(char::from_u32(n).ok_or_else(|| plain(format!("Invalid UTF-32 character 0x{n:x}")))?);
        }
        return Ok(out);
    }
    let utf8 = !bytes.starts_with(&[0xfe, 0xff]) && !bytes.starts_with(&[0xff, 0xfe])
        && !bytes.starts_with(&[0, b'<', 0, b'?']) && !bytes.starts_with(&[b'<', 0, b'?', 0])
        && !bytes.starts_with(&[0, 0, 0, b'<']) && !bytes.starts_with(&[b'<', 0, 0, 0]);
    let data = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let preliminary = super::utf8_string(data);
    let attrs = declaration(&preliminary);
    let label = attrs.get("encoding").map(String::as_str).unwrap_or("UTF-8");
    if utf8 && label.eq_ignore_ascii_case("UTF-8") {
        if let Err(error) = std::str::from_utf8(data) {
            let p = error.valid_up_to();
            let count = std::str::from_utf8(&data[..p]).unwrap().encode_utf16().count();
            let lead = data[p];
            if lead < 0xc0 || lead >= 0xf8 {
                return Err(plain(format!("Invalid UTF-8 start byte 0x{lead:x} (at char #{}, byte #-1)", count + 1)));
            }
            if lead == 0xed && data.get(p + 1).is_some_and(|b| *b >= 0xa0) && data.len() > p + 2 {
                let n = ((lead as u32 & 15) << 12) | ((data[p+1] as u32 & 63) << 6) | (data[p+2] as u32 & 63);
                return Err(plain(format!("Invalid UTF-8 character 0x{n:x}(a surrogate character)  at char #{count}, byte #-1)")));
            }
            let width = if lead < 0xe0 { 2 } else if lead < 0xf0 { 3 } else { 4 };
            let bad = (1..width).find_map(|i| data.get(p+i).filter(|b| **b & 0xc0 != 0x80));
            if let Some(bad) = bad { return Err(plain(format!("Invalid UTF-8 middle byte 0x{bad:x} (at char #{count}, byte #-1)"))); }
        }
    }
    let decoded = if utf8 && label.eq_ignore_ascii_case("UTF-8") {
        let mut out = String::new();
        let mut pos = 0;
        while pos < data.len() {
            let lead = data[pos];
            let width = if lead < 0x80 { 1 } else if lead < 0xe0 { 2 } else if lead < 0xf0 { 3 } else { 4 };
            if data.len() - pos < width { return Err(plain(format!("Unexpected EOF in the middle of a multi-byte char: got {}, needed {width}, at char #{}, byte #{})", data.len() - pos, out.encode_utf16().count(), data.len()))); }
            let mut value = (lead & match width { 1 => 0x7f, 2 => 0x1f, _ => 0x0f }) as u32;
            for byte in &data[pos + 1..pos + width] { value = value << 6 | (byte & 63) as u32; }
            if matches!(value, 0xfffe | 0xffff) {
                // Woodstox rejects these in its UTF-8 reader, before the SAX
                // scanner can report a name error. An XML declaration resets
                // its character counter and leaves the byte cursor on '>'.
                let declaration_len = if out.starts_with("<?xml ") {
                    out.find("?>").map(|end| end + 2).unwrap_or(0)
                } else { 0 };
                let character = out[declaration_len..].encode_utf16().count();
                let byte = declaration_len as isize - 1;
                return Err(plain(format!("Invalid UTF-8 character 0x{value:x} at char #{character}, byte #{byte})")));
            }
            out.push(char::from_u32(value).ok_or_else(|| plain(format!("Invalid UTF-8 character 0x{value:x}")))?);
            pos += width;
        }
        Ok(out)
    } else { super::decode(bytes) };
    let text = decoded.map_err(|error| {
        if let Some(label) = error.message.strip_prefix("UnsupportedEncodingException: ") { plain(format!("Unsupported encoding: {label}")) } else { error }
    })?;
    if declaration(&text).get("version").is_some_and(|v| v == "1.1") {
        if let Some((pos, ch)) = text.char_indices().find(|(_, ch)| matches!(*ch as u32, 0x7f..=0x84 | 0x86..=0x9f)) {
            let declaration_len = text.find("?>").map(|p| p + 2).unwrap_or(0);
            return Err(plain(format!("Invalid character 0x{:x}, can only be included in xml 1.1 using character entities (at char #{}, byte #{})", ch as u32, text[declaration_len..pos].encode_utf16().count(), pos + ch.len_utf8() - 1)));
        }
    }
    Ok(text)
}

/// Internal entity inputs retain their declaration positions and link back to
/// the caller's cursor. Do not report the expanded document's synthetic offsets.
fn entity_error(text: &str, error: Failure) -> Failure {
    if !error.message.starts_with("SAXParseException: Recursive entity reference ") {
        if error.message.starts_with("SAXParseException: ") { return error; }
        return plain(error.message.strip_prefix("SAXParseException: ").unwrap_or_else(|| error.message.split_once(": ").map(|(_, s)| s).unwrap_or(&error.message)));
    }
    let Some(name) = error.message.split('"').nth(1) else { return error; };
    let (definitions, body) = entity_definitions(text);
    let mut frames = vec![(body, text.len(), Vec::<String>::new(), Vec::<usize>::new())];
    while let Some((start, end, path, callers)) = frames.pop() {
        let mut pos = start;
        while let Some(p) = text[pos..end].find('&').map(|p| p + pos) {
            let Some(stop) = text[p..end].find(';').map(|p2| p2 + p + 1) else { break; };
            pos = stop;
            let id = &text[p + 1..stop - 1];
            if path.iter().any(|s| s == id) {
                if id == name {
                    let mut out = at(text, stop, format!("Illegal entity expansion: entity \"{name}\" expands itself recursively."));
                    for caller in callers.iter().rev() {
                        let loc = at(text, *caller, "").message;
                        out.message.push_str(&loc.replace("SAXParseException: \n at ", "\n from "));
                    }
                    return out;
                }
            } else if let Some(&(a, b)) = definitions.get(id) {
                let mut next = path.clone(); next.push(id.into());
                let mut trace = callers.clone();
                trace.push(stop + text[stop..].chars().next().map(char::len_utf8).unwrap_or(0));
                frames.push((a, b, next, trace));
            }
        }
    }
    error
}

fn entity_definitions(text: &str) -> (BTreeMap<String, (usize, usize)>, usize) {
    let mut definitions = BTreeMap::new();
    let mut pos = 0;
    let mut body = 0;
    while let Some(start) = text[pos..].find("<!ENTITY").map(|p| p + pos) {
        let mut cursor = start + 8;
        while text[cursor..].starts_with([' ', '\t', '\r', '\n']) { cursor += 1; }
        let end = text[cursor..].find(|ch: char| !name_char(ch)).map(|p| p + cursor).unwrap_or(text.len());
        let name = &text[cursor..end];
        cursor = end;
        while text[cursor..].starts_with([' ', '\t', '\r', '\n']) { cursor += 1; }
        let Some(q) = text[cursor..].chars().next().filter(|ch| matches!(ch, '\'' | '"')) else { pos = cursor; continue; };
        cursor += 1;
        let Some(stop) = text[cursor..].find(q).map(|p| p + cursor) else { break; };
        definitions.entry(name.into()).or_insert((cursor, stop));
        pos = stop + 1;
        body = text[pos..].find("]>").map(|p| p + pos + 2).unwrap_or(pos);
    }
    (definitions, body)
}

pub(super) fn parse(bytes: &[u8]) -> Result<String, Failure> {
    Ok(parse_document(bytes)?.0)
}

// Parameter entities are expanded by the DTD layer before the document
// scanner sees the internal subset. Check declaration names on the original
// stream so that malformed names retain Woodstox's DTD cursor and message.
fn internal_entity_name_failure(text: &str) -> Option<Failure> {
    let start = text.match_indices("<!DOCTYPE")
        .find(|(position, _)| !matches!(super::context(text, *position).area,
            "comment" | "cdata" | "instruction" | "attribute"))?.0;
    let mut scanner = Scanner {
        text, pos: start,
        version11: declaration(text).get("version").is_some_and(|version| version == "1.1"),
        stack: Vec::new(), namespaces: Vec::new(), seen_root: false,
        entities: BTreeSet::new(), stream: None, nodes: Vec::new(),
    };
    let error = scanner.doctype().err()?;
    error.message.contains(" in internal DTD subset; expected ").then_some(error)
}

pub(super) fn parse_document(bytes: &[u8]) -> Result<(String, BTreeMap<usize, usize>), Failure> {
    let text = decode(bytes)?;
    if let Some(error) = internal_entity_name_failure(&text) { return Err(error); }
    let diagnostic = entities::Stream::internal(&text);
    if let Some(stream) = diagnostic.as_ref() {
        let result = Scanner { text: &stream.text, pos: 0, version11: declaration(&text).get("version").is_some_and(|v| v == "1.1"), stack: Vec::new(), namespaces: Vec::new(), seen_root: false, entities: BTreeSet::new(), stream: Some(&stream), nodes: Vec::new() }.scan();
        stream.finish(result)?;
    }
    let expansion = external::expand_document(&text).map_err(|e| entity_error(&text, e))?;
    let expanded = expansion.xml;
    // Expand external subsets before validating references; entity names are
    // visible without counting or substituting their values in this pass.
    let mut entities = BTreeSet::new();
    let mut rest = expanded.as_str();
    while let Some(start) = rest.find("<!ENTITY") {
        rest = &rest[start + 8..];
        let name = rest.trim_start().split(|ch: char| !name_char(ch)).next().unwrap_or("");
        if !name.is_empty() { entities.insert(name.into()); }
    }
    if diagnostic.is_none() { Scanner { text: &expanded, pos: 0, version11: declaration(&text).get("version").is_some_and(|v| v == "1.1"), stack: Vec::new(), namespaces: Vec::new(), seen_root: false, entities, stream: None, nodes: Vec::new() }.scan()?; }
    let xml = dtd::expand(&expanded)?;
    super::validate_scanned_with_name_rules(&xml, false, false, false)?;
    // The domain mapper consumes the same normalized XML representation used
    // by the built-in path after the SAX provider has accepted the stream.
    let xml = if declaration(&xml).get("version").is_some_and(|v| v == "1.1") {
        xml.replace("\r\u{85}", "\n").replace(['\u{85}', '\u{2028}'], "\n")
    } else { xml };
    let mut lines = node_lines(bytes, &xml);
    if diagnostic.is_none() && !expansion.nodes.is_empty() {
        if let Ok(document) = roxmltree::Document::parse_with_options(&xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() }) {
            let elements = document.descendants().filter(|node| node.is_element()).collect::<Vec<_>>();
            if elements.len() == expansion.nodes.len() && elements.iter().zip(&expansion.nodes).all(|(node, (name, _))| node.tag_name().name() == name.rsplit(':').next().unwrap()) {
                lines = elements.iter().zip(expansion.nodes).map(|(node, (_, line))| (node.range().start, line)).collect();
            }
        }
    }
    Ok((xml, lines))
}

pub(super) fn node_lines(bytes: &[u8], normalized: &str) -> BTreeMap<usize, usize> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let Ok(parsed) = roxmltree::Document::parse_with_options(normalized, options) else { return BTreeMap::new(); };
    let right = parsed.descendants().filter(|node| node.is_element()).collect::<Vec<_>>();
    let fallback = || right.iter().map(|node| (node.range().start, parsed.text_pos_at(node.range().start).row as usize)).collect();
    let Some(stream) = decode(bytes).ok().and_then(|text| entities::Stream::internal(&text)) else { return fallback(); };
    let mut scanner = Scanner { text: &stream.text, pos: 0, version11: declaration(&stream.text).get("version").is_some_and(|v| v == "1.1"), stack: Vec::new(), namespaces: Vec::new(), seen_root: false, entities: BTreeSet::new(), stream: Some(&stream), nodes: Vec::new() };
    if scanner.scan().is_err() || scanner.nodes.len() != right.len() || scanner.nodes.iter().zip(&right).any(|((name,_),node)| name != node.tag_name().name()) { return fallback(); }
    scanner.nodes.into_iter().zip(right).map(|((_,line),node)| (node.range().start, line)).collect()
}
