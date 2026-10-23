use axum::{
    extract::{Multipart, OriginalUri, Request},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use teaql_forge_codegen::{input::ModelInput, semantic};

mod json;

fn json_response<T: serde::Serialize>(value: &T) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json")],
        semantic::json_bytes(value).expect("serializable semantic document"),
    )
        .into_response()
}

async fn upload(
    mut multipart: Multipart,
    require_revision: bool,
) -> Result<(ModelInput, i64), StatusCode> {
    let mut file = None;
    let mut revision = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        match field.name().unwrap_or("") {
            "file" => {
                let filename = field.file_name().unwrap_or("model.xml").to_owned();
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file = Some((filename, bytes));
            }
            "expectedRevision" => {
                revision = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => (),
        }
    }
    let (filename, bytes) = file.ok_or(StatusCode::BAD_REQUEST)?;
    let expected = if require_revision {
        let value = revision.ok_or(StatusCode::BAD_REQUEST)?;
        value
            .trim()
            .parse::<i64>()
            .map_err(|_| StatusCode::BAD_REQUEST)?
    } else {
        0
    };
    let input =
        ModelInput::load(&bytes, &filename).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((input, expected))
}

fn failure(path: &str) -> Response {
    crate::framework_errors::response(StatusCode::INTERNAL_SERVER_ERROR, path)
}

pub async fn initialize(OriginalUri(uri): OriginalUri, multipart: Multipart) -> Response {
    let (input, _) = match upload(multipart, false).await {
        Ok(input) => input,
        Err(status) => return crate::framework_errors::response(status, uri.path()),
    };
    let outcome = match input.try_evaluate("semantic-identity-initialize") {
        Ok(outcome) => outcome,
        Err(_) => return failure(uri.path()),
    };
    if outcome.response.blocks_generation() {
        return failure(uri.path());
    }
    let Some(domain) = outcome.domain else {
        return failure(uri.path());
    };
    match semantic::initialize(&domain) {
        Ok(document) => json_response(&document),
        Err(_) => failure(uri.path()),
    }
}

pub async fn reconcile(OriginalUri(uri): OriginalUri, multipart: Multipart) -> Response {
    let (mut input, expected) = match upload(multipart, true).await {
        Ok(input) => input,
        Err(status) => return crate::framework_errors::response(status, uri.path()),
    };
    let mut document = match input.semantic_document() {
        Ok(Some(document)) => document,
        Ok(None) => return failure(uri.path()),
        Err(_) => return failure(uri.path()),
    };
    input.files.remove(semantic::SIDECAR_PATH);
    let outcome = match input.try_evaluate("semantic-identity-reconcile") {
        Ok(outcome) => outcome,
        Err(_) => return failure(uri.path()),
    };
    if outcome.response.blocks_generation() {
        return failure(uri.path());
    }
    let Some(domain) = outcome.domain else {
        return failure(uri.path());
    };
    match semantic::reconcile(&mut document, expected, &domain) {
        Ok(()) => json_response(&document),
        Err(_) => failure(uri.path()),
    }
}

pub async fn mutate(OriginalUri(uri): OriginalUri, request: Request) -> Response {
    let json_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<mime::Mime>().ok())
        .is_some_and(|value| {
            value.type_() == mime::APPLICATION
                && (value.subtype() == mime::JSON || value.suffix() == Some(mime::JSON))
        });
    if !json_type {
        return crate::framework_errors::response(StatusCode::UNSUPPORTED_MEDIA_TYPE, uri.path());
    }
    let request = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => return crate::framework_errors::response(StatusCode::BAD_REQUEST, uri.path()),
    };
    let operation = uri.path().rsplit('/').next().unwrap();
    match json::mutate(&request, operation) {
        Ok(document) => (
            [(header::CONTENT_TYPE, "application/json")],
            document.bytes(),
        )
            .into_response(),
        Err(status) => crate::framework_errors::response(status, uri.path()),
    }
}
