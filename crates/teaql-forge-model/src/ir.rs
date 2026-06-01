use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub table: Option<String>,
    pub members: Vec<EntityMember>,
    pub is_human: bool,
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
    pub required: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            "I32" | "i32" | "Integer" | "integer" => Ok(FieldType::I32),
            "I64" | "i64" | "Long" | "long" => Ok(FieldType::I64),
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
    pub target: String,
    pub cardinality: Cardinality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Cardinality {
    OneToOne,
    ManyToOne,
    OneToMany,
    ManyToMany,
}
