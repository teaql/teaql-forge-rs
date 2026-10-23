use std::path::{Path, PathBuf};

pub const ALLOWED_TARGETS: &[&str] = &[
    "agent", "doc", "evaluation", "mergemodel",
    "rust-app-console", "rust-lib-core", "rust-web-axum", "rust-web-topcoat",
];

pub fn validate_target(target: &str) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("Target cannot be empty".into());
    }
    let root = target
        .split_once('/')
        .filter(|(root, _)| !root.is_empty())
        .map_or(target, |(root, _)| root);
    if !ALLOWED_TARGETS.contains(&root) && !root.contains("-assist-") {
        return Err(format!(
            "Target '{root}' is not in the whitelist and is not an assist target."
        ));
    }
    let assist = root
        .split_once("-assist-")
        .is_some_and(|(language, action)| {
            ["rust"]
                .contains(&language)
                && [
                    "query", "list-page", "create", "update", "delete", "expression",
                    "ensure-schema", "debug", "i18n", "runtime-custom", "tool-api",
                ]
                .contains(&action)
        });
    let external = self::root(root).is_some_and(|path| path.is_dir());
    if !external && (root == "agent" || (root.contains("-assist-") && !assist)) {
        return Err(format!("Target resources do not exist on server: {root}"));
    }
    Ok(())
}

pub fn validate_preview_path(path: &str) -> Result<(), String> {
    if path.contains('\\') {
        return Err("Preview path is invalid".into());
    }
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err("Preview path must contain a target and a file".into());
    }
    if parts
        .iter()
        .any(|part| part.is_empty() || [".", ".."].contains(part))
    {
        return Err("Preview path contains an invalid segment".into());
    }
    validate_target(parts[0])
}

/// The source preview resolver folds only Markdown data-design casing.
pub fn document_preview_template(filename: &str) -> Option<&'static str> {
    if filename == "model-view.html" {
        Some("model-view")
    } else if crate::input_path::eq_ignore_case(filename, "data-design.md")
        || filename == "data-design.html"
    {
        Some("data-design")
    } else if filename == "data-design-react.html" {
        Some("data-design-react")
    } else {
        None
    }
}

pub fn root(target: &str) -> Option<PathBuf> {
    let nonblank = |root: &String| !root.trim_matches(|ch| ch <= '\u{20}').is_empty();
    let root = crate::process_properties::property("teaql.generator.templateRoot")
        .filter(nonblank)
        .or_else(|| std::env::var("TEAQL_GENERATOR_TEMPLATE_ROOT").ok().filter(nonblank))
        .unwrap_or_else(|| "__teaql_no_external_template_root__".into());
    // java.io.File removes duplicate/trailing platform separators, preserving dots.
    let path = format!("{}/{}", root.trim_end_matches('/'), target.trim_start_matches('/'));
    let mut normalized = String::new();
    for ch in path.chars() {
        if ch != '/' || !normalized.ends_with('/') { normalized.push(ch); }
    }
    Some(PathBuf::from(normalized))
}

pub fn read(path: &str) -> Option<String> {
    if path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return None;
    }
    let (target, relative) = path.split_once('/')?;
    let root = root(target)?;
    let filename = Path::new(relative).file_name()?.to_str()?;
    let alias = document_preview_template(filename).unwrap_or(filename);
    let relative = Path::new(relative).with_file_name(format!("{alias}.stg"));
    let exact = root.join(&relative);
    let path = if exact.is_file() {
        Some(exact)
    } else {
        find(&root, Path::new(&relative).file_name()?.to_str()?)
    }?;
    std::fs::read_to_string(path).ok()
}

fn find(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find(&path, name) {
                return Some(found);
            }
        } else if entry.file_name() == name {
            return Some(path);
        }
    }
    None
}
