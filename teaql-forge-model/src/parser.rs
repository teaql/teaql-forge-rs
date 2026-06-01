use crate::error::ParseError;
use crate::ir::{Cardinality, Domain, Entity, EntityMember, Field, FieldType, Relation};

pub fn parse_model(src: &str) -> Result<Domain, ParseError> {
    let doc = roxmltree::Document::parse(src)?;

    let root_node = doc.root_element();
    if root_node.tag_name().name() != "root" {
        return Err(ParseError::MissingDomain);
    }

    let domain_name = root_node.attribute("name").unwrap_or("AppDomain").to_string();

    let mut entities = Vec::new();

    // Iterate over children of root. Each child is an Entity.
    for node in root_node.children().filter(|n| n.is_element()) {
        let entity_name = node.tag_name().name().to_string();

        let mut members = Vec::new();
        let mut has_id = false;
        let mut is_human = false;

        for attr in node.attributes() {
            let attr_name = attr.name();
            let attr_value = attr.value();

            if attr_name == "_category" || attr_name == "category" {
                if attr_value.eq_ignore_ascii_case("human") {
                    is_human = true;
                }
            }

            // Skip metadata attributes starting with '_'
            if attr_name.starts_with('_') {
                continue;
            }

            let attr_value = attr.value();

            if attr_name == "id" && (attr_value == "id()" || attr_value == "id") {
                members.push(EntityMember::Field(Field {
                    name: attr_name.to_string(),
                    ty: FieldType::Id,
                    required: true,
                    unique: true,
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
                    if inside.contains("relationType=oneToOne") || inside.contains("relationType=OneToOne") {
                        rel_type = Cardinality::OneToOne;
                    }
                    match prefix {
                        "string" | "number" | "bool" | "date" | "datetime" | "money" | "text" | "createTime" | "updateTime" | "id" => (false, "", Cardinality::ManyToOne),
                        _ => (true, prefix, rel_type),
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

                members.push(EntityMember::Field(Field {
                    name: attr_name.to_string(),
                    ty,
                    required: false, // simplified for now
                    unique: false,
                }));
            }
        }

        if !has_id {
            members.insert(0, EntityMember::Field(Field {
                name: "id".to_string(),
                ty: FieldType::Id,
                required: true,
                unique: true,
            }));
        }

        entities.push(Entity {
            name: entity_name,
            table: None, // Can be derived in normalize/naming step
            members,
            is_human,
        });
    }

    Ok(Domain {
        name: domain_name,
        entities,
    })
}
