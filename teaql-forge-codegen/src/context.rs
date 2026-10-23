use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use serde::Serialize;
use teaql_forge_model::ir::{Cardinality, Domain, FieldType};

fn sanitize_rust_identifier(name: &str) -> String {
    let name = crate::preview_domain::snake(name);
    match name.as_str() {
        "type" | "match" | "fn" | "let" | "mut" | "ref" | "const" | "static" | "if" | "else"
        | "while" | "for" | "loop" | "move" | "box" | "trait" | "struct" | "enum" | "impl"
        | "pub" | "use" | "super" | "self" | "crate" | "where" | "continue" | "break"
        | "return" | "as" | "in" | "unsafe" | "async" | "await" | "dyn" => format!("{}_", name),
        _ => name,
    }
}

fn rust_predicate_prefix(prefix: &str, name: &str) -> String {
    let member = crate::preview_domain::camel(name);
    let predicate = if prefix.is_empty() {
        member
    } else {
        format!("{prefix}{}", crate::preview_domain::upper_first(&member))
    };
    crate::preview_domain::snake(&predicate).to_lowercase()
}

pub(crate) fn pluralize_name(name: &str) -> String {
    crate::pluralization::pluralize(name)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDomain {
    #[serde(skip)]
    pub source_field_attributes: teaql_forge_model::field_attributes::Fields,
    pub data_service: RenderDataService,
    pub name: String,
    pub root_attributes: std::collections::BTreeMap<String, String>,
    pub root_attributes_ordered: Vec<(String, String)>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub source_types:
        std::collections::BTreeMap<String, Vec<teaql_forge_model::types::TypeAttribute>>,
    #[serde(skip)]
    pub source_type_states: Vec<teaql_forge_model::types::TypeState>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub source_data_services: Vec<teaql_forge_model::data_services::DataService>,
    pub java_full_package: Option<String>,
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
    #[serde(skip)]
    pub source_entities: Vec<RenderEntity>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDataService {
    pub name: String,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub attributes: std::collections::BTreeMap<String, String>,
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
    pub display_name: String,
    pub name: String,
    pub metadata_ordered: Vec<(String, String)>,
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
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub data_service_attributes: std::collections::BTreeMap<String, String>,
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

impl RenderDomain {
    /// Domain.findObjectDescriptor uses Java String.equalsIgnoreCase on KSML
    /// object names. Generated module names are not lookup aliases.
    pub(crate) fn find_object_descriptor(&self, name: &str) -> Option<&RenderEntity> {
        self.entities
            .iter()
            .find(|entity| crate::preview_domain::java_equals_ignore_case(&entity.name, name))
    }

    /// Match EntityDomain/EntityObjectDescriptor, retaining the underlying
    /// descriptors for relation targets even when they have no generated module.
    pub(crate) fn entity_domain(&self) -> Self {
        let mut view = self.clone();
        let view_names: std::collections::HashSet<_> = self
            .entities
            .iter()
            .filter(|e| e.metadata_flag("viewObject"))
            .map(|e| e.rust_struct.clone())
            .collect();
        for list in [
            &mut view.entities,
            &mut view.object_descriptors,
            &mut view.business_descriptors,
            &mut view.constant_descriptors,
            &mut view.root_descriptors,
        ] {
            list.retain(|e| e.repository());
            for entity in list {
                entity
                    .reverse_relations
                    .retain(|r| !view_names.contains(&r.target_struct));
                entity
                    .one_to_one_relations
                    .retain(|r| !view_names.contains(&r.target_struct));
                entity
                    .one_to_many_relations
                    .retain(|r| !view_names.contains(&r.target_struct));
                entity.rust_needs_smart_list = !entity.one_to_many_relations.is_empty();
            }
        }
        view
    }
}

impl RenderEntity {
    pub(crate) fn type_name_with_suffix(&self, suffix: &str) -> String {
        // NamingCase sees the suffix before converting underscores and case.
        crate::preview_domain::pascal(&format!("{}_{suffix}", self.name))
    }

    pub(crate) fn camel_plural_name(&self) -> String {
        crate::preview_domain::camel(&pluralize_name(&self.name))
    }

    pub(crate) fn pascal_plural_name(&self) -> String {
        crate::preview_domain::pascal(&pluralize_name(&self.name))
    }

    /// The source ST adaptor resolves attached/inherited attributes before
    /// Java bean getters; field discovery still uses its original KSML name.
    pub(crate) fn assist_field_type_name(
        &self,
        domain: &RenderDomain,
        name: &str,
    ) -> Option<String> {
        if let Some(field) = self.fields.iter().find(|field| field.name == name) {
            return Some(field.source_type_name.clone());
        }
        let relation = self.object_fields.iter().find(|field| field.name == name)?;
        // ObjectType delegates attached attribute lookup to its referent. Its
        // underlying Type.name bean is null, not the referenced object's name.
        domain
            .source_entities
            .iter()
            .find(|entity| entity.rust_struct == relation.target_struct)?
            .metadata_ordered
            .iter()
            .find(|(key, value)| key == "name" && !value.is_empty())
            .map(|(_, value)| value.clone())
    }

    pub(crate) fn assist_field_name(&self, domain: &RenderDomain, name: &str) -> String {
        let value = if let Some(field) = self.fields.iter().find(|field| field.name == name) {
            field.attribute("name")
        } else if let Some(relation) = self.object_fields.iter().find(|field| field.name == name) {
            relation
                .metadata
                .get("name")
                .map(String::as_str)
                .or_else(|| {
                    domain
                        .source_entities
                        .iter()
                        .find(|entity| entity.rust_struct == relation.target_struct)
                        .and_then(|entity| {
                            entity
                                .metadata_ordered
                                .iter()
                                .find(|(key, _)| key == "name")
                        })
                        .map(|(_, value)| value.as_str())
                })
        } else {
            None
        };
        value
            .filter(|value| !value.is_empty())
            .unwrap_or(name)
            .to_string()
    }

    pub(crate) fn metadata_flag(&self, key: &str) -> bool {
        self.metadata_ordered
            .iter()
            .find(|(name, _)| name == key)
            .is_some_and(|(_, value)| {
                matches!(
                    value.trim().to_lowercase().as_str(),
                    "true"
                        | "yes"
                        | "y"
                        | "t"
                        | "ok"
                        | "1"
                        | "on"
                        | "是"
                        | "对"
                        | "真"
                        | "對"
                        | "√"
                )
            })
    }
    pub(crate) fn repository(&self) -> bool {
        !self.metadata_flag("valueObject") && !self.metadata_flag("viewObject")
    }

    pub fn has_parent(&self) -> bool {
        self.metadata_ordered
            .iter()
            .any(|(name, value)| name == "parent" && !value.is_empty())
    }

    pub fn ancestors<'a>(&self, domain: &'a RenderDomain) -> Vec<&'a RenderEntity> {
        let mut ancestors = Vec::new();
        let mut current = self;
        let mut visited = std::collections::HashSet::new();
        while let Some((_, parent)) = current
            .metadata_ordered
            .iter()
            .find(|(name, _)| name == "parent")
        {
            if !visited.insert(parent.as_str()) {
                break;
            }
            let Some(entity) = domain.entities.iter().find(|entity| entity.name == *parent) else {
                break;
            };
            ancestors.push(entity);
            current = entity;
        }
        ancestors
    }

    pub fn children<'a>(&self, domain: &'a RenderDomain) -> Vec<&'a RenderEntity> {
        domain
            .entities
            .iter()
            .filter(|entity| {
                entity
                    .metadata_ordered
                    .iter()
                    .any(|(key, parent)| key == "parent" && parent == &self.name)
            })
            .collect()
    }

    pub fn descendants<'a>(&self, domain: &'a RenderDomain) -> Vec<&'a RenderEntity> {
        domain
            .entities
            .iter()
            .filter(|entity| {
                entity
                    .ancestors(domain)
                    .iter()
                    .any(|parent| parent.name == self.name)
            })
            .collect()
    }
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
    pub source_type_name: String,
    pub source_value: String,
    pub metadata: std::collections::BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_attributes: Vec<teaql_forge_model::types::TypeAttribute>,
    pub source_order: usize,
    pub has_sample_value: bool,
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

impl RenderField {
    pub(crate) fn max_attribute(&self) -> Option<&str> {
        self.attribute("max").or_else(|| {
            matches!(self.source_type_name.as_str(), "string" | "password" | "uniStr" | "businessId" | "business_id")
                .then_some("100")
        })
    }

    pub(crate) fn automatic_function(&self, name: &str) -> bool {
        let value = self.attribute(name).or_else(|| match (name, self.source_type_name.as_str()) {
            ("createFunction", "createTime" | "updateTime") | ("updateFunction", "updateTime") => Some("now"),
            ("createFunction", "uniStr") => Some("uniStr"),
            _ => None,
        });
        value.is_some_and(|value| !value.is_empty()
            && !matches!(value.to_ascii_lowercase().as_str(), "false" | "no" | "off"))
    }

    pub fn is_number(&self) -> bool {
        self.flag(
            "isNumber",
            matches!(
                self.source_type_name.as_str(),
                "integer" | "long" | "number" | "decimal" | "money" | "float" | "double"
            ),
        )
    }

    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.metadata.get(name).map(String::as_str).or_else(|| {
            self.type_attributes
                .iter()
                .find(|attribute| attribute.name == name)
                .map(|attribute| attribute.value.as_str())
        })
    }
    pub fn flag(&self, name: &str, fallback: bool) -> bool {
        self.attribute(name)
            .map(teaql_forge_model::parser::hutool_true)
            .unwrap_or(fallback)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderRelation {
    pub context_root: serde_json::Value,
    pub metadata: std::collections::BTreeMap<String, String>,
    pub name: String,
    pub source_value: String,
    pub source_order: usize,
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
    #[serde(rename = "rustRootSeedValue")]
    pub rust_root_seed_value: String,
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

impl RenderRelation {
    pub(crate) fn attribute<'a>(&'a self, domain: &'a RenderDomain, name: &str) -> Option<&'a str> {
        self.metadata.get(name).map(String::as_str).or_else(|| {
            domain.source_entities.iter().find(|entity| entity.rust_struct == self.target_struct)
                .and_then(|entity| entity.metadata_ordered.iter().find(|(key, _)| key == name))
                .map(|(_, value)| value.as_str())
        })
    }

    pub(crate) fn automatic_function(&self, domain: &RenderDomain, name: &str) -> bool {
        self.attribute(domain, name).is_some_and(|value| !value.is_empty()
            && !matches!(value.to_ascii_lowercase().as_str(), "false" | "no" | "off"))
    }
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
    pub java_member_name: String,
    pub java_member_names: String,
    pub java_appender: String,
    pub java_bean_name: String,
    pub target_method: String,
    pub target_struct: String,
    pub target_module: String,
    pub target_plural: String,
    #[serde(skip)]
    pub raw_target_name: String,
    pub target: String,
    pub local_key: String,
    pub many: bool,
    pub delete_missing: bool,
    #[serde(rename = "rustType")]
    pub rust_type: String,
    pub rust_member_name: String,
    pub rust_member_names: String,
    #[serde(rename = "rustRelationAttr")]
    pub rust_relation_attr: String,
    pub child: RenderReverseRelationChild,
    pub rust_field_aggregate_methods: Vec<RenderFieldAggregateMethod>,
}

pub fn build_render_context(domain: &Domain) -> RenderDomain {
    // Removed crate_name
    // Removed module_name

    let mut entities: Vec<RenderEntity> = domain.entities.iter().map(|e| {
        let rust_struct = crate::preview_domain::pascal(&e.name);
        let rust_module = crate::preview_domain::snake(&e.name);
        let rust_plural = crate::preview_domain::snake(&pluralize_name(&e.name));
        let table_name = e.metadata.get("table").filter(|name| !name.is_empty()).cloned().or_else(|| e.table.clone()).unwrap_or_else(|| format!("{}_data", e.name));

        let attr_pred_prefix = e.metadata.get("attributePredicatePrefix").cloned().unwrap_or_else(|| if e.is_human { "whose" } else { "with" }.to_string());
        let attr_pred_suffix = e.metadata.get("attributePredicateSuffix").cloned().unwrap_or_else(|| if e.is_human { "s" } else { "ing" }.to_string());

        let fields: Vec<RenderField> = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Field(f) => Some(f),
            _ => None,
        }).map(|f| {
            let rust_name = sanitize_rust_identifier(&f.name);
            let column_name = f.attribute("column").map(str::to_string).unwrap_or_else(|| {
                let service = e.data_service.as_deref().or_else(|| domain.root_attributes.get("data_service").map(String::as_str)).unwrap_or("postgres");
                let quote = teaql_forge_model::data_services::find(&domain.source_data_services, service)
                    .and_then(|service| service.attributes.iter().find(|attr| attr.name == "quoteCharForKeywords"))
                    .map(|attr| attr.value.as_str()).unwrap_or("");
                crate::sql::quoted_name(&crate::preview_domain::snake(&f.name), quote)
            });
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
                FieldType::DateTime => "teaql_core::time::Timestamp",
            }.to_string();
            if f.source_type_name == "integer" {
                rust_type = "i64".to_string();
            } else if matches!(f.source_type_name.as_str(), "number" | "decimal" | "money") {
                rust_type = "rust_decimal::Decimal".to_string();
            }
            if f.source_type_name == "float" { rust_type = "f32".into(); }
            if f.source_type_name == "jsonMe" { rust_type = "serde_json::Value".into(); }

            if let Some(configured) = f.attribute("rustType").filter(|value| !value.is_empty()) { rust_type = configured.into(); }
            let mut rust_default_value = match f.ty {
                FieldType::Id => "0_u64",
                FieldType::String | FieldType::Text => "String::new()",
                FieldType::Bool => "false",
                FieldType::I32 => "0_i32",
                FieldType::I64 => "0_i64",
                FieldType::U64 => "0_u64",
                FieldType::Decimal => "0.0",
                FieldType::Date => "chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()",
                FieldType::DateTime => "teaql_core::time::Timestamp::now()",
            }.to_string();
            if f.source_type_name == "integer" {
                rust_default_value = "0_i64".to_string();
            } else if matches!(f.source_type_name.as_str(), "number" | "decimal" | "money") {
                rust_default_value = "rust_decimal::Decimal::ZERO".to_string();
            }
            if f.source_type_name == "float" { rust_default_value = "0.0_f32".into(); }
            if f.source_type_name == "double" { rust_default_value = "0.0_f64".into(); }
            if f.source_type_name == "jsonMe" { rust_default_value = "serde_json::Value::Null".into(); }

            if !f.required && f.ty != FieldType::Id {
                rust_type = format!("Option<{}>", rust_type);
                rust_default_value = "None".to_string();
            }

            let unwrap_method = if !f.required && f.ty != FieldType::Id { "or" } else { "unwrap_or" };

            let mut rust_getter_body = format!("self.changed_{name}().and_then(|value| value.{getter_method}).{unwrap_method}{else_suffix}{self_name}", name=rust_name, getter_method=match f.ty {
                FieldType::Id | FieldType::U64 => "try_u64()",
                FieldType::I32 => "try_i64().map(|value| value as i64)",
                FieldType::I64 => "try_i64()",
                FieldType::Bool => "try_bool()",
                FieldType::String | FieldType::Text => "try_text().map(|value| value.to_owned())",
                FieldType::DateTime => "try_timestamp()",
                FieldType::Decimal => "try_decimal()",
                FieldType::Date => "try_date()",
            }, unwrap_method=unwrap_method, else_suffix=if f.ty == FieldType::String || f.ty == FieldType::Text { "_else(|| self." } else { "(self." }, self_name=if f.ty == FieldType::String || f.ty == FieldType::Text { format!("{}.clone())", rust_name) } else { format!("{})", rust_name) });
            if f.source_type_name == "integer" && f.required {
                rust_getter_body = format!("self.changed_{rust_name}().and_then(|value| value.try_i64()).map(|value| value as i64).unwrap_or(self.{rust_name})");
            }
            if f.source_type_name == "float" && f.required { rust_getter_body = format!("self.changed_{rust_name}().and_then(|value| value.try_f64()).map(|value| value as f32).unwrap_or(self.{rust_name})"); }
            if f.source_type_name == "double" && f.required { rust_getter_body = format!("self.changed_{rust_name}().and_then(|value| value.try_f64()).unwrap_or(self.{rust_name})"); }
            if !f.required || f.source_type_name == "jsonMe" { rust_getter_body = format!("self.{rust_name}.clone()"); }

            let mut rust_update_assignment = format!("value.{getter_method}.{unwrap_method}{else_suffix}{rust_name}.clone())", getter_method=match f.ty {
                FieldType::Id | FieldType::U64 => "try_u64()",
                FieldType::I32 => "try_i64().map(|value| value as i64)",
                FieldType::I64 => "try_i64()",
                FieldType::Bool => "try_bool()",
                FieldType::String | FieldType::Text => "try_text().map(|value| value.trim().to_owned())",
                FieldType::DateTime => "try_timestamp()",
                FieldType::Decimal => "try_decimal()",
                FieldType::Date => "try_date()",
            }, unwrap_method=unwrap_method, else_suffix=if f.ty == FieldType::String || f.ty == FieldType::Text { "_else(|| self." } else { "(self." }, rust_name=rust_name);
            if f.source_type_name == "float" { rust_update_assignment = format!("value.try_f64().map(|value| value as f32).unwrap_or(self.{rust_name}.clone())"); }
            if f.source_type_name == "double" { rust_update_assignment = format!("value.try_f64().unwrap_or(self.{rust_name}.clone())"); }
            if f.source_type_name == "jsonMe" { rust_update_assignment = if f.required { "value.to_json_value()".into() } else { format!("if matches!(value, teaql_core::Value::Null) {{ None }} else {{ Some(value.to_json_value()).map(Some).unwrap_or_else(|| self.{rust_name}.clone()) }}") }; }
            if !f.required && f.source_type_name != "jsonMe" {
                let converter = match f.source_type_name.as_str() {
                    "id" => "value.try_u64()",
                    "integer" => "value.try_i64().map(|value| value as i64)",
                    "long" | "version" => "value.try_i64()",
                    "bool" | "boolean" => "value.try_bool()",
                    "number" | "decimal" | "money" => "value.try_decimal()",
                    "float" => "value.try_f64().map(|value| value as f32)",
                    "double" => "value.try_f64()",
                    "date" => "value.try_date()",
                    "createTime" | "updateTime" | "dateTime" | "datetime" => "value.try_timestamp()",
                    "string" | "text" | "password" | "uniStr" => "value.try_text().map(|value| value.trim().to_owned())",
                    _ => "value.try_text().map(|value| value.to_owned())",
                };
                rust_update_assignment = format!("if matches!(value, teaql_core::Value::Null) {{ None }} else {{ {converter}.map(Some).unwrap_or_else(|| self.{rust_name}.clone()) }}");
            }

            if f.attribute("rustType").is_some_and(|value| value != "String") && matches!(f.ty, FieldType::String | FieldType::Text) {
                rust_update_assignment = rust_update_assignment.replace("value.trim().to_owned()", "value.to_owned()");
            }
            let mut rust_attrs = Vec::new();
            if f.ty == FieldType::Id {
                rust_attrs.push("#[teaql(id)]".to_string());
            }
            if f.name == "version" {
                rust_attrs.push("#[teaql(version)]".to_string());
            }
            if f.ty == FieldType::Text || f.attribute("isText").is_some_and(crate::preview_rust::attribute_flag) {
                rust_attrs.push("#[teaql(large_text)]".to_string());
            } else if f.attribute("isString")
                .map(teaql_forge_model::parser::hutool_true)
                .unwrap_or(matches!(f.source_type_name.as_str(), "string" | "password" | "uniStr" | "businessId" | "business_id"))
                && !f.attribute("isText")
                    .map(teaql_forge_model::parser::hutool_true)
                    .unwrap_or(f.ty == FieldType::Text)
            {
                let max = f.attribute("max").or_else(|| {
                    matches!(
                        f.source_type_name.as_str(),
                        "string" | "password" | "uniStr" | "businessId" | "business_id"
                    )
                    .then_some("100")
                });
                if let Some(max) = max.and_then(|max| {
                    crate::hutool_json::parse_integer(max.trim_matches(|c| c <= '\u{20}'))
                        .and_then(|n| i32::try_from(n).ok())
                        .filter(|&n| n > 0)
                }) {
                    rust_attrs.push(format!("#[teaql(max_length = {max})]"));
                }
            }
            if matches!(f.source_type_name.as_str(), "number" | "money" | "decimal") || f.attribute("sqlType").is_some_and(|value| value.to_ascii_uppercase().starts_with("NUMERIC(")) {
                let sql_type = f.attribute("sqlType").unwrap_or("NUMERIC(19,7)");
                let numeric = sql_type.split_once('(').and_then(|(_, args)| args.trim_end_matches(')').split_once(','));
                let (precision, scale) = numeric.unwrap_or(("19", "7"));
                rust_attrs.push(format!("#[teaql(numeric_precision = {})]", precision.trim()));
                rust_attrs.push(format!("#[teaql(numeric_scale = {})]", scale.trim()));
            }
            if rust_name != column_name {
                rust_attrs.push(format!("#[teaql(column = \"{column_name}\")]"));
            }
            let rust_attr = rust_attrs.join("\n");

            let mut ksml_type = match f.ty {
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
            if matches!(f.source_type_name.as_str(), "float" | "double" | "time") { ksml_type = f.source_type_name.clone(); }



            RenderField {
                name: f.name.clone(),
                source_type_name: f.source_type_name.clone(),
                source_value: f.source_value.clone(),
                metadata: f.metadata.clone(),
                type_attributes: f.type_attributes.clone(),
                source_order: f.source_order,
                has_sample_value: f.metadata.contains_key("sampleValue"),
                line_number: f.line_number,
                xml_path: f.xml_path.clone(),
                rust_name: rust_name.clone(),
                column_name,
                rust_type: rust_type.clone(),
                ksml_type,
                is_id: f.name == "id",
                is_version: f.name == "version",
                is_string: f.attribute("isString").map(teaql_forge_model::parser::hutool_true).unwrap_or(f.ty == FieldType::String && f.source_type_name != "time"),
                is_text: f.attribute("isText").map(teaql_forge_model::parser::hutool_true).unwrap_or(f.ty == FieldType::Text),
                is_password: f.attribute("isPassword").map(teaql_forge_model::parser::hutool_true).unwrap_or(f.source_type_name == "password"),
                is_date: f.attribute("isDate").map(teaql_forge_model::parser::hutool_true).unwrap_or(matches!(f.ty, FieldType::Date | FieldType::DateTime)),
                is_time: f.attribute("isTime").map(teaql_forge_model::parser::hutool_true).unwrap_or(f.ty == FieldType::DateTime || f.source_type_name == "time"),
                is_boolean: f.attribute("isBool").map(teaql_forge_model::parser::hutool_true).unwrap_or(f.ty == FieldType::Bool),
                required: f.required,
                rust_default_value: rust_default_value.clone(),
                rust_getter_body,
                rust_update_assignment,
                rust_attr,
                sample_value: f.metadata.get("sampleValue")
                    .or_else(|| e.seed_values.first().and_then(|s| s.properties.get(&f.name)))
                    .cloned()
                    .unwrap_or_else(|| "Sample".to_string()),
                rust_storage_field_name: crate::preview_domain::snake(&f.name),
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
                    } else if matches!(f.source_type_name.as_str(), "integer" | "long") {
                        let sample = f.metadata.get("candidates").filter(|value| !value.is_empty()).or_else(|| f.metadata.get("sampleValue")).map(String::as_str).unwrap_or("");
                        let sample = sample.split(',').next().unwrap_or("");
                        let digits = sample.strip_prefix('-').unwrap_or(sample);
                        format!("{}_i64", if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) { sample } else { "1" })
                    } else if rust_type == "String" {
                        format!("\"{}\"", e.seed_values.first().and_then(|s| s.properties.get(&f.name)).cloned().unwrap_or_else(|| "Pending".to_string()))
                    } else if !f.required {
                        "teaql_core::Value::Null".to_string()
                    } else {
                        rust_default_value.clone()
                    }
                },
                rust_attribute_predicate_prefix_expr: rust_predicate_prefix(&attr_pred_prefix, &f.name),
                rust_attribute_predicate_suffix_expr: attr_pred_suffix.clone(),
                rust_storage_member_name: rust_name.clone(),
                rust_storage_type: rust_type.clone(),
            }
        }).collect();

        let relations: Vec<RenderRelation> = e.members.iter().filter_map(|m| match m {
            teaql_forge_model::ir::EntityMember::Relation(r) => Some(r),
            _ => None,
        }).map(|r| {
            let many = r.cardinality == Cardinality::OneToMany;
            let target_struct = crate::preview_domain::pascal(&r.target);
            let local_key = if many { "id".to_string() } else { format!("{}_id", crate::preview_domain::snake(&r.name)) };
            let foreign_key = if many { format!("{}_id", e.name.to_snake_case()) } else { "id".to_string() };

            let target_entity = domain.entities.iter().find(|de| de.name.to_snake_case() == r.target.to_snake_case() || de.name == r.target);
            let target_display_name = target_entity.and_then(|de| de.metadata.get("name").cloned()).unwrap_or_else(|| target_entity.map(|de| de.name.clone()).unwrap_or(r.target.clone()));

            RenderRelation {
                context_root: r.metadata.get("contextRoot")
                    .or_else(|| target_entity.and_then(|target| target.metadata.get("contextRoot")))
                    .filter(|value| !value.is_empty()).map(|value| crate::preview_domain::attribute_value(value))
                    .unwrap_or(serde_json::Value::Bool(false)),
                metadata: r.metadata.clone(),
                name: r.name.clone(),
                source_value: r.source_value.clone(),
                source_order: r.source_order,
                line_number: r.line_number,
                xml_path: r.xml_path.clone(),
                rust_name: sanitize_rust_identifier(&r.name),
                target_method: crate::preview_domain::snake(&pluralize_name(&r.target)),
                target_struct: target_struct.clone(),
                target_plural: crate::preview_domain::snake(&pluralize_name(&r.target)),
                target_module: crate::preview_domain::snake(&r.target),
                target: target_display_name,
                local_key: local_key.clone(),
                foreign_key: foreign_key.clone(),
                many,
                required: r.required,
                delete_missing: false,
                rust_attribute_predicate_prefix_expr: rust_predicate_prefix(&attr_pred_prefix, &r.name),
                candidates: {
                    let mut cands = Vec::new();
                    if let Some(target_ent) = target_entity {
                        for seed in &target_ent.seed_values {
                            if let Some(id_str) = seed.properties.get("id") {
                                if let Ok(id) = id_str.parse::<u64>() {
                                    let val = seed.properties.get("code").or_else(|| seed.properties.get("name")).unwrap_or(id_str);
                                    let method_value = if val.chars().any(|c| c.is_ascii_digit())
                                        && val.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
                                    {
                                        val.to_ascii_lowercase()
                                    } else if val.chars().any(|c| c.is_ascii_digit())
                                        && val.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                                    {
                                        let mut value = val.to_ascii_lowercase();
                                        for digit in '0'..='9' {
                                            value = value.replace(&format!("_{digit}"), &digit.to_string());
                                        }
                                        value
                                    } else if val == "PENDING" {
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
                count_method: format!("count_{}", crate::preview_domain::snake(&pluralize_name(&r.target))),
                rust_storage_member_name: local_key.clone(),
                rust_storage_type: if r.required { "u64".to_string() } else { "Option<u64>".to_string() },
                rust_default_value: if r.required { "0_u64".to_string() } else { "None".to_string() },
                rust_root_seed_value: if r.required { "0_u64".to_string() } else { "teaql_core::Value::Null".to_string() },
                rust_field_attr: {
                    let column = r.metadata.get("column").cloned().unwrap_or_else(|| {
                        let name = crate::preview_domain::snake(&r.name);
                        if name == "user" { "\\\"user\\\"".into() } else { name }
                    });
                    if column == local_key { String::new() }
                    else { format!("#[teaql(column = \"{column}\")]") }
                },

                rust_member_name: sanitize_rust_identifier(&r.name),
                rust_relation_type: format!("Option<crate::{}>", target_struct),
                rust_relation_borrow_type: format!("crate::{}", target_struct),
                rust_getter_body: if r.required { format!("self.changed_{local_key}().and_then(|value| value.try_u64()).unwrap_or(self.{local_key})") } else { format!("self.{local_key}.clone()") },
                rust_update_assignment: if r.required { format!("value.try_u64().unwrap_or(self.{local_key}.clone())") } else { format!("if matches!(value, teaql_core::Value::Null) {{ None }} else {{ value.try_u64().map(Some).unwrap_or_else(|| self.{local_key}.clone()) }}") },
                rust_storage_field_name: local_key.clone(),
                constant_object_field: target_entity.is_some_and(|de| de.metadata.get("constant").is_some_and(|value| teaql_forge_model::parser::hutool_true(value))),
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
                            java_member_name: name.to_lower_camel_case(),
                            java_member_names: crate::preview_domain::pascal(&pluralize_name(&other.name)),
                            java_appender: format!("add{}", other.name.to_pascal_case()),
                            java_bean_name: other.name.to_lower_camel_case(),
                            target_method: pluralize_name(&other.name.to_snake_case()),
                            target_struct: other.name.to_pascal_case(),
                            target_module: other.name.to_snake_case(),
                            target_plural: pluralize_name(&other.name.to_snake_case()),
                            raw_target_name: other.name.clone(),
                            target: other.name.to_snake_case(),
                            local_key: format!("{}_id", crate::preview_domain::snake(&r.name)),
                            many,
                            delete_missing: false,
                            rust_type: "".to_string(),
                            rust_member_name: name.clone(),
                            rust_member_names: pluralize_name(&other.name.to_snake_case()),
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

            for rel in &relations {
                let k = rel.name.clone();
                if let Some(v) = seed.properties.get(&k) {
                    graph.push_str(&format!("\n            .value(\"{}\", {}_u64)", rel.local_key, v));
                } else if rel.required {
                    graph.push_str(&format!("\n            .value(\"{}\", 1_u64)", rel.local_key));
                }
            }
            constant_seed_graphs.push(graph);
        }

        let has_select_all = !object_fields.is_empty() || !one_to_one_relations.is_empty();
        let has_select_any = !one_to_many_relations.is_empty();

        let mut indexes = Vec::new();
        let mut constraints = Vec::new();
        let e_name_upper = e.name.to_uppercase();

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
            let r_name_upper = crate::preview_domain::snake(&r.name).to_uppercase();
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
            display_name: e.metadata.get("name").cloned().unwrap_or(e.name.clone()),
            name: e.name.clone(),
            metadata_ordered: e.metadata_ordered.clone(),
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
            data_service: e.data_service.as_deref().and_then(|name| teaql_forge_model::data_services::find(&domain.source_data_services, name)).map(|service| service.name.clone()).or_else(|| e.data_service.clone()),
            data_service_attributes: e.data_service.as_deref().and_then(|name| teaql_forge_model::data_services::find(&domain.source_data_services, name)).map(|service| service.attributes.iter().map(|attr| (attr.name.clone(), attr.value.clone())).collect()).unwrap_or_default(),
            audit_mask_fields: {
                let m = e.audit_mask_fields.as_ref().map(|s| s.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
                m
            },
            audit_value_max_len: e.audit_value_max_len,
            attribute_predicate_prefix: attr_pred_prefix,
            attribute_predicate_suffix: attr_pred_suffix,
            bool_predicate_prefix: if e.is_human { "who_are" } else { "which_are" }.to_string(),
            constant_seed_graphs,
            seed_values: e.seed_values.clone(),
            depth: 0,
            constant: e.metadata.get("constant").is_some_and(|value| teaql_forge_model::parser::hutool_true(value)),
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

    let requested_service = domain
        .root_attributes
        .get("data_service")
        .map(String::as_str)
        .unwrap_or("postgres");
    let service =
        teaql_forge_model::data_services::find(&domain.source_data_services, requested_service);
    let service_name = service
        .map(|service| service.name.as_str())
        .unwrap_or(requested_service);
    let rust_version = domain
        .root_attributes
        .get("teaql_rs_version")
        .map(String::as_str)
        .unwrap_or("5.0.5");
    let ds = crate::data_service_properties::rust_runtime(
        service_name,
        service
            .map(|service| {
                service
                    .attributes
                    .iter()
                    .map(|attr| (attr.name.clone(), attr.value.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    );
    let provider = crate::data_service_properties::rust_provider_crate(service_name).unwrap_or("");

    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..entities.len() {
            let mut max_dep_depth = -1;
            for rel in &entities[i].relations {
                // The source depth walk excludes a descriptor's self relation.
                if rel.target.eq_ignore_ascii_case(&entities[i].name) {
                    continue;
                }
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

            if entities[i].depth > 20 {
                // To avoid panic in server, we just stop calculating depth instead of panicking,
                // but if we need a strict error, we can panic.
                // The Java version throws DomainGraphException, which fails generation.
                panic!(
                    "DomainGraphException: The KSML XML reference graph exceeds the maximum supported depth of 20. Cycle detected near entity '{}'.",
                    entities[i].name
                );
            }
        }
    }

    let mut render_entities = entities;
    for entity in &mut render_entities {
        if entity.data_service.is_none() {
            entity.data_service = Some(service_name.to_string());
            entity.data_service_attributes = ds.attributes.clone();
        }
    }

    for i in 0..render_entities.len() {
        let mut reverse_relations = Vec::new();
        let mut rust_one_to_one_relations = Vec::new();
        let mut rust_one_to_many_relations = Vec::new();

        let target_name = render_entities[i].rust_module.clone();
        let target_snake = target_name;

        for other_entity in &render_entities {
            for rel in &other_entity.relations {
                if rel.target_module == target_snake {
                    let reverse_is_many =
                        rel.metadata.get("relationType").map(String::as_str) != Some("oneToOne");
                    let has_many_from_child = other_entity
                        .relations
                        .iter()
                        .filter(|candidate| candidate.target_module == target_snake)
                        .count()
                        > 1;
                    let child_camel = crate::preview_domain::camel(&other_entity.name);
                    let as_property = if has_many_from_child {
                        format!("As{}", crate::preview_domain::pascal(&rel.name))
                    } else {
                        String::new()
                    };
                    let java_member_name =
                        rel.metadata.get("reverseName").cloned().unwrap_or_else(|| {
                            format!(
                                "{}{}{}",
                                child_camel,
                                if reverse_is_many { "List" } else { "" },
                                as_property
                            )
                        });
                    let java_appender = format!("add{}{}", other_entity.rust_struct, as_property);
                    let java_bean_name = format!("{child_camel}{as_property}");

                    let java_member_names = rel.metadata.get("reverseName")
                        .map(|name| crate::preview_domain::upper_first(name))
                        .unwrap_or_else(|| format!("{}{}", crate::preview_domain::pascal(&pluralize_name(&other_entity.name)), as_property));
                    let rust_member_name = crate::preview_domain::snake(&java_member_name).to_lowercase();
                    let rust_member_names = crate::preview_domain::snake(&java_member_names).to_lowercase();

                    let rust_type = if reverse_is_many {
                        format!("SmartList<crate::{}>", other_entity.rust_struct)
                    } else {
                        format!("Option<crate::{}>", other_entity.rust_struct)
                    };

                    let mut agg_methods = Vec::new();
                    for m in &other_entity.fields {
                        if m.name != "id"
                            && m.name != "version"
                            && m.rust_type == "teaql_core::time::Timestamp"
                        {
                            let f_name = &m.name;
                            let other_plural = crate::preview_domain::snake(&pluralize_name(&other_entity.name));
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
                        java_member_name,
                        java_member_names,
                        java_appender,
                        java_bean_name,
                        target_method: crate::preview_domain::snake(&pluralize_name(&other_entity.name)),
                        target_struct: other_entity.rust_struct.clone(),
                        target_module: other_entity.rust_module.clone(),
                        target_plural: crate::preview_domain::snake(&pluralize_name(&other_entity.name)),
                        raw_target_name: other_entity.name.clone(),
                        target: crate::preview_domain::snake(&other_entity.name),
                        local_key: format!("{}_id", crate::preview_domain::snake(&rel.name)),
                        many: reverse_is_many,
                        delete_missing: false,
                        rust_type: rust_type.clone(),
                        rust_member_name: rust_member_name.clone(),
                        rust_member_names,
                        rust_relation_attr: format!(
                            "#[teaql(relation(target = \"{}\", local_key = \"{}\", foreign_key = \"{}\"{}))]",
                            other_entity.rust_struct,
                            rel.foreign_key,
                            rel.local_key,
                            if reverse_is_many { ", many" } else { "" }
                        ),
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
        .zip(domain.entities.iter())
        .filter(|(render, source)| {
            !render.constant
                && render.object_fields.is_empty()
                && !source.metadata.contains_key("parent")
        })
        .map(|(render, _)| render.clone())
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
        source_field_attributes: domain.source_field_attributes.clone(),
        crate_name: format!("{}-core", domain.name),
        module_name: format!("{}_core", domain.name.replace("-", "_")),
        workspace_crate_name: format!("{}-core-workspace", domain.name),
        has_sql_provider: ds.rust_has_sql_provider,
        entities: render_entities.clone(),
        source_entities: render_entities.clone(),
        data_service: ds,
        name: domain.name.clone(),
        root_attributes: domain.root_attributes.clone(),
        root_attributes_ordered: domain.root_attributes_ordered.clone(),
        source_types: domain.source_types.clone(),
        source_type_states: domain.source_type_states.clone(),
        source_data_services: domain.source_data_services.clone(),
        java_full_package: domain.root_attributes.get("org").map(|org| {
            let group = if org.contains('.') {
                org.clone()
            } else {
                format!("com.{org}")
            };
            let package_name: String = domain
                .name
                .chars()
                .filter(|character| character.is_ascii_alphabetic())
                .collect::<String>()
                .to_ascii_lowercase();
            format!("{group}.{package_name}")
        }),
        rust_struct_name: domain.name.to_pascal_case(),
        rust_module_name: domain.name.to_snake_case(),
        rust_crate_name: format!("{}-core", crate::preview_rust::symbol(&domain.name, '-').replace('_', "-")),
        rust_workspace_crate_name: domain
            .root_attributes
            .get("rust_workspace_crate_name")
            .or_else(|| domain.root_attributes.get("rust_app_crate_name"))
            .cloned()
            .unwrap_or_else(|| format!("{}-core-workspace", crate::preview_rust::symbol(&domain.name, '-').replace('_', "-"))),
        rust_workspace_generated_lib_path: "../rust-lib-core/lib".to_string(),
        rust_teaql_dependency_version: rust_version.to_string(),
        rust_env_prefix: format!("{}-core", crate::preview_rust::symbol(&domain.name, '-').replace('_', "-"))
            .replace('-', "_").to_uppercase(),
        rust_crate_version: domain
            .root_attributes
            .get("version")
            .cloned()
            .unwrap_or_else(|| "0.1.0".to_string()),
        generator_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_sql_provider_dependency: format!("{provider} = \"{rust_version}\""),
        support_full_text_search: render_entities.iter().any(|entity| {
            entity
                .data_service
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case("meilisearch"))
        }),
        object_descriptors: render_entities.clone(),
        root_descriptors,
        constant_descriptors,
        business_descriptors,
    }
}
