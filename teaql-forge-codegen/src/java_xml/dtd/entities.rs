//! Declaration order, parameter replacement and lazy external entity streams.
use super::{character_references, declaration_end, parameter_reference, space, Cursor};
use crate::java_xml::security::{consumptions, name_message, number, Consumption};
use crate::java_xml::{external, Failure};
use crate::xml_factory::{Limit, ParserFactory, SecurityLimits};
use std::collections::BTreeMap;
mod woodstox;
use woodstox::Origin;

#[derive(Clone)]
enum Definition {
    Literal(String),
    System(String),
}

#[derive(Clone)]
struct Entity {
    definition: Definition,
    base: Option<reqwest::Url>,
    origin: Option<Origin>,
}

struct MarkupSpan {
    start: usize,
    end: usize,
    origin: Option<Origin>,
    replacement: bool,
}

impl MarkupSpan {
    fn replacement_at(spans: &[Self], position: usize) -> bool {
        spans.iter().any(|span| span.start <= position && position < span.end && span.replacement)
    }

    fn consumed(spans: &[Self], start: usize, length: usize) -> Option<Origin> {
        spans.iter().find(|span| span.start <= start && start < span.end)
            .and_then(|span| span.origin.as_ref().map(|origin| origin.shifted(start - span.start + length)))
    }
}

struct Reader {
    parameters: BTreeMap<String, Entity>,
    generals: BTreeMap<String, Entity>,
    parameter_stack: Vec<String>,
    general_stack: Vec<String>,
    expansions: u64,
    expansion_limit: u64,
    limits: SecurityLimits,
    woodstox: bool,
    xml11: bool,
    total_size: u64,
    replacements: u64,
    origin: Option<Origin>,
    node_locations: Vec<(String, usize)>,
}

struct GeneralFrame {
    text: String,
    pos: usize,
    attribute: bool,
    entity: bool,
    name: String,
    size: u64,
    events: Vec<Consumption>,
    next_event: usize,
    contexts: BTreeMap<usize, (bool, bool)>,
    origin: Option<Origin>,
    node_tokens: Vec<(usize, String)>,
    next_node: usize,
    ends_markup: bool,
}

impl GeneralFrame {
    fn record_nodes(&mut self, start: usize, end: usize, nodes: &mut Vec<(String, usize)>, resumed: &mut Option<usize>) {
        if self.attribute || start == end { return; }
        while let Some((offset, name)) = self.node_tokens.get(self.next_node).filter(|(offset, _)| *offset < end) {
            if *offset >= start {
                if let Some(origin) = &self.origin {
                    let line = if *offset == start { resumed.take().unwrap_or_else(|| origin.shifted(*offset + 1).line()) } else { origin.shifted(*offset + 1).line() };
                    nodes.push((name.clone(), line));
                }
            }
            self.next_node += 1;
        }
        *resumed = None;
    }
}

/// Classify references once per replacement stream. Rescanning the complete
/// prefix at every ampersand makes Woodstox's 100,000-reference limit quadratic.
fn reference_contexts(text: &str) -> BTreeMap<usize, (bool, bool)> {
    let mut contexts = BTreeMap::new();
    let mut pos = 0;
    let mut tag = false;
    let mut quote = None;
    while pos < text.len() {
        let rest = &text[pos..];
        let close = if rest.starts_with("<!--") { Some("-->") }
            else if rest.starts_with("<![CDATA[") { Some("]]>") }
            else if rest.starts_with("<?") { Some("?>") } else { None };
        if !tag {
            if let Some(close) = close {
                let end = pos + rest.find(close).map(|i| i + close.len()).unwrap_or(rest.len());
                for (offset, _) in text[pos..end].match_indices('&') { contexts.insert(pos + offset, (false, false)); }
                pos = end;
                continue;
            }
        }
        let ch = rest.chars().next().unwrap();
        if ch == '&' { contexts.insert(pos, (true, quote.is_some())); }
        if let Some(q) = quote { if ch == q { quote = None; } }
        else if tag && matches!(ch, '\'' | '"') { quote = Some(ch); }
        else if ch == '<' { tag = true; }
        else if ch == '>' { tag = false; }
        pos += ch.len_utf8();
    }
    contexts
}

fn failure(message: String) -> Failure {
    Failure {
        message: format!("SAXParseException: {message}"),
        line: None,
        offset: None,
        factory: None,
    }
}

fn recursion(name: &str, stack: &[String], prefix: &str) -> Failure {
    let index = stack.iter().position(|entry| entry == name).unwrap_or(0);
    let mut cycle = vec![format!("{prefix}{name}")];
    if index + 1 < stack.len().saturating_sub(1) {
        cycle.extend(
            stack[index + 1..stack.len() - 1]
                .iter()
                .map(|entry| format!("{prefix}{entry}")),
        );
    }
    if let Some(last) = stack.last() {
        cycle.push(format!("{prefix}{last}"));
    }
    cycle.push(format!("{prefix}{name}"));
    failure(format!(
        "Recursive entity reference \"{prefix}{name}\". (Reference path: {}),",
        cycle.join(" -> ")
    ))
}

impl Reader {
    fn new(xml11: bool) -> Result<Self, Failure> {
        let limits = SecurityLimits::load(ParserFactory::Sax)?;
        let limit = limits.get(Limit::Expansion);
        Ok(Self {
            parameters: BTreeMap::new(),
            generals: BTreeMap::new(),
            parameter_stack: Vec::new(),
            general_stack: Vec::new(),
            expansions: 0,
            expansion_limit: limit,
            limits,
            woodstox: crate::xml_factory::sax_provider()? == crate::xml_factory::SaxProvider::Woodstox,
            xml11,
            total_size: 0,
            replacements: 0,
            origin: None,
            node_locations: Vec::new(),
        })
    }

    fn read_external(&self, location: &str, base: Option<&reqwest::Url>) -> Result<(String, reqwest::Url, Option<Origin>), Failure> {
        // Xerces first tries its ASCII URI parser, then java.net.URI. A
        // relative ID combining non-ASCII and unescaped whitespace fails both;
        // fixURI only escapes spaces, so its final ASCII attempt also fails.
        // The original then opens the unexpanded ID as a URL.
        if !self.woodstox && !location.contains(':') && !location.is_ascii()
            && location.chars().any(|ch| ch.is_ascii_whitespace()) {
            return Err(Failure { message: format!("MalformedURLException: no protocol: {location}"), line: None, offset: None, factory: None });
        }
        let (value, url, source, offset) = external::read_source(location, base, self.woodstox)
            .map_err(|error| {
                if error.message.starts_with("SAXParseException: ") {
                    error
                } else {
                    self.origin.as_ref().map(|origin| origin.io_error(error.clone())).unwrap_or(error)
                }
            })?;
        let origin = self.origin.as_ref().map(|caller| Origin::external(&source, offset, &url).called_from(caller, true));
        Ok((value, url, origin))
    }

    fn recursive_error(&self, name: &str, stack: &[String], prefix: &str) -> Failure {
        self.origin.as_ref().map(|origin| origin.error(format!("Illegal entity expansion: entity \"{name}\" expands itself recursively.")))
            .unwrap_or_else(|| recursion(name, stack, prefix))
    }

    fn size(
        &mut self,
        name: &str,
        parameter: bool,
        size: u64,
        added: u64,
        declaration: bool,
    ) -> Result<(), Failure> {
        self.total_size += added;
        let limit = if parameter {
            Limit::ParameterSize
        } else {
            Limit::GeneralSize
        };
        if self.limits.over(limit, size) {
            let property = if parameter {
                "jdk.xml.maxParameterEntitySizeLimit"
            } else {
                "jdk.xml.maxGeneralEntitySizeLimit"
            };
            let state = if declaration {
                property
            } else {
                self.limits.state(limit)
            };
            return Err(failure(format!("JAXP00010003: The length of entity \"{name}\" is \"{}\" that exceeds the \"{}\" limit set by \"{state}\".", number(size), number(self.limits.get(limit)))));
        }
        if self.limits.over(Limit::TotalSize, self.total_size) {
            let state = if declaration {
                self.limits.state(Limit::TotalSize)
            } else {
                "jdk.xml.totalEntitySizeLimit"
            };
            return Err(failure(format!("JAXP00010004: The accumulated size of entities is \"{}\" that exceeded the \"{}\" limit set by \"{state}\".", number(self.total_size), number(self.limits.get(Limit::TotalSize)))));
        }
        Ok(())
    }

    fn declaration_size(&mut self, name: &str, parameter: bool, text: &str) -> Result<(), Failure> {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let mut rest = normalized.as_str();
        let mut size = 0;
        let name = if parameter {
            format!("%{name}")
        } else {
            name.into()
        };
        loop {
            let end = match rest.find(['&', '<', ']', '\r', '\n']) {
                Some(start) if rest[start..].starts_with('&') => rest[start..]
                    .find(';')
                    .map(|n| start + n + 1)
                    .unwrap_or(rest.len()),
                Some(start) => start + rest[start..].chars().next().unwrap().len_utf8(),
                None => rest.len(),
            };
            let raw = rest[..end].replace("\r\n", "\n").replace('\r', "\n");
            let added = character_references(&raw).encode_utf16().count() as u64;
            size += added;
            self.size(&name, parameter, size, added, true)?;
            if end == rest.len() {
                break;
            }
            rest = &rest[end..];
        }
        Ok(())
    }

    fn fragment(
        &mut self,
        frame: &mut GeneralFrame,
        _start: usize,
        end: usize,
        _reference: bool,
    ) -> Result<(), Failure> {
        if !frame.entity {
            return Ok(());
        }
        while let Some(event) = frame
            .events
            .get(frame.next_event)
            .filter(|event| event.end <= end)
        {
            frame.size += event.size;
            self.size(&frame.name, false, frame.size, event.size, !event.scanner)?;
            frame.next_event += 1;
            if event.node && !frame.attribute {
                self.replacements += 1;
                if self.limits.over(Limit::Replacement, self.replacements) {
                    return Err(failure(format!("JAXP00010007: The total number of nodes in entity references is \"{}\" that is over the limit \"{}\" set by \"{}\".", number(self.replacements), number(self.limits.get(Limit::Replacement)), self.limits.state(Limit::Replacement))));
                }
            }
        }
        Ok(())
    }

    fn expansion(&mut self) -> Result<(), Failure> {
        self.expansions += 1;
        if self.expansion_limit != 0 && self.expansions > self.expansion_limit {
            if self.woodstox { return Err(failure(format!("Maximum entity expansion count limit ({}) exceeded", self.expansion_limit))); }
            return Err(failure(format!("JAXP00010001: The parser has encountered more than \"{}\" entity expansions in this document; this is the limit imposed by \"jdk.xml.entityExpansionLimit\".", self.expansion_limit)));
        }
        Ok(())
    }

    fn parameter(
        &mut self,
        name: &str,
        external_context: bool,
    ) -> Result<Option<(String, Option<reqwest::Url>, bool, Option<Origin>)>, Failure> {
        let Some(entity) = self.parameters.get(name).cloned() else {
            return Ok(None);
        };
        if self.parameter_stack.iter().any(|entry| entry == name) {
            return Err(self.recursive_error(name, &self.parameter_stack, "%"));
        }
        match entity.definition {
            Definition::Literal(value) => {
                self.expansion()?;
                let origin = entity.origin.and_then(|origin| self.origin.as_ref().map(|caller| origin.called_from(caller, true)));
                Ok(Some((value, entity.base, external_context, origin)))
            }
            Definition::System(location) => {
                let (value, base, origin) = self.read_external(&location, entity.base.as_ref())?;
                self.expansion()?;
                Ok(Some((value, Some(base), true, origin)))
            }
        }
    }

    fn value_parameters(&mut self, value: &str, allowed: bool) -> Result<String, Failure> {
        let mut out = String::new();
        let origin = self.origin.clone();
        let mut frames = vec![(value.to_owned(), 0_usize, false, origin.clone())];
        while let Some((text, pos, _, frame_origin)) = frames.last_mut() {
            let Some(start) = text[*pos..].find('%').map(|n| n + *pos) else {
                out.push_str(&text[*pos..]);
                if frames.pop().unwrap().2 {
                    self.parameter_stack.pop();
                }
                continue;
            };
            out.push_str(&text[*pos..start]);
            let Some((name, len)) = parameter_reference(&text[start..]) else {
                if let Some(origin) = frame_origin {
                    let next = text[start + 1..].chars().next();
                    let point = origin.shifted(start + 1 + next.map(char::len_utf8).unwrap_or(1));
                    let message = next.map(|ch| format!("Unexpected character '{}' (code {}) in external DTD subset; expected an identifier", ch, ch as u32))
                        .unwrap_or_else(|| "Unexpected EOF in external DTD subset".into());
                    return Err(point.error(message));
                }
                return Err(failure("The entity name must immediately follow the '%' in the parameter entity reference.".into()));
            };
            if !allowed {
                return Err(failure(format!("The parameter entity reference \"%{name};\" cannot occur within markup in the internal subset of the DTD.")));
            }
            *pos = start + len;
            self.origin = frame_origin.as_ref().map(|o| o.shifted(*pos));
            if let Some((replacement, _, _, origin)) = self.parameter(name, true)? {
                self.parameter_stack.push(name.into());
                frames.push((replacement, 0, true, origin));
            }
        }
        self.origin = origin;
        Ok(out)
    }

    fn markup_parameters(&mut self, declaration: &str, allowed: bool) -> Result<(String, Vec<MarkupSpan>), Failure> {
        let mut out = String::new();
        let mut spans = Vec::new();
        let mut rest = declaration;
        let mut quote = None;
        let origin = self.origin.clone();
        let mut raw_source_start = 0;
        let mut raw_output_start = 0;
        while !rest.is_empty() {
            let ch = rest.chars().next().unwrap();
            if let Some(delimiter) = quote {
                out.push(ch);
                if ch == delimiter {
                    quote = None;
                }
            } else if matches!(ch, '\'' | '"') {
                quote = Some(ch);
                out.push(ch);
            } else if let Some((name, len)) = parameter_reference(rest) {
                if !allowed {
                    return Err(failure(format!("The parameter entity reference \"%{name};\" cannot occur within markup in the internal subset of the DTD.")));
                }
                let source_position = declaration.len() - rest.len();
                if out.len() > raw_output_start {
                    spans.push(MarkupSpan { start: raw_output_start, end: out.len(),
                                            origin: origin.as_ref().map(|o| o.shifted(raw_source_start)),
                                            replacement: false });
                }
                self.origin = origin.as_ref().map(|o| o.shifted(source_position + len));
                if let Some((replacement, _, _, replacement_origin)) = self.parameter(name, true)? {
                    self.parameter_stack.push(name.into());
                    out.push(' ');
                    let replacement_start = out.len();
                    self.origin = replacement_origin.clone();
                    out.push_str(&self.value_parameters(&replacement, true)?);
                    if out.len() > replacement_start {
                        spans.push(MarkupSpan { start: replacement_start, end: out.len(),
                                                origin: replacement_origin, replacement: true });
                    }
                    out.push(' ');
                    self.parameter_stack.pop();
                }
                rest = &rest[len..];
                raw_source_start = source_position + len;
                raw_output_start = out.len();
                continue;
            } else {
                out.push(ch);
            }
            rest = &rest[ch.len_utf8()..];
        }
        if out.len() > raw_output_start {
            spans.push(MarkupSpan { start: raw_output_start, end: out.len(),
                                    origin: origin.as_ref().map(|o| o.shifted(raw_source_start)),
                                    replacement: false });
        }
        self.origin = origin;
        Ok((out, spans))
    }

    fn declaration(
        &mut self,
        declaration: &str,
        base: Option<&reqwest::Url>,
        external_context: bool,
        out: &mut String,
    ) -> Result<(), Failure> {
        let declaration_origin = self.origin.clone();
        if external_context && declaration.starts_with("<!ENTITY") {
            self.external_entity_name(declaration, declaration_origin.as_ref())?;
        }
        let (prepared, spans) = self.markup_parameters(declaration, external_context)?;
        if external_context && !declaration.starts_with("<!ENTITY") {
            self.external_markup_name(&prepared, &spans)?;
        }
        let Some(body) = prepared.strip_prefix("<!ENTITY") else {
            if prepared.starts_with("<!ATTLIST") {
                // Default AttValues are scanned when the declaration is read,
                // including defaults shadowed by an explicit document value.
                let mut pos = 0;
                while let Some(start) = prepared[pos..].find(['\'', '"']).map(|n| n + pos) {
                    let quote = prepared.as_bytes()[start] as char;
                    let Some(end) = prepared[start + 1..].find(quote).map(|n| n + start + 1) else {
                        break;
                    };
                    out.push_str(&prepared[pos..start + 1]);
                    self.origin = declaration_origin.as_ref().map(|o| o.shifted(start + 1));
                    out.push_str(&self.body(&prepared[start + 1..end], true)?);
                    out.push(quote);
                    pos = end + 1;
                }
                out.push_str(&prepared[pos..]);
            } else {
                out.push_str(&prepared);
            }
            self.origin = declaration_origin;
            return Ok(());
        };
        let mut cursor = Cursor::new(body);
        cursor.spaces();
        let parameter = cursor.rest().starts_with('%');
        if parameter {
            cursor.pos += 1;
        }
        let name = cursor.word().to_owned();
        if let Some(message) = name_message(&self.limits, &name, "[xml]") {
            return Err(failure(message));
        }
        cursor.spaces();
        let value_start = cursor.pos;
        let mut value_origin = None;
        let definition = if let Some((quote, raw)) = cursor.quoted() {
            if external_context && !self.woodstox
                && raw.chars().any(|ch| matches!(ch, '\u{fffe}' | '\u{ffff}')) {
                return Err(failure("InvalidCharInLiteral".into()));
            }
            let start = prepared.len() - body.len() + value_start + 1;
            self.origin = declaration_origin.as_ref().map(|o| o.shifted(start));
            let expanded = self.value_parameters(raw, external_context)?;
            self.declaration_size(&name, parameter, &expanded)?;
            if !parameter {
                let encoded = expanded.replace(quote, if quote == '"' { "&#34;" } else { "&#39;" });
                let start = prepared.len() - body.len() + value_start + 1;
                let end = prepared.len() - body.len() + cursor.pos - 1;
                out.push_str(&prepared[..start]);
                out.push_str(&encoded);
                out.push_str(&prepared[end..]);
            }
            let value = character_references(&expanded);
            value_origin = self.origin.as_ref().map(|o| o.replacement(&value));
            Definition::Literal(value)
        } else {
            let kind = cursor.word();
            if kind == "PUBLIC" {
                cursor.quoted();
            }
            let Some((_, location)) = cursor.quoted() else {
                out.push_str(&prepared);
                return Ok(());
            };
            if !parameter {
                out.push_str(&prepared);
            }
            Definition::System(location.into())
        };
        let entity = Entity {
            definition,
            base: base.cloned(),
            origin: value_origin,
        };
        if parameter {
            // The pinned Xerces XML 1.1 path does not resolve references to
            // non-ASCII parameter names declared in the internal subset.
            if self.woodstox || !self.xml11 || external_context || name.is_ascii() {
                self.parameters.entry(name).or_insert(entity);
            }
            out.extend(prepared.chars().filter(|ch| matches!(ch, '\r' | '\n')));
        } else {
            self.generals.entry(name).or_insert(entity);
        }
        self.origin = declaration_origin;
        Ok(())
    }

    fn external_entity_name(
        &self,
        declaration: &str,
        origin: Option<&Origin>,
    ) -> Result<(), Failure> {
        let mut start = "<!ENTITY".len();
        while declaration[start..].starts_with([' ', '\t', '\r', '\n']) {
            start += 1;
        }
        if declaration[start..].starts_with('%') {
            start += 1;
            while declaration[start..].starts_with([' ', '\t', '\r', '\n']) {
                start += 1;
            }
        }
        let mut end = start;
        for (offset, ch) in declaration[start..].char_indices() {
            let valid = if self.woodstox {
                crate::java_xml::woodstox::woodstox_name_character(ch, offset == 0, self.xml11)
            } else {
                super::super::xml_name_character(ch, offset == 0, self.xml11)
            };
            if !valid {
                break;
            }
            end = start + offset + ch.len_utf8();
        }
        let offending = declaration[end..].chars().next();
        if end != start
            && offending.is_some_and(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'))
        {
            return Ok(());
        }
        if self.woodstox {
            let Some(ch) = offending else {
                return Ok(());
            };
            let expected = if end == start { "an identifier" } else { "a separating white space" };
            let message = format!(
                "Unexpected character {} in external DTD subset; expected {expected}",
                crate::java_xml::woodstox::description(ch)
            );
            return Err(origin
                .map(|origin| origin.shifted(end + ch.len_utf8()).error(message.clone()))
                .unwrap_or_else(|| failure(message)));
        }
        if end == start {
            Err(failure("The name of the entity is required in the entity declaration.".into()))
        } else {
            Err(failure(format!(
                "White space is required between the entity name \"{}\" and the definition in the entity declaration.",
                &declaration[start..end]
            )))
        }
    }

    fn external_markup_name(
        &self,
        declaration: &str,
        spans: &[MarkupSpan],
    ) -> Result<(), Failure> {
        let mut start = if declaration.starts_with("<!ELEMENT") {
            "<!ELEMENT".len()
        } else if declaration.starts_with("<!NOTATION") {
            "<!NOTATION".len()
        } else if declaration.starts_with("<!ATTLIST") {
            "<!ATTLIST".len()
        } else {
            return Ok(());
        };
        while declaration[start..].starts_with([' ', '\t', '\r', '\n']) {
            start += 1;
        }
        if declaration.starts_with("<!ELEMENT") {
            let end = self.external_markup_name_at(declaration, start, "element", "", spans)?;
            let mut content = end;
            while declaration[content..].starts_with([' ', '\t', '\r', '\n']) {
                content += 1;
            }
            if declaration[content..].starts_with('(') {
                let rest = declaration[content + 1..].trim_start_matches(super::space);
                let kind = if rest.starts_with("#PCDATA") { "mixed" } else { "child" };
                self.external_group_names(declaration, content, kind, &declaration[start..end], "", spans)?;
            }
            return Ok(());
        }
        if declaration.starts_with("<!NOTATION") {
            self.external_markup_name_at(declaration, start, "notation", "", spans)?;
            return Ok(());
        }
        let end = self.external_markup_name_at(declaration, start, "attlist_owner", "", spans)?;
        let owner = &declaration[start..end];
        start = end;
        loop {
            while declaration[start..].starts_with([' ', '\t', '\r', '\n']) {
                start += 1;
            }
            if declaration[start..].starts_with('>') || start == declaration.len() {
                return Ok(());
            }
            let end = self.external_markup_name_at(declaration, start, "attribute", owner, spans)?;
            let mut cursor = Cursor { text: declaration, pos: end };
            cursor.spaces();
            let kind = cursor.word();
            if kind == "NOTATION" {
                cursor.spaces();
                if cursor.rest().starts_with('(') {
                    self.external_group_names(declaration, cursor.pos, "notation_type", owner,
                                              &declaration[start..end], spans)?;
                }
            } else if kind.is_empty() && cursor.rest().starts_with('(') {
                self.external_group_names(declaration, cursor.pos, "enumerated_type", owner,
                                          &declaration[start..end], spans)?;
            }
            if kind.is_empty() || kind == "NOTATION" {
                if cursor.rest().starts_with('(') {
                    let Some(close) = cursor.rest().find(')') else { return Ok(()); };
                    cursor.pos += close + 1;
                }
            }
            cursor.spaces();
            if cursor.rest().starts_with("#REQUIRED") {
                cursor.pos += "#REQUIRED".len();
            } else if cursor.rest().starts_with("#IMPLIED") {
                cursor.pos += "#IMPLIED".len();
            } else {
                if cursor.rest().starts_with("#FIXED") {
                    cursor.pos += "#FIXED".len();
                }
                if cursor.quoted().is_none() {
                    return Ok(());
                }
            }
            start = cursor.pos;
        }
    }

    fn external_markup_name_at(
        &self,
        declaration: &str,
        start: usize,
        kind: &str,
        owner: &str,
        spans: &[MarkupSpan],
    ) -> Result<usize, Failure> {
        let mut end = start;
        for (offset, ch) in declaration[start..].char_indices() {
            let valid = if !self.woodstox && ch as u32 >= 0x10000
                && MarkupSpan::replacement_at(spans, start + offset) {
                true
            } else if self.woodstox {
                crate::java_xml::woodstox::woodstox_name_character(ch, offset == 0, self.xml11)
            } else {
                super::super::xml_name_character(ch, offset == 0, self.xml11)
            };
            if !valid {
                break;
            }
            end = start + offset + ch.len_utf8();
        }
        let offending = declaration[end..].chars().next();
        if end != start && offending.is_some_and(super::space) {
            return Ok(end);
        }
        if self.woodstox {
            let Some(ch) = offending else {
                return Ok(end);
            };
            let message = if kind == "attlist_owner" || end == start && kind != "notation" {
                format!(
                    "Unexpected character {} (expected a name start character)",
                    crate::java_xml::woodstox::description(ch)
                )
            } else {
                let expected = if end == start { "an identifier" } else { "a separating white space" };
                format!(
                    "Unexpected character {} in external DTD subset; expected {expected}",
                    crate::java_xml::woodstox::description(ch)
                )
            };
            return Err(MarkupSpan::consumed(spans, end, ch.len_utf8())
                .map(|origin| origin.error(message.clone()))
                .unwrap_or_else(|| failure(message)));
        }
        let name = &declaration[start..end];
        let message = match (kind, end == start) {
            ("element", true) => "The element type is required in the element type declaration.".into(),
            ("element", false) => format!("White space is required after the element type \"{name}\" in the element type declaration."),
            ("attlist_owner", true) => "The element type is required in the attribute-list declaration.".into(),
            ("attlist_owner", false) => format!("White space is required before the attribute name in the attribute-list declaration for element \"{name}\"."),
            ("attribute", true) => format!("The attribute name must be specified in the attribute-list declaration for element \"{owner}\"."),
            ("attribute", false) => format!("White space is required before the attribute type in the declaration of attribute \"{name}\" for element \"{owner}\"."),
            ("notation", true) => "The name of the notation is required in the notation declaration.".into(),
            ("notation", false) => format!("White space is required after the notation name \"{name}\" in the notation declaration."),
            _ => unreachable!(),
        };
        Err(failure(message))
    }

    fn external_group_names(
        &self,
        declaration: &str,
        opening: usize,
        kind: &str,
        owner: &str,
        attribute: &str,
        spans: &[MarkupSpan],
    ) -> Result<(), Failure> {
        if kind == "enumerated_type" {
            return self.external_enumerated_names(declaration, opening, attribute, spans);
        }
        let mut pos = opening + 1;
        while pos < declaration.len() {
            let rest = &declaration[pos..];
            if rest.starts_with('>')
                || (kind == "notation_type" && rest.starts_with(')'))
            {
                return Ok(());
            }
            if kind == "mixed" && rest.starts_with("#PCDATA") {
                pos += "#PCDATA".len();
                continue;
            }
            let ch = rest.chars().next().unwrap();
            if super::space(ch) || matches!(ch, '(' | ')' | '|' | ',' | '?' | '*' | '+') {
                pos += ch.len_utf8();
                continue;
            }
            let start = pos;
            while pos < declaration.len() {
                let ch = declaration[pos..].chars().next().unwrap();
                let valid = if !self.woodstox && ch as u32 >= 0x10000
                    && MarkupSpan::replacement_at(spans, pos) {
                    true
                } else if self.woodstox {
                    crate::java_xml::woodstox::woodstox_name_character(
                        ch,
                        pos == start,
                        self.xml11,
                    )
                } else {
                    super::super::xml_name_character(
                        ch,
                        pos == start,
                        self.xml11,
                    )
                };
                if !valid {
                    break;
                }
                pos += ch.len_utf8();
            }
            let offending = declaration[pos..].chars().next();
            if pos > start && offending.is_some_and(|ch| {
                super::space(ch) || matches!(ch, ')' | '|' | ',' | '?' | '*' | '+')
            }) {
                continue;
            }
            let Some(ch) = offending else { return Ok(()); };
            let first = pos == start;
            if self.woodstox {
                let description = crate::java_xml::woodstox::description(ch);
                let message = match (kind, first) {
                    ("notation_type", true) => format!("Unexpected character {description} in external DTD subset; expected an identifier"),
                    ("notation_type", false) => format!("Unexpected character {description} in external DTD subset; missing '|' separator?"),
                    (_, true) => format!("Unexpected character {description} (expected a name start character)"),
                    ("child", false) => format!("Unexpected character {description} in external DTD subset (missing separator '|' or ','?)"),
                    ("mixed", false) => format!("Unexpected character {description} in external DTD subset; expected either '|' to separate elements, or ')' to close the list"),
                    _ => unreachable!(),
                };
                return Err(MarkupSpan::consumed(spans, pos, ch.len_utf8())
                    .map(|origin| origin.error(message.clone()))
                    .unwrap_or_else(|| failure(message)));
            }
            let message = match (kind, first) {
                ("notation_type", true) => format!("The notation name is required in the notation type list for the \"{attribute}\" attribute declaration."),
                ("notation_type", false) => format!("The notation type list must end with ')' in the \"{attribute}\" attribute declaration."),
                ("child", true) => format!("A '(' character or an element type is required in the declaration of element type \"{owner}\"."),
                ("child", false) => format!("A ')' is required in the declaration of element type \"{owner}\"."),
                ("mixed", true) => format!("An element type is required in the declaration of element type \"{owner}\"."),
                ("mixed", false) => format!(r#"The mixed content model "{owner}" must end with ")*" when the types of child elements are constrained."#),
                _ => unreachable!(),
            };
            return Err(failure(message));
        }
        Ok(())
    }

    fn external_enumerated_names(
        &self,
        declaration: &str,
        opening: usize,
        attribute: &str,
        spans: &[MarkupSpan],
    ) -> Result<(), Failure> {
        let mut pos = opening + 1;
        let mut expect_token = true;
        let mut seen_token = false;
        while pos < declaration.len() {
            let ch = declaration[pos..].chars().next().unwrap();
            if super::space(ch) {
                pos += ch.len_utf8();
                continue;
            }
            if expect_token {
                let start = pos;
                while pos < declaration.len() {
                    let ch = declaration[pos..].chars().next().unwrap();
                    let valid = if !self.woodstox && ch as u32 >= 0x10000
                        && MarkupSpan::replacement_at(spans, pos)
                    {
                        true
                    } else if self.woodstox {
                        crate::java_xml::woodstox::woodstox_name_character(ch, false, self.xml11)
                    } else {
                        super::super::xml_name_character(ch, false, self.xml11)
                    };
                    if !valid {
                        break;
                    }
                    pos += ch.len_utf8();
                }
                if pos > start {
                    expect_token = false;
                    seen_token = true;
                    continue;
                }
            } else if ch == '|' {
                pos += 1;
                expect_token = true;
                continue;
            } else if ch == ')' {
                return Ok(());
            }
            let message = if self.woodstox {
                let description = crate::java_xml::woodstox::description(ch);
                if expect_token && ch == ')' && !seen_token {
                    format!("Unexpected character {description} in external DTD subset (empty list; missing identifier(s))?")
                } else if expect_token {
                    format!("Unexpected character {description} in external DTD subset; expected a NMTOKEN character to start a NMTOKEN")
                } else {
                    format!("Unexpected character {description} in external DTD subset; missing '|' separator?")
                }
            } else if expect_token {
                format!("The name token is required in the enumerated type list for the \"{attribute}\" attribute declaration.")
            } else {
                format!("The enumerated type list must end with ')' in the \"{attribute}\" attribute declaration.")
            };
            return Err(if self.woodstox {
                MarkupSpan::consumed(spans, pos, ch.len_utf8())
                    .map(|origin| origin.error(message.clone()))
                    .unwrap_or_else(|| failure(message))
            } else {
                failure(message)
            });
        }
        Ok(())
    }

    fn conditional(
        &mut self,
        text: &str,
        external_context: bool,
    ) -> Result<(usize, Option<String>), Failure> {
        if !external_context {
            if let Some(origin) = &self.origin {
                return Err(origin.shifted(3).error("Internal DTD subset can not use (INCLUDE/IGNORE) directives (except via external entities)"));
            }
            return Err(failure("The markup declarations contained or pointed to by the document type declaration must be well-formed.".into()));
        }
        let Some(open) = text[3..].find('[').map(|n| n + 3) else {
            return Err(failure(
                "The conditional section must start with <![INCLUDE[ or <![IGNORE[.".into(),
            ));
        };
        let keyword = self.value_parameters(&text[3..open], true)?;
        let keyword = keyword.trim_matches(space);
        if !matches!(keyword, "INCLUDE" | "IGNORE") {
            return Err(failure(
                "The conditional section must start with <![INCLUDE[ or <![IGNORE[.".into(),
            ));
        }
        let mut depth = 1;
        let mut pos = open + 1;
        while pos < text.len() {
            let rest = &text[pos..];
            if keyword == "INCLUDE" {
                let close = if rest.starts_with("<!--") { Some("-->") } else if rest.starts_with("<?") { Some("?>") } else { None };
                if let Some(close) = close {
                    pos += rest.find(close).map(|n| n + close.len()).unwrap_or(rest.len());
                    continue;
                }
            }
            if rest.starts_with("<![") {
                depth += 1;
                pos += 3;
            } else if rest.starts_with("]]>") {
                depth -= 1;
                if depth == 0 {
                    return Ok((
                        pos + 3,
                        (keyword == "INCLUDE").then(|| text[open + 1..pos].to_owned()),
                    ));
                }
                pos += 3;
            } else {
                pos += rest.chars().next().unwrap().len_utf8();
            }
        }
        Err(failure(
            "XML document structures must start and end within the same entity.".into(),
        ))
    }

    fn subset(
        &mut self,
        text: &str,
        base: Option<&reqwest::Url>,
        external_context: bool,
        out: &mut String,
    ) -> Result<(), Failure> {
        struct Frame {
            text: String,
            pos: usize,
            base: Option<reqwest::Url>,
            external: bool,
            parameter: bool,
            origin: Option<Origin>,
        }
        let mut frames = vec![Frame {
            text: text.into(),
            pos: 0,
            base: base.cloned(),
            external: external_context,
            parameter: false,
            origin: self.origin.clone(),
        }];
        while let Some(frame) = frames.last_mut() {
            self.origin = frame.origin.as_ref().map(|o| o.shifted(frame.pos));
            let rest = &frame.text[frame.pos..];
            if rest.is_empty() {
                if frames.pop().unwrap().parameter {
                    self.parameter_stack.pop();
                }
                continue;
            }
            if rest.starts_with("<!--") || rest.starts_with("<?") {
                let end = if rest.starts_with("<!--") {
                    rest.find("-->").map(|n| n + 3)
                } else {
                    rest.find("?>").map(|n| n + 2)
                }
                .unwrap_or(rest.len());
                out.push_str(&rest[..end]);
                frame.pos += end;
            } else if rest.starts_with("<![") {
                let origin = frame.origin.as_ref().map(|o| o.shifted(frame.pos + rest[3..].find('[').map(|n| n + 4).unwrap_or(3)));
                let (end, included) = self.conditional(rest, frame.external)?;
                frame.pos += end;
                if let Some(text) = included {
                    let base = frame.base.clone();
                    frames.push(Frame {
                        text,
                        pos: 0,
                        base,
                        external: true,
                        parameter: false,
                        origin,
                    });
                }
            } else if rest.starts_with("<!") {
                let Some(end) = declaration_end(rest) else {
                    if let Some(origin) = &frame.origin {
                        return Err(origin.shifted(frame.pos + rest.len()).error(if frame.external { "Unexpected EOF in external DTD subset" } else { "Unexpected EOF in internal DTD subset" }));
                    }
                    if let Some(body) = rest.strip_prefix("<!ENTITY") {
                        let mut cursor = Cursor::new(body);
                        cursor.spaces();
                        if cursor.rest().starts_with('%') {
                            cursor.pos += 1;
                        }
                        return Err(failure(format!(
                            "The declaration for the entity \"{}\" must end with '>'.",
                            cursor.word()
                        )));
                    }
                    out.push_str(rest);
                    frame.pos = frame.text.len();
                    continue;
                };
                self.declaration(&rest[..end], frame.base.as_ref(), frame.external, out)?;
                frame.pos += end;
            } else if let Some((name, len)) = parameter_reference(rest) {
                let name = name.to_owned();
                frame.pos += len;
                self.origin = frame.origin.as_ref().map(|o| o.shifted(frame.pos));
                if let Some((text, base, external, origin)) = self.parameter(&name, frame.external)? {
                    self.parameter_stack.push(name);
                    frames.push(Frame {
                        text,
                        pos: 0,
                        base,
                        external,
                        parameter: true,
                        origin,
                    });
                }
            } else {
                let ch = rest.chars().next().unwrap();
                out.push(ch);
                frame.pos += ch.len_utf8();
            }
        }
        Ok(())
    }

    fn body(&mut self, text: &str, parent_attribute: bool) -> Result<String, Failure> {
        fn append(out: &mut String, text: &str, escaped: bool) {
            if escaped {
                // SAX normalizes replacement-text whitespace in attributes.
                // Keep those replacement newlines out of the uploaded source
                // line coordinates used by generated @source comments.
                out.push_str(
                    &text
                        .replace(['\t', '\r', '\n'], " ")
                        .replace('\"', "&#34;")
                        .replace('\'', "&#39;"),
                );
            } else {
                out.push_str(text);
            }
        }
        let mut out = String::new();
        let mut resumed_location = None;
        let (node_tokens, ends_markup) = woodstox::node_tokens(text);
        let mut frames = vec![GeneralFrame {
            text: text.into(),
            pos: 0,
            attribute: parent_attribute,
            entity: false,
            name: String::new(),
            size: 0,
            events: Vec::new(),
            next_event: 0,
            contexts: reference_contexts(text),
            origin: self.origin.clone(),
            node_tokens,
            next_node: 0,
            ends_markup,
        }];
        while let Some(frame) = frames.last_mut() {
            let Some(start) = frame.text[frame.pos..].find('&').map(|n| n + frame.pos) else {
                frame.record_nodes(frame.pos, frame.text.len(), &mut self.node_locations, &mut resumed_location);
                self.fragment(frame, frame.pos, frame.text.len(), false)?;
                append(
                    &mut out,
                    &frame.text[frame.pos..],
                    frame.entity && frame.attribute,
                );
                let finished = frames.pop().unwrap();
                if finished.entity {
                    self.general_stack.pop();
                    // Woodstox snapshots the next token's location before
                    // popping an exhausted markup entity. The first adjacent
                    // parent tag retains that row; whitespace starts a new token.
                    if !finished.attribute && finished.ends_markup && resumed_location.is_none() {
                        resumed_location = finished.origin.as_ref().map(|o| o.shifted(finished.text.len()).line());
                    }
                }
                continue;
            };
            frame.record_nodes(frame.pos, start, &mut self.node_locations, &mut resumed_location);
            self.fragment(frame, frame.pos, start, false)?;
            append(
                &mut out,
                &frame.text[frame.pos..start],
                frame.entity && frame.attribute,
            );
            let rest = &frame.text[start + 1..];
            let Some(end) = rest.find(';') else {
                append(
                    &mut out,
                    &frame.text[start..],
                    frame.entity && frame.attribute,
                );
                frame.pos = frame.text.len();
                continue;
            };
            let name = &rest[..end];
            let (active, attribute) = frame.contexts.get(&start).copied().unwrap_or((true, false));
            let attribute = frame.attribute || attribute;
            if active && !name.starts_with('#') {
                if let Some(message) = name_message(&self.limits, name, "[xml]") {
                    return Err(failure(message));
                }
            }
            frame.pos = start + end + 2;
            self.origin = frame.origin.as_ref().map(|o| o.shifted(frame.pos));
            let owned_name = name.to_owned();
            self.fragment(frame, start, frame.pos, true)?;
            let name = owned_name.as_str();
            let entity = if active && !matches!(name, "amp" | "lt" | "gt" | "quot" | "apos") {
                self.generals.get(name).cloned()
            } else {
                None
            };
            if let Some(entity) = entity {
                if self.general_stack.iter().any(|entry| entry == name) {
                    return Err(self.recursive_error(name, &self.general_stack, ""));
                }
                let (value, external, origin) = match entity.definition {
                    Definition::Literal(value) => {
                        let origin = entity.origin.and_then(|o| self.origin.as_ref().map(|caller| o.called_from(caller, true)));
                        (value, false, origin)
                    }
                    Definition::System(location) => {
                        if attribute {
                            if let Some(origin) = &self.origin {
                                return Err(origin.error(format!("Encountered a reference to external parsed entity \"{name}\" when expanding attribute value: not legal as per XML 1.0/1.1 #3.1")));
                            }
                            return Err(failure(format!("The external entity reference \"&{name};\" is not permitted in an attribute value.")));
                        }
                        let (value, _, origin) = self.read_external(&location, entity.base.as_ref())?;
                        (value, true, origin)
                    }
                };
                self.expansion()?;
                if self.woodstox && self.general_stack.len() >= 500 {
                    return Err(failure("Maximum entity expansion depth limit (500) exceeded".into()));
                }
                self.general_stack.push(name.into());
                let events = consumptions(&value, attribute, external);
                let contexts = reference_contexts(&value);
                let (node_tokens, ends_markup) = woodstox::node_tokens(&value);
                let owned_name = name.to_owned();
                frames.push(GeneralFrame {
                    text: value,
                    pos: 0,
                    attribute,
                    entity: true,
                    name: owned_name,
                    size: 0,
                    events,
                    next_event: 0,
                    contexts,
                    origin,
                    node_tokens,
                    next_node: 0,
                    ends_markup,
                });
            } else {
                self.fragment(frame, start, frame.pos, true)?;
                append(
                    &mut out,
                    &frame.text[start..frame.pos],
                    frame.entity && frame.attribute,
                );
            }
        }
        Ok(out)
    }
}

pub(super) fn subsets(
    internal: &str,
    external_location: Option<&str>,
    body: &str,
    source: &str,
    internal_start: usize,
    body_start: usize,
) -> Result<(String, String, Vec<(String, usize)>), Failure> {
    let xml11 = super::super::declaration(source).get("version").is_some_and(|version| version == "1.1");
    let mut reader = Reader::new(xml11)?;
    if reader.woodstox { reader.origin = Some(Origin::document(source, internal_start)); }
    let mut subset = String::new();
    reader.subset(internal, None, false, &mut subset)?;
    // The internal DTD scanner resets general and total sizes before the
    // external subset/document is scanned; expansion and node counts persist.
    reader.total_size = 0;
    if let Some(location) = external_location {
        if reader.woodstox { reader.origin = Some(Origin::document(source, body_start)); }
        let (external_text, base, origin) = reader.read_external(location, None)?;
        reader.origin = origin.map(|o| o.called_from(&Origin::input_start(), false));
        reader.expansion()?;
        let mut expanded = String::new();
        reader.subset(&external_text, Some(&base), true, &mut expanded)?;
        // External entity line numbers are independent of the uploaded stream.
        subset.push_str(&expanded.replace(['\r', '\n'], " "));
    }
    if reader.woodstox { reader.origin = Some(Origin::document(source, body_start)); }
    let body = reader.body(body, false)?;
    Ok((subset, body, reader.node_locations))
}
