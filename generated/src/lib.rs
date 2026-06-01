pub mod entities;
pub mod q;
pub mod runtime;

pub use entities::*;
pub use q::*;
pub use runtime::*;

pub struct ServiceRuntimeExecutor {
    pub inner: teaql_provider_rusqlite::RusqliteMutationExecutor,
}
impl ServiceRuntimeExecutor {
    pub fn new(inner: teaql_provider_rusqlite::RusqliteMutationExecutor) -> Self { Self { inner } }
}

impl teaql_runtime::QueryExecutor for ServiceRuntimeExecutor {
    type Error = teaql_provider_rusqlite::MutationExecutorError;

    fn fetch_all(&self, query: &teaql_sql::CompiledQuery) -> Result<Vec<teaql_core::Record>, Self::Error> {
        self.inner.fetch_all(query)
    }

    fn execute(&self, query: &teaql_sql::CompiledQuery) -> Result<u64, Self::Error> {
        self.inner.execute(query)
    }

    fn begin_transaction(&self) -> Result<teaql_runtime::GraphTransactionBoundary, Self::Error> {
        teaql_runtime::QueryExecutor::begin_transaction(&self.inner)
    }

    fn commit_transaction(&self) -> Result<(), Self::Error> {
        teaql_runtime::QueryExecutor::commit_transaction(&self.inner)
    }

    fn rollback_transaction(&self) -> Result<(), Self::Error> {
        teaql_runtime::QueryExecutor::rollback_transaction(&self.inner)
    }
}

pub fn module_with_behaviors_and_checkers() -> teaql_runtime::RuntimeModule {
    let mut module = teaql_runtime::RuntimeModule::new();
    
    module = module.entity::<Platform>();
    
    module = module.entity::<TaskStatus>();
    
    module = module.entity::<Task>();
    
    module = module.entity::<TaskExecutionLog>();
    
    module
        .initial_graph(teaql_runtime::GraphNode::new("platform")
            .value("id", teaql_core::Value::U64(1))
            .value("name", teaql_core::Value::Text("Robot System".to_string()))
            .value("version", teaql_core::Value::I64(1))
            .value("deleted", teaql_core::Value::Bool(false))
        )
        .initial_graph(teaql_runtime::GraphNode::new("task_status")
            .value("id", teaql_core::Value::U64(1001))
            .value("code", teaql_core::Value::Text("PLANNED".to_string()))
            .value("name", teaql_core::Value::Text("Planned".to_string()))
            .value("version", teaql_core::Value::I64(1))
            .value("deleted", teaql_core::Value::Bool(false))
        )
        .initial_graph(teaql_runtime::GraphNode::new("task_status")
            .value("id", teaql_core::Value::U64(1002))
            .value("code", teaql_core::Value::Text("READY".to_string()))
            .value("name", teaql_core::Value::Text("Ready".to_string()))
            .value("version", teaql_core::Value::I64(1))
            .value("deleted", teaql_core::Value::Bool(false))
        )
        .initial_graph(teaql_runtime::GraphNode::new("task_status")
            .value("id", teaql_core::Value::U64(1003))
            .value("code", teaql_core::Value::Text("EXECUTING".to_string()))
            .value("name", teaql_core::Value::Text("Executing".to_string()))
            .value("version", teaql_core::Value::I64(1))
            .value("deleted", teaql_core::Value::Bool(false))
        )
        .initial_graph(teaql_runtime::GraphNode::new("task_status")
            .value("id", teaql_core::Value::U64(1004))
            .value("code", teaql_core::Value::Text("VERIFIED".to_string()))
            .value("name", teaql_core::Value::Text("Verified".to_string()))
            .value("version", teaql_core::Value::I64(1))
            .value("deleted", teaql_core::Value::Bool(false))
        )
}

