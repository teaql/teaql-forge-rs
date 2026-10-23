//! The pinned generator's byte-stream SAX and UTF-8 String/DOM XML paths.
use std::collections::BTreeMap;
mod external;
mod dtd;
mod security;
mod woodstox;

pub(crate) fn parse_model(source: &crate::input::ResolvedModel, filename: &str) -> Result<teaql_forge_model::ir::Domain, teaql_forge_model::error::ParseError> {
    use teaql_forge_model::parser::SourceLocations;
    let resolve = |position, path: &str| {
        if path != filename { return None; }
        source.model_node_source_location(position).map(|(source, line)| (source.filename.clone(), line))
    };
    let locations = if crate::xml_factory::sax_provider().ok() == Some(crate::xml_factory::SaxProvider::Woodstox) {
        SourceLocations::Resolve(&resolve)
    } else {
        SourceLocations::EndOfStartTag
    };
    teaql_forge_model::parser::parse_model_with_locations(&source.model_xml, filename, locations)
}

#[derive(Clone, Debug)]
pub(crate) struct Failure {
    pub message: String,
    pub line: Option<usize>,
    pub offset: Option<usize>,
    pub factory: Option<crate::xml_factory::XmlFactoryError>,
}

impl From<crate::xml_factory::XmlFactoryError> for Failure {
    fn from(error: crate::xml_factory::XmlFactoryError) -> Self {
        Self { message: error.to_string(), line: None, offset: None, factory: Some(error) }
    }
}

fn failure(text: &str, offset: usize, message: String) -> Failure {
    Failure {
        message: format!("SAXParseException: {message}"),
        line: Some(line_at(text, offset)),
        offset: Some(offset),
        factory: None,
    }
}

fn line_at(text: &str, offset: usize) -> usize {
    let mut line = 1;
    let mut cr = false;
    for ch in text[..offset.min(text.len())].chars() {
        if ch == '\r' || (ch == '\n' && !cr) {
            line += 1;
        }
        cr = ch == '\r';
    }
    line
}

/// Java replaces a complete UTF-8 encoding of a surrogate once, rather than
/// replacing each byte separately as Rust's lossy UTF-8 conversion does.
pub(crate) fn utf8_string(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                out.push_str(text);
                break;
            }
            Err(error) => {
                let end = error.valid_up_to();
                out.push_str(std::str::from_utf8(&rest[..end]).unwrap());
                rest = &rest[end..];
                let surrogate = rest.len() >= 3
                    && rest[0] == 0xed
                    && (0xa0..=0xbf).contains(&rest[1])
                    && (0x80..=0xbf).contains(&rest[2]);
                let count = if surrogate {
                    3
                } else {
                    error.error_len().unwrap_or(rest.len())
                };
                out.push('\u{fffd}');
                rest = &rest[count..];
            }
        }
    }
    out
}

fn declaration(text: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let Some(mut text) = text.strip_prefix("<?xml") else {
        return values;
    };
    while !text.is_empty() && !text.starts_with("?>") {
        text = text.trim_start_matches([' ', '\t', '\r', '\n']);
        let end = text
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
            .unwrap_or(text.len());
        if end == 0 {
            break;
        }
        let name = &text[..end];
        text = text[end..].trim_start_matches([' ', '\t', '\r', '\n']);
        let Some(next) = text.strip_prefix('=') else {
            break;
        };
        text = next.trim_start_matches([' ', '\t', '\r', '\n']);
        let Some(quote) = text.chars().next().filter(|ch| matches!(ch, '\'' | '"')) else {
            break;
        };
        text = &text[1..];
        let Some(end) = text.find(quote) else { break };
        values.insert(name.into(), text[..end].into());
        text = &text[end + 1..];
    }
    values
}

fn strict_utf8(bytes: &[u8]) -> Result<String, Failure> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| {
            let offset = error.valid_up_to();
            let remaining = &bytes[offset..];
            let lead = remaining[0];
            let width = if (0xc2..=0xdf).contains(&lead) {
                2
            } else if (0xe0..=0xef).contains(&lead) {
                3
            } else if (0xf0..=0xf4).contains(&lead) {
                4
            } else {
                1
            };
            let invalid = if width == 1 {
                1
            } else if remaining.len() > 1
                && ((lead == 0xed && remaining[1] >= 0xa0)
                    || (lead == 0xe0 && remaining[1] < 0xa0)
                    || (lead == 0xf0 && remaining[1] < 0x90)
                    || (lead == 0xf4 && remaining[1] >= 0x90))
            {
                2
            } else {
                error.error_len().unwrap_or(remaining.len()) + 1
            };
            let kind = if error.error_len().is_none() {
                "Expected"
            } else {
                "Invalid"
            };
            let prefix = std::str::from_utf8(&bytes[..offset]).unwrap();
            if let Err(earlier) = validate(prefix, false) {
                if prefix.encode_utf16().count() >= 5
                    && earlier
                        .offset
                        .is_some_and(|position| position < prefix.len())
                    && !earlier
                        .message
                        .contains("XML document structures must start")
                    && !earlier.message.contains("Premature end of file")
                {
                    return earlier;
                }
            }
            failure(
                prefix,
                prefix.len(),
                format!("{kind} byte {invalid} of {width}-byte UTF-8 sequence."),
            )
        })
}

fn decode(bytes: &[u8]) -> Result<String, Failure> {
    let mut data = bytes;
    let mut detected = "UTF-8";
    let mut utf32_le = false;
    if data.starts_with(&[0xfe, 0xff]) {
        data = &data[2..];
        detected = "UTF-16BE";
    } else if data.starts_with(&[0xff, 0xfe]) {
        data = &data[2..];
        detected = "UTF-16LE";
    } else if data.starts_with(&[0xef, 0xbb, 0xbf]) {
        data = &data[3..];
    } else if data.starts_with(&[0, 0, 0, b'<']) {
        detected = "UCS-4BE";
    } else if data.starts_with(&[b'<', 0, 0, 0]) {
        detected = "UCS-4LE";
        utf32_le = true;
    } else if data.starts_with(&[0, b'<', 0, b'?']) {
        detected = "UTF-16BE";
    } else if data.starts_with(&[b'<', 0, b'?', 0]) {
        detected = "UTF-16LE";
    }
    let preliminary = match detected {
        "UTF-16BE" | "UTF-16LE" => String::from_utf16_lossy(
            &data
                .chunks_exact(2)
                .map(|p| {
                    if detected == "UTF-16LE" {
                        u16::from_le_bytes([p[0], p[1]])
                    } else {
                        u16::from_be_bytes([p[0], p[1]])
                    }
                })
                .collect::<Vec<_>>(),
        ),
        "UCS-4BE" | "UCS-4LE" => data
            .chunks_exact(4)
            .map(|p| {
                char::from_u32(if utf32_le {
                    u32::from_le_bytes(p.try_into().unwrap())
                } else {
                    u32::from_be_bytes(p.try_into().unwrap())
                })
                .unwrap_or('\u{fffd}')
            })
            .collect(),
        _ => utf8_string(data),
    };
    let attrs = declaration(&preliminary);
    let label = attrs
        .get("encoding")
        .map(String::as_str)
        .unwrap_or(detected);
    let normalized = label
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect::<String>();
    if matches!(detected, "UTF-16BE" | "UTF-16LE")
        && matches!(normalized.as_str(), "UTF16" | "UTF16LE" | "UTF16BE")
    {
        return Ok(preliminary);
    }
    if detected.starts_with("UCS-4") {
        if utf32_le && normalized == "UTF32" {
            let end = preliminary.find("?>").map(|end| end + 2).unwrap_or(0);
            return Ok(preliminary[..end].to_owned() + "\u{fffd}");
        }
        return Ok(preliminary);
    }
    match normalized.as_str() {
        "UTF8" => strict_utf8(data),
        "ISO88591" | "LATIN1" | "L1" | "IBM819" | "CP819" => {
            Ok(data.iter().map(|value| char::from(*value)).collect())
        }
        _ => {
            let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) else {
                return Err(Failure {
                    message: format!("UnsupportedEncodingException: {label}"),
                    line: None,
                    offset: None,
                    factory: None,
                });
            };
            let (value, _) = encoding.decode_without_bom_handling(data);
            let mut value = value.into_owned();
            if normalized == "WINDOWS1252" || normalized == "CP1252" {
                value = data
                    .iter()
                    .map(|byte| {
                        if matches!(byte, 0x81 | 0x8d | 0x8f | 0x90 | 0x9d) {
                            '\u{fffd}'
                        } else {
                            encoding
                                .decode_without_bom_handling(&[*byte])
                                .0
                                .chars()
                                .next()
                                .unwrap()
                        }
                    })
                    .collect();
            }
            Ok(value)
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Context {
    area: &'static str,
    tag: String,
    attribute: String,
    root_seen: bool,
}
fn context(text: &str, offset: usize) -> Context {
    let bytes = text.as_bytes();
    let end = offset.min(bytes.len());
    let mut i = 0;
    let mut stack = Vec::<String>::new();
    let mut out = Context {
        area: "prolog",
        ..Context::default()
    };
    while i < end {
        if bytes[i] != b'<' {
            out.area = if stack.is_empty() {
                if out.root_seen {
                    "trailing"
                } else {
                    "prolog"
                }
            } else {
                "text"
            };
            i += 1;
            continue;
        }
        if text[i..].starts_with("<!--")
            || text[i..].starts_with("<![CDATA[")
            || text[i..].starts_with("<?")
        {
            let (start, close, area) = if text[i..].starts_with("<!--") {
                (4, "-->", "comment")
            } else if text[i..].starts_with("<![CDATA[") {
                (9, "]]>", "cdata")
            } else {
                (2, "?>", "instruction")
            };
            out.area = area;
            let next = text[i + start..]
                .find(close)
                .map(|n| i + start + n + close.len())
                .unwrap_or(bytes.len());
            if next > end {
                return out;
            }
            i = next;
            out.area = if stack.is_empty() {
                if out.root_seen {
                    "trailing"
                } else {
                    "prolog"
                }
            } else {
                "text"
            };
            continue;
        }
        let closing = bytes.get(i + 1) == Some(&b'/');
        i += if closing { 2 } else { 1 };
        let start = i;
        while i < bytes.len()
            && !matches!(
                bytes[i],
                b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>' | b'=' | 0
            )
        {
            i += 1;
        }
        out.tag = text[start..i].into();
        out.attribute.clear();
        out.area = "markup";
        while i < end {
            while i < end && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= end {
                break;
            }
            if bytes[i] == b'>' || (bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>')) {
                let self_closing = bytes[i] == b'/';
                if closing {
                    stack.pop();
                } else {
                    out.root_seen = true;
                    if !self_closing {
                        stack.push(out.tag.clone());
                    }
                }
                i += if self_closing { 2 } else { 1 };
                out.area = if stack.is_empty() { "trailing" } else { "text" };
                break;
            }
            let start = i;
            while i < end && !matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b'=' | b'>' | b'/')
            {
                i += 1;
            }
            if i == start {
                i += 1;
                continue;
            }
            out.attribute = text[start..i].into();
            while i < end && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'=') {
                i += 1;
                while i < end && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            out.area = "attribute";
            if let Some(quote) = bytes
                .get(i)
                .filter(|ch| matches!(ch, b'\'' | b'"'))
                .copied()
            {
                i += 1;
                while i < end && bytes[i] != quote {
                    i += 1;
                }
                if i < end {
                    i += 1;
                    out.area = "markup";
                }
            } else {
                if i >= end {
                    break;
                }
                i += 1;
            }
        }
    }
    out
}

fn offset(text: &str, position: roxmltree::TextPos) -> usize {
    let mut row = 1;
    let mut col = 1;
    for (i, ch) in text.char_indices() {
        if row == position.row && col == position.col {
            return i;
        }
        if ch == '\n' {
            row += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    text.len()
}
fn invalid_character(text: &str, index: usize, ch: char) -> Failure {
    let location = context(text, index);
    let tail = match location.area {
        "attribute" => format!(
            "in the value of attribute \"{}\" and element is \"{}\".",
            location.attribute, location.tag
        ),
        "comment" => "in the comment.".into(),
        "cdata" => "in the CDATA section.".into(),
        "prolog" | "" => return failure(text, index, "Content is not allowed in prolog.".into()),
        _ => "in the element content of the document.".into(),
    };
    failure(
        text,
        index,
        format!(
            "An invalid XML character (Unicode: 0x{:x}) was found {tail}",
            ch as u32
        ),
    )
}

fn syntax_failure(text: &str, error: roxmltree::Error) -> Failure {
    use roxmltree::Error::*;
    let index = if matches!(error, NoRootNode | UnclosedRootNode | UnexpectedEndOfStream) {
        text.len()
    } else {
        offset(text, error.pos())
    };
    let place = context(text, index);
    let message=match error {
        NonXmlChar(ch,_)=>return invalid_character(text,index,ch),
        UnexpectedDeclaration(_)=>"The processing instruction target matching \"[xX][mM][lL]\" is not allowed.".into(),
        UnexpectedCloseTag(expected,_,_)=>format!("The element type \"{expected}\" must be terminated by the matching end-tag \"</{expected}>\"."),
        DuplicatedAttribute(name,_)=>format!("Attribute \"{name}\" was already specified for element \"{}\".",place.tag),
        UnknownNamespace(prefix,_)=>format!("The prefix \"{prefix}\" for element \"{}\" is not bound.",place.tag),
        InvalidAttributeValue(_)=>format!("The value of attribute \"{}\" associated with an element type \"{}\" must not contain the '<' character.",place.attribute,place.tag),
        UnknownEntityReference(name,_)=>format!("The entity \"{name}\" was referenced, but not declared."),
        InvalidComment(_)=>"The string \"--\" is not permitted within comments.".into(),
        NoRootNode=>if place.tag.is_empty(){"Premature end of file.".into()}else{"XML document structures must start and end within the same entity.".into()},
        UnclosedRootNode|UnexpectedEndOfStream=>"XML document structures must start and end within the same entity.".into(),
        InvalidName(_)|UnknownToken(_)=>if place.area=="trailing"{if text[index..].starts_with('<'){"The markup in the document following the root element must be well-formed.".into()}else{"Content is not allowed in trailing section.".into()}}else if place.area=="prolog"{"Content is not allowed in prolog.".into()}else{"The markup in the document preceding the root element must be well-formed.".into()},
        MalformedEntityReference(_)=>{
            let start=text[..index.min(text.len())].rfind('&').filter(|start|!text[*start..index.min(text.len())].contains(';')).or_else(||text[index.min(text.len())..].find('&').map(|n|index+n)).unwrap_or(index);
            let value=&text[start..];
            if value.starts_with("&#") {
                let hex=value.starts_with("&#x");let prefix=if hex{3}else{2};
                let digits=value[prefix..].chars().take_while(|ch|if hex{ch.is_ascii_hexdigit()}else{ch.is_ascii_digit()}).collect::<String>();
                if digits.is_empty(){if hex{"A hexadecimal representation must immediately follow the \"&#x\" in a character reference.".into()}else{"A decimal representation must immediately follow the \"&#\" in a character reference.".into()}}
                else{format!("Character reference \"{}{}\" is an invalid XML character.",&value[..prefix],digits)}
            }else{
                let name=value.strip_prefix('&').unwrap_or(value).chars().take_while(|ch|ch.is_alphanumeric() || matches!(ch,'_'|':'|'.'|'-')).collect::<String>();
                format!("The reference to entity \"{name}\" must end with the ';' delimiter.")
            }
        }
        EntityReferenceLoop(_)=>if let Some((name,path))=recursive_entity_path(text) {
            format!("Recursive entity reference \"{name}\". (Reference path: {}),",path.join(" -> "))
        }else{"a possible entity reference loop is detected".into()},
        InvalidChar(_,b'<',_)=>format!("The value of attribute \"{}\" associated with an element type \"{}\" must not contain the '<' character.",place.attribute,place.tag),
        InvalidChar2("a quote",_,_)=>format!("Open quote is expected for attribute \"{}\" associated with an  element type  \"{}\".",place.attribute,place.tag),
        InvalidChar2("a whitespace",_,_)=>format!("Element type \"{}\" must be followed by either attribute specifications, \">\" or \"/>\".",place.tag),
        other=>other.to_string(),
    };
    failure(text, index, message)
}

// JDK 25 Xerces uses the XMLChar (XML 1.0) and XML11Char (XML 1.1)
// name tables. roxmltree accepts some newer XML 1.0 names that XMLChar rejects.
const XML_NAME_FLAGS: &[u8] = include_bytes!("data/java-xml-name-25.bin");

fn xml_name_character(ch: char, first: bool, version11: bool) -> bool {
    let code = ch as u32;
    if code >= 0x10000 {
        return version11 && code < 0xf0000;
    }
    let mask = if version11 {
        if first { 4 } else { 8 }
    } else if first {
        1
    } else {
        2
    };
    XML_NAME_FLAGS[code as usize] & mask != 0
}

fn invalid_xml_name_character(name: &str, version11: bool) -> Option<(usize, &str)> {
    name.char_indices().find_map(|(offset, ch)| {
        (!xml_name_character(ch, offset == 0, version11))
            .then_some((offset, &name[..offset]))
    })
}

// Xerces scans internal DTD declaration names before expanding the document
// body. roxmltree's DTD reader accepts a different set of Unicode names, so
// keep this check on the uploaded text, ahead of our entity expansion.
fn active_entity_reference(text: &str, start: usize, name: &str) -> Option<usize> {
    let reference = format!("&{name};");
    let mut cursor = start;
    while cursor < text.len() {
        let rest = &text[cursor..];
        let close = if rest.starts_with("<!--") { Some("-->") }
            else if rest.starts_with("<![CDATA[") { Some("]]>") }
            else if rest.starts_with("<?") { Some("?>") } else { None };
        if let Some(close) = close {
            cursor += rest.find(close).map_or(rest.len(), |end| end + close.len());
            continue;
        }
        if rest.starts_with(&reference) { return Some(cursor); }
        cursor += rest.chars().next()?.len_utf8();
    }
    None
}

fn jdk_entity_declaration_failure(text: &str) -> Option<Failure> {
    let version11 = declaration(text).get("version").is_some_and(|version| version == "1.1");
    let start = text.match_indices("<!DOCTYPE")
        .find(|(position, _)| !matches!(context(text, *position).area, "comment" | "cdata" | "instruction" | "attribute"))?.0;
    let (finish, range) = dtd::doctype_span(text, start)?;
    let range = range?;
    let mut cursor = range.start;
    let mut named_references = Vec::new();
    while cursor < range.end {
        let rest = &text[cursor..range.end];
        if rest.starts_with("<!--") || rest.starts_with("<?") {
            let close = if rest.starts_with("<!--") { "-->" } else { "?>" };
            cursor += rest.find(close).map_or(rest.len(), |end| end + close.len());
            continue;
        }
        if rest.starts_with("<!ENTITY") {
            let mut name_start = cursor + "<!ENTITY".len();
            while text[name_start..].starts_with([' ', '\t', '\r', '\n']) { name_start += 1; }
            let parameter = text[name_start..].starts_with('%');
            if parameter {
                name_start += 1;
                while text[name_start..].starts_with([' ', '\t', '\r', '\n']) { name_start += 1; }
            }
            if name_start >= range.end {
                return Some(failure(text, name_start,
                    "The name of the entity is required in the entity declaration.".into()));
            }
            let mut name_end = name_start;
            for (offset, ch) in text[name_start..range.end].char_indices() {
                if !xml_name_character(ch, offset == 0, version11) { break; }
                name_end = name_start + offset + ch.len_utf8();
            }
            if name_end == name_start {
                return Some(failure(text, name_start,
                    "The name of the entity is required in the entity declaration.".into()));
            }
            let name = &text[name_start..name_end];
            if !text[name_end..].starts_with([' ', '\t', '\r', '\n']) {
                return Some(failure(text, name_end, format!(
                    "White space is required between the entity name \"{name}\" and the definition in the entity declaration.")));
            }
            if version11 && !parameter && !name.is_ascii() {
                // The pinned JDK 25 SAX path leaves XML 1.1 references to
                // non-ASCII internal entity names unresolved.
                named_references.push(name);
            }
        }
        // Skip complete declarations, including quoted replacement text.
        if rest.starts_with("<!") {
            cursor += 2;
            let mut quote = None;
            while cursor < range.end {
                let ch = text[cursor..].chars().next()?;
                cursor += ch.len_utf8();
                if let Some(expected) = quote {
                    if ch == expected { quote = None; }
                } else if matches!(ch, '\'' | '"') { quote = Some(ch); }
                else if ch == '>' { break; }
            }
        } else {
            cursor += rest.chars().next()?.len_utf8();
        }
    }
    named_references.into_iter().filter_map(|name| {
        active_entity_reference(text, finish, name).map(|offset| (offset, name))
    }).min_by_key(|(position, _)| *position).map(|(position, name)| failure(text, position,
        format!("The entity \"{name}\" was referenced, but not declared.")))
}

fn xml_name_failure(text: &str, document: &roxmltree::Document<'_>, version11: bool) -> Option<Failure> {
    for node in document.descendants() {
        if let Some(instruction) = node.pi() {
            if let Some((offset, _)) = invalid_xml_name_character(instruction.target, version11) {
                let message = if offset == 0 {
                    "The processing instruction must begin with the name of the target."
                } else {
                    "White space is required between the processing instruction target and data."
                };
                return Some(failure(text, node.range().start + 2 + offset, message.into()));
            }
            // roxmltree can treat an invalid continuation character as PI
            // content without requiring the separator that Xerces checks.
            let after_target = node.range().start + 2 + instruction.target.len();
            if !text[after_target..].starts_with("?>")
                && text[after_target..].chars().next().is_some_and(|ch| !matches!(ch, ' ' | '\t' | '\r' | '\n'))
            {
                return Some(failure(text, after_target,
                    "White space is required between the processing instruction target and data.".into()));
            }
            continue;
        }
        if !node.is_element() {
            continue;
        }
        let name_start = node.range().start + 1;
        let name_end = text[name_start..]
            .find([' ', '\t', '\n', '\r', '/', '>'])
            .map_or(text.len(), |relative| name_start + relative);
        let name = &text[name_start..name_end];
        if let Some((offset, prefix)) = invalid_xml_name_character(name, version11) {
            let message = if offset == 0 {
                if node.parent().is_some_and(|parent| parent.is_root()) {
                    "The markup in the document preceding the root element must be well-formed.".into()
                } else {
                    "The content of elements must consist of well-formed character data or markup.".into()
                }
            } else {
                format!("Element type \"{prefix}\" must be followed by either attribute specifications, \">\" or \"/>\".")
            };
            return Some(failure(text, name_start + offset, message));
        }
        for attribute in node.attributes() {
            let range = attribute.range_qname();
            let attribute_name = &text[range.clone()];
            if let Some((offset, prefix)) = invalid_xml_name_character(attribute_name, version11) {
                let message = if offset == 0 {
                    format!("Element type \"{name}\" must be followed by either attribute specifications, \">\" or \"/>\".")
                } else {
                    format!("Attribute name \"{prefix}\" associated with an element type \"{name}\" must be followed by the ' = ' character.")
                };
                return Some(failure(text, range.start + offset, message));
            }
        }
    }
    None
}

#[cfg(test)]
mod xml_name_tests {
    use super::validate_scanned;

    #[test]
    fn jdk_25_xml_name_boundaries_match_dom_and_sax_messages() {
        for (xml, message) in [
            ("<root><nodeſ/></root>",
             "Element type \"node\" must be followed by either attribute specifications, \">\" or \"/>\"."),
            ("<root><𐐀node/></root>",
             "The content of elements must consist of well-formed character data or markup."),
            ("<root aſ=\"v\"/>",
             "Attribute name \"a\" associated with an element type \"root\" must be followed by the ' = ' character."),
            ("<root 𐐀a=\"v\"/>",
             "Element type \"root\" must be followed by either attribute specifications, \">\" or \"/>\"."),
            ("<root><?piſ data?></root>",
             "White space is required between the processing instruction target and data."),
            ("<root><?𐐀pi data?></root>",
             "The processing instruction must begin with the name of the target."),
        ] {
            let error = validate_scanned(xml, false, false).unwrap_err();
            assert_eq!(error.message, format!("SAXParseException: {message}"));
        }
        for xml in [
            "<root><nodeİ/><nodeK/></root>",
            "<?xml version=\"1.1\"?><root><nodeſ/><𐐀node/></root>",
            "<?xml version=\"1.1\"?><root><?piſ data?><?𐐀pi data?></root>",
        ] {
            validate_scanned(xml, false, false).unwrap();
        }
    }
}

fn validate(text: &str, disallow_dtd: bool) -> Result<(), Failure> {
    validate_scanned(text, disallow_dtd, true)
}

fn validate_scanned(text: &str, disallow_dtd: bool, check_limits: bool) -> Result<(), Failure> {
    validate_scanned_with_name_rules(text, disallow_dtd, check_limits, true)
}

fn validate_scanned_with_name_rules(text: &str, disallow_dtd: bool, check_limits: bool, jdk_names: bool) -> Result<(), Failure> {
    let attrs = declaration(text);
    if let Some(version) = attrs
        .get("version")
        .filter(|v| v.as_str() != "1.0" && v.as_str() != "1.1")
    {
        return Err(failure(
            text,
            0,
            format!("XML version \"{version}\" is not supported, only XML 1.0 is supported."),
        ));
    }
    if disallow_dtd {
        if let Some((index, _)) = text.match_indices("<!DOCTYPE").find(|(index, _)| {
            !matches!(
                context(text, *index).area,
                "comment" | "cdata" | "instruction" | "attribute"
            )
        }) {
            return Err(failure(text,index,"DOCTYPE is disallowed when the feature \"http://apache.org/xml/features/disallow-doctype-decl\" set to true.".into()));
        }
    }
    let version11 = attrs.get("version").is_some_and(|v| v == "1.1");
    let restricted = version11
        .then(|| {
            text.char_indices()
                .find(|(_, ch)| matches!(*ch as u32,0x7f..=0x84|0x86..=0x9f))
        })
        .flatten();
    let parsed = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: !disallow_dtd,
            ..Default::default()
        },
    );
    if let Some((index, ch)) = restricted {
        if parsed
            .as_ref()
            .err()
            .is_none_or(|error| offset(text, error.pos()) >= index)
        {
            return Err(invalid_character(text, index, ch));
        }
    }
    let syntax = parsed
        .map(|document| if jdk_names { xml_name_failure(text, &document, version11).map_or(Ok(()), Err) } else { Ok(()) })
        .unwrap_or_else(|error| Err(syntax_failure(text, error)));
    if check_limits {
        if let Err(limit) = security::scan_for(text, crate::xml_factory::ParserFactory::Dom) {
            if syntax.as_ref().err().is_none_or(|error| error.offset.unwrap_or(text.len()) >= limit.offset.unwrap_or(text.len())) {
                return Err(limit);
            }
        }
    }
    syntax
}

pub(crate) fn sax(bytes: &[u8]) -> Result<String, Failure> {
    crate::xml_factory::check_mapper_configuration()?;
    if crate::xml_factory::sax_provider()? == crate::xml_factory::SaxProvider::Woodstox {
        return woodstox::parse(bytes);
    }
    crate::xml_factory::entity_expansion_limit(crate::xml_factory::ParserFactory::Sax)?;
    let decoded = decode(bytes)?;
    if let Some(error) = jdk_entity_declaration_failure(&decoded) { return Err(error); }
    let expanded = external::expand(&decoded)?;
    // The source checks attributes before nonvalidating DTD defaults are added.
    security::scan(&expanded)?;
    let xml = dtd::expand(&expanded)?;
    validate_scanned(&xml, false, false)?;
    Ok(
        if declaration(&xml).get("version").is_some_and(|v| v == "1.1") {
            xml.replace("\r\u{85}", "\n")
                .replace(['\u{85}', '\u{2028}'], "\n")
        } else {
            xml
        },
    )
}

/// Keep locations from the same external reads that produced the accepted XML.
pub(crate) fn sax_document(bytes: &[u8]) -> Result<(String, BTreeMap<usize, usize>), Failure> {
    crate::xml_factory::check_mapper_configuration()?;
    if crate::xml_factory::sax_provider()? == crate::xml_factory::SaxProvider::Woodstox {
        woodstox::parse_document(bytes)
    } else { sax(bytes).map(|xml| (xml, BTreeMap::new())) }
}

pub(crate) fn dom(bytes: &[u8]) -> Result<String, Failure> {
    let raw = utf8_string(bytes);
    let blank = raw.chars().all(crate::input_path::blank_char);
    if !blank {
        crate::xml_factory::entity_expansion_limit(crate::xml_factory::ParserFactory::Dom)?;
    }
    let xml = raw
        .chars()
        .filter(|ch| !matches!(*ch as u32,0..=8|11..=12|14..=31))
        .collect::<String>();
    let parsed = if blank {
        Err(())
    } else if xml.starts_with('\u{feff}') {
        Err(())
    } else {
        validate(&xml, true).map_err(|_| ())
    };
    if parsed.is_ok() {
        // SourceIndex always creates its SAX factory after a successful DOM
        // parse. It catches Exception for best-effort locations, not Error.
        if let Err(error) = crate::xml_factory::source_index_available() {
            if !error.is_bad_request() { return Err(error.into()); }
        }
        return Ok(
            if declaration(&xml).get("version").is_some_and(|v| v == "1.1") {
                xml.replace("\r\u{85}", "\n")
                    .replace(['\u{85}', '\u{2028}'], "\n")
            } else {
                xml
            },
        );
    }
    // SourceIndex catches Exception while recovering a line, but not Error.
    // A malformed properties escape is IllegalArgumentException at the SAX
    // lookup boundary, so the original XML diagnostic survives without a line.
    let index_failure = match crate::xml_factory::source_index_available() {
        Err(error) if error.is_bad_request() => None,
        Err(error) => return Err(error.into()),
        Ok(false) => None,
        Ok(true) => decode(bytes).and_then(|decoded| index_validate(&decoded)).err(),
    };
    Err(Failure {
        message: if blank {
            "XML content string is empty !".into()
        } else {
            "Parse XML from stream error!".into()
        },
        line: index_failure.and_then(|failure| failure.line),
        offset: None,
        factory: None,
    })
}

pub(crate) fn qname<'a, 'input: 'a, N: std::borrow::Borrow<roxmltree::Node<'a, 'input>>>(
    node: N,
) -> &'input str {
    let node = *node.borrow();
    let text = node.document().input_text();
    let start = node.range().start + 1;
    let end = text[start..]
        .find(|ch: char| ch.is_ascii_whitespace() || matches!(ch, '/' | '>'))
        .map(|n| start + n)
        .unwrap_or(text.len());
    &text[start..end]
}

/// SourceIndex records a root location only after its SAX startElement callback.
pub(crate) fn root_line(bytes: &[u8]) -> Option<usize> {
    source_locations(bytes)
        .first()
        .and_then(|location| location.line)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Location {
    pub line: Option<usize>,
    pub attributes: BTreeMap<String, usize>,
}

fn java_space(unit: u16) -> bool {
    matches!(unit,9..=13|0x1c..=0x20|0x1680|0x2000..=0x2006|0x2008..=0x200a|0x2028..=0x2029|0x205f|0x3000)
}

/// Mirrors KsmlSourceIndex's SAX callbacks and UTF-16 offsets into the original
/// UTF-8-decoded upload. A SAX failure leaves later DOM nodes without locations.
pub(crate) fn source_locations(bytes: &[u8]) -> Vec<Location> {
    if !crate::xml_factory::source_index_available().unwrap_or(false) { return Vec::new(); }
    let decoded = decode(bytes);
    let (text, first_error) = match decoded {
        Ok(text) => {
            let error = index_validate(&text).err();
            (text, error)
        }
        Err(error) if error.offset.is_some() => (utf8_string(bytes), Some(error)),
        Err(_) => return Vec::new(),
    };
    let mut callbacks = Vec::new();
    let mut scan = 0;
    while let Some(start) = text[scan..].find('<').map(|n| scan + n) {
        let suffix = &text[start..];
        if suffix.starts_with("<!--") || suffix.starts_with("<![CDATA[") || suffix.starts_with("<?")
        {
            let close = if suffix.starts_with("<!--") {
                "-->"
            } else if suffix.starts_with("<![CDATA[") {
                "]]>"
            } else {
                "?>"
            };
            let Some(end) = suffix.find(close) else { break };
            scan = start + end + close.len();
            continue;
        }
        let mut quote = None;
        let mut end = None;
        for (offset, ch) in suffix.char_indices() {
            if let Some(delimiter) = quote {
                if ch == delimiter {
                    quote = None;
                }
            } else if matches!(ch, '\'' | '"') {
                quote = Some(ch);
            } else if ch == '>' {
                end = Some(start + offset + 1);
                break;
            }
        }
        let Some(end) = end else { break };
        scan = end;
        if suffix.starts_with("</") || suffix.starts_with("<!") {
            continue;
        }
        if first_error
            .as_ref()
            .and_then(|error| error.offset)
            .is_some_and(|offset| offset < end)
        {
            break;
        }
        let mut row = 1;
        let mut column = 1;
        let mut carriage_return = false;
        let version11 = declaration(&text)
            .get("version")
            .is_some_and(|v| v == "1.1");
        for ch in text[..end].chars() {
            if ch == '\r'
                || (ch == '\n' && !carriage_return)
                || (version11 && matches!(ch, '\u{85}' | '\u{2028}'))
            {
                row += 1;
                column = 1;
            } else if ch != '\n' || !carriage_return {
                column += ch.len_utf16();
            }
            carriage_return = ch == '\r';
        }
        callbacks.push((row, column));
    }
    let raw = utf8_string(bytes).encode_utf16().collect::<Vec<_>>();
    let starts = std::iter::once(0)
        .chain(
            raw.iter()
                .enumerate()
                .filter(|(_, ch)| **ch == 10)
                .map(|(i, _)| i + 1),
        )
        .collect::<Vec<_>>();
    let mut cursor = 0;
    let mut line = 1;
    let mut result = Vec::new();
    for (end_line, column) in callbacks {
        let end = starts.get(end_line - 1).map(|start| start + column - 1);
        let start = end.and_then(|end| raw[..end.min(raw.len())].iter().rposition(|ch| *ch == 60));
        let Some(start) = start.filter(|start| *start >= cursor) else {
            result.push(Location {
                line: Some(end_line),
                ..Default::default()
            });
            continue;
        };
        while cursor < start {
            if raw[cursor] == 10 {
                line += 1;
            }
            cursor += 1;
        }
        let mut location = Location {
            line: Some(line),
            ..Default::default()
        };
        let end = end.unwrap().min(raw.len());
        let mut i = start + 1;
        while i < end && !java_space(raw[i]) && raw[i] != 62 {
            i += 1;
        }
        let mut attribute_line = line;
        for unit in &raw[start..i] {
            if *unit == 10 {
                attribute_line += 1;
            }
        }
        while i < end {
            while i < end && java_space(raw[i]) {
                if raw[i] == 10 {
                    attribute_line += 1;
                }
                i += 1;
            }
            if i >= end || matches!(raw[i], 47 | 62) {
                break;
            }
            let begin = i;
            while i < end && !java_space(raw[i]) && !matches!(raw[i], 61 | 62) {
                i += 1;
            }
            if begin == i {
                break;
            }
            location
                .attributes
                .insert(String::from_utf16_lossy(&raw[begin..i]), attribute_line);
            while i < end && java_space(raw[i]) {
                if raw[i] == 10 {
                    attribute_line += 1;
                }
                i += 1;
            }
            if i >= end || raw[i] != 61 {
                break;
            }
            i += 1;
            while i < end && java_space(raw[i]) {
                if raw[i] == 10 {
                    attribute_line += 1;
                }
                i += 1;
            }
            if i >= end {
                break;
            }
            let quote = raw[i];
            i += 1;
            if !matches!(quote, 34 | 39) {
                break;
            }
            while i < end && raw[i] != quote {
                if raw[i] == 10 {
                    attribute_line += 1;
                }
                i += 1;
            }
            if i < end {
                i += 1;
            }
        }
        result.push(location);
    }
    result
}

/// SourceIndex's SAX factory is namespace-unaware. Preserve source offsets while
/// disabling namespace interpretation for its well-formedness/location scan.
fn index_validate(text: &str) -> Result<(), Failure> {
    let mut bytes = text.as_bytes().to_vec();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if text[i..].starts_with("<!--")
            || text[i..].starts_with("<![CDATA[")
            || text[i..].starts_with("<?")
        {
            let close = if text[i..].starts_with("<!--") {
                "-->"
            } else if text[i..].starts_with("<![CDATA[") {
                "]]>"
            } else {
                "?>"
            };
            i = text[i..]
                .find(close)
                .map(|end| i + end + close.len())
                .unwrap_or(bytes.len());
            continue;
        }
        i += 1;
        if bytes.get(i) == Some(&b'/') {
            i += 1;
        }
        while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>') {
            if bytes[i] == b':' {
                bytes[i] = b'_';
            }
            i += 1;
        }
        while i < bytes.len() && bytes[i] != b'>' {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || matches!(bytes[i], b'/' | b'>') {
                break;
            }
            let start = i;
            while i < bytes.len()
                && !matches!(bytes[i], b'=' | b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>')
            {
                if bytes[i] == b':' {
                    bytes[i] = b'_';
                }
                i += 1;
            }
            if &bytes[start..i] == b"xmlns" {
                bytes[start] = b'_';
            }
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'=') {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            if let Some(quote) = bytes
                .get(i)
                .filter(|ch| matches!(ch, b'\'' | b'"'))
                .copied()
            {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        i += 1;
    }
    validate(std::str::from_utf8(&bytes).unwrap(), true)
}

fn recursive_entity_path(text: &str) -> Option<(String, Vec<String>)> {
    let declarations =
        regex::Regex::new(r#"(?s)<!ENTITY\s+([^\s<>%]+)\s+(?:"([^"]*)"|'([^']*)')\s*>"#).unwrap();
    let references = regex::Regex::new(r"&([^;&#\s<>]+);").unwrap();
    let mut entities = BTreeMap::new();
    let mut last_definition = 0;
    for captures in declarations.captures_iter(text) {
        last_definition = last_definition.max(captures.get(0).unwrap().end());
        let value = captures
            .get(2)
            .or_else(|| captures.get(3))
            .unwrap()
            .as_str();
        entities.insert(
            captures[1].to_owned(),
            references
                .captures_iter(value)
                .map(|capture| capture[1].to_owned())
                .collect::<Vec<_>>(),
        );
    }
    fn visit(
        name: &str,
        entities: &BTreeMap<String, Vec<String>>,
        path: &mut Vec<String>,
    ) -> Option<(String, Vec<String>)> {
        if let Some(index) = path.iter().position(|entry| entry == name) {
            let mut cycle = vec![name.to_owned()];
            if index + 1 < path.len().saturating_sub(1) {
                cycle.extend(path[index + 1..path.len() - 1].iter().cloned());
            }
            cycle.push(path.last().unwrap().clone());
            cycle.push(name.to_owned());
            return Some((name.into(), cycle));
        }
        let children = entities.get(name)?;
        path.push(name.into());
        for child in children {
            if let Some(cycle) = visit(child, entities, path) {
                return Some(cycle);
            }
        }
        path.pop();
        None
    }
    for capture in references.captures_iter(&text[last_definition..]) {
        if let Some(cycle) = visit(&capture[1], &entities, &mut Vec::new()) {
            return Some(cycle);
        }
    }
    None
}
