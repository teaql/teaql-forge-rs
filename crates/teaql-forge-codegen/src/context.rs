use heck::{ToPascalCase, ToSnakeCase};
use serde::Serialize;
use teaql_forge_model::ir::{Domain, Entity, FieldType};

#[derive(Debug, Serialize)]
pub struct RenderDomain {
    pub crate_name: String,
    pub module_name: String,
    pub entities: Vec<RenderEntity>,
}

#[derive(Debug, Serialize)]
pub struct RenderEntity {
    pub name: String,
    pub rust_struct: String,
    pub rust_module: String,
    pub table_name: String,
    pub fields: Vec<RenderField>,
    pub relations: Vec<RenderRelation>,
}

#[derive(Debug, Serialize)]
pub struct RenderField {
    pub name: String,
    pub rust_name: String,
    pub column_name: String,
    pub rust_type: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct RenderRelation {
    pub name: String,
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

    let mut reverse_relations: std::collections::HashMap<String, Vec<RenderRelation>> = std::collections::HashMap::new();
    for e in &domain.entities {
        for r in &e.relations {
            let reverse_name = format!("{}_list", e.name.to_snake_case());
            let reverse_rust_name = inflector::string::pluralize::to_plural(&e.name.to_snake_case());
            reverse_relations.entry(r.target.to_pascal_case()).or_default().push(RenderRelation {
                name: reverse_name,
                rust_name: reverse_rust_name.clone(),
                target_method: inflector::string::pluralize::to_plural(&e.name.to_snake_case()),
                target_struct: e.name.to_pascal_case(),
                target_module: e.name.to_snake_case(),
                target: e.name.clone(),
                local_key: format!("{}_id", r.name.to_snake_case()),
                many: true,
                delete_missing: e.name == "TaskExecutionLog",
            });
        }
    }

    let entities = domain.entities.iter().map(|e| {
        let rust_struct = e.name.to_pascal_case();
        let rust_module = e.name.to_snake_case();
        let table_name = e.table.clone().unwrap_or_else(|| e.name.to_snake_case());

        let fields = e.fields.iter().map(|f| {
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
                rust_name,
                column_name,
                rust_type,
                required: f.required,
            }
        }).collect();

        let mut relations: Vec<RenderRelation> = e.relations.iter().map(|r| {
            RenderRelation {
                name: r.name.clone(),
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
        
        if let Some(rev_rels) = reverse_relations.remove(&rust_struct) {
            relations.extend(rev_rels);
        }

        RenderEntity {
            name: e.name.clone(),
            rust_struct,
            rust_module,
            table_name,
            fields,
            relations,
        }
    }).collect();

    RenderDomain {
        crate_name,
        module_name,
        entities,
    }
}
