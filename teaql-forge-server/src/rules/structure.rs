use crate::eval::{EvaluationItem, EvaluationResponse};
use regex::Regex;
use roxmltree::{Document, Node};
use std::collections::HashSet;

pub fn evaluate_structure_rule(doc: &Document, response: &mut EvaluationResponse, xml_path: &str) {
    let root = doc.root_element();
    if root.tag_name().name() != "root" && root.tag_name().name() != "_root" {
        return;
    }

    // 1. Check nested object definitions (they must be direct children of <root>)
    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }
        let tag = node.tag_name().name();
        if tag == "root" || tag == "_root" || tag.starts_with('_') {
            continue;
        }

        if let Some(parent) = node.parent() {
            if parent.tag_name().name() != "root" && parent.tag_name().name() != "_root" {
                response.errors.push(EvaluationItem {
                    rule_id: "KSML-STRUCTURE-001".to_string(),
                    title: "Nested object definition".to_string(),
                    message: format!("Object <{}> must be a direct child of <root>.", tag),
                    path: get_path(&node),
                    object_name: tag.to_string(),
                    field_name: None,
                    xml_path: xml_path.to_string(),
                    line_number: doc.text_pos_at(node.range().start).row as usize,
                });
            }
        }
    }

    // 2. Check unique object names and format casing
    let mut seen_objects = HashSet::new();
    let snake_re = Regex::new(r"^[a-z0-9]+(_[a-z0-9]+)*$").unwrap();

    for node in root.children().filter(|n| n.is_element()) {
        let tag = node.tag_name().name();
        if tag.starts_with('_') {
            continue;
        }

        let path = get_path(&node);

        if seen_objects.contains(tag) {
            response.errors.push(EvaluationItem {
                rule_id: "KSML-STRUCTURE-002".to_string(),
                title: "Duplicate object name".to_string(),
                message: format!("Object name '{}' is defined multiple times.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(node.range().start).row as usize,
            });
        }
        seen_objects.insert(tag.to_string());

        if !snake_re.is_match(tag) {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-STRUCTURE-003".to_string(),
                title: "Non-standard object name".to_string(),
                message: format!("Object name should be lowercase snake_case (e.g. stock_item), but found '{}'.", tag),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(node.range().start).row as usize,
            });
        }
    }
}

fn get_path(node: &Node) -> String {
    let mut path = String::new();
    let mut curr = Some(*node);
    while let Some(n) = curr {
        if n.is_element() {
            path.insert_str(0, &format!("/{}", n.tag_name().name()));
        }
        curr = n.parent();
    }
    path
}
