use crate::eval::{EvaluationItem, EvaluationResponse};
use regex::Regex;
use roxmltree::Document;

pub fn evaluate_root_rule(doc: &Document, response: &mut EvaluationResponse, xml_path: &str) {
    let root = doc.root_element();
    if root.tag_name().name() != "root" && root.tag_name().name() != "_root" {
        return;
    }

    let path = "/root".to_string();

    // 1. root name check
    let name = root.attribute("name").unwrap_or("").trim();
    if name.is_empty() {
        response.errors.push(EvaluationItem {
            rule_id: "KSML-ROOT-001".to_string(),
            title: "Missing root name".to_string(),
            message: "The <root> element must define a non-empty name attribute.".to_string(),
            path: path.clone(),
            object_name: "root".to_string(),
            field_name: None,
            xml_path: xml_path.to_string(),
            line_number: doc.text_pos_at(root.range().start).row as usize,
        });
    } else {
        let kebab_re = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
        let is_kebab = kebab_re.is_match(name);
        let ends_with_service = name.ends_with("-service");

        if !is_kebab || !ends_with_service {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-ROOT-002".to_string(),
                title: "Non-standard root name".to_string(),
                message: format!("Root name should preferably be lowercase kebab-case and end with -service (e.g. crm-service), but found '{}'.", name),
                path: path.clone(),
                object_name: "root".to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(root.range().start).row as usize,
            });
        }
    }

    // 2. alias_model_name check
    if let Some(alias) = root.attribute("alias_model_name") {
        let snake_re = Regex::new(r"^[a-z0-9]+(_[a-z0-9]+)*$").unwrap();
        if !snake_re.is_match(alias.trim()) {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-ROOT-004".to_string(),
                title: "Non-standard alias model name".to_string(),
                message: format!("alias_model_name should preferably be snake_case, but found '{}'.", alias),
                path: path.clone(),
                object_name: "root".to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(root.range().start).row as usize,
            });
        }
    }

    // 3. data_service check
    let data_service = root.attribute("data_service").unwrap_or("").trim();
    if data_service.is_empty() {
        response.warnings.push(EvaluationItem {
            rule_id: "KSML-ROOT-005".to_string(),
            title: "Missing data service".to_string(),
            message: "Please explicitly define data_service attribute on the <root> element. Candidates supported by current tech stack (rust): [sqlite, rusqlite, postgres, mysql]".to_string(),
            path: path.clone(),
            object_name: "root".to_string(),
            field_name: None,
            xml_path: xml_path.to_string(),
            line_number: doc.text_pos_at(root.range().start).row as usize,
        });
    } else if data_service != "sqlite" && data_service != "postgres" && data_service != "mysql" && data_service != "rusqlite" {
        response.warnings.push(EvaluationItem {
            rule_id: "KSML-ROOT-005".to_string(),
            title: "Non-standard data service".to_string(),
            message: format!("AI-generated models typically use data_service=\"sqlite\" or \"rusqlite\", but found '{}'.", data_service),
            path: path.clone(),
            object_name: "root".to_string(),
            field_name: None,
            xml_path: xml_path.to_string(),
            line_number: doc.text_pos_at(root.range().start).row as usize,
        });
    }

    // 4. org check
    if let Some(org) = root.attribute("org") {
        if org != "doublechaintech" {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-ROOT-006".to_string(),
                title: "Non-standard organization name".to_string(),
                message: format!("AI-generated models typically use org=\"doublechaintech\", but found '{}'.", org),
                path: path.clone(),
                object_name: "root".to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(root.range().start).row as usize,
            });
        }
    }

    // 5. _module_key check
    if let Some(module_key) = root.attribute("_module_key") {
        if module_key != "root" {
            response.warnings.push(EvaluationItem {
                rule_id: "KSML-ROOT-007".to_string(),
                title: "Non-standard module key".to_string(),
                message: format!("_module_key for root should be \"root\", but found '{}'.", module_key),
                path: path.clone(),
                object_name: "root".to_string(),
                field_name: None,
                xml_path: xml_path.to_string(),
                line_number: doc.text_pos_at(root.range().start).row as usize,
            });
        }
    }
}
