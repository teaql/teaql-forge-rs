use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub name: Option<String>,
    pub status_id: Option<u64>,
    pub platform_id: Option<u64>,
    pub task_execution_log_list: Vec<crate::entities::task_execution_log::TaskExecutionLog>,
    pub version: i64,
    pub comment: String,
    pub deleted: bool,
}

impl Task {
    pub fn new() -> Self {
        Self {
            id: 0,
            name: None,
            status_id: None,
            platform_id: None,
            task_execution_log_list: vec![],
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
    pub fn task_execution_log_list_mut(&mut self) -> &mut Vec<crate::entities::task_execution_log::TaskExecutionLog> {
        &mut self.task_execution_log_list
    }
    pub fn name(&self) -> String {
        self.name.clone().unwrap_or_default()
    }
    pub fn update_name(&mut self, value: impl Into<String>) -> &mut Self {
        self.name = Some(value.into());
        self
    }
    pub fn status_id(&self) -> u64 {
        self.status_id.unwrap_or_default()
    }

    pub fn update_status_id(&mut self, value: impl Into<u64>) -> &mut Self {
        self.status_id = Some(value.into());
        self
    }
    pub fn platform_id(&self) -> u64 {
        self.platform_id.unwrap_or_default()
    }

    pub fn update_platform_id(&mut self, value: impl Into<u64>) -> &mut Self {
        self.platform_id = Some(value.into());
        self
    }
    pub fn update_status_to_planned(&mut self) -> &mut Self {
        self.status_id = Some(1001);
        self
    }
    pub fn update_status_to_ready(&mut self) -> &mut Self {
        self.status_id = Some(1002);
        self
    }
    pub fn update_status_to_executing(&mut self) -> &mut Self {
        self.status_id = Some(1003);
        self
    }
    pub fn update_status_to_verified(&mut self) -> &mut Self {
        self.status_id = Some(1004);
        self
    }
    pub fn generate_execution_log<C>(&self, action: &str, detail: &str, _ctx: &C) -> crate::entities::task_execution_log::TaskExecutionLog {
        let mut log = crate::entities::task_execution_log::TaskExecutionLog::new();
        log.update_action(action.to_string())
           .update_detail(detail.to_string())
           .update_task_id(self.id);
        log
    }
    pub fn transition_status(&mut self, _cmd: &impl std::any::Any) -> Result<(), String> {
        Ok(())
    }

    pub fn save<'a, C>(&'a self, ctx: &'a C) -> Pin<Box<dyn Future<Output = Result<Self, std::io::Error>> + 'a>> {
        let self_clone = self.clone();
        Box::pin(async move {
            Ok(self_clone)
        })
    }
}

impl teaql_core::TeaqlEntity for Task {
    fn entity_descriptor() -> teaql_core::EntityDescriptor {
        teaql_core::EntityDescriptor { 
            name: "task".to_string(),
            table_name: "task".to_string(),
            properties: vec![],
            relations: vec![],
        }
    }
}

impl teaql_core::Entity for Task {
    fn from_record(_: std::collections::BTreeMap<String, teaql_core::Value>) -> Result<Self, teaql_core::EntityError> {
        Ok(Self::new())
    }

    fn into_record(self) -> std::collections::BTreeMap<String, teaql_core::Value> {
        std::collections::BTreeMap::new()
    }
}
