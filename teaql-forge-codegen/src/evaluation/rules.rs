use super::{EvaluationItem, EvaluationOutcome, EvaluationResponse};
use roxmltree::{Document, Node};
use std::collections::HashSet;
use teaql_forge_model::ir::{Domain, EntityMember, FieldType};

fn item(
    rule_id: &str,
    title: &str,
    message: String,
    path: String,
    object_name: Option<&str>,
    field_name: Option<&str>,
    filename: Option<&str>,
    line: Option<usize>,
) -> EvaluationItem {
    EvaluationItem {
        rule_id: rule_id.into(),
        title: title.into(),
        message,
        path,
        object_name: object_name.map(str::to_string),
        field_name: field_name.map(str::to_string),
        xml_path: filename.map(str::to_string),
        line_number: line,
    }
}

fn line(node: Node<'_, '_>) -> usize {
    node.document().text_pos_at(node.range().start).row as usize
}

fn attribute_line(node: Node<'_, '_>, name: &str) -> usize {
    node.attribute_node(name)
        .map(|attribute| node.document().text_pos_at(attribute.range().start).row as usize)
        .unwrap_or_else(|| line(node))
}

pub(super) fn opening_tag_end_line(node: Node<'_, '_>) -> usize {
    let text = node.document().input_text();
    let start = node.range().start;
    let offset = text[start..].find('>').unwrap_or(0);
    node.document().text_pos_at(start + offset).row as usize
}

fn objects<'a, 'input>(root: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    root.children()
        .filter(|node| node.is_element() && !crate::java_xml::qname(node).starts_with('_'))
}

fn is_constant(object: Node<'_, '_>) -> bool {
    object.attribute("_constant") == Some("true")
}

fn root_object_name(root: Node<'_, '_>) -> String {
    let business = objects(root)
        .filter(|object| !is_constant(*object))
        .map(|object| crate::java_xml::qname(object).to_string())
        .collect::<HashSet<_>>();
    if business.is_empty() {
        return "platform".into();
    }
    for object in objects(root).filter(|object| !is_constant(*object)) {
        let tag = crate::java_xml::qname(object);
        let outgoing = object.attributes().any(|attribute| {
            let value = attribute.value();
            value
                .split_once('(')
                .is_some_and(|(target, _)| business.contains(target.trim()) && target.trim() != tag)
        });
        if !outgoing {
            return tag.into();
        }
    }
    objects(root)
        .find(|object| !is_constant(*object))
        .map(|object| crate::java_xml::qname(object).into())
        .unwrap_or_else(|| "platform".into())
}

pub(super) fn evaluate_model(
    xml: &str,
    filename: &str,
    scope: &str,
    parsed_domain: Option<Domain>,
    sidecar: Result<Option<crate::semantic::SemanticIdentityDocument>, String>,
    included_objects: &HashSet<String>,
) -> EvaluationOutcome {
    let mut response = EvaluationResponse::default();
    response.solids.push(item(
        "KSML-UPLOAD-001",
        "Model upload is non-empty",
        "The upload contains 1 files.".into(),
        "model.zip".into(),
        None,
        None,
        None,
        None,
    ));
    let document = match Document::parse(xml) {
        Ok(document) => document,
        Err(error) => {
            response.errors.push(item(
                "KSML-XML-001",
                "XML parse error",
                format!("The XML in {filename} is malformed: Parse XML from stream error!"),
                filename.into(),
                None,
                None,
                Some(filename),
                Some(error.pos().row as usize),
            ));
            return EvaluationOutcome {
                response,
                domain: None,
            };
        }
    };
    let root = document.root_element();
    let root_line = line(root);
    if filename.ends_with(".xml") || filename.ends_with(".ksml") {
        response.solids.push(item(
            "KSML-UPLOAD-005",
            "Valid entrypoint found",
            format!("Resolved entrypoint: {filename}"),
            filename.into(),
            None,
            None,
            Some(filename),
            Some(root_line),
        ));
    } else {
        response.warnings.push(item(
            "KSML-UPLOAD-004",
            "Non-standard file extension",
            "Single-file upload should preferably end with .xml or .ksml.".into(),
            filename.into(),
            None,
            None,
            Some(filename),
            Some(root_line),
        ));
    }
    response.solids.push(item(
        "KSML-XML-002",
        "XML is well-formed",
        format!("The XML document {filename} was parsed successfully."),
        filename.into(),
        None,
        None,
        Some(filename),
        Some(root_line),
    ));
    if crate::java_xml::qname(root) != "root" && crate::java_xml::qname(root) != "_root" {
        response.errors.push(item(
            "KSML-XML-003",
            "Invalid document root element",
            format!(
                "The document root element in {filename} must be <root>, but found <{}>.",
                crate::java_xml::qname(root)
            ),
            format!("/{}", crate::java_xml::qname(root)),
            None,
            None,
            Some(filename),
            Some(root_line),
        ));
    }
    let domain = parsed_domain;
    {
        root_rule(root, filename, scope, domain.as_ref(), &mut response);
        structure_rule(root, filename, &mut response);
        object_metadata_rule(root, filename, &mut response);
        constant_rule(root, filename, &mut response);
        business_rule(root, filename, &mut response);
        logging_rule(root, filename, &mut response);
        reference_rule(root, filename, domain.as_ref(), &mut response);
        module_rule(root, filename, included_objects, &mut response);
        tenancy_rule(root, filename, &mut response);
        domain_root_rule(root, filename, &mut response);
        ui_rule(root, filename, &mut response);
        boolean_name_rule(root, domain.as_ref(), filename, &mut response);
        business_id_rule(root, domain.as_ref(), filename, &mut response);
        if let Some(domain) = &domain {
            match sidecar {
                Ok(Some(document)) => {
                    for message in crate::semantic::validate(&document, Some(domain)) {
                        response.errors.push(item(
                            "KSML-SEMANTIC-ID-002",
                            "Stable semantic identity",
                            message,
                            crate::semantic::SIDECAR_PATH.into(),
                            None,
                            None,
                            None,
                            None,
                        ));
                    }
                }
                Err(message) => response.errors.push(item(
                    "KSML-SEMANTIC-ID-001",
                    "Stable semantic identity",
                    message,
                    crate::semantic::SIDECAR_PATH.into(),
                    None,
                    None,
                    None,
                    None,
                )),
                Ok(None) => (),
            }
        }
        super::keywords::evaluate(root, domain.as_ref(), filename, &mut response);
        if domain.is_some() {
            super::privacy::evaluate(root, filename, &mut response);
        }
    }
    EvaluationOutcome { response, domain }
}

fn strip_translation(value: &str) -> &str {
    value
        .split_once(':')
        .or_else(|| value.split_once('：'))
        .map_or(value, |(_, translated)| translated.trim())
}

fn snake_case(name: &str) -> bool {
    name.split('_').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    })
}

fn structure_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    for element in root.descendants().filter(|node| node.is_element()) {
        let tag = crate::java_xml::qname(element);
        if tag == "root" || tag == "_root" || tag.starts_with('_') {
            continue;
        }
        let parent = element.parent().filter(|node| node.is_element());
        if parent == Some(root) {
            continue;
        }
        let path = element
            .ancestors()
            .filter(|node| node.is_element())
            .map(|node| format!("/{}", crate::java_xml::qname(node)))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        let likely_field = parent.is_some_and(|parent| {
            parent.parent() == Some(root)
                && !is_constant(parent)
                && element.attributes().len() == 0
                && snake_case(tag)
                && !element.children().any(|child| child.is_element())
        });
        if likely_field {
            let object = parent.unwrap().tag_name().name();
            response.errors.push(item(
                "KSML-STRUCTURE-004",
                "Field written as child element",
                format!("Field '{tag}' is written as a child element under business object '{object}'. KSML business object fields must be XML attributes. Replace <{object}><{tag}>...</{tag}></{object}> with <{object} {tag}=\"...\"/>. Only constant objects use <_value> child elements."),
                path,
                Some(object),
                Some(tag),
                Some(filename),
                Some(line(element)),
            ));
        } else {
            response.errors.push(item(
                "KSML-STRUCTURE-001",
                "Nested object definition",
                format!("Object <{tag}> must be a direct child of <root>."),
                path,
                None,
                None,
                Some(filename),
                Some(line(element)),
            ));
        }
    }
    let mut seen = HashSet::new();
    for object in objects(root) {
        let tag = crate::java_xml::qname(object);
        let path = format!("/root/{tag}");
        if !seen.insert(tag) {
            response.errors.push(item(
                "KSML-STRUCTURE-002",
                "Duplicate object name",
                format!("Object name '{tag}' is defined multiple times."),
                path.clone(),
                None,
                None,
                Some(filename),
                Some(line(object)),
            ));
        }
        if !snake_case(tag) {
            response.warnings.push(item(
                "KSML-STRUCTURE-003",
                "Non-standard object name",
                format!("Object name should be lowercase snake_case (e.g. stock_item), but found '{tag}'."),
                path,
                None,
                None,
                Some(filename),
                Some(line(object)),
            ));
        }
    }
}

fn tenancy_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    let mut total = 0;
    let mut with_tenancy = 0;
    let mut sample = None;
    for object in objects(root).filter(|object| !is_constant(*object)) {
        total += 1;
        let has_tenancy = object.attributes().any(|attribute| {
            ["merchant", "tenant", "platform"]
                .iter()
                .any(|name| attribute.name().eq_ignore_ascii_case(name))
        });
        if has_tenancy {
            with_tenancy += 1;
            sample = Some(object);
        }
    }
    if total > 2 && (with_tenancy as f64 / total as f64) > 0.8 {
        if let Some(object) = sample {
            let tag = crate::java_xml::qname(object);
            response.warnings.push(item(
                "KSML-TENANCY-001",
                "Questionable tenancy assumption",
                format!("Almost all business objects ({with_tenancy}/{total}) contain tenant/merchant fields. Ensure that tenancy is explicitly required, rather than just copied from examples."),
                format!("/root/{tag}"),
                Some(tag),
                None,
                Some(filename),
                Some(line(object)),
            ));
        }
    }
}

fn logging_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    let timestamps = [
        "log_timestamp",
        "event_time",
        "created_at",
        "create_time",
        "log_time",
        "action_time",
        "occurred_at",
        "event_date",
    ];
    let users = [
        "action_user",
        "user_id",
        "action_operator",
        "operator_id",
        "log_author",
        "created_by",
        "actor_id",
    ];
    let user_names = [
        "action_user_name",
        "user_name",
        "action_operator_name",
        "operator_name",
        "log_author_name",
        "created_by_name",
        "actor_name",
    ];
    let purposes = [
        "log_purpose",
        "log_reason",
        "log_description",
        "log_message",
        "action_type",
        "event_type",
        "log_type",
        "log_category",
    ];
    for object in objects(root) {
        let tag = crate::java_xml::qname(object);
        let path = format!("/root/{tag}");
        let location = Some(line(object));
        let via_features = ["_features", "_features_delta"].iter().any(|key| {
            object.attribute(*key).is_some_and(|value| {
                value
                    .split(',')
                    .any(|feature| feature.trim().eq_ignore_ascii_case("log"))
            })
        });
        let logging = object.attribute("_log") == Some("true") || via_features;
        let incompatible = ["_viewObject", "_valueObject", "_transient", "_constant"];
        if !logging {
            if tag.ends_with("_log")
                && !is_constant(object)
                && incompatible[..3]
                    .iter()
                    .all(|key| object.attribute(*key) != Some("true"))
            {
                response.suggestions.push(item(
                    "KSML-LOG-007",
                    "Object name suggests logging semantics",
                    format!("Object '{tag}' has a name ending with '_log' but is not marked as a logging object. Consider adding _log=\"true\" if this object represents append-only log entries."),
                    path,
                    Some(tag),
                    None,
                    Some(filename),
                    location,
                ));
            }
            continue;
        }
        let mut has_error = false;
        for (key, label) in [
            ("_viewObject", "view"),
            ("_valueObject", "value"),
            ("_transient", "transient"),
            ("_constant", "constant"),
        ] {
            if object.attribute(key) == Some("true") {
                has_error = true;
                response.errors.push(item(
                    "KSML-LOG-002",
                    "Incompatible logging object type",
                    format!("Logging object '{tag}' cannot also be a {label} object ({key}=\"true\"). These behaviors are mutually exclusive."),
                    path.clone(),
                    Some(tag),
                    None,
                    Some(filename),
                    location,
                ));
            }
        }
        if has_error {
            continue;
        }
        let fields = object
            .attributes()
            .filter(|attribute| !attribute.name().starts_with('_'))
            .map(|attribute| attribute.name().to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let has_timestamp = timestamps.iter().any(|name| fields.contains(*name));
        let has_user = users.iter().any(|name| fields.contains(*name));
        let has_user_name = user_names.iter().any(|name| fields.contains(*name));
        let has_purpose = purposes.iter().any(|name| fields.contains(*name));
        if !has_timestamp {
            response.errors.push(item(
                "KSML-LOG-003",
                "Missing timestamp field",
                format!("Logging object '{tag}' has no recognizable timestamp field. A logging object must include a timestamp field (e.g. log_timestamp, event_time, created_at, log_time)."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                location,
            ));
        }
        if !has_user {
            response.errors.push(item(
                "KSML-LOG-004",
                "Missing user field",
                format!("Logging object '{tag}' has no recognizable user field. Add both operator_id and operator_name, or an equivalent pair such as user_id and user_name. The ID identifies the operation actor; the name stores the actor's display name at the time the log entry is created, so historical logs remain unchanged if the actor is renamed later. Do not use the single-word name 'operator' because it is a reserved keyword in some target languages."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                location,
            ));
        }
        if has_user && !has_user_name {
            response.suggestions.push(item(
                "KSML-LOG-008",
                "Add operation actor name snapshot",
                format!("Logging object '{tag}' identifies the operation actor but does not store the actor's display name. Add a matching two-word snake_case name field, for example operator_name alongside operator_id, or user_name alongside user_id. Store the name as it appears when the log entry is created so later renames do not rewrite the meaning of historical logs."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                location,
            ));
        }
        if via_features && object.attribute("_log") != Some("true") {
            response.suggestions.push(item(
                "KSML-LOG-005",
                "Prefer canonical _log syntax",
                format!("Object '{tag}' uses _features=\"log\" to mark itself as a logging object. Consider using the canonical syntax _log=\"true\" instead for clarity."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                location,
            ));
        }
        if !has_purpose {
            response.suggestions.push(item(
                "KSML-LOG-006",
                "Consider adding a purpose field",
                format!("Logging object '{tag}' has no recognizable purpose field. Adding a field like log_purpose, log_reason, or event_type helps explain why the log entry exists."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                location,
            ));
        }
        if has_timestamp && has_user {
            response.solids.push(item(
                "KSML-LOG-001",
                "Valid logging object",
                format!("Object '{tag}' is correctly defined as a logging (insert-only) object with required timestamp and user fields."),
                path,
                Some(tag),
                None,
                Some(filename),
                location,
            ));
        }
    }
}

fn reference_rule(
    root: Node<'_, '_>,
    filename: &str,
    domain: Option<&Domain>,
    response: &mut EvaluationResponse,
) {
    let names = objects(root)
        .map(|object| crate::java_xml::qname(object).to_string())
        .collect::<HashSet<_>>();
    let mut scalar_types = [
        "string",
        "id",
        "number",
        "createTime",
        "updateTime",
        "boolean",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<HashSet<_>>();
    if let Some(domain) = domain {
        scalar_types.extend(teaql_forge_model::types::builtin_types().into_keys());
        scalar_types.extend(domain.source_types.keys().cloned());
    }
    let finite_fields = [
        "status",
        "category",
        "kind",
        "classification",
        "gender",
        "priority",
        "urgency",
    ];
    for object in objects(root) {
        let tag = crate::java_xml::qname(object);
        let path = format!("/root/{tag}");
        let mut attributes = object
            .attributes()
            .filter(|attribute| !attribute.name().starts_with('_'))
            .collect::<Vec<_>>();
        attributes.sort_by_key(|attribute| attribute.name());
        for attribute in attributes {
            let name = attribute.name();
            let value = strip_translation(attribute.value());
            let value_target = value
                .split_once('(')
                .filter(|(_, rest)| rest.contains(')'))
                .map(|(target, _)| target.trim());
            if let Some(target) = value_target {
                if value.starts_with("object(") {
                    response.errors.push(item(
                        "KSML-REFERENCE-001",
                        "Invalid reference format",
                        format!("Reference field '{name}' in '{tag}' must reference target type directly (e.g. merchant=\"merchant()\"), not wrap it in object()."),
                        format!("{path}/{name}"),
                        Some(tag),
                        Some(name),
                        Some(filename),
                        Some(attribute_line(object, name)),
                    ));
                }
                if names.contains(target) {
                    if name.ends_with("_id") {
                        response.warnings.push(item(
                            "KSML-REFERENCE-002",
                            "Non-standard reference field name",
                            format!("Reference field '{name}' in '{tag}' should preferably be named '{target}' without the '_id' suffix."),
                            format!("{path}/{name}"),
                            Some(tag),
                            Some(name),
                            Some(filename),
                            Some(attribute_line(object, name)),
                        ));
                    }
                    response.solids.push(item(
                        "KSML-REFERENCE-003",
                        "Reference target exists",
                        format!("Reference '{name}' in '{tag}' successfully resolves to target object '{target}'."),
                        format!("{path}/{name}"),
                        Some(tag),
                        Some(name),
                        Some(filename),
                        Some(attribute_line(object, name)),
                    ));
                } else if !scalar_types.contains(target) {
                    response.errors.push(item(
                        "KSML-REFERENCE-003",
                        "Missing referenced object",
                        format!("Field '{name}' in '{tag}' references target object '{target}' which does not exist in the model."),
                        format!("{path}/{name}"),
                        Some(tag),
                        Some(name),
                        Some(filename),
                        Some(attribute_line(object, name)),
                    ));
                }
            }
            if finite_fields.contains(&name.to_ascii_lowercase().as_str()) {
                let references_constant = value_target.is_some_and(|target| {
                    objects(root).any(|candidate| {
                        crate::java_xml::qname(candidate) == target && is_constant(candidate)
                    })
                });
                if !references_constant {
                    response.warnings.push(item(
                        "KSML-REFERENCE-005",
                        "Finite-set field does not reference constant object",
                        format!("Field '{name}' in '{tag}' represents a finite set and should preferably reference a constant object."),
                        format!("{path}/{name}"),
                        Some(tag),
                        Some(name),
                        Some(filename),
                        Some(attribute_line(object, name)),
                    ));
                }
            }
        }
    }
}

fn module_rule(
    root: Node<'_, '_>,
    filename: &str,
    included_objects: &HashSet<String>,
    response: &mut EvaluationResponse,
) {
    let root_name = root_object_name(root);
    let mut groups: Vec<(&str, Vec<Node<'_, '_>>)> = Vec::new();
    for object in objects(root)
        .filter(|object| !is_constant(*object) && crate::java_xml::qname(object) != root_name)
    {
        let module = object
            .attribute("_module")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Unassigned");
        if let Some((_, members)) = groups.iter_mut().find(|(name, _)| *name == module) {
            members.push(object);
        } else {
            groups.push((module, vec![object]));
        }
    }
    for (module, members) in groups {
        if module == "Unassigned" || members.len() > 3 {
            continue;
        }
        let Some(object) = members
            .iter()
            .find(|object| included_objects.contains(crate::java_xml::qname(*object)))
        else {
            continue;
        };
        let names = members
            .iter()
            .map(|object| crate::java_xml::qname(object))
            .collect::<Vec<_>>()
            .join(", ");
        let name = crate::java_xml::qname(object);
        response.warnings.push(item("KSML-MODULE-003", "Small module", format!("Module '{module}' contains {} domain object{} ({names}). Consider merging it with a related module or moving its definitions into the entrypoint {filename}.", members.len(), if members.len() == 1 { "" } else { "s" }), format!("/root/{name}"), Some(name), None, Some(filename), Some(line(*object))));
    }
    for constant in objects(root).filter(|object| is_constant(*object)) {
        let constant_name = crate::java_xml::qname(constant);
        let referencing = objects(root)
            .filter(|object| !is_constant(*object))
            .filter(|object| {
                object.attributes().any(|attribute| {
                    if attribute.name().starts_with('_') {
                        return false;
                    }
                    strip_translation(attribute.value())
                        .split_once('(')
                        .is_some_and(|(target, rest)| {
                            rest.contains(')') && target.trim() == constant_name
                        })
                })
            })
            .collect::<Vec<_>>();
        if let [business] = referencing.as_slice() {
            let business_name = crate::java_xml::qname(business);
            let business_key = business
                .attribute("_module_key")
                .filter(|key| !key.trim().is_empty())
                .unwrap_or("Unassigned");
            let constant_key = constant
                .attribute("_module_key")
                .filter(|key| !key.trim().is_empty())
                .unwrap_or("Unassigned");
            let path = format!("/root/{constant_name}");
            if business_key == constant_key {
                response.solids.push(item(
                    "KSML-MODULE-004",
                    "Local constant module key matches",
                    format!("Constant object '{constant_name}' is local to business area '{business_name}' and correctly shares its _module_key '{constant_key}'."),
                    path,
                    Some(constant_name),
                    None,
                    Some(filename),
                    Some(line(constant)),
                ));
            } else {
                response.warnings.push(item(
                    "KSML-MODULE-004",
                    "Local constant module key mismatch",
                    format!("Constant object '{constant_name}' is referenced by exactly one business object ('{business_name}'), but its _module_key ('{constant_key}') does not match the business object's _module_key ('{business_key}'). The finite set should preferably share the same module key because it is local to that business area rather than shared basic data."),
                    path,
                    Some(constant_name),
                    None,
                    Some(filename),
                    Some(line(constant)),
                ));
            }
        }
    }
}

fn boolean_name_rule(
    root: Node<'_, '_>,
    domain: Option<&Domain>,
    filename: &str,
    response: &mut EvaluationResponse,
) {
    let Some(domain) = domain else { return };
    for entity in &domain.entities {
        let Some(object) = objects(root).find(|object| crate::java_xml::qname(object) == entity.name)
        else {
            continue;
        };
        for member in &entity.members {
            let EntityMember::Field(field) = member else {
                continue;
            };
            if field.ty != FieldType::Bool
                || !field.name.starts_with("is_")
                || field.name.len() == 3
            {
                continue;
            }
            let name = &field.name;
            let replacement = &name[3..];
            let original = object
                .attribute(name.as_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("boolean()");
            response.errors.push(item(
                "KSML-NAMING-BOOLEAN-001",
                "Boolean field naming",
                format!("Boolean field '{}.{name}' must not use the 'is_' prefix because it produces redundant generated API names. Rename the field while preserving its original value exactly: {name}=\"{original}\" -> {replacement}=\"{original}\". Also update all model references that use '{name}'.", entity.name),
                format!("/root/{}/@{name}", entity.name),
                Some(&entity.name),
                Some(name),
                Some(filename),
                Some(opening_tag_end_line(object)),
            ));
        }
    }
}

fn business_id_rule(
    root: Node<'_, '_>,
    domain: Option<&Domain>,
    filename: &str,
    response: &mut EvaluationResponse,
) {
    let Some(domain) = domain else { return };
    let rendered = crate::context::build_render_context(domain);
    for entity in &rendered.entities {
        let object_name = entity.name.as_str();
        for field in &entity.fields {
            if !crate::business_id::is_business_id(field) {
                continue;
            }
            let field_name = field.name.as_str();
            let config = crate::business_id::field_values(field, object_name);
            let profile = config["businessIdProfile"].as_str().unwrap();
            let prefix = config["businessIdPrefix"].as_str().unwrap();
            let digits = config["businessIdDigits"].as_i64().unwrap();
            let date_format = config["businessIdDateFormat"].as_str().unwrap();
            let reset = config["businessIdReset"].as_str().unwrap();
            let separator = config["businessIdSeparator"].as_str().unwrap();
            let policy_version = config["businessIdPolicyVersion"].as_i64().unwrap();
            let path = format!("/root/{object_name}/@{field_name}");
            let field_line = Some(
                objects(root)
                    .find(|object| crate::java_xml::qname(object) == object_name)
                    .map(opening_tag_end_line)
                    .unwrap_or(field.line_number),
            );
            let mut error = |suffix: &str, message: &str| {
                response.errors.push(item(
                    &format!("KSML-BUSINESS-ID-{suffix}"),
                    "Business ID definition",
                    message.into(),
                    path.clone(),
                    Some(object_name),
                    Some(field_name),
                    Some(filename),
                    field_line,
                ));
            };
            if crate::business_id::attribute(field, "optional")
                .is_some_and(crate::preview_rust::attribute_flag)
            {
                error("001", "Business ID fields must be required; remove '?'.");
            }
            if profile != "daily-permuted-v1" && profile != "daily-sequence" {
                error(
                    "002",
                    "Supported profiles are daily-permuted-v1 and explicit legacy daily-sequence.",
                );
            }
            let prefix_valid = prefix
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_uppercase())
                && prefix
                    .chars()
                    .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit());
            if !prefix_valid {
                error("003", "Business ID prefix must match [A-Z][A-Z0-9]*.");
            }
            if (profile == "daily-permuted-v1" && digits != 6)
                || (profile != "daily-permuted-v1" && !(1..=18).contains(&digits))
            {
                error(
                    "004",
                    if profile == "daily-permuted-v1" {
                        "daily-permuted-v1 requires preserve_digits=6."
                    } else {
                        "preserve_digits must be between 1 and 18."
                    },
                );
            }
            if date_format != "yyyyMMdd" || reset != "daily" {
                error(
                    "005",
                    "Portable daily profiles require dateFormat=yyyyMMdd and reset=daily.",
                );
            }
            if separator.chars().all(crate::business_id::java_whitespace) {
                error("006", "Business ID separator must not be blank.");
            }
            if policy_version != 1 {
                error(
                    "007",
                    "Portable Business ID profile V1 requires policyVersion=1.",
                );
            }
            let is_root = !entity.has_parent() && entity.object_fields.is_empty();
            let linked_to_root = std::iter::once(entity)
                .chain(entity.ancestors(&rendered))
                .any(|owner| {
                    owner.fields.iter().any(|field| {
                        crate::business_id::attribute(field, "contextRoot")
                            .is_some_and(crate::preview_rust::attribute_flag)
                    }) || owner.object_fields.iter().any(|relation| {
                        let target = rendered
                            .entities
                            .iter()
                            .find(|target| target.rust_struct == relation.target_struct);
                        let context = relation
                            .metadata
                            .get("contextRoot")
                            .map(String::as_str)
                            .or_else(|| {
                                target.and_then(|target| {
                                    target
                                        .metadata_ordered
                                        .iter()
                                        .find(|(key, _)| key == "contextRoot")
                                        .map(|(_, value)| value.as_str())
                                })
                            })
                            .is_some_and(crate::preview_rust::attribute_flag);
                        context
                            || target.is_some_and(|target| {
                                !target.has_parent() && target.object_fields.is_empty()
                            })
                    })
                });
            if !is_root && !linked_to_root {
                error(
                    "008",
                    "A Business ID aggregate must reference the domain root.",
                );
            }
        }
    }
}

fn ui_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    let status_fields = [
        "status",
        "state",
        "stage",
        "phase",
        "step",
        "progress",
        "workflow_status",
    ];
    let status_suffixes = [
        "_status",
        "_state",
        "_stage",
        "_phase",
        "_step",
        "_progress",
    ];
    let status_codes = [
        "DRAFT",
        "CREATED",
        "PENDING",
        "SUBMITTED",
        "CONFIRMED",
        "IN_PROGRESS",
        "PROCESSING",
        "SHIPPED",
        "DELIVERING",
        "REVIEW",
        "COMPLETED",
        "DONE",
        "CLOSED",
        "CANCELLED",
        "REJECTED",
        "FAILED",
    ];
    for constant in objects(root).filter(|object| is_constant(*object)) {
        let tag = crate::java_xml::qname(constant);
        let path = format!("/root/{tag}");
        let object_line = Some(line(constant));
        let values = constant
            .children()
            .filter(|child| child.is_element() && crate::java_xml::qname(child) == "_value")
            .collect::<Vec<_>>();
        let mut score = 0;
        for business in objects(root).filter(|object| !is_constant(*object)) {
            for attribute in business.attributes() {
                if attribute.name().starts_with('_') {
                    continue;
                }
                if strip_translation(attribute.value())
                    .split_once('(')
                    .is_some_and(|(target, rest)| rest.contains(')') && target.trim() == tag)
                    && status_fields.contains(&attribute.name().to_ascii_lowercase().as_str())
                {
                    score += 3;
                }
            }
        }
        let lower_tag = tag.to_ascii_lowercase();
        if status_suffixes
            .iter()
            .any(|suffix| lower_tag.ends_with(suffix))
        {
            score += 2;
        }
        if constant.attribute("progress").is_some() {
            score += 2;
        }
        score += values
            .iter()
            .filter(|value| {
                value
                    .attribute("code")
                    .is_some_and(|code| status_codes.contains(&code.to_ascii_uppercase().as_str()))
            })
            .take(3)
            .count() as i32;
        if ["workflow", "lifecycle", "approval"]
            .iter()
            .any(|word| lower_tag.contains(word))
        {
            score += 1;
        }
        let status_like = score >= 3;
        let color = constant.attribute("color").is_some();
        let display_order = constant.attribute("display_order").is_some();
        let progress = constant.attribute("progress").is_some();
        if status_like && !color {
            response.suggestions.push(item(
                "KSML-UI-001",
                "Add status display metadata",
                format!("Status-like constant '{tag}' can add color=\"string()\" to declare visual badge colors for values."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
        }
        if !display_order {
            response.suggestions.push(item(
                "KSML-UI-002",
                "Add stable UI ordering",
                format!("Constant object '{tag}' can add display_order=\"number()\" for stable UI sorting."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
        }
        if status_like && !progress {
            response.suggestions.push(item(
                "KSML-UI-003",
                "Add status progress visualization",
                format!("Status-like constant '{tag}' can add progress=\"number()\" to visualize completion progress percentage (0-100)."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
        }
        if color {
            for (index, value) in values.iter().enumerate() {
                let code = value.attribute("code").unwrap_or("");
                let color_value = value.attribute("color").unwrap_or("");
                let value_path = format!("{path}/_value[{}]", index + 1);
                let finding = if color_value.trim().is_empty() {
                    Some((
                        "KSML-UI-011",
                        "Missing color value",
                        format!(
                            "Constant value '{code}' in constant '{tag}' is missing 'color' value."
                        ),
                    ))
                } else {
                    let digits = color_value.strip_prefix('#').unwrap_or("");
                    if !matches!(digits.len(), 3 | 6)
                        || !digits
                            .chars()
                            .all(|character| character.is_ascii_hexdigit())
                    {
                        Some((
                            "KSML-UI-012",
                            "Invalid hex color format",
                            format!(
                                "Hex color '{color_value}' in constant value '{code}' must be a valid hex color format (e.g. #16A34A)."
                            ),
                        ))
                    } else {
                        None
                    }
                };
                if let Some((id, title, message)) = finding {
                    response.warnings.push(item(
                        id,
                        title,
                        message,
                        value_path,
                        Some(tag),
                        Some("color"),
                        Some(filename),
                        Some(line(*value)),
                    ));
                }
            }
        }
        if display_order {
            let mut seen = HashSet::new();
            for (index, value) in values.iter().enumerate() {
                let code = value.attribute("code").unwrap_or("");
                let order = value.attribute("display_order").unwrap_or("");
                let finding = if order.trim().is_empty() {
                    Some((
                        "KSML-UI-021",
                        "Missing display_order value",
                        format!(
                            "Constant value '{code}' in constant '{tag}' is missing 'display_order' value."
                        ),
                    ))
                } else if let Ok(number) = order.parse::<i32>() {
                    if seen.insert(number) {
                        None
                    } else {
                        Some((
                            "KSML-UI-023",
                            "Duplicate display_order value",
                            format!(
                                "Duplicate display_order value '{number}' found in constant value '{code}'."
                            ),
                        ))
                    }
                } else {
                    Some((
                        "KSML-UI-022",
                        "Non-numeric display_order value",
                        format!(
                            "Display order '{order}' in constant value '{code}' must be numeric."
                        ),
                    ))
                };
                if let Some((id, title, message)) = finding {
                    response.warnings.push(item(
                        id,
                        title,
                        message,
                        format!("{path}/_value[{}]", index + 1),
                        Some(tag),
                        Some("display_order"),
                        Some(filename),
                        Some(line(*value)),
                    ));
                }
            }
        }
        if progress {
            for (index, value) in values.iter().enumerate() {
                let code = value.attribute("code").unwrap_or("");
                let progress_value = value.attribute("progress").unwrap_or("");
                let finding = if progress_value.trim().is_empty() {
                    Some((
                        "KSML-UI-031",
                        "Missing progress value",
                        format!(
                            "Constant value '{code}' in constant '{tag}' is missing 'progress' value."
                        ),
                    ))
                } else if let Ok(number) = progress_value.parse::<i32>() {
                    if !(0..=100).contains(&number) {
                        Some((
                            "KSML-UI-033",
                            "Invalid progress range",
                            format!(
                                "Progress value '{number}' in constant value '{code}' must be between 0 and 100."
                            ),
                        ))
                    } else {
                        None
                    }
                } else {
                    Some((
                        "KSML-UI-032",
                        "Non-numeric progress value",
                        format!(
                            "Progress value '{progress_value}' in constant value '{code}' must be numeric."
                        ),
                    ))
                };
                if let Some((id, title, message)) = finding {
                    response.warnings.push(item(
                        id,
                        title,
                        message,
                        format!("{path}/_value[{}]", index + 1),
                        Some(tag),
                        Some("progress"),
                        Some(filename),
                        Some(line(*value)),
                    ));
                }
            }
        }
    }
}

fn constant_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    let root_object = root_object_name(root);
    for object in objects(root).filter(|object| is_constant(*object)) {
        let tag = crate::java_xml::qname(object);
        let path = format!("/root/{tag}");
        let object_line = Some(line(object));
        let id_ok = object.attribute("id") == Some("id()");
        let name_ok = object
            .attribute("name")
            .is_some_and(|value| value.contains("string"));
        let code_ok = object
            .attribute("code")
            .is_some_and(|value| value.contains("string"));
        let identifier_ok = object.attribute("_identifier") == Some("code");
        if id_ok {
            response.solids.push(item(
                "KSML-CONSTANT-007",
                "Constant object ID definition valid",
                format!("Constant object '{tag}' correctly defines id=\"id()\"."),
                format!("{path}/id"),
                Some(tag),
                Some("id"),
                Some(filename),
                Some(attribute_line(object, "id")),
            ));
        } else {
            response.errors.push(item(
                "KSML-CONSTANT-007",
                "Constant object must define id=\"id()\"",
                format!("Constant object '{tag}' must explicitly define id=\"id()\". Only constant objects manually declare the ID field."),
                format!("{path}/id"),
                Some(tag),
                Some("id"),
                Some(filename),
                object_line,
            ));
        }
        if name_ok && code_ok && identifier_ok {
            response.solids.push(item(
                "KSML-CONSTANT-001",
                "Constant object properties valid",
                format!("Constant object '{tag}' has correctly defined constant properties."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
        } else {
            response.errors.push(item(
                "KSML-CONSTANT-001",
                "Invalid constant object definition",
                format!("Constant object '{tag}' must define name=\"string()\", code=\"string()\", and _identifier=\"code\"."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
        }
        if object.attribute(root_object.as_str()).is_some() {
            response.solids.push(item(
                "KSML-CONSTANT-002",
                "Constant object linked to root",
                format!("Constant object '{tag}' is correctly associated with the root object."),
                path.clone(),
                Some(tag),
                Some(&root_object),
                Some(filename),
                object_line,
            ));
        } else {
            response.errors.push(item(
                "KSML-CONSTANT-002",
                "Constant object missing root reference",
                format!("Constant object '{tag}' must be associated with the root object. For example, add {root_object}=\"{root_object}()\" to the definition."),
                path.clone(),
                Some(tag),
                Some(&root_object),
                Some(filename),
                object_line,
            ));
        }
        let mut attributes = object.attributes().collect::<Vec<_>>();
        attributes.sort_by_key(|attribute| attribute.name());
        for attribute in &attributes {
            let name = attribute.name();
            let value = attribute.value();
            if !name.starts_with('_') && !value.contains('(') && !value.contains(')') {
                response.errors.push(item(
                    "KSML-CONSTANT-003",
                    "Constant field definition contains concrete value",
                    format!("Constant field definition '{name}=\"{value}\"' should use a type function like string(), not a concrete value."),
                    format!("{path}/{name}"),
                    Some(tag),
                    Some(name),
                    Some(filename),
                    Some(attribute_line(object, name)),
                ));
            }
        }
        let values = object
            .children()
            .filter(|child| child.is_element() && crate::java_xml::qname(child) == "_value")
            .collect::<Vec<_>>();
        if values.is_empty() {
            response.errors.push(item(
                "KSML-CONSTANT-004",
                "Missing constant values",
                format!("Constant object '{tag}' must have at least one <_value> child element."),
                path,
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
            continue;
        }
        response.solids.push(item(
            "KSML-CONSTANT-004",
            "Constant values present",
            format!(
                "Constant object '{tag}' has {} constant value records.",
                values.len()
            ),
            path.clone(),
            Some(tag),
            None,
            Some(filename),
            object_line,
        ));
        let required_fields = attributes
            .iter()
            .filter(|attribute| {
                let name = attribute.name();
                let value = attribute.value();
                !name.starts_with('_')
                    && name != root_object
                    && !value.starts_with("createTime(")
                    && !value.starts_with("updateTime(")
                    && value.contains('(')
                    && value.contains(')')
            })
            .map(|attribute| attribute.name())
            .collect::<Vec<_>>();
        let mut sequence_ok = true;
        let mut complete = true;
        let mut expected_id = 1001;
        for (index, value) in values.iter().enumerate() {
            let value_path = format!("{path}/_value[{}]", index + 1);
            let value_id = value.attribute("id").unwrap_or("");
            let value_code = value.attribute("code").unwrap_or("");
            for field in &required_fields {
                if value
                    .attribute(*field)
                    .is_none_or(|content| content.trim().is_empty())
                {
                    complete = false;
                    let identity = if !value_code.trim().is_empty() {
                        value_code.to_string()
                    } else if !value_id.trim().is_empty() {
                        value_id.to_string()
                    } else {
                        format!("#{}", index + 1)
                    };
                    response.errors.push(item(
                        "KSML-CONSTANT-008",
                        "Incomplete constant value record",
                        format!("Constant value '{identity}' in constant object '{tag}' must explicitly provide field '{field}'. Every <_value> must provide every declared constant field, including nullable fields."),
                        format!("{value_path}/{field}"),
                        Some(tag),
                        Some(field),
                        Some(filename),
                        Some(line(*value)),
                    ));
                }
            }
            match value_id.parse::<i32>() {
                Ok(id) => {
                    if index == 0 && id != 1001 && id != 1 {
                        sequence_ok = false;
                        expected_id = id;
                    }
                    if id != expected_id + index as i32 {
                        sequence_ok = false;
                    }
                }
                Err(_) => sequence_ok = false,
            }
            let code_ok = value_code.split('_').all(|part| {
                !part.is_empty()
                    && part.chars().all(|character| {
                        character.is_ascii_uppercase() || character.is_ascii_digit()
                    })
            });
            if !value_code.trim().is_empty() && !code_ok {
                response.warnings.push(item(
                    "KSML-CONSTANT-006",
                    "Non-standard constant code format",
                    format!("Constant value code '{value_code}' in constant '{tag}' should be UPPERCASE_WITH_UNDERSCORES."),
                    value_path,
                    Some(tag),
                    Some("code"),
                    Some(filename),
                    Some(line(*value)),
                ));
            }
        }
        if !sequence_ok {
            response.warnings.push(item(
                "KSML-CONSTANT-005",
                "Non-standard constant value ID sequence",
                "Constant value IDs should preferably start from 1001 and increment sequentially."
                    .into(),
                path.clone(),
                Some(tag),
                Some("id"),
                Some(filename),
                object_line,
            ));
        }
        if complete {
            response.solids.push(item(
                "KSML-CONSTANT-008",
                "Constant value records complete",
                format!("Every value in constant object '{tag}' explicitly provides every declared constant field."),
                path,
                Some(tag),
                None,
                Some(filename),
                object_line,
            ));
        }
    }
}

fn object_metadata_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    if let Some(prompt) = root
        .descendants()
        .find(|node| node.is_element() && crate::java_xml::qname(node) == "_data_prompt")
    {
        response.warnings.push(item(
            "KSML-OBJECT-003",
            "Non-standard data prompt placement",
            "Object data prompt should preferably be defined as a metadata attribute _data_prompt=\"...\", not as a separate element.".into(),
            format!("/root/{}", crate::java_xml::qname(prompt)),
            None,
            None,
            Some(filename),
            Some(line(prompt)),
        ));
    }
    for object in objects(root) {
        let tag = crate::java_xml::qname(object);
        let path = format!("/root/{tag}");
        let mut missing = Vec::new();
        for attribute in ["_name", "_module", "_module_key"] {
            if object
                .attribute(attribute)
                .is_none_or(|value| value.trim().is_empty())
            {
                missing.push(attribute);
            }
        }
        if missing.is_empty() {
            response.solids.push(item(
                "KSML-OBJECT-001",
                "Object display metadata exists",
                format!("Object '{tag}' defines display name, module, and module key metadata."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                Some(line(object)),
            ));
        } else {
            response.warnings.push(item(
                "KSML-OBJECT-001",
                "Missing object display metadata",
                format!(
                    "Object '{tag}' is missing display metadata: {}",
                    missing.join(", ")
                ),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                Some(line(object)),
            ));
        }
        if let Some(module_key) = object.attribute("_module_key") {
            let kebab = module_key.split('-').all(|part| {
                !part.is_empty()
                    && part.chars().all(|character| {
                        character.is_ascii_lowercase() || character.is_ascii_digit()
                    })
            });
            if !kebab {
                response.warnings.push(item(
                    "KSML-OBJECT-002",
                    "Non-standard module key format",
                    format!("Module key '{module_key}' on object '{tag}' should be lowercase kebab-case."),
                    path,
                    Some(tag),
                    Some("_module_key"),
                    Some(filename),
                    Some(line(object)),
                ));
            }
        }
    }
}

fn domain_root_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    let business = objects(root)
        .filter(|object| !is_constant(*object))
        .map(|object| crate::java_xml::qname(object).to_string())
        .collect::<HashSet<_>>();
    if business.is_empty() {
        return;
    }
    let candidates = objects(root)
        .filter(|object| !is_constant(*object))
        .filter(|object| {
            let tag = crate::java_xml::qname(object);
            !object.attributes().any(|attribute| {
                if attribute.name().starts_with('_') {
                    return false;
                }
                attribute
                    .value()
                    .split_once('(')
                    .is_some_and(|(target, suffix)| {
                        suffix.contains(')')
                            && business.contains(target.trim())
                            && target.trim() != tag
                    })
            })
        })
        .map(|object| crate::java_xml::qname(object))
        .collect::<Vec<_>>();
    let (rule_id, title, message, solid) = match candidates.as_slice() {
        [] => (
            "KSML-DOMAIN-ROOT-001",
            "Missing domain root",
            "No clear domain root candidate found (all business objects reference other business objects).".to_string(),
            false,
        ),
        [only] => (
            "KSML-DOMAIN-ROOT-003",
            "One domain root found",
            format!("Exactly one domain root candidate '{only}' was found."),
            true,
        ),
        many => (
            "KSML-DOMAIN-ROOT-002",
            "Multiple domain roots",
            format!("Multiple independent business objects found: {}. The model may have disconnected graphs.", many.join(", ")),
            false,
        ),
    };
    let finding = item(
        rule_id,
        title,
        message,
        "/root".into(),
        None,
        None,
        Some(filename),
        Some(line(root)),
    );
    if solid {
        response.solids.push(finding);
    } else {
        response.warnings.push(finding);
    }
}

fn root_rule(
    root: Node<'_, '_>,
    filename: &str,
    scope: &str,
    domain: Option<&Domain>,
    response: &mut EvaluationResponse,
) {
    let path = "/root".to_string();
    let source_line = Some(line(root));
    match root
        .attribute("name")
        .filter(|name| !name.trim().is_empty())
    {
        None => response.errors.push(item(
            "KSML-ROOT-001",
            "Missing root name",
            "The <root> element must define a non-empty name attribute.".into(),
            path.clone(),
            None,
            None,
            Some(filename),
            source_line,
        )),
        Some(name) => {
            let kebab = name.split('-').all(|part| {
                !part.is_empty()
                    && part.chars().all(|character| {
                        character.is_ascii_lowercase() || character.is_ascii_digit()
                    })
            });
            if kebab && name.ends_with("-service") {
                response.solids.push(item(
                    "KSML-ROOT-003",
                    "Standard root name",
                    format!("Root name '{name}' is well-formed."),
                    path.clone(),
                    None,
                    None,
                    Some(filename),
                    source_line,
                ));
            } else {
                response.warnings.push(item(
                    "KSML-ROOT-002",
                    "Non-standard root name",
                    format!("Root name should preferably be lowercase kebab-case and end with -service (e.g. crm-service), but found '{name}'."),
                    path.clone(),
                    None,
                    None,
                    Some(filename),
                    source_line,
                ));
            }
        }
    }
    for attribute in ["alias_model_name", "english_name", "chinese_name"] {
        if root.attribute(attribute).is_some() {
            response.errors.push(item(
                "KSML-ROOT-004",
                "Deprecated root model-name attribute",
                format!(
                    "Remove '{attribute}'. Root 'name' is the only supported model-name attribute."
                ),
                path.clone(),
                None,
                None,
                Some(filename),
                source_line,
            ));
        }
    }
    match root
        .attribute("data_service")
        .filter(|value| !value.trim().is_empty())
    {
        None => {
            let stack = if scope.contains("rust") {
                "rust"
            } else {
                "java"
            };
            let candidates = domain
                .map(|domain| {
                    teaql_forge_model::data_services::all(&domain.source_data_services)
                        .iter()
                        .filter(|service| service.support_stack.contains(stack))
                        .map(|service| service.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "sqlite, rusqlite, postgres, mysql, linux".into());
            response.warnings.push(item(
                "KSML-ROOT-005",
                "Missing data service",
                format!("Please explicitly define data_service attribute on the <root> element. Candidates supported by current tech stack ({stack}): [{candidates}]"),
                path.clone(),
                None,
                None,
                Some(filename),
                source_line,
            ));
        }
        Some(value)
            if !matches!(
                value,
                "sqlite" | "postgres" | "mysql" | "rusqlite" | "linux" | "mssql" | "sqlserver"
            ) =>
        {
            response.warnings.push(item(
                "KSML-ROOT-005",
                "Non-standard data service",
                format!("AI-generated models typically use data_service=\"sqlite\" or \"rusqlite\", but found '{value}'."),
                path.clone(),
                None,
                None,
                Some(filename),
                source_line,
            ));
        }
        _ => {}
    }
    if root
        .attribute("org")
        .is_none_or(|org| org.trim().is_empty())
    {
        response.warnings.push(item(
            "KSML-ROOT-006",
            "Missing organization name",
            "Please explicitly define the org attribute on <root>. When the user does not specify an organization, use org=\"example\".".into(),
            path.clone(),
            None,
            None,
            Some(filename),
            source_line,
        ));
    }
    if let Some(module_key) = root.attribute("_module_key") {
        if module_key != "root" {
            response.warnings.push(item(
                "KSML-ROOT-007",
                "Non-standard module key",
                format!("_module_key for root should be \"root\", but found '{module_key}'."),
                path.clone(),
                None,
                None,
                Some(filename),
                source_line,
            ));
        }
    }
    match root.attribute("json_field_naming").filter(|value| !value.trim().is_empty()) {
        None => response.solids.push(item(
            "KSML-ROOT-008",
            "Default JSON field naming",
            "External JSON business fields use the default camelCase profile.".into(),
            path,
            None,
            None,
            Some(filename),
            source_line,
        )),
        Some(value) if matches!(value, "camelCase" | "snake_case" | "PascalCase") => {
            response.solids.push(item(
                "KSML-ROOT-008",
                "Explicit JSON field naming",
                format!("External JSON business fields use the {value} profile."),
                format!("{path}/@json_field_naming"),
                None,
                None,
                Some(filename),
                source_line,
            ));
        }
        Some(value) => response.errors.push(item(
            "KSML-ROOT-008",
            "Unsupported JSON field naming",
            format!("json_field_naming must be one of camelCase, snake_case, or PascalCase, but found '{value}'."),
            format!("{path}/@json_field_naming"),
            None,
            None,
            Some(filename),
            source_line,
        )),
    }
}

fn business_rule(root: Node<'_, '_>, filename: &str, response: &mut EvaluationResponse) {
    let root_object = root_object_name(root);
    let mut non_root_count = 0;
    let mut object_count = 0;
    for object in objects(root).filter(|object| !is_constant(*object)) {
        object_count += 1;
        let tag = crate::java_xml::qname(object);
        if tag != root_object {
            non_root_count += 1;
        }
        let path = format!("/root/{tag}");
        if let Some(id_value) = object.attribute("id") {
            response.errors.push(item(
                "KSML-BUSINESS-004",
                "Business object must not define an ID field",
                format!("Business object '{tag}' must not manually define the 'id' attribute. Remove id=\"{id_value}\"; the framework automatically injects the u64 primary key during generation."),
                format!("{path}/id"),
                Some(tag),
                Some("id"),
                Some(filename),
                Some(attribute_line(object, "id")),
            ));
        }
        if object.attribute("_identifier").is_some()
            || object
                .children()
                .any(|child| child.is_element() && crate::java_xml::qname(child) == "_value")
        {
            response.errors.push(item(
                "KSML-BUSINESS-001",
                "Contradictory business object definition",
                format!("Business object '{tag}' contains constant-only definitions (_identifier or <_value> child elements)."),
                path.clone(),
                Some(tag),
                None,
                Some(filename),
                Some(line(object)),
            ));
        }
        // Xerces' NamedNodeMap traverses attributes by name, not XML position.
        let mut attributes = object.attributes().collect::<Vec<_>>();
        attributes.sort_by_key(|attribute| attribute.name());
        for attribute in attributes {
            let name = attribute.name();
            if name.starts_with('_') {
                continue;
            }
            let value = attribute.value();
            let value = value
                .split_once(':')
                .or_else(|| value.split_once('：'))
                .map_or(value, |(_, translated)| translated.trim());
            if value.contains("string(") || value == "string()" {
                response.warnings.push(item(
                    "KSML-BUSINESS-002",
                    "Field uses generic string() definition",
                    format!("Field '{name}' in business object '{tag}' should preferably use a rich concrete typical value instead of generic string()."),
                    format!("{path}/{name}"),
                    Some(tag),
                    Some(name),
                    Some(filename),
                    Some(attribute_line(object, name)),
                ));
            }
        }
    }
    if object_count > 0 && non_root_count == 0 {
        response.errors.push(item(
            "KSML-BUSINESS-003",
            "Missing business objects",
            format!("The model must define at least one business object other than the root object ('{root_object}') and constant objects."),
            "/root".into(),
            None,
            None,
            Some(filename),
            Some(line(root)),
        ));
    }
}
