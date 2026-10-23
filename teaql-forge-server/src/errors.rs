use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

const SUPPORTED_SCOPES: &[&str] = &[
    "data-design",
    "doc",
    "dotnet-app-console",
    "dotnet-lib-core",
    "dotnet-web-aspnet",
    "frontend",
    "golang-app-console",
    "golang-lib-core",
    "golang-web-gin",
    "java-app-console",
    "java-lib-core",
    "java-web-micronaut",
    "java-web-quarkus",
    "java-web-spring-boot",
    "model-view",
    "python-app-console",
    "python-lib-core",
    "python-web-fastapi",
    "rust-app-console",
    "rust-lib-core",
    "rust-web-axum",
    "rust-web-topcoat",
    "swift-app-console",
    "swift-lib-core",
    "tool",
    "typescript-app-console",
    "typescript-app-expo",
    "typescript-lib-core",
    "typescript-node-lib-core",
    "typescript-web-hono",
];

pub fn markdown(status: StatusCode, body: String) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/markdown;charset=utf-8")],
        body,
    )
        .into_response()
}

pub fn invalid_target(message: &str) -> Response {
    let mut body = format!("## ❌ Invalid Target (400)\n\n{message}\n\n### Available Targets:\n");
    for target in teaql_forge_codegen::templates::ALLOWED_TARGETS {
        body.push_str(&format!("- `{target}`\n"));
    }
    body.push_str("- *any target containing `-assist-`*\n\nPlease check your requested target.");
    markdown(StatusCode::BAD_REQUEST, body)
}

pub fn generation_error(message: &str, bad_request: bool) -> Response {
    let (status, heading, description) = if bad_request {
        (
            StatusCode::BAD_REQUEST,
            "Bad Request (400)",
            "A bad request error occurred during code generation:",
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error (500)",
            "An internal server error occurred during code generation:",
        )
    };
    markdown(status, format!("## ❌ {heading}\n\n{description}\n\n```\n{message}\n```\n\nPlease check the server logs for more details."))
}

/// The source controller preserves IllegalArgumentException for entrance and
/// domain-name failures; SAX and reflective getter exceptions remain internal.
pub fn model_parse_error(input: &teaql_forge_codegen::input::ModelInput, message: &str) -> Response {
    if let Some(error) = input.preview_factory_failure() {
        if error.is_bad_request() { return generation_error(&error.to_string(), true); }
        if error.is_internal_model_charset_failure() {
            return generation_error("Unknown Server Error", false);
        }
        return generation_error(&error.servlet_message(), false);
    }
    let bad_request = input.entrance_file.as_ref().is_some_and(|filename| {
        !input.files.contains_key(filename)
            || message == format!("model root in entrance file({filename}) must define a non-empty name attribute")
    });
    generation_error(message, bad_request)
}

pub fn unsupported_scope(scope: &str) -> Response {
    let supported = SUPPORTED_SCOPES
        .iter()
        .map(|scope| format!("  - {scope}"))
        .collect::<Vec<_>>()
        .join("\n");
    generation_error(&format!("Invalid or unsupported scope requested.\n\nRequested scope: {}\n\nSupported scopes:\n{supported}", scope.trim().to_lowercase()), true)
}

pub fn validate_target(target: &str) -> Result<(), String> {
    teaql_forge_codegen::templates::validate_target(target)
}

pub fn validate_preview_path(path: &str) -> Result<(), String> {
    teaql_forge_codegen::templates::validate_preview_path(path)
}
