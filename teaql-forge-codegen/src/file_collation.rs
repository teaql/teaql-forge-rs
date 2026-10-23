//! File URL directory ordering from the pinned JDK's default English collator.
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Deserialize)]
struct Tables {
    scalars: BTreeMap<u32, Vec<u32>>,
    contractions: BTreeMap<String, Vec<u32>>,
    #[cfg(test)]
    max_secondary: u32,
    #[cfg(test)]
    max_tertiary: u32,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        serde_json::from_str(include_str!("data/jdk25-en-file-collation.json"))
            .expect("pinned file collation tables")
    })
}

fn elements(text: &str) -> Vec<u32> {
    let table = tables();
    let mut output = Vec::new();
    let mut rest = text;
    while let Some(ch) = rest.chars().next() {
        if let Some((sequence, elements)) = table
            .contractions
            .iter()
            .filter(|(sequence, _)| rest.starts_with(sequence.as_str()))
            .max_by_key(|(sequence, _)| sequence.len())
        {
            for &element in elements {
                output.push(element);
            }
            rest = &rest[sequence.len()..];
        } else {
            if let Some(elements) = table.scalars.get(&(ch as u32)) {
                for &element in elements {
                    output.push(element);
                }
            } else {
                output.push(0x7fff0000);
                for unit in ch.encode_utf16(&mut [0; 2]) {
                    output.push((*unit as u32) << 16);
                }
            }
            rest = &rest[ch.len_utf8()..];
        }
    }
    output
}

fn compare(left: &[u32], right: &[u32]) -> std::cmp::Ordering {
    use std::cmp::Ordering::{Equal, Greater, Less};
    let (mut i, mut j) = (0, 0);
    let mut result = Equal;
    let mut secondary = true;
    let mut tertiary = true;
    while i < left.len() && j < right.len() {
        let (a, b) = (left[i], right[j]);
        i += 1;
        j += 1;
        if a == b {
            continue;
        }
        let (ap, bp) = (a >> 16, b >> 16);
        if ap != bp {
            if a == 0 {
                j -= 1;
            } else if b == 0 {
                i -= 1;
            } else if ap == 0 {
                if secondary {
                    result = Greater;
                    secondary = false;
                }
                j -= 1;
            } else if bp == 0 {
                if secondary {
                    result = Less;
                    secondary = false;
                }
                i -= 1;
            } else {
                return ap.cmp(&bp);
            }
        } else if secondary {
            let order = ((a >> 8) & 0xff).cmp(&((b >> 8) & 0xff));
            if order != Equal {
                result = order;
                secondary = false;
            } else if tertiary {
                let order = (a & 0xff).cmp(&(b & 0xff));
                if order != Equal {
                    result = order;
                    tertiary = false;
                }
            }
        }
    }
    for &element in &left[i..] {
        if element >> 16 != 0 {
            return Greater;
        }
        if (element >> 8) & 0xff != 0 && secondary {
            result = Greater;
            secondary = false;
        }
    }
    for &element in &right[j..] {
        if element >> 16 != 0 {
            return Less;
        }
        if (element >> 8) & 0xff != 0 && secondary {
            result = Less;
            secondary = false;
        }
    }
    result
}

pub(crate) fn sort(names: &mut [String]) {
    // FileURLConnection uses Collator.compare, whose treatment of trailing
    // ignorables differs from comparing CollationKey bytes in this JDK.
    let mut cached: Vec<_> = names
        .iter_mut()
        .map(|name| {
            let value = std::mem::take(name);
            let elements = elements(&value);
            (value, elements)
        })
        .collect();
    cached.sort_by(|left, right| compare(&left.1, &right.1));
    for (name, (value, _)) in names.iter_mut().zip(cached) {
        *name = value;
    }
}

#[cfg(test)]
fn key(text: &str) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let table = tables();
    let (mut primary, mut secondary, mut tertiary) = (Vec::new(), Vec::new(), Vec::new());
    for element in elements(text) {
        let (p, s, t) = (element >> 16, (element >> 8) & 0xff, element & 0xff);
        if p != 0 {
            primary.push(p + 1);
            secondary.push(s + 1);
            tertiary.push(t + 1);
        } else {
            if s != 0 {
                secondary.push(s + table.max_secondary + 1);
            }
            if t != 0 {
                tertiary.push(t + table.max_tertiary + 1);
            }
        }
    }
    (primary, secondary, tertiary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_keys_and_directory_order_match_pinned_jdk() {
        let golden: serde_json::Value =
            serde_json::from_str(include_str!("../tests/expected/file-collation-6f2d0a.json"))
                .unwrap();
        for (name, expected) in golden["keys"].as_object().unwrap() {
            let (primary, secondary, tertiary) = key(name);
            let units = primary
                .into_iter()
                .chain([0])
                .chain(secondary)
                .chain([0])
                .chain(tertiary);
            let bytes = units
                .flat_map(|unit| (unit as u16).to_be_bytes())
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(bytes, expected.as_str().unwrap(), "{name:?}");
        }
        let mut names: Vec<String> = golden["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        sort(&mut names);
        let expected = golden["sorted"].as_array().unwrap();
        assert_eq!(names.len(), expected.len());
        for (index, (actual, expected)) in names.iter().zip(expected).enumerate() {
            assert_eq!(
                actual,
                expected.as_str().unwrap(),
                "directory order at {index}"
            );
        }
    }
}
