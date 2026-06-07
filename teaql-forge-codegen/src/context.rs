use heck::{ToPascalCase, ToSnakeCase};
use serde::Serialize;
use teaql_forge_model::ir::{Domain, FieldType};

#[derive(Debug, Serialize)]
pub struct RenderDomain {
    pub crate_name: String,
    pub module_name: String,
    pub workspace_crate_name: String,
    pub has_sql_provider: bool,
    pub entities: Vec<RenderEntity>,
}

#[derive(Debug, Serialize)]
pub struct RenderEntity {
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    pub rust_struct: String,
    pub rust_module: String,
    pub table_name: String,
    pub fields: Vec<RenderField>,
    pub relations: Vec<RenderRelation>,
    pub reverse_relations: Vec<RenderReverseRelation>,
    pub is_human: bool,
    pub data_service: Option<String>,
    pub audit_mask_fields: Vec<String>,
    pub audit_value_max_len: Option<usize>,
    pub attribute_predicate_prefix: String,
    pub attribute_predicate_suffix: String,
    pub bool_predicate_prefix: String,
}

#[derive(Debug, Serialize)]
pub struct RenderField {
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    pub rust_name: String,
    pub column_name: String,
    pub rust_type: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct RenderRelation {
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    pub rust_name: String,
    pub target_method: String,
    pub target_struct: String,
    pub target_module: String,
    pub target: String,
    pub local_key: String,
    pub many: bool,
    pub delete_missing: bool,
}

#[derive(Debug, Serialize)]
pub struct RenderReverseRelation {
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    pub rust_name: String,
    pub target_method: String,
    pub target_struct: String,
    pub target_module: String,
    pub target: String,
    pub local_key: String,
    pub many: bool,
    pub delete_missing: bool,
}

pub fn build_render_context(domain: &Domain) -> RenderDomain {
    let crate_name = domain.name.to_snake_case();
    let module_name = crate_name.clone();

    let mut reverse_relations_map: std::collections::HashMap<String, Vec<RenderReverseRelation>> = std::collections::HashMap::new();
    for e in &domain.entities {
        let relations_only = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Relation(r) => Some(r),
            _ => None,
        }).collect::<Vec<_>>();

        for r in &relations_only {
            let target_struct = r.target.to_pascal_case();
            let collision_count = relations_only.iter().filter(|r2| r2.target == r.target).count();
            let has_many = collision_count > 1;

            let many = match r.cardinality {
                teaql_forge_model::ir::Cardinality::OneToOne => false,
                _ => true,
            };

            let reverse_name = if !many {
                if has_many {
                    format!("{}_as_{}", e.name.to_snake_case(), r.name.to_snake_case())
                } else {
                    e.name.to_snake_case()
                }
            } else if has_many {
                format!("{}_list_as_{}", e.name.to_snake_case(), r.name.to_snake_case())
            } else {
                format!("{}_list", e.name.to_snake_case())
            };

            let reverse_rust_name = if !many {
                if has_many {
                    format!("{}_as_{}", e.name.to_snake_case(), r.name.to_snake_case())
                } else {
                    e.name.to_snake_case()
                }
            } else if has_many {
                format!("{}_as_{}", inflector::string::pluralize::to_plural(&e.name.to_snake_case()), r.name.to_snake_case())
            } else {
                inflector::string::pluralize::to_plural(&e.name.to_snake_case())
            };

            let target_method = reverse_rust_name.clone();

            reverse_relations_map.entry(target_struct.clone()).or_default().push(RenderReverseRelation {
                name: reverse_name,
                line_number: r.line_number,
                xml_path: r.xml_path.clone(),
                rust_name: reverse_rust_name,
                target_method,
                target_struct: e.name.to_pascal_case(),
                target_module: e.name.to_snake_case(),
                target: e.name.clone(),
                local_key: format!("{}_id", r.name.to_snake_case()),
                many,
                delete_missing: false,
            });
        }
    }

    let entities = domain.entities.iter().map(|e| {
        let rust_struct = e.name.to_pascal_case();
        let rust_module = e.name.to_snake_case();
        let table_name = e.table.clone().unwrap_or_else(|| e.name.to_snake_case());

        let fields = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Field(f) => Some(f),
            _ => None,
        }).map(|f| {
            let rust_name = f.name.to_snake_case();
            let column_name = f.name.to_snake_case();
            let rust_type = match f.ty {
                FieldType::Id => "u64".to_string(),
                FieldType::String => "String".to_string(),
                FieldType::Text => "String".to_string(),
                FieldType::Bool => "bool".to_string(),
                FieldType::I32 => "i32".to_string(),
                FieldType::I64 => "i64".to_string(),
                FieldType::U64 => "u64".to_string(),
                FieldType::Decimal => "f64".to_string(), // MVP simplify
                FieldType::Date => "chrono::NaiveDate".to_string(),
                FieldType::DateTime => "chrono::DateTime<chrono::Utc>".to_string(),
            };
            
            let rust_type = if f.required {
                rust_type
            } else {
                format!("Option<{}>", rust_type)
            };

            RenderField {
                name: f.name.clone(),
                line_number: f.line_number,
                xml_path: f.xml_path.clone(),
                rust_name,
                column_name,
                rust_type,
                required: f.required,
            }
        }).collect();

        let relations: Vec<RenderRelation> = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Relation(r) => Some(r),
            _ => None,
        }).map(|r| {
            RenderRelation {
                name: r.name.clone(),
                line_number: r.line_number,
                xml_path: r.xml_path.clone(),
                rust_name: r.name.to_snake_case(),
                target_method: inflector::string::pluralize::to_plural(&r.target.to_snake_case()),
                target_struct: r.target.to_pascal_case(),
                target_module: r.target.to_snake_case(),
                target: r.target.clone(),
                local_key: format!("{}_id", r.name.to_snake_case()),
                many: false,
                delete_missing: false,
            }
        }).collect();

        let reverse_relations = reverse_relations_map.remove(&rust_struct).unwrap_or_default();

        let data_service = e.data_service.clone();
        let audit_mask_fields = e.audit_mask_fields.as_ref()
            .map(|s| s.split(',').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect())
            .unwrap_or_default();
        let audit_value_max_len = e.audit_value_max_len;

        RenderEntity {
            name: e.name.clone(),
            line_number: e.line_number,
            xml_path: e.xml_path.clone(),
            rust_struct,
            rust_module,
            table_name,
            fields,
            relations,
            reverse_relations,
            is_human: e.is_human,
            data_service,
            audit_mask_fields,
            audit_value_max_len,
            attribute_predicate_prefix: if e.is_human { "whose".to_string() } else { "with".to_string() },
            attribute_predicate_suffix: if e.is_human { "s".to_string() } else { "ing".to_string() },
            bool_predicate_prefix: if e.is_human { "who_are".to_string() } else { "which_are".to_string() },
        }
    }).collect();

    let workspace_crate_name = format!("{}-workspace", crate_name);
    let has_sql_provider = true; // Hardcoded for now, assuming SQL is supported

    RenderDomain {
        crate_name,
        module_name,
        workspace_crate_name,
        has_sql_provider,
        entities,
    }
}
