use crate::eval::{EvaluationItem, EvaluationResponse};
use regex::Regex;
use roxmltree::{Document, Node};

pub fn evaluate_object_metadata_rule(doc: &Document, response: &mut EvaluationResponse, xml_path: &str) {
    let root = doc.root_element();
    if root.tag_name().name() != "root" && root.tag_name().name() != "_root" {
        return;
    }

    // Check for `<_data_prompt>` elements
    for node in doc.descendants() {
        if node.is_element() && node.tag_name().name() == "_data_prompt" {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-OBJECT-003".to_string(),
                title: "Non-standard data prompt placement".to_string(),
                message: "Object data prompt should preferably be defined as a metadata attribute _data_prompt=\"...\", not as a separate element.".to_string(),
                path: get_path(&node),
                object_name: "_data_prompt".to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(node.range().start).row as usize,
            });
        }
    }

    let kebab_re = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();

    for obj_el in root.children().filter(|n| n.is_element()) {
        let tag = obj_el.tag_name().name();
        if tag.starts_with('_') || tag == "root" {
            continue;
        }

        let path = get_path(&obj_el);
        let name = obj_el.attribute("_name");
        let module = obj_el.attribute("_module");
        let module_key = obj_el.attribute("_module_key");

        let mut missing = Vec::new();
        if name.is_none() { missing.push("_name"); }
        if module.is_none() { missing.push("_module"); }
        if module_key.is_none() { missing.push("_module_key"); }

        if !missing.is_empty() {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-OBJECT-001".to_string(),
                title: "Missing object display metadata".to_string(),
                message: format!("Object '{}' is missing display metadata: {}", tag, missing.join(", ")),
                path: path.clone(),
                object_name: tag.to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(obj_el.range().start).row as usize,
            });
        }

        if let Some(mk) = module_key {
            if !kebab_re.is_match(mk) {
                response.warnings.push(EvaluationItem {
                    rule_id: "KSML-OBJECT-002".to_string(),
                    title: "Non-standard module key format".to_string(),
                    message: format!("Module key '{}' on object '{}' should be lowercase kebab-case.", mk, tag),
                    path: path.clone(),
                    object_name: tag.to_string(),
                    field_name: Some("_module_key".to_string()),
                    xml_path: xml_path.to_string(),
                    line_number: doc.text_pos_at(obj_el.range().start).row as usize,
                });
            }
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
