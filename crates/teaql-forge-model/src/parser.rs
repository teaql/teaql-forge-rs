use crate::error::ParseError;
use crate::ir::{Cardinality, Domain, Entity, Field, FieldType, Relation};

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

        let mut fields = Vec::new();
        let mut relations = Vec::new();
        let mut has_id = false;

        for attr in node.attributes() {
            let attr_name = attr.name();
            // Skip metadata attributes starting with '_'
            if attr_name.starts_with('_') {
                continue;
            }

            let attr_value = attr.value();

            if attr_name == "id" && (attr_value == "id()" || attr_value == "id") {
                fields.push(Field {
                    name: attr_name.to_string(),
                    ty: FieldType::Id,
                    required: true,
                    unique: true,
                });
                has_id = true;
                continue;
            }

            // Simple heuristic to distinguish fields vs relations:
            // if value is like `xxx()` and `xxx` is another entity, it's a relation.
            // For MVP, if it contains `()` we check the prefix.
            let (is_relation, rel_target) = if attr_value.ends_with("()") {
                let prefix = attr_value.trim_end_matches("()");
                match prefix {
                    "string" | "number" | "bool" | "date" | "datetime" | "money" | "text" | "createTime" => (false, ""),
                    _ => (true, prefix),
                }
            } else {
                (false, "")
            };

            if is_relation {
                relations.push(Relation {
                    name: attr_name.to_string(),
                    target: rel_target.to_string(),
                    cardinality: Cardinality::ManyToOne,
                });
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

                fields.push(Field {
                    name: attr_name.to_string(),
                    ty,
                    required: false, // simplified for now
                    unique: false,
                });
            }
        }

        if !has_id {
            fields.insert(0, Field {
                name: "id".to_string(),
                ty: FieldType::Id,
                required: true,
                unique: true,
            });
        }

        entities.push(Entity {
            name: entity_name,
            table: None, // Can be derived in normalize/naming step
            fields,
            relations,
        });
    }

    Ok(Domain {
        name: domain_name,
        entities,
    })
}
