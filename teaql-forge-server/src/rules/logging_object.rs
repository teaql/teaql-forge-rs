use crate::eval::{EvaluationItem, EvaluationResponse};
use roxmltree::Document;
use std::collections::HashSet;

pub fn evaluate_logging_object_rule(doc: &Document, response: &mut EvaluationResponse, xml_path: &str) {
    let timestamp_names: HashSet<&str> = [
        "log_timestamp", "event_time", "created_at", "create_time",
        "log_time", "action_time", "occurred_at", "event_date"
    ].iter().cloned().collect();

    let user_names: HashSet<&str> = [
        "action_user", "user_id", "action_operator", "operator_id",
        "log_author", "created_by", "actor_id"
    ].iter().cloned().collect();

    let user_display_name_names: HashSet<&str> = [
        "action_user_name", "user_name", "action_operator_name", "operator_name",
        "log_author_name", "created_by_name", "actor_name"
    ].iter().cloned().collect();

    let purpose_names: HashSet<&str> = [
        "log_purpose", "log_reason", "log_description", "log_message",
        "action_type", "event_type", "log_type", "log_category"
    ].iter().cloned().collect();

    let root = doc.root_element();
    if root.tag_name().name() != "root" && root.tag_name().name() != "_root" {
        return;
    }

    for obj_el in root.children().filter(|n| n.is_element()) {
        let tag = obj_el.tag_name().name();
        if tag.starts_with('_') {
            continue;
        }

        let path = format!("/root/{}", tag);
        let line_number = doc.text_pos_at(obj_el.range().start).row as usize;

        let is_log = obj_el.attribute("_log").map(|v| v == "true").unwrap_or(false) || is_log_via_features(&obj_el);
        let is_constant = obj_el.attribute("_constant").map(|v| v == "true").unwrap_or(false);
        let is_view_object = obj_el.attribute("_viewObject").map(|v| v == "true").unwrap_or(false);
        let is_value_object = obj_el.attribute("_valueObject").map(|v| v == "true").unwrap_or(false);
        let is_transient = obj_el.attribute("_transient").map(|v| v == "true").unwrap_or(false);

        if !is_log {
            if tag.ends_with("_log") && !is_constant && !is_view_object && !is_value_object && !is_transient {
                response.warnings.push(EvaluationItem {
                    rule_id: "KSML-LOG-007".to_string(),
                    title: "Object name suggests logging semantics".to_string(),
                    message: format!("Object '{}' has a name ending with '_log' but is not marked as a logging object. Consider adding _log=\"true\" if this object represents append-only log entries.", tag),
                    path: path.clone(),
                    object_name: tag.to_string(),
                    field_name: None,
                    xml_path: xml_path.to_string(),
                    line_number,
                });
            }
            continue;
        }

        let mut has_error = false;
        if is_view_object {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-LOG-002".to_string(),
                title: "Incompatible logging object type".to_string(),
                message: format!("Logging object '{}' cannot also be a view object (_viewObject=\"true\"). These behaviors are mutually exclusive.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
            has_error = true;
        }
        if is_value_object {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-LOG-002".to_string(),
                title: "Incompatible logging object type".to_string(),
                message: format!("Logging object '{}' cannot also be a value object (_valueObject=\"true\"). These behaviors are mutually exclusive.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
            has_error = true;
        }
        if is_transient {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-LOG-002".to_string(),
                title: "Incompatible logging object type".to_string(),
                message: format!("Logging object '{}' cannot also be a transient object (_transient=\"true\"). These behaviors are mutually exclusive.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
            has_error = true;
        }
        if is_constant {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-LOG-002".to_string(),
                title: "Incompatible logging object type".to_string(),
                message: format!("Logging object '{}' cannot also be a constant object (_constant=\"true\"). These behaviors are mutually exclusive.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
            has_error = true;
        }

        if has_error {
            continue;
        }

        let mut field_names = HashSet::new();
        for attr in obj_el.attributes() {
            if !attr.name().starts_with('_') {
                field_names.insert(attr.name().to_lowercase());
            }
        }

        let has_timestamp = field_names.iter().any(|name| timestamp_names.contains(name.as_str()));
        if !has_timestamp {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-LOG-003".to_string(),
                title: "Missing timestamp field".to_string(),
                message: format!("Logging object '{}' has no recognizable timestamp field. A logging object must include a timestamp field (e.g. log_timestamp, event_time, created_at, log_time).", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
        }

        let has_user = field_names.iter().any(|name| user_names.contains(name.as_str()));
        if !has_user {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-LOG-004".to_string(),
                title: "Missing user field".to_string(),
                message: format!("Logging object '{}' has no recognizable user field. Add both operator_id and operator_name, or an equivalent pair such as user_id and user_name. The ID identifies the operation actor; the name stores the actor's display name at the time the log entry is created, so historical logs remain unchanged if the actor is renamed later. Do not use the single-word name 'operator' because it is a reserved keyword in some target languages.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
        }

        let has_user_display_name = field_names.iter().any(|name| user_display_name_names.contains(name.as_str()));
        if has_user && !has_user_display_name {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-LOG-008".to_string(),
                title: "Add operation actor name snapshot".to_string(),
                message: format!("Logging object '{}' identifies the operation actor but does not store the actor's display name. Add a matching two-word snake_case name field, for example operator_name alongside operator_id, or user_name alongside user_id. Store the name as it appears when the log entry is created so later renames do not rewrite the meaning of historical logs.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
        }

        if is_log_via_features(&obj_el) && obj_el.attribute("_log") != Some("true") {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-LOG-005".to_string(),
                title: "Prefer canonical _log syntax".to_string(),
                message: format!("Object '{}' uses _features=\"log\" to mark itself as a logging object. Consider using the canonical syntax _log=\"true\" instead for clarity.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
        }

        let has_purpose = field_names.iter().any(|name| purpose_names.contains(name.as_str()));
        if !has_purpose {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-LOG-006".to_string(),
                title: "Consider adding a purpose field".to_string(),
                message: format!("Logging object '{}' has no recognizable purpose field. Adding a field like log_purpose, log_reason, or event_type helps explain why the log entry exists.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number,
            });
        }
        
        // Note: Java also adds solids for KSML-LOG-001, but response struct only has errors and warnings in Rust version.
    }
}

fn is_log_via_features(obj_el: &roxmltree::Node) -> bool {
    if let Some(features) = obj_el.attribute("_features") {
        for f in features.split(',') {
            if f.trim().eq_ignore_ascii_case("log") {
                return true;
            }
        }
    }
    if let Some(features_delta) = obj_el.attribute("_features_delta") {
        for f in features_delta.split(',') {
            if f.trim().eq_ignore_ascii_case("log") {
                return true;
            }
        }
    }
    false
}
