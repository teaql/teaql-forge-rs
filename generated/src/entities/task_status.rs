use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub id: u64,
    pub name: Option<String>,
    pub code: Option<String>,
    pub color: Option<String>,
    pub display_order: Option<i32>,
    pub progress: Option<i32>,
    pub version: i64,
    pub comment: String,
    pub deleted: bool,
}

impl TaskStatus {
    pub fn new() -> Self {
        Self {
            id: 0,
            name: None,
            code: None,
            color: None,
            display_order: None,
            progress: None,
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
    pub fn code(&self) -> String {
        self.code.clone().unwrap_or_default()
    }
    pub fn update_code(&mut self, value: impl Into<String>) -> &mut Self {
        self.code = Some(value.into());
        self
    }
    pub fn color(&self) -> String {
        self.color.clone().unwrap_or_default()
    }
    pub fn update_color(&mut self, value: impl Into<String>) -> &mut Self {
        self.color = Some(value.into());
        self
    }
    pub fn display_order(&self) -> i32 {
        self.display_order.clone().unwrap_or_default()
    }
    pub fn update_display_order(&mut self, value: impl Into<i32>) -> &mut Self {
        self.display_order = Some(value.into());
        self
    }
    pub fn progress(&self) -> i32 {
        self.progress.clone().unwrap_or_default()
    }
    pub fn update_progress(&mut self, value: impl Into<i32>) -> &mut Self {
        self.progress = Some(value.into());
        self
    }

    pub fn save<'a, C>(&'a self, ctx: &'a C) -> Pin<Box<dyn Future<Output = Result<Self, std::io::Error>> + 'a>> {
        let self_clone = self.clone();
        Box::pin(async move {
            Ok(self_clone)
        })
    }
}

impl teaql_core::TeaqlEntity for TaskStatus {
    fn entity_descriptor() -> teaql_core::EntityDescriptor {
        teaql_core::EntityDescriptor { 
            name: "task_status".to_string(),
            table_name: "task_status".to_string(),
            properties: vec![],
            relations: vec![],
        }
    }
}

impl teaql_core::Entity for TaskStatus {
    fn from_record(_: std::collections::BTreeMap<String, teaql_core::Value>) -> Result<Self, teaql_core::EntityError> {
        Ok(Self::new())
    }

    fn into_record(self) -> std::collections::BTreeMap<String, teaql_core::Value> {
        std::collections::BTreeMap::new()
    }
}
