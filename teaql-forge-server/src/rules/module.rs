use crate::eval::{EvaluationItem, EvaluationResponse};
use roxmltree::{Document, Node};
use std::collections::{HashMap, HashSet};

pub fn evaluate_module_rule(doc: &Document, response: &mut EvaluationResponse, xml_path: &str) {
    let root = doc.root_element();
    if root.tag_name().name() != "root" && root.tag_name().name() != "_root" {
        return;
    }

    let mut object_elements = Vec::new();
    for node in root.children() {
        if node.is_element() {
            let tag = node.tag_name().name();
            if !tag.starts_with('_') && tag != "root" {
                object_elements.push(node);
            }
        }
    }

    let mut module_groups: HashMap<String, Vec<Node>> = HashMap::new();
    for obj in &object_elements {
        let module = obj.attribute("_module").unwrap_or("Unassigned");
        module_groups.entry(module.to_string()).or_default().push(*obj);
    }

    let root_object_name = get_root_object_name(&object_elements);

    // 1. Single-object modules check
    for (module_name, elements) in &module_groups {
        if elements.len() == 1 && module_name != "Unassigned" {
            let single_el = &elements[0];
            let tag = single_el.tag_name().name();
            if tag != root_object_name {
                response.warnings.push(EvaluationItem {
                    rule_id: "KSML-MODULE-003".to_string(),
                    title: "Single-object module".to_string(),
                    message: format!("Module '{}' contains only one object ({}). Standard modules should group multiple related objects.", module_name, tag),
                    path: format!("/root/{}", tag),
                    object_name: tag.to_string(),
                    field_name: None,
                    xml_path: xml_path.to_string(),
                    line_number: doc.text_pos_at(single_el.range().start).row as usize,
                });
            }
        }
    }

    // 2. Local constant module_key check
    for const_el in &object_elements {
        if !is_constant(const_el) {
            continue;
        }
        let const_name = const_el.tag_name().name();
        let mut referencing_bizs = Vec::new();

        for biz_el in &object_elements {
            if is_constant(biz_el) {
                continue;
            }
            let mut references = false;
            for attr in biz_el.attributes() {
                let attr_name = attr.name();
                if attr_name.starts_with('_') {
                    continue;
                }
                let attr_val = strip_translation(attr.value());
                if let Some(open_paren) = attr_val.find('(') {
                    let target_type = attr_val[..open_paren].trim();
                    if target_type == const_name {
                        references = true;
                        break;
                    }
                }
            }
            if references {
                referencing_bizs.push(*biz_el);
            }
        }

        if referencing_bizs.len() == 1 {
            let biz_el = &referencing_bizs[0];
            let biz_name = biz_el.tag_name().name();
            let biz_module_key = biz_el.attribute("_module_key").unwrap_or("Unassigned");
            let const_module_key = const_el.attribute("_module_key").unwrap_or("Unassigned");

            if const_module_key != biz_module_key {
                response.warnings.push(EvaluationItem {
                    rule_id: "KSML-MODULE-004".to_string(),
                    title: "Local constant module key mismatch".to_string(),
                    message: format!("Constant object '{}' is referenced by exactly one business object ('{}'), but its _module_key ('{}') does not match the business object's _module_key ('{}'). The finite set should preferably share the same module key because it is local to that business area rather than shared basic data.", const_name, biz_name, const_module_key, biz_module_key),
                    path: format!("/root/{}", const_name),
                    object_name: const_name.to_string(),
                    field_name: None,
                    xml_path: xml_path.to_string(),
                    line_number: doc.text_pos_at(const_el.range().start).row as usize,
                });
            }
        }
    }
}

fn is_constant(node: &Node) -> bool {
    node.attribute("_constant") == Some("true")
}

fn is_business(node: &Node) -> bool {
    !is_constant(node)
}

fn strip_translation(val: &str) -> String {
    if let Some(idx) = val.find(':').or_else(|| val.find('：')) {
        val[idx + 1..].trim().to_string()
    } else {
        val.to_string()
    }
}

fn get_root_object_name(objects: &[Node]) -> String {
    let mut business_objects = HashSet::new();
    for obj in objects {
        if is_business(obj) {
            business_objects.insert(obj.tag_name().name().to_string());
        }
    }

    if business_objects.is_empty() {
        return "platform".to_string();
    }

    let mut root_candidates = Vec::new();
    for obj in objects {
        if is_business(obj) {
            let tag = obj.tag_name().name();
            let mut outgoing_business_refs = 0;
            
            for attr in obj.attributes() {
                let attr_val = attr.value();
                if let Some(open_paren) = attr_val.find('(') {
                    if attr_val.contains(')') {
                        let target_type = attr_val[..open_paren].trim();
                        if business_objects.contains(target_type) && target_type != tag {
                            outgoing_business_refs += 1;
                        }
                    }
                }
            }
            if outgoing_business_refs == 0 {
                root_candidates.push(tag.to_string());
            }
        }
    }

    if root_candidates.is_empty() {
        business_objects.into_iter().next().unwrap()
    } else {
        root_candidates[0].clone()
    }
}
