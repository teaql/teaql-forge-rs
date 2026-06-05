use axum::{extract::Multipart, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use teaql_forge_model::parser::parse_model;

#[derive(Serialize)]
pub struct EvaluationResponse {
    pub errors: Vec<EvaluationItem>,
    pub warnings: Vec<EvaluationItem>,
}

#[derive(Serialize)]
pub struct EvaluationItem {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub title: String,
    pub message: String,
    pub path: String,
    #[serde(rename = "objectName")]
    pub object_name: String,
    #[serde(rename = "fieldName")]
    pub field_name: Option<String>,
    #[serde(rename = "xmlPath")]
    pub xml_path: String,
    #[serde(rename = "lineNumber")]
    pub line_number: usize,
}

pub async fn evaluate_handler(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_content = None;
    let mut xml_path = "model.xml".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(file_name) = field.file_name() {
                xml_path = file_name.to_string();
            }
            let data = field.bytes().await.unwrap();
            file_content = Some(String::from_utf8_lossy(&data).to_string());
        }
    }

    let xml = match file_content {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, "Missing file part").into_response(),
    };

    let domain = match parse_model(&xml, &xml_path) {
        Ok(d) => d,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Parse error: {}", e)).into_response(),
    };

    let mut response = EvaluationResponse {
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    let rust_kw: HashSet<&str> = ["as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield", "try", "macro", "union"].into_iter().collect();
    let java_kw: HashSet<&str> = ["abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class", "const", "continue", "default", "do", "double", "else", "enum", "extends", "final", "finally", "float", "for", "goto", "if", "implements", "import", "instanceof", "int", "interface", "long", "native", "new", "null", "package", "private", "protected", "public", "return", "short", "static", "strictfp", "super", "switch", "synchronized", "this", "throw", "throws", "transient", "try", "void", "volatile", "while", "true", "false", "var", "yield", "record", "sealed", "non-sealed", "permits"].into_iter().collect();
    let go_kw: HashSet<&str> = ["break", "default", "func", "interface", "select", "case", "defer", "go", "map", "struct", "chan", "else", "goto", "package", "switch", "const", "fallthrough", "if", "range", "type", "continue", "for", "import", "return", "var"].into_iter().collect();
    let swift_kw: HashSet<&str> = ["associatedtype", "class", "deinit", "enum", "extension", "fileprivate", "func", "import", "init", "inout", "internal", "let", "open", "operator", "private", "precedencegroup", "protocol", "public", "rethrows", "static", "struct", "subscript", "typealias", "var", "break", "case", "continue", "default", "defer", "do", "else", "fallthrough", "for", "guard", "if", "in", "repeat", "return", "switch", "where", "while", "as", "any", "false", "is", "nil", "self", "Self", "super", "true", "try"].into_iter().collect();
    let dart_kw: HashSet<&str> = ["abstract", "as", "assert", "async", "await", "break", "case", "catch", "class", "const", "continue", "covariant", "default", "deferred", "do", "dynamic", "else", "enum", "export", "extends", "extension", "external", "factory", "false", "final", "finally", "for", "Function", "get", "hide", "if", "implements", "import", "in", "interface", "is", "late", "library", "mixin", "new", "null", "on", "operator", "part", "required", "rethrow", "return", "set", "show", "static", "super", "switch", "sync", "this", "throw", "true", "try", "typedef", "var", "void", "when", "while", "with", "yield"].into_iter().collect();

    let mut languages = HashMap::new();
    languages.insert("Rust", rust_kw);
    languages.insert("Java", java_kw);
    languages.insert("Go", go_kw);
    languages.insert("Swift", swift_kw);
    languages.insert("Dart", dart_kw);

    for entity in domain.entities {
        let get_conflicts = |name: &str| -> Vec<&str> {
            let mut conflicts = Vec::new();
            for (lang, kws) in &languages {
                if kws.contains(name) {
                    conflicts.push(*lang);
                }
            }
            conflicts
        };

        let entity_conflicts = get_conflicts(&entity.name);
        if !entity_conflicts.is_empty() {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-KEYWORD-001".to_string(),
                title: "Object name conflicts with reserved keyword".to_string(),
                message: format!("Object name '{}' conflicts with reserved keywords in: {}. This will cause compilation/generation errors.", entity.name, entity_conflicts.join(", ")),
                path: format!("/root/{}", entity.name),
                object_name: entity.name.clone(),
                field_name: None,
                xml_path: entity.xml_path.clone(),
                line_number: entity.line_number,
            });
        }

        for member in entity.members {
            let (name, line_number, field_xml_path) = match member {
                teaql_forge_model::ir::EntityMember::Field(f) => (f.name, f.line_number, f.xml_path),
                teaql_forge_model::ir::EntityMember::Relation(r) => (r.name, r.line_number, r.xml_path),
            };

            let field_conflicts = get_conflicts(&name);
            if !field_conflicts.is_empty() {
                response.errors.push(EvaluationItem {
                    rule_id: "KSML-KEYWORD-002".to_string(),
                    title: "Field name conflicts with reserved keyword".to_string(),
                    message: format!("Field '{}' in object '{}' conflicts with reserved keywords in: {}. (Defined at {}:{}, attribute '{}'). Please rename the field to avoid generation/compilation errors.", name, entity.name, field_conflicts.join(", "), field_xml_path, line_number, name),
                    path: format!("/root/{}/{}", entity.name, name),
                    object_name: entity.name.clone(),
                    field_name: Some(name),
                    xml_path: field_xml_path,
                    line_number,
                });
            }
        }
    }

    Json(response).into_response()
}
