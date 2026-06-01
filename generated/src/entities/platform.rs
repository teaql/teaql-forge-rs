use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub id: u64,
    pub name: Option<String>,
    pub founded: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i64,
    pub comment: String,
    pub deleted: bool,
}

impl Platform {
    pub fn new() -> Self {
        Self {
            id: 0,
            name: None,
            founded: None,
            version: 0,
            comment: String::new(),
            deleted: false,
        }
    }
    
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn update_id(&mut self, id: impl Into<u64>) -> &mut Self {
        self.id = id.into();
        self
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn update_version(&mut self, version: i64) -> &mut Self {
        self.version = version;
        self
    }

    pub fn set_comment(&mut self, comment: &str) {
        self.comment = comment.to_string();
    }

    pub fn mark_as_delete(&mut self) {
        self.deleted = true;
    }
    pub fn name(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
    pub fn update_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.name = Some(value.into());
        self
    }
    pub fn founded(&self) -> chrono::DateTime<chrono::Utc> {
        self.founded.clone().unwrap_or_default()
    }
    pub fn update_founded(&mut self, value: impl Into<chrono::DateTime<chrono::Utc>>) -> &mut Self {
        self.founded = Some(value.into());
        self
    }

    pub fn save<'a, C>(&'a self, ctx: &'a C) -> Pin<Box<dyn Future<Output = Result<Self, std::io::Error>> + 'a>> {
        let self_clone = self.clone();
        Box::pin(async move {
            Ok(self_clone)
        })
    }
}

impl teaql_core::TeaqlEntity for Platform {
    fn entity_descriptor() -> teaql_core::EntityDescriptor {
        teaql_core::EntityDescriptor { 
            name: "platform".to_string(),
            table_name: "platform".to_string(),
            properties: vec![],
            relations: vec![],
        }
    }
}

impl teaql_core::Entity for Platform {
    fn from_record(_: std::collections::BTreeMap<String, teaql_core::Value>) -> Result<Self, teaql_core::EntityError> {
        Ok(Self::new())
    }

    fn into_record(self) -> std::collections::BTreeMap<String, teaql_core::Value> {
        std::collections::BTreeMap::new()
    }
}
