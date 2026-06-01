pub mod entities;
pub mod q;
pub mod runtime;

pub use entities::*;
pub use q::*;
pub use runtime::*;

pub struct ServiceRuntimeExecutor;
impl ServiceRuntimeExecutor {
    pub fn new<T>(_t: T) -> Self { Self }
}

pub fn module_with_behaviors_and_checkers() -> DummyModule { DummyModule }
pub struct DummyModule;
impl DummyModule {
    pub fn into_context(self) -> teaql_runtime::UserContext { teaql_runtime::UserContext::new() }
}

