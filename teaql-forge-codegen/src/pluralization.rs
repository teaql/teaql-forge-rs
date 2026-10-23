// Copyright 2011 Atteo. Evo Inflector 1.3 rule data and algorithm are licensed
// under the Apache License, Version 2.0: https://www.apache.org/licenses/LICENSE-2.0
//! Ordered Evo rules plus the pinned generator's centralized naming exceptions.
use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

enum Rule {
    Category {
        suffixes: Vec<String>,
        singular: String,
        replacement: String,
    },
    Regex {
        expression: Regex,
        replacement: String,
    },
}

fn ascii_case_pattern(pattern: &str) -> String {
    // Java CASE_INSENSITIVE does not imply UNICODE_CASE. Expand ASCII letters
    // while retaining Unicode-aware character boundaries for negated classes.
    let mut output = String::new();
    let mut class = false;
    for ch in pattern.replace("(?i)", "").chars() {
        match ch {
            '[' => {
                class = true;
                output.push(ch);
            }
            ']' => {
                class = false;
                output.push(ch);
            }
            ch if ch.is_ascii_alphabetic() => {
                if !class {
                    output.push('[');
                }
                output.push(ch.to_ascii_lowercase());
                output.push(ch.to_ascii_uppercase());
                if !class {
                    output.push(']');
                }
            }
            _ => output.push(ch),
        }
    }
    output
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        #[derive(Deserialize)]
        struct Spec {
            kind: String,
            #[serde(default)]
            suffixes: Vec<String>,
            #[serde(default)]
            singular: String,
            replacement: String,
            #[serde(default)]
            pattern: String,
            #[serde(default)]
            flags: u8,
        }
        #[derive(Deserialize)]
        struct Catalog {
            rules: Vec<Spec>,
        }
        let catalog: Catalog = serde_json::from_str(include_str!("data/evo-inflector-1.3.json"))
            .expect("pinned Evo rules");
        let group = Regex::new(r"\$(\d+)").unwrap();
        catalog
            .rules
            .into_iter()
            .map(|rule| {
                if rule.kind == "category" {
                    Rule::Category {
                        suffixes: rule.suffixes,
                        singular: rule.singular,
                        replacement: rule.replacement,
                    }
                } else {
                    assert_eq!(rule.flags, 2, "pinned Java CASE_INSENSITIVE flags");
                    let replacement = group
                        .replace_all(&rule.replacement, |captures: &regex::Captures<'_>| {
                            format!("${{{}}}", &captures[1])
                        })
                        .into_owned();
                    Rule::Regex {
                        expression: Regex::new(&ascii_case_pattern(&rule.pattern))
                            .expect("Evo regex"),
                        replacement,
                    }
                }
            })
            .collect()
    })
}

fn evo(word: &str) -> String {
    let lower = word.to_lowercase();
    // Java's default $ also matches immediately before one final terminator.
    let input = word.strip_suffix("\r\n").unwrap_or(word);
    let input = if input.len() == word.len() {
        word.strip_suffix(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}'])
            .unwrap_or(word)
    } else {
        input
    };
    let ending = &word[input.len()..];
    for rule in rules() {
        match rule {
            Rule::Category {
                suffixes,
                singular,
                replacement,
            } if suffixes.iter().any(|suffix| lower.ends_with(suffix)) => {
                return format!("{}{replacement}", &word[..word.len() - singular.len()]);
            }
            Rule::Regex {
                expression,
                replacement,
            } if expression.is_match(input) => {
                return expression.replacen(input, 1, replacement).into_owned() + ending;
            }
            _ => {}
        }
    }
    unreachable!("Evo has a final catch-all rule")
}

pub(crate) fn pluralize(word: &str) -> String {
    if word.is_empty() {
        return word.into();
    }
    static WORDS: OnceLock<Regex> = OnceLock::new();
    let normalized = WORDS
        .get_or_init(|| Regex::new(r"([a-z])([A-Z]+)").unwrap())
        .replace_all(word, "${1}_${2}")
        .to_lowercase();
    let last = normalized
        .rsplit('_')
        .next()
        .filter(|last| !last.is_empty())
        .unwrap_or(&normalized);
    if matches!(
        last,
        "info"
            | "information"
            | "data"
            | "metadata"
            | "software"
            | "hardware"
            | "firmware"
            | "middleware"
            | "equipment"
            | "machinery"
            | "traffic"
            | "telemetry"
            | "feedback"
            | "evidence"
            | "access"
            | "knowledge"
            | "research"
            | "storage"
            | "memory"
            | "cpu"
            | "ram"
            | "disk"
            | "network"
            | "inventory"
    ) {
        return word.into();
    }
    let irregular = match last {
        "person" => Some("people"),
        "man" => Some("men"),
        "woman" => Some("women"),
        "child" => Some("children"),
        "mouse" => Some("mice"),
        "goose" => Some("geese"),
        _ => None,
    };
    if let Some(irregular) = irregular {
        let start = word.len() - last.len();
        let suffix = &word[start..];
        let replacement = if suffix
            .chars()
            .next()
            .is_some_and(|ch| crate::preview_domain::character_case(ch).2 & 2 != 0)
        {
            crate::preview_domain::upper_first(irregular)
        } else {
            irregular.into()
        };
        return format!("{}{replacement}", &word[..start]);
    }
    evo(word)
}

#[cfg(all(test, feature = "parity-probes"))]
mod tests {
    #[test]
    #[ignore = "explicit fixed-source pluralization corpus export"]
    fn export_source_pluralization_corpus() {
        let words: Vec<String> = serde_json::from_slice(
            &std::fs::read(std::env::var("TEAQL_PLURAL_WORDS").unwrap()).unwrap(),
        )
        .unwrap();
        let result = words
            .iter()
            .map(|word| (word.clone(), super::pluralize(word)))
            .collect::<std::collections::BTreeMap<_, _>>();
        std::fs::write(
            std::env::var("TEAQL_PLURAL_OUTPUT").unwrap(),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
}
