//! Group declaration tokens retain source positions across comments and values.
use super::*;

#[derive(Clone)]
pub(super) struct Alias {
    pub position: usize,
    pub name: String,
    pub target: String,
}

#[derive(Clone)]
pub(super) struct Header {
    pub position: usize,
    pub name: String,
    pub arguments: String,
    pub parsed_args: Option<Vec<FormalArgument>>,
    pub missing_body: bool,
    pub opening: usize,
    pub end: usize,
}
#[derive(Clone)]
pub(super) struct Dictionary {
    pub position: usize,
    pub name: String,
    pub opening: usize,
    pub end: usize,
    pub parsed_entries: Option<Vec<(String, Option<String>)>>,
}
pub(super) struct Group {
    pub delimiters: delimiters::Delimiters,
    pub ranges: Vec<(usize, usize)>,
    pub headers: Vec<Header>,
    pub dictionaries: Vec<Dictionary>,
    pub aliases: Vec<Alias>,
}
pub(super) fn scan(source: &str) -> Result<Group, String> {
    Ok(super::group_parser::scan(source))
}

pub(super) fn legacy_headers(source: &str) -> Result<Vec<Header>, String> {
    let values = dictionary::group_values(source)?;
    let pattern = Regex::new(
        r"(?m)^([A-Za-z_][A-Za-z0-9_]*|@[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z_][A-Za-z0-9_]*)\s*\(",
    )
    .unwrap();
    let mut result = Vec::new();
    for c in pattern.captures_iter(source) {
        let whole = c.get(0).unwrap();
        if dictionary::contains(&values, whole.start()) {
            continue;
        }
        let close = expression::matching_end(source, whole.end() - 1, '(', ')')
            .ok_or("unterminated formal arguments")?;
        let Some(suffix) = source[close + 1..].trim_start().strip_prefix("::=") else {
            continue;
        };
        let suffix = suffix.trim_start();
        result.push(Header {
            position: whole.start(),
            name: c[1].into(),
            arguments: source[whole.end()..close].into(),
            parsed_args: None,
            missing_body: false,
            opening: source.len() - suffix.len(),
            end: 0,
        });
    }
    Ok(result)
}
pub(super) fn legacy_dictionaries(source: &str) -> Vec<Dictionary> {
    let pattern = Regex::new(r"(?m)^([A-Za-z_][A-Za-z0-9_]*)\s*::=\s*\[").unwrap();
    pattern
        .captures_iter(source)
        .map(|c| Dictionary {
            position: c.get(0).unwrap().start(),
            name: c[1].into(),
            opening: c.get(0).unwrap().end() - 1,
            end: 0,
            parsed_entries: None,
        })
        .collect()
}

pub(super) fn body(input: &str) -> Result<String, String> {
    let text = if input.starts_with('"') && input.ends_with('"') {
        let token = input.replace("\\\"", "\"");
        token[1..token.len() - 1].to_owned()
    } else if input.starts_with("<<") && input.ends_with(">>") {
        dictionary::right_angles(&input[2..input.len() - 2])
    } else if input.starts_with("<%") && input.ends_with("%>") {
        input[2..input.len() - 2].replace("%\\>", "%>")
    } else {
        return Err("invalid group template value".into());
    };
    let text = text
        .strip_prefix("\r\n")
        .or_else(|| text.strip_prefix('\n'))
        .unwrap_or(&text);
    Ok(text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text)
        .to_owned())
}

pub(super) fn apply_before(
    aliases: &[Alias],
    cursor: &mut usize,
    position: usize,
    definitions: &mut HashMap<String, Definition>,
) {
    while let Some(alias) = aliases
        .get(*cursor)
        .filter(|alias| alias.position < position)
    {
        // Undefined aliases are rejected immediately. A subsequent explicit
        // declaration may retry, but loading later targets never repairs them.
        if let Some(target) = definitions.get(&alias.target).cloned() {
            definitions.entry(alias.name.clone()).or_insert(target);
        }
        *cursor += 1;
    }
}
