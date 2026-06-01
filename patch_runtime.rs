pub mod runtime {
    use crate::*;
    use teaql_provider_rusqlite::RusqliteProviderExt as _;
    pub type DataServiceDialect = teaql_provider_rusqlite::RusqliteDialect;
    pub type DataServiceMutationExecutor =
        teaql_provider_rusqlite::RusqliteMutationExecutor;
    pub type DataServiceMutationError =
        teaql_provider_rusqlite::MutationExecutorError;
    pub type DataServiceIdGenerator =
        teaql_provider_rusqlite::RusqliteIdSpaceGenerator;
    pub type DataServicePool = rusqlite::Connection;
    pub type DataServiceExecutor = ServiceRuntimeExecutor;
    pub type ServiceRuntime = teaql_runtime::UserContext;
    pub const DATABASE_URL_ENV: &str = "ROBOT_KANBAN_SERVICE_DATABASE_URL";
    pub const DATABASE_USER_ENV: &str = "ROBOT_KANBAN_SERVICE_DATABASE_USER";
    pub const DATABASE_PASSWORD_ENV: &str =
        "ROBOT_KANBAN_SERVICE_DATABASE_PASSWORD";
    pub struct ServiceRuntimeConfig {
        pub database_url: String,
        pub database_user: String,
        pub database_password: String,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ServiceRuntimeConfig {
        #[inline]
        fn clone(&self) -> ServiceRuntimeConfig {
            ServiceRuntimeConfig {
                database_url: ::core::clone::Clone::clone(&self.database_url),
                database_user: ::core::clone::Clone::clone(&self.database_user),
                database_password: ::core::clone::Clone::clone(&self.database_password),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ServiceRuntimeConfig {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(f,
                "ServiceRuntimeConfig", "database_url", &self.database_url,
                "database_user", &self.database_user, "database_password",
                &&self.database_password)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for ServiceRuntimeConfig { }
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ServiceRuntimeConfig {
        #[inline]
        fn eq(&self, other: &ServiceRuntimeConfig) -> bool {
            self.database_url == other.database_url &&
                    self.database_user == other.database_user &&
                self.database_password == other.database_password
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for ServiceRuntimeConfig {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_fields_are_eq(&self) {
            let _: ::core::cmp::AssertParamIsEq<String>;
        }
    }
    impl ServiceRuntimeConfig {
        pub fn from_env() -> Result<Self, ServiceRuntimeError> {
            Ok(Self {
                    database_url: env_value(DATABASE_URL_ENV)?,
                    database_user: env_value(DATABASE_USER_ENV)?,
                    database_password: env_value(DATABASE_PASSWORD_ENV)?,
                })
        }
    }
    pub enum ServiceRuntimeError {
        MissingEnv {
            name: &'static str,
            source: std::env::VarError,
        },
        Runtime(teaql_runtime::RuntimeError),
        Rusqlite(rusqlite::Error),
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for ServiceRuntimeError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                ServiceRuntimeError::MissingEnv {
                    name: __self_0, source: __self_1 } =>
                    ::core::fmt::Formatter::debug_struct_field2_finish(f,
                        "MissingEnv", "name", __self_0, "source", &__self_1),
                ServiceRuntimeError::Runtime(__self_0) =>
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f,
                        "Runtime", &__self_0),
                ServiceRuntimeError::Rusqlite(__self_0) =>
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f,
                        "Rusqlite", &__self_0),
            }
        }
    }
    impl std::fmt::Display for ServiceRuntimeError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ServiceRuntimeError::MissingEnv { name, source } => {
                    f.write_fmt(format_args!("missing environment variable {0}: {1}",
                            name, source))
                }
                ServiceRuntimeError::Runtime(err) =>
                    f.write_fmt(format_args!("runtime error: {0}", err)),
                ServiceRuntimeError::Rusqlite(err) =>
                    f.write_fmt(format_args!("rusqlite error: {0}", err)),
            }
        }
    }
    impl std::error::Error for ServiceRuntimeError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ServiceRuntimeError::MissingEnv { source, .. } =>
                    Some(source),
                ServiceRuntimeError::Runtime(err) => Some(err),
                ServiceRuntimeError::Rusqlite(err) => Some(err),
            }
        }
    }
    impl From<rusqlite::Error> for ServiceRuntimeError {
        fn from(err: rusqlite::Error) -> Self {
            ServiceRuntimeError::Rusqlite(err)
        }
    }
    impl From<teaql_runtime::RuntimeError> for ServiceRuntimeError {
        fn from(err: teaql_runtime::RuntimeError) -> Self {
            ServiceRuntimeError::Runtime(err)
        }
    }
    pub struct ServiceRuntimeExecutor {
        inner: DataServiceMutationExecutor,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ServiceRuntimeExecutor {
        #[inline]
        fn clone(&self) -> ServiceRuntimeExecutor {
            ServiceRuntimeExecutor {
                inner: ::core::clone::Clone::clone(&self.inner),
            }
        }
    }
    impl ServiceRuntimeExecutor {
        pub fn new(inner: DataServiceMutationExecutor) -> Self {
            Self { inner }
        }
        pub fn inner(&self) -> &DataServiceMutationExecutor { &self.inner }
    }
    impl teaql_runtime::QueryExecutor for ServiceRuntimeExecutor {
        type Error = DataServiceMutationError;
        fn fetch_all(&self, query: &teaql_sql::CompiledQuery)
            -> Result<Vec<teaql_core::Record>, Self::Error> {
            let inner = self.inner.clone();
            let query = query.clone();
            inner.fetch_all(&query)
        }
        fn execute(&self, query: &teaql_sql::CompiledQuery)
            -> Result<u64, Self::Error> {
            let inner = self.inner.clone();
            let query = query.clone();
            inner.execute(&query)
        }
        fn begin_transaction(&self)
            -> Result<teaql_runtime::GraphTransactionBoundary, Self::Error> {
            teaql_runtime::QueryExecutor::begin_transaction(&self.inner)
        }
        fn commit_transaction(&self) -> Result<(), Self::Error> {
            teaql_runtime::QueryExecutor::commit_transaction(&self.inner)
        }
        fn rollback_transaction(&self) -> Result<(), Self::Error> {
            teaql_runtime::QueryExecutor::rollback_transaction(&self.inner)
        }
    }
    fn block_on_data_service<F, T>(future: F) -> T where
        F: std::future::Future<Output = T> + Send + 'static, T: Send +
        'static {
        if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::spawn(move ||
                            {
                                tokio::runtime::Builder::new_current_thread().enable_all().build().expect("data service runtime").block_on(future)
                            }).join().expect("data service runtime thread")
        } else {
            tokio::runtime::Builder::new_current_thread().enable_all().build().expect("data service runtime").block_on(future)
        }
    }
    pub async fn service_runtime_from_pool(pool: DataServicePool)
        -> Result<ServiceRuntime, ServiceRuntimeError> {
        let mutation_executor = DataServiceMutationExecutor::new(pool);
        let id_generator =
            DataServiceIdGenerator::from_executor(mutation_executor.clone());
        let runtime_executor =
            ServiceRuntimeExecutor::new(mutation_executor.clone());
        let mut context = module_with_behaviors_and_checkers().into_context();
        context.set_internal_id_generator(id_generator);
        context.use_rusqlite_provider(mutation_executor);
        context.insert_resource(runtime_executor);
        context.ensure_schema().await?;
        Ok(context)
    }
    fn env_value(name: &'static str) -> Result<String, ServiceRuntimeError> {
        std::env::var(name).map_err(|source|
                ServiceRuntimeError::MissingEnv { name, source })
    }
    pub fn repository_registry()
        -> teaql_runtime::InMemoryRepositoryRegistry {
        teaql_runtime::InMemoryRepositoryRegistry::new().with_entity("Platform").with_entity("TaskStatus").with_entity("Task").with_entity("TaskExecutionLog")
    }
    pub fn behavior_registry()
        -> teaql_runtime::InMemoryRepositoryBehaviorRegistry {
        teaql_runtime::InMemoryRepositoryBehaviorRegistry::new().with_behavior("Platform",
                        PlatformBehavior::default()).with_behavior("TaskStatus",
                    TaskStatusBehavior::default()).with_behavior("Task",
                TaskBehavior::default()).with_behavior("TaskExecutionLog",
            TaskExecutionLogBehavior::default())
    }
    pub fn checker_registry() -> teaql_runtime::InMemoryCheckerRegistry {
        teaql_runtime::InMemoryCheckerRegistry::new().with_checker(teaql_runtime::TypedEntityChecker::<Platform,
                                _>::new(PlatformChecker::default())).with_checker(teaql_runtime::TypedEntityChecker::<TaskStatus,
                            _>::new(TaskStatusChecker::default())).with_checker(teaql_runtime::TypedEntityChecker::<Task,
                        _>::new(TaskChecker::default())).with_checker(teaql_runtime::TypedEntityChecker::<TaskExecutionLog,
                    _>::new(TaskExecutionLogChecker::default()))
    }
    pub fn module() -> teaql_runtime::RuntimeModule {
        teaql_runtime::RuntimeModule::new().entity::<Platform>().entity::<TaskStatus>().entity::<Task>().entity::<TaskExecutionLog>().initial_graph(teaql_runtime::GraphNode::new("Platform").value("id",
                                            1_u64).value("name",
                                        "Robot System").value("founded",
                                    chrono::Utc::now()).value("version",
                                1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                    1_u64).value("name",
                                                "Planned").value("code",
                                            "PLANNED").value("color",
                                        "#94A3B8").value("display_order",
                                    "10").value("progress",
                                "0").value("version",
                            1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                1001_u64).value("name",
                                            "Planned").value("code",
                                        "PLANNED").value("color",
                                    "#94A3B8").value("display_order",
                                "10").value("progress",
                            "0").value("version",
                        1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                            1002_u64).value("name",
                                        "Process").value("code",
                                    "PROCESS").value("color",
                                "#F59E0B").value("display_order",
                            "20").value("progress",
                        "50").value("version",
                    1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                        1003_u64).value("name",
                                    "Done").value("code",
                                "DONE").value("color",
                            "#16A34A").value("display_order",
                        "30").value("progress", "100").value("version", 1_i64))
    }
    pub fn module_with_checkers() -> teaql_runtime::RuntimeModule {
        teaql_runtime::RuntimeModule::new().entity::<Platform>().checker(teaql_runtime::TypedEntityChecker::<Platform,
                                                                _>::new(PlatformChecker::default())).entity::<TaskStatus>().checker(teaql_runtime::TypedEntityChecker::<TaskStatus,
                                                        _>::new(TaskStatusChecker::default())).entity::<Task>().checker(teaql_runtime::TypedEntityChecker::<Task,
                                                _>::new(TaskChecker::default())).entity::<TaskExecutionLog>().checker(teaql_runtime::TypedEntityChecker::<TaskExecutionLog,
                                        _>::new(TaskExecutionLogChecker::default())).initial_graph(teaql_runtime::GraphNode::new("Platform").value("id",
                                            1_u64).value("name",
                                        "Robot System").value("founded",
                                    chrono::Utc::now()).value("version",
                                1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                    1_u64).value("name",
                                                "Planned").value("code",
                                            "PLANNED").value("color",
                                        "#94A3B8").value("display_order",
                                    "10").value("progress",
                                "0").value("version",
                            1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                1001_u64).value("name",
                                            "Planned").value("code",
                                        "PLANNED").value("color",
                                    "#94A3B8").value("display_order",
                                "10").value("progress",
                            "0").value("version",
                        1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                            1002_u64).value("name",
                                        "Process").value("code",
                                    "PROCESS").value("color",
                                "#F59E0B").value("display_order",
                            "20").value("progress",
                        "50").value("version",
                    1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                        1003_u64).value("name",
                                    "Done").value("code",
                                "DONE").value("color",
                            "#16A34A").value("display_order",
                        "30").value("progress", "100").value("version", 1_i64))
    }
    pub fn module_with_behaviors() -> teaql_runtime::RuntimeModule {
        teaql_runtime::RuntimeModule::new().entity_with_behavior::<Platform,
                                            _>(PlatformBehavior::default()).entity_with_behavior::<TaskStatus,
                                        _>(TaskStatusBehavior::default()).entity_with_behavior::<Task,
                                    _>(TaskBehavior::default()).entity_with_behavior::<TaskExecutionLog,
                                _>(TaskExecutionLogBehavior::default()).initial_graph(teaql_runtime::GraphNode::new("Platform").value("id",
                                            1_u64).value("name",
                                        "Robot System").value("founded",
                                    chrono::Utc::now()).value("version",
                                1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                    1_u64).value("name",
                                                "Planned").value("code",
                                            "PLANNED").value("color",
                                        "#94A3B8").value("display_order",
                                    "10").value("progress",
                                "0").value("version",
                            1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                1001_u64).value("name",
                                            "Planned").value("code",
                                        "PLANNED").value("color",
                                    "#94A3B8").value("display_order",
                                "10").value("progress",
                            "0").value("version",
                        1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                            1002_u64).value("name",
                                        "Process").value("code",
                                    "PROCESS").value("color",
                                "#F59E0B").value("display_order",
                            "20").value("progress",
                        "50").value("version",
                    1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                        1003_u64).value("name",
                                    "Done").value("code",
                                "DONE").value("color",
                            "#16A34A").value("display_order",
                        "30").value("progress", "100").value("version", 1_i64))
    }
    pub fn module_with_behaviors_and_checkers()
        -> teaql_runtime::RuntimeModule {
        teaql_runtime::RuntimeModule::new().entity_with_behavior::<Platform,
                                                            _>(PlatformBehavior::default()).checker(teaql_runtime::TypedEntityChecker::<Platform,
                                                                _>::new(PlatformChecker::default())).entity_with_behavior::<TaskStatus,
                                                    _>(TaskStatusBehavior::default()).checker(teaql_runtime::TypedEntityChecker::<TaskStatus,
                                                        _>::new(TaskStatusChecker::default())).entity_with_behavior::<Task,
                                            _>(TaskBehavior::default()).checker(teaql_runtime::TypedEntityChecker::<Task,
                                                _>::new(TaskChecker::default())).entity_with_behavior::<TaskExecutionLog,
                                    _>(TaskExecutionLogBehavior::default()).checker(teaql_runtime::TypedEntityChecker::<TaskExecutionLog,
                                        _>::new(TaskExecutionLogChecker::default())).initial_graph(teaql_runtime::GraphNode::new("Platform").value("id",
                                            1_u64).value("name",
                                        "Robot System").value("founded",
                                    chrono::Utc::now()).value("version",
                                1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                    1_u64).value("name",
                                                "Planned").value("code",
                                            "PLANNED").value("color",
                                        "#94A3B8").value("display_order",
                                    "10").value("progress",
                                "0").value("version",
                            1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                                1001_u64).value("name",
                                            "Planned").value("code",
                                        "PLANNED").value("color",
                                    "#94A3B8").value("display_order",
                                "10").value("progress",
                            "0").value("version",
                        1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                            1002_u64).value("name",
                                        "Process").value("code",
                                    "PROCESS").value("color",
                                "#F59E0B").value("display_order",
                            "20").value("progress",
                        "50").value("version",
                    1_i64)).initial_graph(teaql_runtime::GraphNode::new("TaskStatus").value("id",
                                        1003_u64).value("name",
                                    "Done").value("code",
                                "DONE").value("color",
                            "#16A34A").value("display_order",
                        "30").value("progress", "100").value("version", 1_i64))
    }
}
