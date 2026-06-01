use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionLog {
    pub id: u64,
    pub action: Option<String>,
    pub detail: Option<String>,
    pub task_id: Option<u64>,
    pub version: i64,
    pub comment: String,
    pub deleted: bool,
}

impl TaskExecutionLog {
    pub fn new() -> Self {
        Self {
            id: 0,
            action: None,
            detail: None,
            task_id: None,
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
    pub fn action(&self) -> String {
        self.action.clone().unwrap_or_default()
    }
    pub fn update_action(&mut self, value: impl Into<String>) -> &mut Self {
        self.action = Some(value.into());
        self
    }
    pub fn detail(&self) -> String {
        self.detail.clone().unwrap_or_default()
    }
    pub fn update_detail(&mut self, value: impl Into<String>) -> &mut Self {
        self.detail = Some(value.into());
        self
    }
    pub fn task_id(&self) -> u64 {
        self.task_id.unwrap_or_default()
    }

    pub fn update_task_id(&mut self, value: impl Into<u64>) -> &mut Self {
        self.task_id = Some(value.into());
        self
    }

    pub fn save<'a, C>(&'a self, ctx: &'a C) -> Pin<Box<dyn Future<Output = Result<Self, std::io::Error>> + 'a>> {
        let self_clone = self.clone();
        Box::pin(async move {
            Ok(self_clone)
        })
    }
}

impl teaql_core::TeaqlEntity for TaskExecutionLog {
    fn entity_descriptor() -> teaql_core::EntityDescriptor {
        teaql_core::EntityDescriptor { 
            name: "task_execution_log".to_string(),
            table_name: "task_execution_log".to_string(),
            properties: vec![],
            relations: vec![],
        }
    }
}

impl teaql_core::Entity for TaskExecutionLog {
    fn from_record(_: std::collections::BTreeMap<String, teaql_core::Value>) -> Result<Self, teaql_core::EntityError> {
        Ok(Self::new())
    }

    fn into_record(self) -> std::collections::BTreeMap<String, teaql_core::Value> {
        std::collections::BTreeMap::new()
    }
}
