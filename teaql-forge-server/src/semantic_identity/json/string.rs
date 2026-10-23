//! Java strings retain every UTF-16 code unit, including lone surrogates.
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct JavaString(Vec<u16>);

impl From<&str> for JavaString {
    fn from(value: &str) -> Self {
        Self(value.encode_utf16().collect())
    }
}

impl JavaString {
    pub(super) fn is(&self, value: &str) -> bool {
        self.0.iter().copied().eq(value.encode_utf16())
    }
    pub(super) fn is_blank(&self) -> bool {
        teaql_forge_codegen::semantic::is_blank(&String::from_utf16_lossy(&self.0))
    }
    pub(super) fn text(&self) -> Result<String, ()> {
        String::from_utf16(&self.0).map_err(|_| ())
    }
    pub(super) fn write(&self, output: &mut Vec<u8>) {
        output.push(b'"');
        for &unit in &self.0 {
            match unit {
                8 => output.extend_from_slice(br"\b"),
                9 => output.extend_from_slice(br"\t"),
                10 => output.extend_from_slice(br"\n"),
                12 => output.extend_from_slice(br"\f"),
                13 => output.extend_from_slice(br"\r"),
                34 => output.extend_from_slice(br#"\""#),
                92 => output.extend_from_slice(br"\\"),
                0..=31 | 0xd800..=0xdfff => {
                    output.extend_from_slice(format!("\\u{unit:04X}").as_bytes())
                }
                _ => {
                    let mut encoded = [0; 4];
                    output.extend_from_slice(
                        char::from_u32(u32::from(unit))
                            .unwrap()
                            .encode_utf8(&mut encoded)
                            .as_bytes(),
                    );
                }
            }
        }
        output.push(b'"');
    }
}

impl<'de> Deserialize<'de> for JavaString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StringVisitor;
        impl Visitor<'_> for StringVisitor {
            type Value = JavaString;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON string")
            }
            fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<JavaString, E> {
                // serde_json's byte-string reader preserves lone surrogates
                // as WTF-8. Decode those bytes into the original UTF-16 units.
                let mut units = Vec::new();
                let mut index = 0;
                while index < bytes.len() {
                    let first = bytes[index];
                    let (length, mut code) = match first {
                        0..=0x7f => (1, u32::from(first)),
                        0xc2..=0xdf => (2, u32::from(first & 31)),
                        0xe0..=0xef => (3, u32::from(first & 15)),
                        0xf0..=0xf4 => (4, u32::from(first & 7)),
                        _ => return Err(E::custom("invalid UTF-8")),
                    };
                    if index + length > bytes.len() {
                        return Err(E::custom("truncated UTF-8"));
                    }
                    for byte in &bytes[index + 1..index + length] {
                        if byte & 0xc0 != 0x80 {
                            return Err(E::custom("invalid UTF-8 continuation"));
                        }
                        code = (code << 6) | u32::from(byte & 63);
                    }
                    if (length == 2 && code < 0x80)
                        || (length == 3 && code < 0x800)
                        || (length == 4 && code < 0x10000)
                        || code > 0x10ffff
                    {
                        return Err(E::custom("invalid UTF-8 code point"));
                    }
                    if code <= 0xffff {
                        units.push(code as u16);
                    } else {
                        let code = code - 0x10000;
                        units.push(0xd800 + (code >> 10) as u16);
                        units.push(0xdc00 + (code & 1023) as u16);
                    }
                    index += length;
                }
                Ok(JavaString(units))
            }
        }
        deserializer.deserialize_bytes(StringVisitor)
    }
}
