use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    #[serde(skip)]
    pub source_field_attributes: crate::field_attributes::Fields,
    pub name: String,
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub root_attributes: std::collections::BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub root_attributes_ordered: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub source_types: std::collections::BTreeMap<String, Vec<crate::types::TypeAttribute>>,
    #[serde(skip)]
    pub source_type_states: Vec<crate::types::TypeState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_data_services: Vec<crate::data_services::DataService>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub table: Option<String>,
    pub members: Vec<EntityMember>,
    pub is_human: bool,
    pub data_service: Option<String>,
    pub audit_mask_fields: Option<String>,
    pub audit_value_max_len: Option<usize>,
    pub metadata: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub metadata_ordered: Vec<(String, String)>,
    pub seed_values: Vec<SeedValue>,
    #[serde(default)]
    pub line_number: usize,
    #[serde(default)]
    pub xml_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedValue {
    pub properties: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityMember {
    Field(Field),
    Relation(Relation),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    #[serde(default)]
    pub source_type_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_attributes: Vec<crate::types::TypeAttribute>,
    #[serde(default)]
    pub source_order: usize,
    pub required: bool,
    pub unique: bool,
    #[serde(default)]
    pub line_number: usize,
    #[serde(default)]
    pub xml_path: String,
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub source_value: String,
}

impl Field {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.metadata.get(name).map(String::as_str).or_else(|| {
            self.type_attributes
                .iter()
                .find(|attribute| attribute.name == name)
                .map(|attribute| attribute.value.as_str())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Id,
    String,
    Text,
    Bool,
    I32,
    I64,
    U64,
    Decimal,
    Date,
    DateTime,
}

impl std::str::FromStr for FieldType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Id" | "id" => Ok(FieldType::Id),
            "String" | "string" => Ok(FieldType::String),
            "Text" | "text" => Ok(FieldType::Text),
            "Bool" | "bool" | "Boolean" | "boolean" => Ok(FieldType::Bool),
            "I32" | "i32" => Ok(FieldType::I32),
            "I64" | "i64" | "Long" | "long" | "Integer" | "integer" => Ok(FieldType::I64),
            "U64" | "u64" => Ok(FieldType::U64),
            "Decimal" | "decimal" => Ok(FieldType::Decimal),
            "Date" | "date" => Ok(FieldType::Date),
            "DateTime" | "datetime" => Ok(FieldType::DateTime),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub name: String,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub metadata: std::collections::BTreeMap<String, String>,
    pub target: String,
    #[serde(default)]
    pub source_order: usize,
    pub cardinality: Cardinality,
    pub required: bool,
    #[serde(default)]
    pub source_value: String,
    #[serde(default)]
    pub line_number: usize,
    #[serde(default)]
    pub xml_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    OneToOne,
    ManyToOne,
    OneToMany,
    ManyToMany,
}
