//! Java strings retain individual UTF-16 code units until the HTTP UTF-8 boundary.
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub(super) struct Text(pub Vec<u16>);

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self(value.encode_utf16().collect())
    }
}
impl From<String> for Text {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
impl Text {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn push(&mut self, ch: char) {
        self.0.extend(ch.encode_utf16(&mut [0; 2]).iter().copied());
    }
    pub fn push_str(&mut self, value: &str) {
        self.0.extend(value.encode_utf16());
    }
    pub fn append(&mut self, value: &Self) {
        self.0.extend_from_slice(&value.0);
    }
    pub fn lines(&self) -> impl Iterator<Item = Self> + '_ {
        self.0
            .split(|unit| *unit == 10)
            .map(|units| Self(units.to_vec()))
    }
    pub fn trim_start_indent(self) -> Self {
        Self(
            self.0
                .into_iter()
                .skip_while(|c| matches!(c, 9 | 32))
                .collect(),
        )
    }
    pub fn trim(self) -> Self {
        let start = self.0.iter().position(|c| *c > 32).unwrap_or(self.len());
        let end = self
            .0
            .iter()
            .rposition(|c| *c > 32)
            .map(|p| p + 1)
            .unwrap_or(start);
        Self(self.0[start..end].to_vec())
    }
    pub fn trim_end_newlines(mut self) -> Self {
        while self.0.last() == Some(&10) {
            self.0.pop();
        }
        self
    }
    pub fn valid_string(&self) -> Option<String> {
        String::from_utf16(&self.0).ok()
    }
    pub fn utf8(&self) -> String {
        // String.getBytes(UTF_8), as used by the reference HTTP response, uses
        // one ASCII '?' for each malformed surrogate (not U+FFFD).
        char::decode_utf16(self.0.iter().copied())
            .map(|ch| ch.unwrap_or('?'))
            .collect()
    }
    pub fn json_literal(&self) -> String {
        if let Some(value) = self.valid_string() {
            return serde_json::to_string(&value).unwrap();
        }
        let mut result = String::from("\"");
        for unit in &self.0 {
            use std::fmt::Write;
            write!(result, "\\u{unit:04x}").unwrap();
        }
        result.push('"');
        result
    }
    pub fn from_json_literal(input: &str) -> Option<Self> {
        let inner = input.strip_prefix('"')?.strip_suffix('"')?;
        let mut units = Vec::new();
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                let escaped = chars.next()?;
                if escaped == 'u' {
                    let hex = chars.by_ref().take(4).collect::<String>();
                    if hex.len() != 4 {
                        return None;
                    }
                    units.push(u16::from_str_radix(&hex, 16).ok()?);
                } else {
                    units.push(match escaped {
                        '"' => 34,
                        '\\' => 92,
                        '/' => 47,
                        'b' => 8,
                        'f' => 12,
                        'n' => 10,
                        'r' => 13,
                        't' => 9,
                        _ => return None,
                    });
                }
            } else {
                if ch == '"' || ch < '\u{20}' {
                    return None;
                }
                units.extend(ch.encode_utf16(&mut [0; 2]).iter().copied());
            }
        }
        Some(Self(units))
    }
    pub fn from_value(value: &Value) -> Option<Self> {
        if let Some(value) = value.as_str() {
            return Some(Self::from(value));
        }
        let object = value.as_object()?;
        if object.len() != 1 {
            return None;
        }
        let units = object.get("__stg_utf16_string")?.as_array()?;
        units
            .iter()
            .map(|v| u16::try_from(v.as_u64()?).ok())
            .collect::<Option<Vec<_>>>()
            .map(Self)
    }
    pub fn into_value(self) -> Value {
        match self.valid_string() {
            Some(value) => Value::String(value),
            None => serde_json::json!({"__stg_utf16_string":self.0}),
        }
    }
}
