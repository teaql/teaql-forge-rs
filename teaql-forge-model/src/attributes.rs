//! Ordered XML attribute updates used by source domain/type/include parsing.

pub fn blank_char(ch: char) -> bool {
    matches!(ch, '\u{0000}' | '\u{0009}'..='\u{000d}' | '\u{001c}'..='\u{0020}'
        | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'..='\u{202a}'
        | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn merge_xml_attribute(attributes: &mut Vec<(String, String)>, name: &str, value: &str) {
    let (name, append) = name
        .strip_suffix("_delta")
        .map(|name| (name, true))
        .unwrap_or((name, false));
    if let Some((_, existing)) = attributes.iter_mut().find(|(key, _)| key == name) {
        if append {
            existing.push(',');
            existing.push_str(value);
        } else {
            *existing = value.to_string();
        }
    } else {
        attributes.push((name.to_string(), value.to_string()));
    }
}
