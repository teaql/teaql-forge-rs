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
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Parse error: {}", e),
            )
                .into_response()
        }
    };

    let mut response = EvaluationResponse {
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    if let Ok(doc) = roxmltree::Document::parse(&xml) {
        crate::rules::evaluate_root_rule(&doc, &mut response, &xml_path);
        crate::rules::evaluate_structure_rule(&doc, &mut response, &xml_path);
        crate::rules::evaluate_object_metadata_rule(&doc, &mut response, &xml_path);
        crate::rules::evaluate_module_rule(&doc, &mut response, &xml_path);
        crate::rules::evaluate_logging_object_rule(&doc, &mut response, &xml_path);
    }

    let rust_kw: HashSet<&str> = [
        "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do",
        "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
        "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
        "super", "trait", "true", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where",
        "while", "yield", "try", "macro", "union",
    ]
    .into_iter()
    .collect();
    let java_kw: HashSet<&str> = [
        "abstract",
        "assert",
        "boolean",
        "break",
        "byte",
        "case",
        "catch",
        "char",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extends",
        "final",
        "finally",
        "float",
        "for",
        "goto",
        "if",
        "implements",
        "import",
        "instanceof",
        "int",
        "interface",
        "long",
        "native",
        "new",
        "null",
        "package",
        "private",
        "protected",
        "public",
        "return",
        "short",
        "static",
        "strictfp",
        "super",
        "switch",
        "synchronized",
        "this",
        "throw",
        "throws",
        "transient",
        "try",
        "void",
        "volatile",
        "while",
        "true",
        "false",
        "var",
        "yield",
        "record",
        "sealed",
        "non-sealed",
        "permits",
    ]
    .into_iter()
    .collect();
    let go_kw: HashSet<&str> = [
        "break",
        "default",
        "func",
        "interface",
        "select",
        "case",
        "defer",
        "go",
        "map",
        "struct",
        "chan",
        "else",
        "goto",
        "package",
        "switch",
        "const",
        "fallthrough",
        "if",
        "range",
        "type",
        "continue",
        "for",
        "import",
        "return",
        "var",
    ]
    .into_iter()
    .collect();
    let swift_kw: HashSet<&str> = [
        "associatedtype",
        "class",
        "deinit",
        "enum",
        "extension",
        "fileprivate",
        "func",
        "import",
        "init",
        "inout",
        "internal",
        "let",
        "open",
        "operator",
        "private",
        "precedencegroup",
        "protocol",
        "public",
        "rethrows",
        "static",
        "struct",
        "subscript",
        "typealias",
        "var",
        "break",
        "case",
        "continue",
        "default",
        "defer",
        "do",
        "else",
        "fallthrough",
        "for",
        "guard",
        "if",
        "in",
        "repeat",
        "return",
        "switch",
        "where",
        "while",
        "as",
        "any",
        "false",
        "is",
        "nil",
        "self",
        "Self",
        "super",
        "true",
        "try",
    ]
    .into_iter()
    .collect();
    let dart_kw: HashSet<&str> = [
        "abstract",
        "as",
        "assert",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "covariant",
        "default",
        "deferred",
        "do",
        "dynamic",
        "else",
        "enum",
        "export",
        "extends",
        "extension",
        "external",
        "factory",
        "false",
        "final",
        "finally",
        "for",
        "Function",
        "get",
        "hide",
        "if",
        "implements",
        "import",
        "in",
        "interface",
        "is",
        "late",
        "library",
        "mixin",
        "new",
        "null",
        "on",
        "operator",
        "part",
        "required",
        "rethrow",
        "return",
        "set",
        "show",
        "static",
        "super",
        "switch",
        "sync",
        "this",
        "throw",
        "true",
        "try",
        "typedef",
        "var",
        "void",
        "when",
        "while",
        "with",
        "yield",
    ]
    .into_iter()
    .collect();

    
    let sql_kw: HashSet<&str> = [
        "transaction", "select", "table",
    ]
    .into_iter()
    .collect();

    let mut languages = HashMap::new();
    languages.insert("Rust", rust_kw);
    languages.insert("Java", java_kw);
    languages.insert("Go", go_kw);
    languages.insert("Swift", swift_kw);
    languages.insert("Dart", dart_kw);
    languages.insert("SQL", sql_kw);

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
                message: format!("KSML XML naming error: element <{}> uses the reserved name '{}', which conflicts with: {}. Rename the XML element itself to a descriptive two-word snake_case name such as <{}_record>, and update references to the old element name.", entity.name, entity.name, entity_conflicts.join(", "), entity.name),
                path: format!("/root/{}", entity.name),
                object_name: entity.name.clone(),
                field_name: None,
                xml_path: entity.xml_path.clone(),
                line_number: entity.line_number,
            });
        }

        for member in entity.members {
            let (name, line_number, field_xml_path) = match member {
                teaql_forge_model::ir::EntityMember::Field(f) => {
                    (f.name, f.line_number, f.xml_path)
                }
                teaql_forge_model::ir::EntityMember::Relation(r) => {
                    (r.name, r.line_number, r.xml_path)
                }
            };

            let field_conflicts = get_conflicts(&name);
            if !field_conflicts.is_empty() {
                let suggested_name = if name == "operator" {
                    "operator_id".to_string()
                } else if name == "user" {
                    "user_id".to_string()
                } else {
                    format!("{}_{}", entity.name, name)
                };
                
                response.errors.push(EvaluationItem {
                    rule_id: "KSML-KEYWORD-002".to_string(),
                    title: "Field name conflicts with reserved keyword".to_string(),
                    message: format!("KSML XML naming error at {}:{}: element <{}> defines an XML attribute named '{}', which conflicts with reserved keywords in: {}. Rename the attribute in the KSML XML itself and keep its current value; for example, change {}=\"...\" to {}=\"...\". Also update any <_value> attributes or other model references that use the old field name.", field_xml_path, line_number, entity.name, name, field_conflicts.join(", "), name, suggested_name),
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

#[cfg(test)]
mod tests {
    use super::*;
    use teaql_forge_model::parser::parse_model;

    fn run_eval(xml: &str) -> EvaluationResponse {
        let domain = parse_model(xml, "main.xml").expect("Should parse");
        let mut response = EvaluationResponse {
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        let doc = roxmltree::Document::parse(xml).expect("Should parse xml doc");
        
        crate::rules::evaluate_root_rule(&doc, &mut response, "main.xml");
        crate::rules::evaluate_structure_rule(&doc, &mut response, "main.xml");
        crate::rules::evaluate_object_metadata_rule(&doc, &mut response, "main.xml");
        crate::rules::evaluate_module_rule(&doc, &mut response, "main.xml");
        crate::rules::evaluate_logging_object_rule(&doc, &mut response, "main.xml");
        
        response
    }

    #[test]
    fn test_missing_organization_suggests_example_organization() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <root name="crm-service" data_service="sqlite" _module_key="root">
            <company _name="Company" _module="Organization" _module_key="organization" name="Example Company"/>
        </root>
        "#;
        
        let response = run_eval(xml);
        for w in &response.warnings {
            println!("WARNING: [{}] {}", w.rule_id, w.message);
        }
        assert!(response.warnings.iter().any(|item| item.rule_id == "KSML-ROOT-006" && item.message.contains("org=\"example\"")));
    }

    #[test]
    fn test_wrong_root_wrapper_returns_xml_error() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <model name="crm-service">
            <merchant name="string()" />
        </model>
        "#;
        
        let err = parse_model(xml, "main.xml").unwrap_err();
        println!("ERROR: {}", err.to_string());
        assert!(err.to_string().contains("Domain tag is missing"));
    }

    #[test]
    fn test_logging_object_missing_timestamp_and_user() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <root name="test-service" org="example">
            <payment_log _log="true" amount="number()" />
        </root>
        "#;
        
        let response = run_eval(xml);
        
        let has_timestamp_err = response.errors.iter().any(|i| i.rule_id == "KSML-LOG-003");
        let has_user_err = response.errors.iter().any(|i| i.rule_id == "KSML-LOG-004");
        assert!(has_timestamp_err);
        assert!(has_user_err);
    }

    #[test]
    fn test_logging_object_incompatible_types() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <root name="test-service" org="example">
            <payment_log _log="true" _constant="true" log_time="createTime()" user_id="string()" user_name="string()"/>
        </root>
        "#;
        
        let response = run_eval(xml);
        assert!(response.errors.iter().any(|i| i.rule_id == "KSML-LOG-002"));
    }

    #[test]
    fn test_logging_object_suggests_log_purpose() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <root name="test-service" org="example">
            <payment_log _log="true" log_time="createTime()" user_id="string()" user_name="string()"/>
        </root>
        "#;
        
        let response = run_eval(xml);
        assert!(response.warnings.iter().any(|i| i.rule_id == "KSML-LOG-006"));
    }

    #[test]
    fn test_suggests_adding_log_to_name() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <root name="test-service" org="example">
            <payment_log log_time="createTime()" user_id="string()" user_name="string()"/>
        </root>
        "#;
        
        let response = run_eval(xml);
        assert!(response.warnings.iter().any(|i| i.rule_id == "KSML-LOG-007"));
    }
}
