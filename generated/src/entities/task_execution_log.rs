use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionLog {
    pub id: u64,
    pub action: Option<String>,
    pub detail: Option<String>,
    pub task: Option<Box<crate::entities::task::Task>>,
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
            task: None,
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

    pub fn save(mut self, ctx: &teaql_runtime::UserContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<teaql_runtime::GraphNode, std::io::Error>> + Send + '_>> {
        Box::pin(async move {
            let repo = ctx.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, crate::ServiceRuntimeExecutor>("task_execution_log")
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            let mut node = teaql_runtime::GraphNode::new("task_execution_log");
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
            
            let values = teaql_core::Entity::into_record(self);
            node.values = values;
            repo.save_graph(node).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })
    }
}

impl teaql_core::TeaqlEntity for TaskExecutionLog {
    fn entity_descriptor() -> teaql_core::EntityDescriptor {
        teaql_core::EntityDescriptor { 
            name: "task_execution_log".to_string(),
            table_name: "task_execution_log_data".to_string(),
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
                    name: "action".to_string(),
                    column_name: "action".to_string(),
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
                    name: "detail".to_string(),
                    column_name: "detail".to_string(),
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
                    name: "task_id".to_string(),
                    column_name: "task".to_string(),
                    data_type: teaql_core::DataType::U64,
                    nullable: false, // relations usually not null in robot-kanban
                    is_id: false,
                    is_version: false,
                },
            ],
            relations: vec![
                teaql_core::RelationDescriptor {
                    name: "task".to_string(),
                    target_entity: "task".to_string(),
                    local_key: "task_id".to_string(),
                    foreign_key: "id".to_string(),
                    many: false,
                    attach: false,
                    delete_missing: false,
                },
            ],
        }
    }
}

impl teaql_core::Entity for TaskExecutionLog {
    fn from_record(mut record: std::collections::BTreeMap<String, teaql_core::Value>) -> Result<Self, teaql_core::EntityError> {
        let mut entity = Self::new();
        if let Some(val) = record.remove("id") {
            if let teaql_core::Value::U64(v) = val { entity.id = v; }
            else if let teaql_core::Value::I64(v) = val { entity.id = v as u64; }
        }
        if let Some(val) = record.remove("version") {
            if let teaql_core::Value::I64(v) = val { entity.version = v; }
        }
        if let Some(val) = record.remove("task_id").or_else(|| record.remove("task")) {
            if let teaql_core::Value::U64(v) = val { entity.task_id = Some(v); }
            else if let teaql_core::Value::I64(v) = val { entity.task_id = Some(v as u64); }
        }
        if let Some(val) = record.remove("action") {
            if let teaql_core::Value::Text(v) = val { entity.action = Some(v); }
        }
        if let Some(val) = record.remove("detail") {
            if let teaql_core::Value::Text(v) = val { entity.detail = Some(v); }
        }
        Ok(entity)
    }

    fn into_record(self) -> std::collections::BTreeMap<String, teaql_core::Value> {
        let mut record = std::collections::BTreeMap::new();
        record.insert("id".to_string(), teaql_core::Value::U64(self.id));
        record.insert("version".to_string(), teaql_core::Value::I64(self.version));
        if let Some(v) = self.task_id {
            record.insert("task_id".to_string(), teaql_core::Value::U64(v));
        }
        if let Some(v) = self.action { record.insert("action".to_string(), teaql_core::Value::Text(v)); }
        if let Some(v) = self.detail { record.insert("detail".to_string(), teaql_core::Value::Text(v)); }
        record
    }
}
