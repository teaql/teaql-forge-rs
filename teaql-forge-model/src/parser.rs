use crate::error::ParseError;
use crate::ir::{Cardinality, Domain, Entity, EntityMember, Field, FieldType, Relation};

pub fn parse_model(src: &str, xml_path: &str) -> Result<Domain, ParseError> {
    let doc = roxmltree::Document::parse(src)?;

    let root_node = doc.root_element();
    if root_node.tag_name().name() != "root" {
        return Err(ParseError::MissingDomain);
    }

    let domain_name = root_node
        .attribute("name")
        .unwrap_or("AppDomain")
        .to_string();
    let global_data_service = root_node.attribute("data_service").map(|s| s.to_string());
    let global_audit_mask_fields = root_node
        .attribute("audit_mask_fields")
        .map(|s| s.to_string());
    let global_audit_value_max_len = root_node
        .attribute("audit_value_max_len")
        .and_then(|s| s.parse().ok());

    let mut entities = Vec::new();

    // Iterate over children of root. Each child is an Entity.
    for node in root_node.children().filter(|n| n.is_element()) {
        let entity_name = node.tag_name().name().to_string();

        let mut members = Vec::new();
        let mut has_id = false;
        let mut is_human = false;
        let mut data_service = None;
        let mut audit_mask_fields = None;
        let mut audit_value_max_len = None;
        let mut metadata = std::collections::BTreeMap::new();
        let start_pos = node.range().start;
        let text = node.document().input_text();
        let sub_text = &text[start_pos..];
        let offset = sub_text.find('>').unwrap_or(0);
        let end_pos = start_pos + offset;
        let line_number = node.document().text_pos_at(end_pos).row as usize;

        for attr in node.attributes() {
            let attr_name = attr.name();
            let attr_value = attr.value();

            if let Some(key) = attr_name.strip_prefix('_') {
                metadata.insert(key.to_string(), attr_value.to_string());

                if key == "category" && attr_value.eq_ignore_ascii_case("human") {
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
            } else if attr_name == "data_service" {
                data_service = Some(attr_value.to_string());
            } else if attr_name == "audit_mask_fields" {
                audit_mask_fields = Some(attr_value.to_string());
            } else if attr_name == "audit_value_max_len" {
                audit_value_max_len = attr_value.parse().ok();
            } else if attr_name != "id" && attr_name != "version" {
            }

            let attr_value = attr.value();

            if attr_name == "id" && (attr_value == "id()" || attr_value == "id") {
                members.push(EntityMember::Field(Field {
                    name: attr_name.to_string(),
                    ty: FieldType::Id,
                    required: true,
                    unique: true,
                    line_number,
                    xml_path: xml_path.to_string(),
                    metadata: std::collections::BTreeMap::new(),
                }));
                has_id = true;
                continue;
            }

            // Parse relations with or without metadata in parenthesis
            let (is_relation, rel_target, cardinality) = if let Some(idx) = attr_value.find('(') {
                if attr_value.ends_with(')') {
                    let prefix = &attr_value[..idx];
                    let inside = &attr_value[idx + 1..attr_value.len() - 1];
                    let mut rel_type = Cardinality::ManyToOne;
                    if inside.contains("relationType=oneToOne")
                        || inside.contains("relationType=OneToOne")
                    {
                        rel_type = Cardinality::OneToOne;
                    }
                    match prefix {
                        "string" | "number" | "bool" | "date" | "datetime" | "money" | "text"
                        | "createTime" | "updateTime" | "id" => (false, "", Cardinality::ManyToOne),
                        _ => {
                            let mut target = if let Some(pipe_idx) = inside.find('|') {
                                &inside[..pipe_idx]
                            } else {
                                inside
                            };
                            if target.is_empty() || target == "context" {
                                target = prefix;
                            }
                            (true, target, rel_type)
                        }
                    }
                } else {
                    (false, "", Cardinality::ManyToOne)
                }
            } else {
                (false, "", Cardinality::ManyToOne)
            };

            if is_relation {
                members.push(EntityMember::Relation(Relation {
                    name: attr_name.to_string(),
                    target: rel_target.to_string(),
                    cardinality,
                    required: !attr_value.ends_with('?'),
                    line_number,
                    xml_path: xml_path.to_string(),
                }));
            } else {
                let ty = if attr_value.contains("string") || attr_value.contains('|') {
                    FieldType::String
                } else if attr_value.contains("number") {
                    FieldType::I32
                } else if attr_value.contains("text") {
                    FieldType::Text
                } else if attr_value.contains("bool") {
                    FieldType::Bool
                } else if attr_value.contains("createTime") || attr_value.contains("updateTime") {
                    FieldType::DateTime
                } else {
                    FieldType::String // fallback
                };
                let mut metadata = std::collections::BTreeMap::new();
                let sample_value = if attr_value.starts_with("string|sampleValue=") {
                    Some(attr_value.strip_prefix("string|sampleValue=").unwrap())
                } else if attr_value.starts_with("string|") {
                    Some(attr_value.strip_prefix("string|").unwrap())
                } else if attr_value.contains("sampleValue=") {
                    let idx = attr_value.find("sampleValue=").unwrap();
                    let rest = &attr_value[idx + 12..];
                    if let Some(end) = rest.find('|') {
                        Some(&rest[..end])
                    } else {
                        Some(rest)
                    }
                } else if !attr_value.ends_with("()") && !attr_value.is_empty() {
                    Some(attr_value)
                } else {
                    None
                };
                if let Some(sv) = sample_value {
                    metadata.insert("sampleValue".to_string(), sv.to_string());
                }

                members.push(EntityMember::Field(Field {
                    name: attr_name.to_string(),
                    ty,
                    required: !attr_value.ends_with('?'),
                    unique: false,
                    line_number,
                    xml_path: xml_path.to_string(),
                    metadata,
                }));
            }
        }

        if !has_id {
            members.insert(
                0,
                EntityMember::Field(Field {
                    name: "id".to_string(),
                    ty: FieldType::Id,
                    required: true,
                    unique: true,
                    line_number: 0,
                    xml_path: "".to_string(),
                    metadata: std::collections::BTreeMap::new(),
                }),
            );
        }

        let has_version = members.iter().any(|m| match m {
            EntityMember::Field(f) => f.name == "version",
            _ => false,
        });

        if !has_version {
            members.push(EntityMember::Field(Field {
                name: "version".to_string(),
                ty: FieldType::I64, // Assume version is i64
                required: true,
                unique: false,
                line_number: 0,
                xml_path: "".to_string(),
                metadata: std::collections::BTreeMap::new(),
            }));
        }

        let mut seed_values = Vec::new();
        for child in node.children() {
            if child.tag_name().name() == "_value" {
                let mut properties = std::collections::BTreeMap::new();
                for attr in child.attributes() {
                    let name = attr.name().to_string();
                    let value = attr.value().to_string();
                    properties.insert(name.clone(), value.clone());

                    if !name.starts_with('_') {
                        if let Some(EntityMember::Field(f)) = members.iter_mut().find(|m| match m {
                            EntityMember::Field(field) => field.name == name,
                            _ => false,
                        }) {
                            let existing = f
                                .metadata
                                .entry("candidates".to_string())
                                .or_insert_with(String::new);
                            if !existing.is_empty() {
                                existing.push(',');
                            }
                            existing.push_str(&value);
                        }
                    }
                }
                seed_values.push(crate::ir::SeedValue { properties });
            }
        }

        entities.push(Entity {
            name: entity_name,
            table: None, // Can be derived in normalize/naming step
            members,
            is_human,
            data_service: data_service.or(global_data_service.clone()),
            audit_mask_fields: audit_mask_fields.or(global_audit_mask_fields.clone()),
            audit_value_max_len: audit_value_max_len.or(global_audit_value_max_len),
            metadata,
            seed_values,
            line_number,
            xml_path: xml_path.to_string(),
        });
    }

    Ok(Domain {
        name: domain_name,
        entities,
    })
}
