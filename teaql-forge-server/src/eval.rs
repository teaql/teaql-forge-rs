use axum::{
    extract::{FromRequest, Multipart, OriginalUri, Request},
    http::{header, StatusCode},
    response::IntoResponse,
};
use teaql_forge_codegen::evaluation::{render_markdown_report, EvaluationItem, EvaluationResponse};
use teaql_forge_codegen::input::ModelInput;

pub async fn evaluate_handler(
    OriginalUri(uri): OriginalUri,
    request: Request,
) -> axum::response::Response {
    crate::telemetry::operation(1, "");
    let mut file_content = None;
    let mut filename = "model.xml".to_string();
    if request.headers().contains_key(header::CONTENT_TYPE) {
        let mut multipart = match Multipart::from_request(request, &()).await {
            Ok(multipart) => multipart,
            Err(rejection) => return rejection.into_response(),
        };
        while let Some(field) = multipart.next_field().await.unwrap_or(None) {
            if field.name() == Some("file") {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                file_content = Some(field.bytes().await.unwrap().to_vec());
            }
        }
    }
    let response = if let Some(xml) = file_content {
        crate::telemetry::upload(&xml, &filename);
        match ModelInput::load(&xml, &filename) {
            Ok(input) => match input.try_evaluate("evaluate") {
                Ok(outcome) => outcome.response,
                Err(_) => return crate::framework_errors::response(StatusCode::INTERNAL_SERVER_ERROR, uri.path()),
            },
            Err(_) => {
                return crate::framework_errors::response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    uri.path(),
                )
            }
        }
    } else {
        let mut response = EvaluationResponse::default();
        response.errors.push(EvaluationItem {
            rule_id: "KSML-UPLOAD-001".into(),
            title: "Missing model upload".into(),
            message: "The upload contains no files.".into(),
            path: "model.zip".into(),
            ..EvaluationItem::default()
        });
        response
    };
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/markdown;charset=UTF-8")],
        render_markdown_report(&response),
    )
        .into_response()
}

pub async fn evaluate_get_handler(OriginalUri(uri): OriginalUri) -> axum::response::Response {
    crate::telemetry::operation(1, "");
    let input = match crate::demo_input() {
        Ok(input) => input,
        Err(_) => return crate::framework_errors::response(StatusCode::INTERNAL_SERVER_ERROR, uri.path()),
    };
    let response = match input.try_evaluate("evaluate") {
        Ok(outcome) => outcome.response,
        Err(_) => return crate::framework_errors::response(StatusCode::INTERNAL_SERVER_ERROR, uri.path()),
    };
    (
        [(header::CONTENT_TYPE, "text/markdown;charset=UTF-8")],
        render_markdown_report(&response),
    ).into_response()
}

#[cfg(test)]
mod tests {
    use teaql_forge_codegen::evaluation::evaluate_model;

    #[test]
    fn missing_organization_suggests_example_organization() {
        let xml = r#"<root name="crm-service" data_service="sqlite" _module_key="root">
            <company _name="Company" _module="Organization" _module_key="organization" name="Example Company"/>
        </root>"#;
        let response = evaluate_model(xml, "main.xml", "evaluate").response;
        assert!(response.warnings.iter().any(|item| {
            item.rule_id == "KSML-ROOT-006" && item.message.contains("org=\"example\"")
        }));
    }

    #[test]
    fn logging_findings_use_source_categories() {
        let xml = r#"<root name="test-service" org="example" data_service="sqlite">
            <platform name="Example"/>
            <payment_log _log="true" amount="number()" platform="platform()"/>
        </root>"#;
        let response = evaluate_model(xml, "main.xml", "evaluate").response;
        assert!(response
            .errors
            .iter()
            .any(|item| item.rule_id == "KSML-LOG-003"));
        assert!(response
            .errors
            .iter()
            .any(|item| item.rule_id == "KSML-LOG-004"));
        assert!(response
            .suggestions
            .iter()
            .any(|item| item.rule_id == "KSML-LOG-006"));
    }
}
