use crate::error::ParseError;
use crate::ir::{Cardinality, Domain, Entity, EntityMember, Field, FieldType, Relation};
use std::collections::HashSet;

// DomainParser and ObjectDescriptor flags use Hutool BooleanUtil.toBoolean(String).
// String.trim() removes only U+0000..U+0020, then Hutool checks TRUE_SET.
pub fn hutool_true(value: &str) -> bool {
    matches!(
        value
            .trim_matches(|ch| ch <= '\u{20}')
            .to_lowercase()
            .as_str(),
        "true" | "yes" | "y" | "t" | "ok" | "1" | "on" | "是" | "对" | "真" | "對" | "√"
    )
}

#[cfg(test)]
mod hutool_boolean_tests {
    use super::hutool_true;

    #[test]
    fn delete_flag_uses_hutool_trim_and_true_set() {
        for value in [" true ", "\tON\n", " 真 ", "對", "√"] {
            assert!(hutool_true(value), "{value:?}");
        }
        for value in ["", " false ", "not true", "\u{a0}true\u{a0}"] {
            assert!(!hutool_true(value), "{value:?}");
        }
    }
}

fn inferred_source_type(value: &str) -> &str {
    // Named type calls are resolved against the actual source catalog first.
    // In particular, decimal/money/boolean/datetime are not built-in types.
    if let Some(number) = value.strip_suffix(['f', 'F', 'd', 'D']) {
        if !number.is_empty()
            && number.parse::<f64>().is_ok()
            && number
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
        {
            return if value.ends_with(['f', 'F']) {
                "float"
            } else {
                "double"
            };
        }
    }
    if !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) {
        return "integer";
    }
    if value
        .strip_suffix(['l', 'L'])
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
    {
        return "long";
    }
    if value.split_once('.').is_some_and(|(whole, fraction)| {
        !whole.is_empty()
            && !fraction.is_empty()
            && whole.chars().all(|c| c.is_ascii_digit())
            && fraction.chars().all(|c| c.is_ascii_digit())
    }) {
        return "number";
    }
    if matches!(value, "true" | "false" | "on" | "off" | "yes" | "no") {
        return "bool";
    }
    let date_parts = value.split('-').collect::<Vec<_>>();
    if date_parts.len() == 3
        && date_parts[0].len() == 4
        && (1..=2).contains(&date_parts[1].len())
        && (1..=2).contains(&date_parts[2].len())
        && date_parts
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_digit()))
    {
        return "date";
    }
    "string"
}

fn field_attributes(
    value: &str,
    json_literal: bool,
) -> (String, std::collections::BTreeMap<String, String>) {
    let mut metadata = std::collections::BTreeMap::new();
    if json_literal {
        metadata.insert("sampleValue".into(), value.trim().into());
        return (value.trim().into(), metadata);
    }
    let mut parts = crate::field_attributes::split(value, '|');
    if parts.len() > 1 {
        if let Some(range) = parts
            .last()
            .and_then(|part| part.strip_prefix('['))
            .and_then(|part| part.strip_suffix(']'))
        {
            if let Some((min, max)) = range.split_once(',') {
                metadata.insert("min".into(), min.trim().into());
                metadata.insert("max".into(), max.trim().into());
                parts.pop();
            }
        }
    }
    if let Some(last) = parts.last_mut() {
        if last.ends_with('?') {
            last.pop();
            metadata.insert("optional".into(), "true".into());
        }
    }
    let sample = parts.last().cloned().unwrap_or_default();
    metadata.insert("sampleValue".into(), sample.clone());
    if parts.len() > 1 {
        metadata.insert("candidates".into(), parts.join(","));
    }
    if let Some((_, options)) = sample.split_once(['(', '{']) {
        if let Some(options) = options.strip_suffix([')', '}']) {
            for (key, value) in crate::field_attributes::options(options) {
                metadata.insert(key, value);
            }
            if parts.len() > 1 {
                parts.pop();
                metadata.insert("candidates".into(), parts.join(","));
            }
        }
    }
    (sample, metadata)
}

pub fn parse_model(src: &str, xml_path: &str) -> Result<Domain, ParseError> {
    parse_model_with_locations(src, xml_path, SourceLocations::EndOfStartTag)
}

/// Providers expose different SAX locator positions at startElement.
#[derive(Clone, Copy)]
pub enum SourceLocations<'a> {
    StartOfStartTag,
    EndOfStartTag,
    Resolve(&'a dyn Fn(usize, &str) -> Option<(String, usize)>),
}

pub fn parse_model_with_locations(src: &str, xml_path: &str, locations: SourceLocations) -> Result<Domain, ParseError> {
    let doc = roxmltree::Document::parse_with_options(
        src,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )?;

    let root_node = doc.root_element();
    let mut root_attributes_ordered = Vec::new();
    for attribute in root_node.attributes() {
        crate::attributes::merge_xml_attribute(
            &mut root_attributes_ordered,
            attribute.name(),
            attribute.value(),
        );
    }
    if let Some(error) = crate::types::inheritance_failure(&doc) {
        return Err(match error {
            crate::types::InheritanceFailure::Cycle => ParseError::TypeInheritanceCycle,
            crate::types::InheritanceFailure::MissingParent => {
                ParseError::UndefinedTypeParent(crate::types::MISSING_PARENT_MESSAGE.into())
            }
        });
    }
    let source_types = crate::types::definitions(&doc);
    let builtin_types = crate::types::builtin_types();
    if !matches!(root_node.tag_name().name(), "root" | "_root") {
        return Err(ParseError::MissingDomain);
    }

    let root_attribute = |name: &str| {
        root_attributes_ordered
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let domain_name = root_attribute("name").unwrap_or("AppDomain").to_string();
    let global_data_service = root_attribute("data_service").map(|s| s.to_string());
    let global_audit_mask_fields = root_attribute("audit_mask_fields").map(|s| s.to_string());
    let global_audit_value_max_len =
        root_attribute("audit_value_max_len").and_then(|s| s.parse().ok());

    let library_documents = [
        (
            "iam",
            roxmltree::Document::parse(include_str!("data/iam.xml")).expect("pinned IAM model"),
        ),
        (
            "saas-service-basic",
            roxmltree::Document::parse(include_str!("data/saas-service-basic.xml"))
                .expect("pinned SaaS model"),
        ),
    ]
    .into_iter()
    .collect::<std::collections::BTreeMap<_, _>>();
    let mut nodes = Vec::new();
    collect_nodes(
        root_node,
        xml_path,
        &library_documents,
        &mut nodes,
        &mut HashSet::new(),
    );
    let mut entities = Vec::new();
    let mut source_field_attributes = crate::field_attributes::Fields::new();
    // DomainParser shares one Domain across the entrance XML and included
    // files. Once an object is deleted, a later declaration cannot revive it.
    let mut deleted_names = HashSet::new();
    let entity_names = nodes
        .iter()
        .filter(|(node, _)| !node.tag_name().name().starts_with('_'))
        .map(|(node, _)| node.tag_name().name().to_string())
        .collect::<HashSet<_>>();

    // Library definitions and repeated objects follow the SAX input order.
    for (node, node_path) in nodes {
        let entity_name = node.tag_name().name().to_string();
        if entity_name.starts_with('_') || entity_name == "root" {
            continue;
        }

        let mut members = Vec::new();
        let mut has_id = false;
        let mut is_human = false;
        let mut data_service = None;
        let mut audit_mask_fields = None;
        let mut audit_value_max_len = None;
        let mut metadata = std::collections::BTreeMap::new();
        let mut metadata_ordered = Vec::new();
        let start_pos = node.range().start;
        let text = node.document().input_text();
        let sub_text = &text[start_pos..];
        let offset = sub_text.find('>').unwrap_or(0);
        let end_pos = start_pos + offset;
        let location_pos = match locations {
            SourceLocations::StartOfStartTag | SourceLocations::Resolve(_) => start_pos,
            SourceLocations::EndOfStartTag => end_pos,
        };
        let default_line = node.document().text_pos_at(location_pos).row as usize;
        let (node_path, line_number) = match locations {
            SourceLocations::Resolve(resolve) => resolve(start_pos, node_path)
                .unwrap_or_else(|| (node_path.to_owned(), default_line)),
            _ => (node_path.to_owned(), default_line),
        };

        for attr in node.attributes() {
            let attr_name = attr.name();
            let original_value = attr.value();
            let parsed_value = crate::field_attributes::parse_value(original_value);
            let json_literal = parsed_value.json_literal;
            let display_name = parsed_value.label.as_deref();
            let attr_value = parsed_value.value.as_str();

            if let Some(key) = attr_name.strip_prefix('_') {
                metadata.insert(key.to_string(), attr_value.to_string());
                metadata_ordered.push((key.to_string(), attr_value.to_string()));

                if key == "cat" && attr_value.eq_ignore_ascii_case("human") {
                    is_human = true;
                }
                if key == "data_service" {
                    data_service = Some(attr_value.to_string());
                }
                if key == "audit_mask_fields" {
                    audit_mask_fields = Some(attr_value.to_string());
                }
                if key == "audit_value_max_len" {
                    audit_value_max_len = attr_value.parse::<usize>().ok();
                }
                continue;
            } else if attr_name == "audit_mask_fields" {
                audit_mask_fields = Some(attr_value.to_string());
            } else if attr_name == "audit_value_max_len" {
                audit_value_max_len = attr_value.parse().ok();
            } else if attr_name != "id" && attr_name != "version" {
            }

            crate::field_attributes::declare(&mut source_field_attributes, &entity_name, attr_name, attr_value, display_name, json_literal);
            if attr_name == "id" && (attr_value == "id()" || attr_value == "id") {
                members.push(EntityMember::Field(Field {
                    name: attr_name.to_string(),
                    ty: FieldType::Id,
                    source_type_name: "id".to_string(),
                    type_attributes: source_types.get("id").cloned().unwrap_or_default(),
                    source_order: 0,
                    required: true,
                    unique: true,
                    line_number,
                    xml_path: node_path.to_string(),
                    metadata: std::collections::BTreeMap::new(),
                    source_value: attr_value.to_string(),
                }));
                has_id = true;
                continue;
            }

            let (type_sample, mut field_metadata) = field_attributes(attr_value, json_literal);
            let sample = type_sample.as_str();
            // Type matching happens after ranges and the optional suffix are removed.
            let (is_relation, rel_target, cardinality) = if let Some(idx) = sample.find(['(', '{'])
            {
                if sample.ends_with([')', '}']) {
                    let prefix = &sample[..idx];
                    let inside = &sample[idx + 1..sample.len() - 1];
                    let mut rel_type = Cardinality::ManyToOne;
                    if inside.contains("relationType=oneToOne")
                        || inside.contains("relationType=OneToOne")
                    {
                        rel_type = Cardinality::OneToOne;
                    }
                    match prefix {
                        "string" | "number" | "bool" | "date" | "datetime" | "money" | "text"
                        | "createTime" | "updateTime" | "id" | "decimal" => {
                            (false, "", Cardinality::ManyToOne)
                        }
                        _ => {
                            let mut target = if entity_names.contains(prefix) {
                                prefix
                            } else if let Some(pipe_idx) = inside.find('|') {
                                &inside[..pipe_idx]
                            } else {
                                inside
                            };
                            if target.is_empty() || target == "context" {
                                target = prefix;
                            }
                            (entity_names.contains(target), target, rel_type)
                        }
                    }
                } else {
                    (false, "", Cardinality::ManyToOne)
                }
            } else {
                (false, "", Cardinality::ManyToOne)
            };

            if is_relation {
                if let Some(display_name) = display_name {
                    field_metadata.insert("zh_CN".into(), display_name.into());
                }
                members.push(EntityMember::Relation(Relation {
                    name: attr_name.to_string(),
                    target: rel_target.to_string(),
                    metadata: field_metadata.clone(),
                    source_order: members.len() + 1,
                    cardinality,
                    required: field_metadata.get("optional").map(String::as_str) != Some("true"),
                    source_value: attr_value.to_string(),
                    line_number,
                    xml_path: node_path.to_string(),
                }));
            } else {
                let declared = sample
                    .split_once(['(', '{'])
                    .filter(|_| sample.ends_with([')', '}']))
                    .map(|(name, _)| name);
                let source_type_name = declared
                    .and_then(|declared| {
                        source_types
                            .keys()
                            .chain(builtin_types.keys())
                            .find(|name| name.eq_ignore_ascii_case(declared))
                            .map(String::as_str)
                    })
                    .unwrap_or_else(|| inferred_source_type(sample));
                let type_attributes = source_types
                    .get(source_type_name)
                    .cloned()
                    .unwrap_or_default();
                let type_attribute = |name: &str| {
                    type_attributes
                        .iter()
                        .find(|attribute| attribute.name == name)
                        .map(|attribute| attribute.value.as_str())
                };
                let ty = match source_type_name {
                    "id" => FieldType::Id,
                    "number" | "decimal" | "money" | "float" | "double" => FieldType::Decimal,
                    "integer" => FieldType::I32,
                    "long" => FieldType::I64,
                    "text" | "jsonMe" => FieldType::Text,
                    "bool" | "boolean" => FieldType::Bool,
                    "date" => FieldType::Date,
                    "datetime" | "dateTime" | "createTime" | "updateTime" => FieldType::DateTime,
                    _ if type_attribute("isText") == Some("true") => FieldType::Text,
                    _ => FieldType::String,
                };
                if let Some(display_name) = display_name {
                    field_metadata.insert("zh_CN".to_string(), display_name.to_string());
                }

                members.push(EntityMember::Field(Field {
                    name: attr_name.to_string(),
                    ty,
                    source_type_name: source_type_name.to_string(),
                    source_order: members.len() + 1,
                    required: field_metadata
                        .get("optional")
                        .map(String::as_str)
                        .or_else(|| type_attribute("optional"))
                        .map(|value| value != "true")
                        .unwrap_or(!matches!(source_type_name, "text" | "jsonMe")),
                    unique: false,
                    line_number,
                    xml_path: node_path.to_string(),
                    metadata: field_metadata,
                    type_attributes,
                    source_value: attr_value.to_string(),
                }));
            }
        }

        if !has_id {
            members.insert(
                0,
                EntityMember::Field(Field {
                    name: "id".to_string(),
                    ty: FieldType::Id,
                    source_type_name: "id".to_string(),
                    type_attributes: source_types.get("id").cloned().unwrap_or_default(),
                    source_order: 0,
                    required: true,
                    unique: true,
                    line_number: 0,
                    xml_path: "".to_string(),
                    metadata: std::collections::BTreeMap::new(),
                    source_value: String::new(),
                }),
            );
        }

        let has_version = members.iter().any(|m| match m {
            EntityMember::Field(f) => f.name == "version",
            _ => false,
        });

        if !has_version && !metadata.contains_key("parent") {
            members.push(EntityMember::Field(Field {
                name: "version".to_string(),
                ty: FieldType::I64, // Assume version is i64
                source_type_name: "version".to_string(),
                type_attributes: source_types.get("version").cloned().unwrap_or_default(),
                source_order: usize::MAX,
                required: true,
                unique: false,
                line_number: 0,
                xml_path: "".to_string(),
                metadata: std::collections::BTreeMap::new(),
                source_value: String::new(),
            }));
        }

        let deleted_here = metadata.get("deleted").is_some_and(|value| hutool_true(value));
        if deleted_here {
            deleted_names.insert(entity_name.clone());
        }

        let entity = Entity {
            name: entity_name,
            table: None, // Can be derived in normalize/naming step
            members,
            is_human,
            data_service: data_service.or(global_data_service.clone()),
            audit_mask_fields: audit_mask_fields.or(global_audit_mask_fields.clone()),
            audit_value_max_len: audit_value_max_len.or(global_audit_value_max_len),
            metadata,
            metadata_ordered,
            seed_values: Vec::new(),
            line_number,
            xml_path: node_path.to_string(),
        };
        let entity_name = entity.name.clone();
        if let Some(identifier) = entity.metadata.get("identifier") {
            crate::field_attributes::identify(&mut source_field_attributes, &entity_name, identifier);
        }
        merge_entity(&mut entities, entity);
        let entity = entities
            .iter_mut()
            .find(|entity| entity.name == entity_name)
            .unwrap();
        for child in node
            .children()
            .filter(|child| child.is_element() && child.tag_name().name() == "_value")
        {
            let remove = child.attribute("_delete").is_some_and(hutool_true);
            for attribute in child
                .attributes()
                .filter(|attribute| !attribute.name().starts_with('_'))
            {
                let Some(member) = entity
                    .members
                    .iter_mut()
                    .find(|member| member_name(member) == attribute.name())
                else {
                    continue;
                };
                let metadata = match member {
                    EntityMember::Field(field) => &mut field.metadata,
                    EntityMember::Relation(relation) => &mut relation.metadata,
                };
                crate::field_attributes::candidate(&mut source_field_attributes, &entity_name, attribute.name(), attribute.value(), remove);
                if remove {
                    if let Some(existing) = metadata.get_mut("candidates") {
                        let mut values = crate::field_attributes::split_java_commas(existing)
                            .into_iter().map(str::to_string).collect::<Vec<_>>();
                        for value in crate::field_attributes::split_java_commas(attribute.value()) {
                            if let Some(index) =
                                values.iter().position(|existing| existing == value)
                            {
                                values.remove(index);
                            }
                        }
                        *existing = values.join(",");
                    }
                } else if let Some(existing) = metadata.get_mut("candidates") {
                    existing.push(',');
                    existing.push_str(attribute.value());
                } else {
                    metadata.insert("candidates".into(), attribute.value().into());
                }
            }
        }
    }

    entities.retain(|entity| !deleted_names.contains(&entity.name));
    let source_data_services = crate::data_services::definitions(&doc);
    let candidates = crate::data_services::all(&source_data_services)
        .iter()
        .map(|service| service.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    for entity in &entities {
        let explicit = entity.metadata.get("data_service");
        let service = explicit
            .or(global_data_service.as_ref())
            .map(String::as_str)
            .unwrap_or("postgres");
        if crate::data_services::find(&source_data_services, service).is_none() {
            let location = explicit
                .map(|_| format!(" on ({})", entity.name))
                .unwrap_or_default();
            return Err(ParseError::UndefinedDataService(format!(
                "data service({service}){location} is not defined, candidates: [{candidates}]"
            )));
        }
    }
    let type_names = builtin_types.keys().chain(source_types.keys()).cloned().collect::<Vec<_>>();
    crate::field_attributes::finish(&mut source_field_attributes, &entities, &type_names);
    // The source parses options only after all repeated declarations and value
    // rows have merged. Replace the eager metadata with that final physical state.
    for entity in &mut entities {
        if let Some(fields) = source_field_attributes.get(&entity.name) {
            for member in &mut entity.members {
                let (name, metadata) = match member {
                    EntityMember::Field(field) => (&field.name, &mut field.metadata),
                    EntityMember::Relation(field) => (&field.name, &mut field.metadata),
                };
                if let Some(attributes) = fields.get(name) {
                    *metadata = attributes.iter().map(|attribute|(attribute.name.clone(),attribute.value.clone())).collect();
                }
            }
        }
    }

    // The source builds constant rows from the finalized candidate attributes.
    // Type resolution can remove the sample token and trim candidate boundaries.
    // Column candidate lists remain independent, so a deletion may shift rows.
    for entity in &mut entities {
        let rows = entity
            .members
            .first()
            .and_then(|member| match member {
                EntityMember::Field(field) => field.metadata.get("candidates"),
                EntityMember::Relation(relation) => relation.metadata.get("candidates"),
            })
            .map(|values| values.split(',').filter(|value| !value.is_empty()).count())
            .unwrap_or(0);
        for index in 0..rows {
            let mut properties = std::collections::BTreeMap::new();
            for member in &entity.members {
                match member {
                    EntityMember::Field(field) if field.name != "version" => {
                        let value = field
                            .metadata
                            .get("candidates")
                            .or_else(|| field.metadata.get("sampleValue"))
                            .and_then(|value| {
                                value
                                    .split(',')
                                    .filter(|value| !value.is_empty())
                                    .nth(index)
                            })
                            .unwrap_or("1");
                        properties.insert(field.name.clone(), value.to_string());
                    }
                    EntityMember::Relation(relation) => {
                        properties.insert(relation.name.clone(), "1".into());
                    }
                    _ => {}
                }
            }
            entity.seed_values.push(crate::ir::SeedValue { properties });
        }
    }

    Ok(Domain {
        source_field_attributes,
        source_type_states: crate::types::states(&doc, xml_path),
        source_types,
        source_data_services,
        name: domain_name,
        entities,
        root_attributes: root_attributes_ordered.iter().cloned().collect(),
        root_attributes_ordered,
    })
}

fn member_name(member: &EntityMember) -> &str {
    match member {
        EntityMember::Field(field) => &field.name,
        EntityMember::Relation(relation) => &relation.name,
    }
}

fn merge_entity(entities: &mut Vec<Entity>, mut incoming: Entity) {
    let Some(existing) = entities
        .iter_mut()
        .find(|entity| entity.name == incoming.name)
    else {
        entities.push(incoming);
        return;
    };
    // Synthesized id/version are added by the source after all definitions are read.
    existing
        .members
        .retain(|member| !matches!(member, EntityMember::Field(field) if field.line_number == 0));
    incoming
        .members
        .retain(|member| !matches!(member, EntityMember::Field(field) if field.line_number == 0));
    for mut member in incoming.members {
        if let Some(index) = existing
            .members
            .iter()
            .position(|item| member_name(item) == member_name(&member))
        {
            if let (EntityMember::Field(old), EntityMember::Field(new)) =
                (&existing.members[index], &mut member)
            {
                let mut metadata = old.metadata.clone();
                metadata.extend(new.metadata.clone());
                new.required = metadata
                    .get("optional")
                    .map(String::as_str)
                    .or_else(|| {
                        new.type_attributes
                            .iter()
                            .find(|attribute| attribute.name == "optional")
                            .map(|attribute| attribute.value.as_str())
                    })
                    .map(|value| value != "true")
                    .unwrap_or(new.required);
                new.metadata = metadata;
                new.source_order = old.source_order;
            }
            existing.members[index] = member;
        } else {
            match &mut member {
                EntityMember::Field(field) => field.source_order = existing.members.len() + 1,
                EntityMember::Relation(relation) => {
                    relation.source_order = existing.members.len() + 1
                }
            }
            existing.members.push(member);
        }
    }
    for (name, value) in incoming.metadata_ordered {
        existing.metadata_ordered.retain(|(key, _)| key != &name);
        existing.metadata_ordered.push((name, value));
    }
    existing.metadata.extend(incoming.metadata);
    existing.is_human = existing
        .metadata
        .get("cat")
        .is_some_and(|value| value.eq_ignore_ascii_case("human"));
    existing.data_service = existing
        .metadata
        .get("data_service")
        .cloned()
        .or(existing.data_service.clone());
    existing.audit_mask_fields = existing
        .metadata
        .get("audit_mask_fields")
        .cloned()
        .or(existing.audit_mask_fields.clone());
    existing.audit_value_max_len = existing
        .metadata
        .get("audit_value_max_len")
        .and_then(|value| value.parse().ok())
        .or(existing.audit_value_max_len);
    existing.line_number = incoming.line_number;
    existing.xml_path = incoming.xml_path;
    existing.seed_values.extend(incoming.seed_values);
    // Retain the source's final synthetic fields at the beginning and end.
    if !existing
        .members
        .iter()
        .any(|member| member_name(member) == "id")
    {
        existing.members.insert(
            0,
            EntityMember::Field(Field {
                name: "id".into(),
                ty: FieldType::Id,
                source_type_name: "id".into(),
                type_attributes: Vec::new(),
                source_order: 0,
                required: true,
                unique: true,
                line_number: 0,
                xml_path: String::new(),
                metadata: Default::default(),
                source_value: String::new(),
            }),
        );
    }
    if !existing.metadata.contains_key("parent")
        && !existing
            .members
            .iter()
            .any(|member| member_name(member) == "version")
    {
        existing.members.push(EntityMember::Field(Field {
            name: "version".into(),
            ty: FieldType::I64,
            source_type_name: "version".into(),
            type_attributes: Vec::new(),
            source_order: usize::MAX,
            required: true,
            unique: false,
            line_number: 0,
            xml_path: String::new(),
            metadata: Default::default(),
            source_value: String::new(),
        }));
    }
}

fn collect_nodes<'a, 'input>(
    parent: roxmltree::Node<'a, 'input>,
    path: &'a str,
    libraries: &'a std::collections::BTreeMap<&str, roxmltree::Document<'input>>,
    output: &mut Vec<(roxmltree::Node<'a, 'input>, &'a str)>,
    active: &mut HashSet<String>,
) {
    for node in parent.children().filter(|node| node.is_element()) {
        if node.tag_name().name() == "_include" {
            if let Some(name) = node.attribute("lib") {
                if let Some(document) = libraries.get(name) {
                    if active.insert(name.to_string()) {
                        collect_nodes(document.root_element(), "null", libraries, output, active);
                        active.remove(name);
                    }
                    continue;
                }
            }
        }
        output.push((node, path));
        collect_nodes(node, path, libraries, output, active);
    }
}
