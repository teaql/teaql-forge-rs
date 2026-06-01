use crate::entities::*;

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
        Self { query: teaql_core::SelectQuery::new("platform") }
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

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, _ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<Platform>, String> {
        // FIXME: use ctx.execute_query or ctx.repository().find
        Ok(teaql_core::SmartList { data: vec![], facets: Default::default(), ..Default::default() })
    }

    pub async fn execute_for_one(self, _ctx: &teaql_runtime::UserContext) -> Result<Option<Platform>, String> {
        Ok(None)
    }

    pub async fn execute_for_count(self, _ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        Ok(0)
    }
}
#[derive(Clone)]
pub struct TaskStatusQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl TaskStatusQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("task_status") }
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

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, _ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<TaskStatus>, String> {
        // FIXME: use ctx.execute_query or ctx.repository().find
        Ok(teaql_core::SmartList { data: vec![], facets: Default::default(), ..Default::default() })
    }

    pub async fn execute_for_one(self, _ctx: &teaql_runtime::UserContext) -> Result<Option<TaskStatus>, String> {
        Ok(None)
    }

    pub async fn execute_for_count(self, _ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        Ok(0)
    }
}
#[derive(Clone)]
pub struct TaskQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl TaskQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("task") }
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

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, _ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<Task>, String> {
        // FIXME: use ctx.execute_query or ctx.repository().find
        Ok(teaql_core::SmartList { data: vec![], facets: Default::default(), ..Default::default() })
    }

    pub async fn execute_for_one(self, _ctx: &teaql_runtime::UserContext) -> Result<Option<Task>, String> {
        Ok(None)
    }

    pub async fn execute_for_count(self, _ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        Ok(0)
    }
}
#[derive(Clone)]
pub struct TaskExecutionLogQueryBuilder {
    pub query: teaql_core::SelectQuery,
}

impl TaskExecutionLogQueryBuilder {
    pub fn new() -> Self {
        Self { query: teaql_core::SelectQuery::new("task_execution_log") }
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

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    
    pub fn count_tasks(self) -> Self {
        self
    }

    pub async fn execute_for_list(self, _ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<TaskExecutionLog>, String> {
        // FIXME: use ctx.execute_query or ctx.repository().find
        Ok(teaql_core::SmartList { data: vec![], facets: Default::default(), ..Default::default() })
    }

    pub async fn execute_for_one(self, _ctx: &teaql_runtime::UserContext) -> Result<Option<TaskExecutionLog>, String> {
        Ok(None)
    }

    pub async fn execute_for_count(self, _ctx: &teaql_runtime::UserContext) -> Result<usize, String> {
        Ok(0)
    }
}