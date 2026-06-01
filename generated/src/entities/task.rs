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

    pub fn save(mut self, ctx: &teaql_runtime::UserContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<teaql_runtime::GraphNode, std::io::Error>> + Send + '_>> {
        Box::pin(async move {
            let repo = ctx.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, crate::ServiceRuntimeExecutor>("task")
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            let mut node = teaql_runtime::GraphNode::new("task");
            if self.deleted {
                node.operation = teaql_runtime::GraphOperation::Remove;
            } else if self.id == 0 {
                node.operation = teaql_runtime::GraphOperation::Create;
            } else {
                node.operation = teaql_runtime::GraphOperation::Upsert;
            }
            if !self.comment.is_empty() {
                node.comment = Some(self.comment.clone());
            }
            let mut logs = Vec::new();
            let log_list = std::mem::take(&mut self.task_execution_log_list);
            for log in log_list {
                let mut log_node = teaql_runtime::GraphNode::new("task_execution_log");
                if log.deleted {
                    log_node.operation = teaql_runtime::GraphOperation::Remove;
                } else if log.id == 0 {
                    log_node.operation = teaql_runtime::GraphOperation::Create;
                } else {
                    log_node.operation = teaql_runtime::GraphOperation::Upsert;
                }
                if !log.comment.is_empty() {
                    log_node.comment = Some(log.comment.clone());
                }
                log_node.values = teaql_core::Entity::into_record(log);
                logs.push(log_node);
            }
            if !logs.is_empty() {
                node.relations.insert("task_execution_log_list".to_string(), logs);
            }
            let mut values = teaql_core::Entity::into_record(self);
            node.values = values;
            repo.save_graph(node).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })
    }
}

impl teaql_core::TeaqlEntity for Task {
    fn entity_descriptor() -> teaql_core::EntityDescriptor {
        teaql_core::EntityDescriptor { 
            name: "task".to_string(),
            table_name: "task_data".to_string(),
            properties: vec![
                teaql_core::PropertyDescriptor {
                    name: "id".to_string(),
                    column_name: "id".to_string(),
                    data_type: teaql_core::DataType::U64,
                    nullable: false,
                    is_id: true,
                    is_version: false,
                },
                teaql_core::PropertyDescriptor {
                    name: "version".to_string(),
                    column_name: "version".to_string(),
                    data_type: teaql_core::DataType::I64,
                    nullable: false,
                    is_id: false,
                    is_version: true,
                },
                teaql_core::PropertyDescriptor {
                    name: "name".to_string(),
                    column_name: "name".to_string(),
                    data_type: match "Option<String>" {
                        "String" | "Option<String>" => teaql_core::DataType::Text,
                        "u64" | "Option<u64>" => teaql_core::DataType::U64,
                        "i64" | "Option<i64>" => teaql_core::DataType::I64,
                        "i32" | "Option<i32>" => teaql_core::DataType::I64,
                        "bool" | "Option<bool>" => teaql_core::DataType::Bool,
                        "chrono::NaiveDate" | "Option<chrono::NaiveDate>" => teaql_core::DataType::Date,
                        "chrono::DateTime<chrono::Utc>" | "Option<chrono::DateTime<chrono::Utc>>" => teaql_core::DataType::Timestamp,
                        _ => teaql_core::DataType::Text,
                    },
                    nullable: true,
                    is_id: false,
                    is_version: false,
                },
                teaql_core::PropertyDescriptor {
                    name: "status_id".to_string(),
                    column_name: "status".to_string(),
                    data_type: teaql_core::DataType::U64,
                    nullable: false, // relations usually not null in robot-kanban
                    is_id: false,
                    is_version: false,
                },
                teaql_core::PropertyDescriptor {
                    name: "platform_id".to_string(),
                    column_name: "platform".to_string(),
                    data_type: teaql_core::DataType::U64,
                    nullable: false, // relations usually not null in robot-kanban
                    is_id: false,
                    is_version: false,
                },
            ],
            relations: vec![
                teaql_core::RelationDescriptor {
                    name: "task_execution_log_list".to_string(),
                    target_entity: "task_execution_log".to_string(),
                    local_key: "id".to_string(),
                    foreign_key: "task_id".to_string(),
                    many: true,
                    attach: true,
                    delete_missing: true,
                },
            ],
        }
    }
}

impl teaql_core::Entity for Task {
    fn from_record(mut record: std::collections::BTreeMap<String, teaql_core::Value>) -> Result<Self, teaql_core::EntityError> {
        let mut entity = Self::new();
        if let Some(val) = record.remove("id") {
            if let teaql_core::Value::U64(v) = val { entity.id = v; }
            else if let teaql_core::Value::I64(v) = val { entity.id = v as u64; }
        }
        if let Some(val) = record.remove("version") {
            if let teaql_core::Value::I64(v) = val { entity.version = v; }
        }
        if let Some(val) = record.remove("status_id").or_else(|| record.remove("status")) {
            if let teaql_core::Value::U64(v) = val { entity.status_id = Some(v); }
            else if let teaql_core::Value::I64(v) = val { entity.status_id = Some(v as u64); }
        }
        if let Some(val) = record.remove("platform_id").or_else(|| record.remove("platform")) {
            if let teaql_core::Value::U64(v) = val { entity.platform_id = Some(v); }
            else if let teaql_core::Value::I64(v) = val { entity.platform_id = Some(v as u64); }
        }
        if let Some(val) = record.remove("name") {
            if let teaql_core::Value::Text(v) = val { entity.name = Some(v); }
        }
        Ok(entity)
    }

    fn into_record(self) -> std::collections::BTreeMap<String, teaql_core::Value> {
        let mut record = std::collections::BTreeMap::new();
        record.insert("id".to_string(), teaql_core::Value::U64(self.id));
        record.insert("version".to_string(), teaql_core::Value::I64(self.version));
        if let Some(v) = self.status_id {
            record.insert("status_id".to_string(), teaql_core::Value::U64(v));
        }
        if let Some(v) = self.platform_id {
            record.insert("platform_id".to_string(), teaql_core::Value::U64(v));
        }
        if let Some(v) = self.name { record.insert("name".to_string(), teaql_core::Value::Text(v)); }
        record
    }
}
