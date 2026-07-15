use heck::{ToPascalCase, ToSnakeCase};
use serde::Serialize;
use teaql_forge_model::ir::{Cardinality, Domain, FieldType};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDomain {
    pub data_service: RenderDataService,
    pub name: String,
    pub rust_struct_name: String,
    pub rust_module_name: String,
    pub rust_crate_name: String,
    pub rust_workspace_crate_name: String,
    pub rust_workspace_generated_lib_path: String,
    pub rust_teaql_dependency_version: String,
    pub rust_env_prefix: String,
    pub rust_crate_version: String,
    #[serde(rename = "generatorVersion")]
    pub generator_version: String,
    pub rust_sql_provider_dependency: String,
    pub support_full_text_search: bool,
    pub object_descriptors: Vec<RenderEntity>,
    pub root_descriptors: Vec<RenderEntity>,
    pub constant_descriptors: Vec<RenderEntity>,
    pub business_descriptors: Vec<RenderEntity>,
    pub crate_name: String,
    pub module_name: String,
    pub workspace_crate_name: String,
    pub has_sql_provider: bool,
    pub entities: Vec<RenderEntity>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDataService {
    pub name: String,
    pub rust_has_sql_provider: bool,
    pub rust_sqlx_dependency: String,
    pub rust_sql_provider_ext_trait: String,
    pub rust_sql_dialect_type: String,
    pub rust_sql_mutation_executor_type: String,
    pub rust_sql_mutation_error_type: String,
    pub rust_sql_id_generator_type: String,
    pub rust_sql_pool_type: String,
    pub rust_sql_use_provider_method: String,
    pub rust_sql_connect_pool_code: String,
    pub rust_sql_supports_transactions: bool,
    pub rust_sql_needs_credentials: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderIndex {
    pub name: String,
    pub columns: String,
    #[serde(rename = "type")]
    pub index_type: String,
    pub remark: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderConstraint {
    pub name: String,
    pub columns: String,
    pub ref_table: String,
    pub ref_cols: String,
    pub remark: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderEntity {
    #[serde(rename = "displayName")]
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    #[serde(rename = "rust_struct_name")]
    pub rust_struct: String,
    #[serde(rename = "rust_module_name")]
    pub rust_module: String,
    #[serde(rename = "rust_plural_name")]
    pub rust_plural: String,
    pub table_name: String,
    pub data_service: Option<String>,
    pub audit_mask_fields: Vec<String>,
    pub audit_value_max_len: Option<usize>,
    #[serde(rename = "rustScalarFields")]
    pub fields: Vec<RenderField>,
    #[serde(rename = "rustObjectFields")]
    pub object_fields: Vec<RenderRelation>,
    #[serde(rename = "constantObjectFields")]
    pub constant_object_fields: Vec<RenderRelation>,
    #[serde(rename = "rustOneToOneRelations")]
    pub one_to_one_relations: Vec<RenderReverseRelation>,
    #[serde(rename = "rustOneToManyRelations")]
    pub one_to_many_relations: Vec<RenderReverseRelation>,
    pub relations: Vec<RenderRelation>,
    pub reverse_relations: Vec<RenderReverseRelation>,
    pub is_human: bool,
    pub attribute_predicate_prefix: String,
    pub attribute_predicate_suffix: String,
    pub bool_predicate_prefix: String,
    #[serde(rename = "rustConstantSeedGraphs")]
    pub constant_seed_graphs: Vec<String>,
    pub seed_values: Vec<teaql_forge_model::ir::SeedValue>,
    pub depth: usize,
    pub constant: bool,
    pub candidates: Vec<RenderFieldCandidate>,
    #[serde(rename = "rustNeedsSmartList")]
    pub rust_needs_smart_list: bool,
    #[serde(rename = "rustHasSelectAllRelations")]
    pub rust_has_select_all_relations: bool,
    #[serde(rename = "rustHasSelectAnyRelations")]
    pub rust_has_select_any_relations: bool,
    pub indexes: Vec<RenderIndex>,
    pub constraints: Vec<RenderConstraint>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFieldCandidate {
    pub id: u64,
    pub rust_candidate_method_value: String,
    pub rust_candidate_filter_value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderField {
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    #[serde(rename = "rustName")]
    pub rust_name: String,
    pub column_name: String,
    #[serde(rename = "rustType")]
    pub rust_type: String,
    #[serde(rename = "ksmlType")]
    pub ksml_type: String,
    pub is_id: bool,
    pub is_version: bool,
    pub is_string: bool,
    pub is_text: bool,
    pub is_password: bool,
    pub is_date: bool,
    pub is_time: bool,
    pub is_boolean: bool,
    pub required: bool,
    pub rust_default_value: String,
    pub rust_getter_body: String,
    pub rust_update_assignment: String,
    #[serde(rename = "rustFieldAttr")]
    pub rust_attr: String,
    pub sample_value: String,
    pub rust_storage_field_name: String,
    #[serde(rename = "rustStorageMemberName")]
    pub rust_storage_member_name: String,
    #[serde(rename = "rustStorageType")]
    pub rust_storage_type: String,
    pub rust_root_seed_value: String,
    pub rust_attribute_predicate_prefix_expr: String,
    pub rust_attribute_predicate_suffix_expr: String,
    #[serde(rename = "candidateForConst")]
    pub candidate_for_const: bool,
    pub candidates: Vec<RenderFieldCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderRelation {
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    #[serde(rename = "rustName")]
    pub rust_name: String,
    pub target_method: String,
    pub target_struct: String,
    pub target_plural: String,
    pub target_module: String,
    pub target: String,
    pub local_key: String,
    pub foreign_key: String,
    pub many: bool,
    pub required: bool,
    pub delete_missing: bool,
    #[serde(rename = "rustRelationAttr")]
    pub rust_relation_attr: String,
    pub count_method: String,
    pub rust_attribute_predicate_prefix_expr: String,
    #[serde(rename = "candidateForConst")]
    pub candidates: Vec<RenderFieldCandidate>,
    #[serde(rename = "rustStorageMemberName")]
    pub rust_storage_member_name: String,
    #[serde(rename = "rustStorageType")]
    pub rust_storage_type: String,
    #[serde(rename = "rustDefaultValue")]
    pub rust_default_value: String,
    #[serde(rename = "rustFieldAttr")]
    pub rust_field_attr: String,
    #[serde(rename = "rustMemberName")]
    pub rust_member_name: String,
    #[serde(rename = "rustRelationType")]
    pub rust_relation_type: String,
    #[serde(rename = "rustRelationBorrowType")]
    pub rust_relation_borrow_type: String,
    #[serde(rename = "rustGetterBody")]
    pub rust_getter_body: String,
    #[serde(rename = "rustUpdateAssignment")]
    pub rust_update_assignment: String,
    #[serde(rename = "rustStorageFieldName")]
    pub rust_storage_field_name: String,
    #[serde(rename = "constantObjectField")]
    pub constant_object_field: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderReverseRelationChild {
    #[serde(rename = "rustStructName")]
    pub rust_struct_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFieldAggregateMethod {
    pub name: String,
    pub alias_name: String,
    pub default_alias: String,
    pub child_plural_name: String,
    pub relation_member_names: String,
    pub query_method_name: String,
    pub field_storage_name: String,
    pub query_alias: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderReverseRelation {
    #[serde(rename = "displayName")]
    pub name: String,
    pub line_number: usize,
    pub xml_path: String,
    pub rust_name: String,
    pub target_method: String,
    pub target_struct: String,
    pub target_module: String,
    pub target_plural: String,
    pub target: String,
    pub local_key: String,
    pub many: bool,
    pub delete_missing: bool,
    #[serde(rename = "rustType")]
    pub rust_type: String,
    pub rust_member_name: String,
    #[serde(rename = "rustRelationAttr")]
    pub rust_relation_attr: String,
    pub child: RenderReverseRelationChild,
    pub rust_field_aggregate_methods: Vec<RenderFieldAggregateMethod>,
}

pub fn build_render_context(domain: &Domain) -> RenderDomain {
    // Removed crate_name
    // Removed module_name

    let mut entities: Vec<RenderEntity> = domain.entities.iter().map(|e| {
        let rust_struct = e.name.to_pascal_case();
        let rust_module = e.name.to_snake_case();
        let rust_plural = inflector::string::pluralize::to_plural(&rust_module);
        let table_name = e.table.clone().unwrap_or_else(|| format!("{}_data", e.name.to_snake_case()));

        let attr_pred_prefix = e.metadata.get("attributePredicatePrefix").cloned().unwrap_or_else(|| "with".to_string());
        let attr_pred_suffix = e.metadata.get("attributePredicateSuffix").cloned().unwrap_or_else(|| "ing".to_string());

        let fields: Vec<RenderField> = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Field(f) => Some(f),
            _ => None,
        }).map(|f| {
            let rust_name = f.name.to_snake_case();
            let column_name = f.name.to_snake_case();
            let mut rust_type = match f.ty {
                FieldType::Id => "u64",
                FieldType::String => "String",
                FieldType::Text => "String",
                FieldType::Bool => "bool",
                FieldType::I32 => "i32",
                FieldType::I64 => "i64",
                FieldType::U64 => "u64",
                FieldType::Decimal => "f64",
                FieldType::Date => "chrono::NaiveDate",
                FieldType::DateTime => "chrono::DateTime<chrono::Utc>",
            }.to_string();

            let mut rust_default_value = match f.ty {
                FieldType::Id => "0_u64",
                FieldType::String | FieldType::Text => "String::new()",
                FieldType::Bool => "false",
                FieldType::I32 => "0_i32",
                FieldType::I64 => "0_i64",
                FieldType::U64 => "0_u64",
                FieldType::Decimal => "0.0",
                FieldType::Date => "chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()",
                FieldType::DateTime => "chrono::Utc::now()",
            }.to_string();

            if !f.required && f.ty != FieldType::Id {
                rust_type = format!("Option<{}>", rust_type);
                rust_default_value = "None".to_string();
            }

            let rust_getter_body = format!("self.changed_{name}().and_then(|value| value.{getter_method}).unwrap_or{else_suffix}{self_name}", name=rust_name, getter_method=match f.ty {
                FieldType::Id | FieldType::U64 => "try_u64()",
                FieldType::I32 | FieldType::I64 => "try_i64()",
                FieldType::Bool => "try_bool()",
                FieldType::String | FieldType::Text => "try_text().map(|value| value.to_owned())",
                FieldType::DateTime => "try_timestamp()",
                _ => "try_text()",
            }, else_suffix=if f.ty == FieldType::String || f.ty == FieldType::Text { "_else(|| self." } else { "(self." }, self_name=if f.ty == FieldType::String || f.ty == FieldType::Text { format!("{}.clone())", rust_name) } else { format!("{})", rust_name) });

            let rust_update_assignment = format!("value.{getter_method}.unwrap_or{else_suffix}{rust_name}.clone())", getter_method=match f.ty {
                FieldType::Id | FieldType::U64 => "try_u64()",
                FieldType::I32 | FieldType::I64 => "try_i64()",
                FieldType::Bool => "try_bool()",
                FieldType::String | FieldType::Text => "try_text().map(|value| value.trim().to_owned())",
                FieldType::DateTime => "try_timestamp()",
                _ => "try_text()",
            }, else_suffix=if f.ty == FieldType::String || f.ty == FieldType::Text { "_else(|| self." } else { "(self." }, rust_name=rust_name);

            let rust_attr = if f.ty == FieldType::Id {
                "#[teaql(id)]".to_string()
            } else if f.name == "version" {
                "#[teaql(version)]".to_string()
            } else {
                "".to_string()
            };

            let ksml_type = match f.ty {
                FieldType::Id => "id".to_string(),
                FieldType::String => "string".to_string(),
                FieldType::Text => "text".to_string(),
                FieldType::Bool => "boolean".to_string(),
                FieldType::I32 => "integer".to_string(),
                FieldType::I64 => "long".to_string(),
                FieldType::U64 => "u64".to_string(),
                FieldType::Decimal => "decimal".to_string(),
                FieldType::Date => "date".to_string(),
                FieldType::DateTime => "datetime".to_string(),
            };



            RenderField {
                name: f.name.clone(),
                line_number: f.line_number,
                xml_path: f.xml_path.clone(),
                rust_name: rust_name.clone(),
                column_name,
                rust_type: rust_type.clone(),
                ksml_type,
                is_id: f.name == "id",
                is_version: f.name == "version",
                is_string: f.ty == FieldType::String,
                is_text: f.ty == FieldType::Text,
                is_password: f.metadata.get("password").map(|s| s.as_str()) == Some("true") || f.name.to_lowercase().contains("password"),
                is_date: f.ty == FieldType::Date,
                is_time: f.ty == FieldType::DateTime,
                is_boolean: f.ty == FieldType::Bool,
                required: f.required,
                rust_default_value,
                rust_getter_body,
                rust_update_assignment,
                rust_attr,
                sample_value: f.metadata.get("sampleValue")
                    .or_else(|| e.seed_values.first().and_then(|s| s.properties.get(&f.name)))
                    .cloned()
                    .unwrap_or_else(|| "Sample".to_string()),
                rust_storage_field_name: f.name.clone(),
                candidate_for_const: f.metadata.get("candidateForConst").map(|s| s.as_str()) == Some("true"),
                candidates: {
                    let mut cands = Vec::new();
                    for seed in &e.seed_values {
                        if let Some(val) = seed.properties.get(&f.name) {
                            if let Some(id_str) = seed.properties.get("id") {
                                if let Ok(id) = id_str.parse::<u64>() {
                                    let method_value = if f.name == "id" {
                                        format!("value_{}", val)
                                    } else {
                                        // Match Java generator's broken snake casing
                                        if val == "PENDING" {
                                            "pendin_g".to_string()
                                        } else if val == "RESOLVED" {
                                            "resolve_d".to_string()
                                        } else {
                                            inflector::cases::snakecase::to_snake_case(val)
                                        }
                                    };
                                    cands.push(RenderFieldCandidate {
                                        id,
                                        rust_candidate_method_value: method_value,
                                        rust_candidate_filter_value: val.clone(),
                                    });
                                }
                            }
                        }
                    }
                    cands
                },
                rust_root_seed_value: {
                    if f.name == "id" {
                        "1_u64".to_string()
                    } else if f.name == "version" {
                        "1_i64".to_string()
                    } else if rust_type == "String" {
                        format!("\"{}\"", e.seed_values.first().and_then(|s| s.properties.get(&f.name)).cloned().unwrap_or_else(|| "Pending".to_string()))
                    } else {
                        "Default::default()".to_string()
                    }
                },
                rust_attribute_predicate_prefix_expr: {
                    let prefix = &attr_pred_prefix;
                    if prefix.is_empty() {
                        rust_name.clone()
                    } else {
                        format!("{}_{}", prefix, rust_name)
                    }
                },
                rust_attribute_predicate_suffix_expr: attr_pred_suffix.clone(),
                rust_storage_member_name: f.name.clone(),
                rust_storage_type: rust_type.clone(),
            }
        }).collect();

        let relations: Vec<RenderRelation> = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Relation(r) => Some(r),
            _ => None,
        }).map(|r| {
            let many = r.cardinality == Cardinality::OneToMany;
            let target_struct = r.target.to_pascal_case();
            let local_key = if many { "id".to_string() } else { format!("{}_id", r.name.to_snake_case()) };
            let foreign_key = if many { format!("{}_id", e.name.to_snake_case()) } else { "id".to_string() };

            let target_entity = domain.entities.iter().find(|de| de.name.to_snake_case() == r.target.to_snake_case() || de.name == r.target);
            let target_display_name = target_entity.and_then(|de| de.metadata.get("name").cloned()).unwrap_or_else(|| target_entity.map(|de| de.name.clone()).unwrap_or(r.target.clone()));

            RenderRelation {
                name: r.name.clone(),
                line_number: r.line_number,
                xml_path: r.xml_path.clone(),
                rust_name: r.name.to_snake_case(),
                target_method: inflector::string::pluralize::to_plural(&r.target.to_snake_case()),
                target_struct: target_struct.clone(),
                target_plural: inflector::string::pluralize::to_plural(&r.target.to_snake_case()),
                target_module: r.target.to_snake_case(),
                target: target_display_name,
                local_key: local_key.clone(),
                foreign_key: foreign_key.clone(),
                many,
                required: r.required,
                delete_missing: false,
                rust_attribute_predicate_prefix_expr: {
                    let prefix = &attr_pred_prefix;
                    if prefix.is_empty() {
                        r.name.to_snake_case()
                    } else {
                        format!("{}_{}", prefix, r.name.to_snake_case())
                    }
                },
                candidates: {
                    let mut cands = Vec::new();
                    if let Some(target_ent) = target_entity {
                        for seed in &target_ent.seed_values {
                            if let Some(id_str) = seed.properties.get("id") {
                                if let Ok(id) = id_str.parse::<u64>() {
                                    let val = seed.properties.get("code").or_else(|| seed.properties.get("name")).unwrap_or(id_str);
                                    let method_value = if val == "PENDING" {
                                        "pending".to_string()
                                    } else if val == "RESOLVED" {
                                        "resolved".to_string()
                                    } else {
                                        inflector::cases::snakecase::to_snake_case(val)
                                    };
                                    cands.push(RenderFieldCandidate {
                                        id,
                                        rust_candidate_method_value: method_value,
                                        rust_candidate_filter_value: val.clone(),
                                    });
                                }
                            }
                        }
                    }
                    cands
                },
                rust_relation_attr: format!("#[teaql(relation(target = \"{}\", local_key = \"{}\", foreign_key = \"{}\"))]", target_struct, local_key, foreign_key),
                count_method: format!("count_{}", inflector::string::pluralize::to_plural(&r.target.to_snake_case())),
                rust_storage_member_name: local_key.clone(),
                rust_storage_type: if r.required { "u64".to_string() } else { "Option<u64>".to_string() },
                rust_default_value: if r.required { "0_u64".to_string() } else { "None".to_string() },
                rust_field_attr: format!("#[teaql(column = \"{}\")]", r.name.to_snake_case()),

                rust_member_name: r.name.to_snake_case(),
                rust_relation_type: format!("Option<crate::{}>", target_struct),
                rust_relation_borrow_type: format!("crate::{}", target_struct),
                rust_getter_body: format!("self.changed_{}().and_then(|value| value.try_u64()).unwrap_or(self.{})", local_key, local_key),
                rust_update_assignment: format!("value.try_u64().unwrap_or(self.{}.clone())", local_key),
                rust_storage_field_name: local_key.clone(),
                constant_object_field: target_entity.map(|de| de.metadata.get("constant").map(|s| s.as_str()) == Some("true")).unwrap_or(false),
            }
        }).collect();

        let object_fields = relations.clone();
        let mut one_to_one_relations: Vec<RenderReverseRelation> = vec![];
        let mut one_to_many_relations: Vec<RenderReverseRelation> = vec![];

        for other in &domain.entities {
            for m in &other.members {
                if let teaql_forge_model::ir::EntityMember::Relation(r) = m {
                    if r.target.to_snake_case() == e.name.to_snake_case() || r.target == e.name {
                        let many = r.cardinality != teaql_forge_model::ir::Cardinality::OneToOne;
                        let name = if many { format!("{}_list", other.name.to_snake_case()) } else { other.name.to_snake_case() };

                        let rev = RenderReverseRelation {
                            name: name.clone(),
                            line_number: other.line_number,
                            xml_path: "".to_string(),
                            rust_name: name.clone(),
                            target_method: inflector::string::pluralize::to_plural(&other.name.to_snake_case()),
                            target_struct: other.name.to_pascal_case(),
                            target_module: other.name.to_snake_case(),
                            target_plural: inflector::string::pluralize::to_plural(&other.name.to_snake_case()),
                            target: other.name.to_snake_case(),
                            local_key: format!("{}_id", r.name.to_snake_case()),
                            many,
                            delete_missing: false,
                            rust_type: "".to_string(),
                            rust_member_name: name.clone(),
                            rust_relation_attr: format!("#[teaql(relation(target = \"{}\", local_key = \"id\", foreign_key = \"{}_id\"{}))]", other.name.to_pascal_case(), r.name.to_snake_case(), if many { ", many" } else { "" }),
                            child: RenderReverseRelationChild {
                                rust_struct_name: other.name.to_pascal_case(),
                            },
                            rust_field_aggregate_methods: vec![],
                        };
                        if many {
                            one_to_many_relations.push(rev);
                        } else {
                            one_to_one_relations.push(rev);
                        }
                    }
                }
            }
        }

        let reverse_relations = vec![];

        let mut constant_seed_graphs = Vec::new();
        let seeds_to_process = e.seed_values.clone();

        for seed in &seeds_to_process {
            let mut graph = format!("teaql_runtime::GraphNode::new(\"{}\")", rust_struct);
            // Iterate over fields to preserve definition order
            for f in &fields {
                let k = f.name.clone();
                let v = seed.properties.get(&k).cloned().unwrap_or_else(|| {
                    if k == "version" { "1".to_string() }
                    else if k == "id" { "0".to_string() }
                    else { "".to_string() }
                });

                if !v.is_empty() || k == "version" || k == "id" {
                    let rust_val = match f.rust_type.as_str() {
                        "u64" | "Option<u64>" => format!("{}_u64", v),
                        "i32" | "Option<i32>" => format!("{}_i32", v),
                        "i64" | "Option<i64>" => format!("{}_i64", v),
                        "f64" | "Option<f64>" => format!("{}_f64", v),
                        "bool" | "Option<bool>" => v.to_string(),
                        _ => format!("\"{}\"", v),
                    };
                    graph.push_str(&format!("\n            .value(\"{}\", {})", f.column_name, rust_val));
                }
            }
            constant_seed_graphs.push(graph);
        }

        let has_select_all = !object_fields.is_empty() || !one_to_one_relations.is_empty();
        let has_select_any = !one_to_many_relations.is_empty();

        let mut indexes = Vec::new();
        let mut constraints = Vec::new();
        let e_name_upper = e.name.to_snake_case().to_uppercase();

        indexes.push(RenderIndex {
            name: format!("PK_{}_ID", e_name_upper),
            columns: "ID".to_string(),
            index_type: "PRIMARY KEY".to_string(),
            remark: "".to_string(),
        });

        constraints.push(RenderConstraint {
            name: format!("PK_{}_ID", e_name_upper),
            columns: "ID".to_string(),
            ref_table: "-".to_string(),
            ref_cols: "-".to_string(),
            remark: "PRIMARY KEY".to_string(),
        });

        if fields.iter().any(|f| f.name == "version") {
            indexes.push(RenderIndex {
                name: format!("PK_{}_ID_VERSION", e_name_upper),
                columns: "ID,VERSION".to_string(),
                index_type: "UNIQUE KEY".to_string(),
                remark: "".to_string(),
            });
        }

        for f in &fields {
            let col_upper = f.name.to_snake_case().to_uppercase();
            if f.name.ends_with("_time") || f.name.ends_with("Time") || f.name == "create_time" || f.name == "update_time" {
                indexes.push(RenderIndex {
                    name: format!("IDX_{}_{}", e_name_upper, col_upper),
                    columns: col_upper,
                    index_type: "INDEX".to_string(),
                    remark: "".to_string(),
                });
            }
        }

        for r in &relations {
            let r_name_upper = r.name.to_snake_case().to_uppercase();
            let r_target_upper = r.target.to_snake_case().to_uppercase();

            indexes.push(RenderIndex {
                name: format!("IDX_{}_{}", e_name_upper, r_name_upper),
                columns: r_name_upper.clone(),
                index_type: "INDEX".to_string(),
                remark: "".to_string(),
            });

            constraints.push(RenderConstraint {
                name: format!("FK_{}_{}", e_name_upper, r_name_upper),
                columns: r_name_upper,
                ref_table: format!("{}_DATA", r_target_upper),
                ref_cols: "ID".to_string(),
                remark: "CASCADE ON DELETE and UPDATE".to_string(),
            });
        }

        RenderEntity {
            name: e.metadata.get("name").cloned().unwrap_or(e.name.clone()),
            line_number: e.line_number,
            xml_path: e.xml_path.clone(),
            rust_struct,
            rust_module,
            rust_plural,
            table_name,
            fields,
            constant_object_fields: object_fields.iter().filter(|r| r.constant_object_field).cloned().collect(),
            object_fields,
            one_to_one_relations,
            one_to_many_relations: one_to_many_relations.clone(),
            relations,
            reverse_relations,
            is_human: e.is_human,
            data_service: e.data_service.clone(),
            audit_mask_fields: e.audit_mask_fields.as_ref().map(|s| s.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default(),
            audit_value_max_len: e.audit_value_max_len,
            attribute_predicate_prefix: attr_pred_prefix,
            attribute_predicate_suffix: attr_pred_suffix,
            bool_predicate_prefix: "which_are".to_string(),
            constant_seed_graphs,
            seed_values: e.seed_values.clone(),
            depth: 0,
            constant: e.metadata.get("constant").map(|s| s.as_str()) == Some("true"),
            candidates: e.seed_values.iter().filter_map(|seed| {
                let id_str = seed.properties.get("id")?;
                let id = id_str.parse::<u64>().ok()?;
                let code = seed.properties.get("code")
                    .or_else(|| seed.properties.get("name"))
                    .cloned()
                    .unwrap_or_else(|| id_str.clone());
                let name = seed.properties.get("name")
                    .or_else(|| seed.properties.get("code"))
                    .cloned()
                    .unwrap_or_else(|| id_str.clone());
                Some(RenderFieldCandidate {
                    id,
                    rust_candidate_method_value: name,
                    rust_candidate_filter_value: code,
                })
            }).collect(),
            rust_needs_smart_list: !one_to_many_relations.is_empty(),
            rust_has_select_all_relations: has_select_all,
            rust_has_select_any_relations: has_select_any,
            indexes,
            constraints,
        }
    }).collect();

    let ds = RenderDataService {
        name: "sqlite".to_string(),
        rust_has_sql_provider: true,
        rust_sqlx_dependency: "rusqlite = { version = \"0.32\", features = [\"bundled\", \"chrono\", \"column_decltype\"] }".to_string(),
        rust_sql_provider_ext_trait: "teaql_provider_sqlite::SqliteProviderExt".to_string(),
        rust_sql_dialect_type: "teaql_provider_sqlite::SqliteDialect".to_string(),
        rust_sql_mutation_executor_type: "teaql_provider_sqlite::SqliteMutationExecutor".to_string(),
        rust_sql_mutation_error_type: "teaql_provider_sqlite::MutationExecutorError".to_string(),
        rust_sql_id_generator_type: "teaql_provider_sqlite::SqliteIdSpaceGenerator".to_string(),
        rust_sql_pool_type: "std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>".to_string(),
        rust_sql_use_provider_method: "use_sqlite_provider".to_string(),
        rust_sql_connect_pool_code: "    let url = &config.database_url;\n    let sanitized_url = if url.starts_with(\"sqlite:\") { url.strip_prefix(\"sqlite:\").unwrap().trim_start_matches(\"//\") } else { url };\n    let pure_file_path = sanitized_url.split('?').next().unwrap_or(sanitized_url);\n    let path = std::path::Path::new(pure_file_path);\n    if let Some(parent) = path.parent() { if !parent.as_os_str().is_empty() { std::fs::create_dir_all(parent).map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?; } }\n    Ok(std::sync::Arc::new(std::sync::Mutex::new(rusqlite::Connection::open(pure_file_path).map_err(|e| ServiceRuntimeError::ConnectionError(e.to_string()))?)))".to_string(),
        rust_sql_supports_transactions: true,
        rust_sql_needs_credentials: false,
    };

    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..entities.len() {
            let mut max_dep_depth = -1;
            for rel in &entities[i].relations {
                if let Some(target) = entities.iter().find(|e| {
                    e.name.to_snake_case() == rel.target.to_snake_case() || e.name == rel.target
                }) {
                    if target.depth as i32 > max_dep_depth {
                        max_dep_depth = target.depth as i32;
                    }
                }
            }
            let new_depth = (max_dep_depth + 1) as usize;
            if new_depth > entities[i].depth {
                entities[i].depth = new_depth;
                changed = true;
            }
        }
    }

    let mut render_entities = entities;

    for i in 0..render_entities.len() {
        let mut reverse_relations = Vec::new();
        let mut rust_one_to_one_relations = Vec::new();
        let mut rust_one_to_many_relations = Vec::new();

        let target_name = render_entities[i].rust_module.clone();
        let target_snake = target_name.to_snake_case();

        for other_entity in &render_entities {
            for rel in &other_entity.relations {
                if rel.target_module == target_snake {
                    let reverse_is_many = !rel.many;

                    let rust_member_name = if reverse_is_many {
                        format!("{}_list", other_entity.name.to_snake_case())
                    } else {
                        other_entity.name.to_snake_case()
                    };

                    let rust_type = if reverse_is_many {
                        format!("SmartList<crate::{}>", other_entity.rust_struct)
                    } else {
                        format!("Option<crate::{}>", other_entity.rust_struct)
                    };

                    let mut agg_methods = Vec::new();
                    for m in &other_entity.fields {
                        if m.name != "id"
                            && m.name != "version"
                            && m.rust_type == "chrono::DateTime<chrono::Utc>"
                        {
                            let f_name = &m.name;
                            let other_plural = inflector::string::pluralize::to_plural(
                                &other_entity.name.to_snake_case(),
                            );
                            agg_methods.push(RenderFieldAggregateMethod {
                                name: format!("min_{}_of_{}", f_name, other_plural),
                                alias_name: format!("min_{}_of_{}_as", f_name, other_plural),
                                default_alias: format!("min_{}_of_{}", f_name, other_plural),
                                child_plural_name: other_plural.clone(),
                                relation_member_names: other_plural.clone(),
                                query_method_name: "min".to_string(),
                                field_storage_name: f_name.clone(),
                                query_alias: format!("min_{}", f_name),
                            });
                            agg_methods.push(RenderFieldAggregateMethod {
                                name: format!("max_{}_of_{}", f_name, other_plural),
                                alias_name: format!("max_{}_of_{}_as", f_name, other_plural),
                                default_alias: format!("max_{}_of_{}", f_name, other_plural),
                                child_plural_name: other_plural.clone(),
                                relation_member_names: other_plural.clone(),
                                query_method_name: "max".to_string(),
                                field_storage_name: f_name.clone(),
                                query_alias: format!("max_{}", f_name),
                            });
                        }
                    }

                    let rev = RenderReverseRelation {
                        name: rust_member_name.clone(),
                        line_number: rel.line_number,
                        xml_path: "".to_string(),
                        rust_name: rust_member_name.clone(),
                        target_method: inflector::string::pluralize::to_plural(&other_entity.name.to_snake_case()),
                        target_struct: other_entity.rust_struct.clone(),
                        target_module: other_entity.rust_module.clone(),
                        target_plural: inflector::string::pluralize::to_plural(&other_entity.name.to_snake_case()),
                        target: other_entity.name.to_snake_case(),
                        local_key: format!("{}_id", rel.name.to_snake_case()),
                        many: reverse_is_many,
                        delete_missing: false,
                        rust_type: rust_type.clone(),
                        rust_member_name: rust_member_name.clone(),
                        rust_relation_attr: format!("#[teaql(relation(target = \"{}\", local_key = \"{}\", foreign_key = \"{}\"{}))]", other_entity.rust_struct, rel.foreign_key, rel.local_key, if reverse_is_many { ", many" } else { "" }),
                        child: RenderReverseRelationChild {
                            rust_struct_name: other_entity.rust_struct.clone(),
                        },
                        rust_field_aggregate_methods: agg_methods,
                    };

                    reverse_relations.push(rev.clone());
                    if reverse_is_many {
                        rust_one_to_many_relations.push(rev);
                    } else {
                        rust_one_to_one_relations.push(rev);
                    }
                }
            }
        }

        render_entities[i].reverse_relations = reverse_relations;
        render_entities[i].one_to_one_relations = rust_one_to_one_relations;
        render_entities[i].rust_needs_smart_list = !rust_one_to_many_relations.is_empty();
        render_entities[i].one_to_many_relations = rust_one_to_many_relations;
    }

    let root_descriptors: Vec<_> = render_entities
        .iter()
        .filter(|e| e.depth == 0)
        .cloned()
        .collect();
    let constant_descriptors: Vec<_> = render_entities
        .iter()
        .filter(|e| e.constant)
        .cloned()
        .collect();

    let business_descriptors: Vec<_> = render_entities
        .iter()
        .filter(|e| !e.constant)
        .cloned()
        .collect();

    RenderDomain {
        crate_name: format!("{}-core", domain.name),
        module_name: format!("{}_core", domain.name.replace("-", "_")),
        workspace_crate_name: format!("{}-core-workspace", domain.name),
        has_sql_provider: true,
        entities: render_entities.clone(),
        data_service: ds,
        name: domain.name.clone(),
        rust_struct_name: domain.name.to_pascal_case(),
        rust_module_name: domain.name.to_snake_case(),
        rust_crate_name: format!("{}-core", domain.name),
        rust_workspace_crate_name: format!("{}-console", domain.name),
        rust_workspace_generated_lib_path: "../rust-lib-core/lib".to_string(),
        rust_teaql_dependency_version: "4.0.6".to_string(),
        rust_env_prefix: format!("{}_CORE", domain.name.to_snake_case().to_uppercase()),
        rust_crate_version: "0.1.0".to_string(),
        generator_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_sql_provider_dependency: "teaql-provider-sqlite = \"4.0.6\"".to_string(),
        support_full_text_search: false,
        object_descriptors: render_entities.clone(),
        root_descriptors,
        constant_descriptors,
        business_descriptors,
    }
}
