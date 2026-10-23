//! Spring Boot 2.7 default JSON error response for controllers without handlers.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub fn response(status: StatusCode, path: &str) -> Response {
    #[derive(Serialize)]
    struct ErrorBody<'a> {
        timestamp: String,
        status: u16,
        error: &'a str,
        path: &'a str,
    }
    let format = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]+00:00",
    )
    .unwrap();
    let body = ErrorBody {
        timestamp: time::OffsetDateTime::now_utc().format(&format).unwrap(),
        status: status.as_u16(),
        error: status.canonical_reason().unwrap_or("None"),
        path,
    };
    (status, Json(body)).into_response()
}
