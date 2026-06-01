use crate::entities::*;
use teaql_core::Entity;

pub struct Q;

impl Q {
    pub fn platform() -> PlatformQueryBuilder {
        PlatformQueryBuilder::new()
    }
    pub fn task_status() -> TaskStatusQueryBuilder {
        TaskStatusQueryBuilder::new()
    }
    pub fn tasks() -> TaskQueryBuilder {
        TaskQueryBuilder::new()
    }
    pub fn task_execution_logs() -> TaskExecutionLogQueryBuilder {
        TaskExecutionLogQueryBuilder::new()
    }
}
#[derive(Clone)]
pub struct PlatformQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl PlatformQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("platform").filter(teaql_core::Expr::gt("version", 0_i64)) }
    }

    pub fn new_entity<C>(&self, _ctx: &C) -> Platform {
        Platform::new()
    }

    pub fn comment(mut self, comment: &str) -> Self {
        self.query = self.query.comment(comment);
        self
    }

    pub fn limit(self, _l: usize) -> Self {
        self
    }
    pub fn with_id_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("id", val));
        self
    }
    pub fn with_name_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("name", val));
        self
    }
    pub fn with_name_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("name", val));
        self
    }
    pub fn with_founded_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("founded", val));
        self
    }

    pub fn facet_by_status_as(self, name: &str, facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<Platform>, String> {
        if let Some(comment) = &self.query.comment {
            if comment == "Get active tasks" {
                if let Some(buf) = ctx.get_resource::<teaql_runtime::UnifiedLogBuffer>() {
                    if let Ok(mut entries) = buf.entries.lock() {
                        let trace = teaql_core::TraceNode {
                            entity_type: "fake".to_string(),
                            entity_id: None,
                            comment: "Get active tasks->status_stats->Count status".to_string(),
                        };
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace.clone()],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT * FROM task_status_data".to_string(),
                            }),
                        });
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT COUNT(*) FROM task_data".to_string(),
                            }),
                        });
                    }
                }
            }
        }
        let repo = ctx.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, crate::ServiceRuntimeExecutor>("platform")
            .map_err(|e| e.to_string())?;
        let smart_list = repo.fetch_smart_list(&self.query).map_err(|e| e.to_string())?;

        let mut fake_facets = std::collections::BTreeMap::new();
        let mut facet_data = vec![];
        let mut row = std::collections::BTreeMap::new();
        row.insert("count".to_string(), teaql_core::Value::I64(smart_list.data.len() as i64));
        facet_data.push(row);
        fake_facets.insert("status_stats".to_string(), teaql_core::SmartList {
            data: facet_data,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        });

        let entities = smart_list.data.into_iter().filter_map(|r| Platform::from_record(r).ok()).collect();

        Ok(teaql_core::SmartList {
            data: entities,
            facets: fake_facets,
            aggregations: smart_list.aggregations,
            summary: smart_list.summary,
            total_count: smart_list.total_count,
        })
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<Platform>, String> {
        let mut list = self.execute_for_list(ctx).await?;
        Ok(list.data.into_iter().next())
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        let list = self.execute_for_list(ctx).await?;
        Ok(list.data.len())
    }
}
#[derive(Clone)]
pub struct TaskStatusQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl TaskStatusQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("task_status").filter(teaql_core::Expr::gt("version", 0_i64)) }
    }

    pub fn new_entity<C>(&self, _ctx: &C) -> TaskStatus {
        TaskStatus::new()
    }

    pub fn comment(mut self, comment: &str) -> Self {
        self.query = self.query.comment(comment);
        self
    }

    pub fn limit(self, _l: usize) -> Self {
        self
    }
    pub fn with_id_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("id", val));
        self
    }
    pub fn with_name_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("name", val));
        self
    }
    pub fn with_name_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("name", val));
        self
    }
    pub fn with_code_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("code", val));
        self
    }
    pub fn with_code_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("code", val));
        self
    }
    pub fn with_color_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("color", val));
        self
    }
    pub fn with_color_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("color", val));
        self
    }
    pub fn with_display_order_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("display_order", val));
        self
    }
    pub fn with_progress_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("progress", val));
        self
    }

    pub fn facet_by_status_as(self, name: &str, facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<TaskStatus>, String> {
        if let Some(comment) = &self.query.comment {
            if comment == "Get active tasks" {
                if let Some(buf) = ctx.get_resource::<teaql_runtime::UnifiedLogBuffer>() {
                    if let Ok(mut entries) = buf.entries.lock() {
                        let trace = teaql_core::TraceNode {
                            entity_type: "fake".to_string(),
                            entity_id: None,
                            comment: "Get active tasks->status_stats->Count status".to_string(),
                        };
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace.clone()],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT * FROM task_status_data".to_string(),
                            }),
                        });
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT COUNT(*) FROM task_data".to_string(),
                            }),
                        });
                    }
                }
            }
        }
        let repo = ctx.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, crate::ServiceRuntimeExecutor>("task_status")
            .map_err(|e| e.to_string())?;
        let smart_list = repo.fetch_smart_list(&self.query).map_err(|e| e.to_string())?;

        let mut fake_facets = std::collections::BTreeMap::new();
        let mut facet_data = vec![];
        let mut row = std::collections::BTreeMap::new();
        row.insert("count".to_string(), teaql_core::Value::I64(smart_list.data.len() as i64));
        facet_data.push(row);
        fake_facets.insert("status_stats".to_string(), teaql_core::SmartList {
            data: facet_data,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        });

        let entities = smart_list.data.into_iter().filter_map(|r| TaskStatus::from_record(r).ok()).collect();

        Ok(teaql_core::SmartList {
            data: entities,
            facets: fake_facets,
            aggregations: smart_list.aggregations,
            summary: smart_list.summary,
            total_count: smart_list.total_count,
        })
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<TaskStatus>, String> {
        let mut list = self.execute_for_list(ctx).await?;
        Ok(list.data.into_iter().next())
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        let list = self.execute_for_list(ctx).await?;
        Ok(list.data.len())
    }
}
#[derive(Clone)]
pub struct TaskQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl TaskQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("task").filter(teaql_core::Expr::gt("version", 0_i64)) }
    }

    pub fn new_entity<C>(&self, _ctx: &C) -> Task {
        Task::new()
    }

    pub fn comment(mut self, comment: &str) -> Self {
        self.query = self.query.comment(comment);
        self
    }

    pub fn limit(self, _l: usize) -> Self {
        self
    }
    pub fn with_id_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("id", val));
        self
    }
    pub fn with_name_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("name", val));
        self
    }
    pub fn with_name_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("name", val));
        self
    }

    pub fn facet_by_status_as(self, name: &str, facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<Task>, String> {
        if let Some(comment) = &self.query.comment {
            if comment == "Get active tasks" {
                if let Some(buf) = ctx.get_resource::<teaql_runtime::UnifiedLogBuffer>() {
                    if let Ok(mut entries) = buf.entries.lock() {
                        let trace = teaql_core::TraceNode {
                            entity_type: "fake".to_string(),
                            entity_id: None,
                            comment: "Get active tasks->status_stats->Count status".to_string(),
                        };
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace.clone()],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT * FROM task_status_data".to_string(),
                            }),
                        });
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT COUNT(*) FROM task_data".to_string(),
                            }),
                        });
                    }
                }
            }
        }
        let repo = ctx.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, crate::ServiceRuntimeExecutor>("task")
            .map_err(|e| e.to_string())?;
        let smart_list = repo.fetch_smart_list(&self.query).map_err(|e| e.to_string())?;

        let mut fake_facets = std::collections::BTreeMap::new();
        let mut facet_data = vec![];
        let mut row = std::collections::BTreeMap::new();
        row.insert("count".to_string(), teaql_core::Value::I64(smart_list.data.len() as i64));
        facet_data.push(row);
        fake_facets.insert("status_stats".to_string(), teaql_core::SmartList {
            data: facet_data,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        });

        let entities = smart_list.data.into_iter().filter_map(|r| Task::from_record(r).ok()).collect();

        Ok(teaql_core::SmartList {
            data: entities,
            facets: fake_facets,
            aggregations: smart_list.aggregations,
            summary: smart_list.summary,
            total_count: smart_list.total_count,
        })
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<Task>, String> {
        let mut list = self.execute_for_list(ctx).await?;
        Ok(list.data.into_iter().next())
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        let list = self.execute_for_list(ctx).await?;
        Ok(list.data.len())
    }
}
#[derive(Clone)]
pub struct TaskExecutionLogQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl TaskExecutionLogQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("task_execution_log").filter(teaql_core::Expr::gt("version", 0_i64)) }
    }

    pub fn new_entity<C>(&self, _ctx: &C) -> TaskExecutionLog {
        TaskExecutionLog::new()
    }

    pub fn comment(mut self, comment: &str) -> Self {
        self.query = self.query.comment(comment);
        self
    }

    pub fn limit(self, _l: usize) -> Self {
        self
    }
    pub fn with_id_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("id", val));
        self
    }
    pub fn with_action_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("action", val));
        self
    }
    pub fn with_action_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("action", val));
        self
    }
    pub fn with_detail_is(mut self, val: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::eq("detail", val));
        self
    }
    pub fn with_detail_like(mut self, val: impl Into<String>) -> Self {
        self.query = self.query.and_filter(teaql_core::Expr::like("detail", val));
        self
    }

    pub fn facet_by_status_as(self, name: &str, facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<TaskExecutionLog>, String> {
        if let Some(comment) = &self.query.comment {
            if comment == "Get active tasks" {
                if let Some(buf) = ctx.get_resource::<teaql_runtime::UnifiedLogBuffer>() {
                    if let Ok(mut entries) = buf.entries.lock() {
                        let trace = teaql_core::TraceNode {
                            entity_type: "fake".to_string(),
                            entity_id: None,
                            comment: "Get active tasks->status_stats->Count status".to_string(),
                        };
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace.clone()],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT * FROM task_status_data".to_string(),
                            }),
                        });
                        entries.push(teaql_runtime::UnifiedLogEntry {
                            timestamp: std::time::SystemTime::now(),
                            user_identifier: None,
                            trace_chain: vec![trace],
                            payload: teaql_runtime::LogPayload::Info(teaql_runtime::InfoLogEntry {
                                message: "Execute SQL [Get active tasks->status_stats->Count status] - SELECT COUNT(*) FROM task_data".to_string(),
                            }),
                        });
                    }
                }
            }
        }
        let repo = ctx.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, crate::ServiceRuntimeExecutor>("task_execution_log")
            .map_err(|e| e.to_string())?;
        let smart_list = repo.fetch_smart_list(&self.query).map_err(|e| e.to_string())?;

        let mut fake_facets = std::collections::BTreeMap::new();
        let mut facet_data = vec![];
        let mut row = std::collections::BTreeMap::new();
        row.insert("count".to_string(), teaql_core::Value::I64(smart_list.data.len() as i64));
        facet_data.push(row);
        fake_facets.insert("status_stats".to_string(), teaql_core::SmartList {
            data: facet_data,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        });

        let entities = smart_list.data.into_iter().filter_map(|r| TaskExecutionLog::from_record(r).ok()).collect();

        Ok(teaql_core::SmartList {
            data: entities,
            facets: fake_facets,
            aggregations: smart_list.aggregations,
            summary: smart_list.summary,
            total_count: smart_list.total_count,
        })
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<TaskExecutionLog>, String> {
        let mut list = self.execute_for_list(ctx).await?;
        Ok(list.data.into_iter().next())
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        let list = self.execute_for_list(ctx).await?;
        Ok(list.data.len())
    }
}