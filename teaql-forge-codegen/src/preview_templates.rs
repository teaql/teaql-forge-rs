// Builtin previews shipped in the standalone Rust edition.
pub(super) fn source(path: &str) -> Option<&'static str> {
    Some(match path {
        "evaluation/eval-report.md" => include_str!("../templates/preview-source/evaluation/eval-report.md.stg"),
        "rust-app-console/AGENTS.md" => include_str!("../templates/rust-app-console-source/AGENTS.md.stg"),
        "rust-app-console/README.md" => include_str!("../templates/rust-app-console-source/README.md.stg"),
        "rust-app-console/RUNTIME_CUSTOM_GUIDE.md" => include_str!("../templates/rust-app-console-source/RUNTIME_CUSTOM_GUIDE.md.stg"),
        "rust-app-console/TOOL_API_GUIDE.md" => include_str!("../templates/rust-app-console-source/TOOL_API_GUIDE.md.stg"),
        "rust-web-axum/AGENTS.md" => include_str!("../templates/rust-web-axum-source/AGENTS.md.stg"),
        "rust-web-axum/README.md" => include_str!("../templates/rust-web-axum-source/README.md.stg"),
        "rust-web-axum/RUNTIME_CUSTOM_GUIDE.md" => include_str!("../templates/rust-web-axum-source/RUNTIME_CUSTOM_GUIDE.md.stg"),
        "rust-web-axum/TOOL_API_GUIDE.md" => include_str!("../templates/rust-web-axum-source/TOOL_API_GUIDE.md.stg"),
        "rust-web-topcoat/AGENTS.md" => include_str!("../templates/rust-web-topcoat-source/AGENTS.md.stg"),
        "rust-web-topcoat/README.md" => include_str!("../templates/rust-web-topcoat-source/README.md.stg"),
        "rust-web-topcoat/RUNTIME_CUSTOM_GUIDE.md" => include_str!("../templates/rust-web-topcoat-source/RUNTIME_CUSTOM_GUIDE.md.stg"),
        "rust-web-topcoat/TOOL_API_GUIDE.md" => include_str!("../templates/rust-web-topcoat-source/TOOL_API_GUIDE.md.stg"),
        _ => return None,
    })
}
