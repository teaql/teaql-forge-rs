use crate::runtime::*;
use crate::entities::*;
use teaql_core::Entity;
use teaql_core::Expr;
use std::marker::PhantomData;

pub mod request_support {
    #![allow(unused_imports)]
    use std::{collections::BTreeMap, future::Future, marker::PhantomData};

    use serde_json::Value as JsonValue;
    use teaql_core::{
        BinaryOp, Expr, Record,
        RelationAggregate as RuntimeRelationAggregate, SelectQuery, SmartList,
    };
    use teaql_runtime::{ContextError, GraphNode, QueryExecutor, RepositoryError, RuntimeError, UserContext};

    pub(crate) const COUNT_ALIAS: &str = "count";
    pub(crate) const TYPE_FIELD: &str = "internal_type";
    pub(crate) const TYPE_GROUP_FIELD: &str = "type_group";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FieldOperator {
        Equal,
        NotEqual,
        GreaterThan,
        GreaterThanOrEqual,
        LessThan,
        LessThanOrEqual,
        Between,
        In,
        NotIn,
        Contain,
        NotContain,
        BeginWith,
        NotBeginWith,
        EndWith,
        NotEndWith,
        SoundsLike,
        IsNull,
        IsNotNull,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct DateRange<T> {
        pub start: T,
        pub end: T,
    }

    impl<T> DateRange<T> {
        pub fn new(start: T, end: T) -> Self {
            Self { start, end }
        }
    }

    pub trait EntityReference {
        fn entity_id_value(self) -> teaql_core::Value;
    }

    pub trait TeaqlRecordRepository {
        type Error: std::error::Error + Send + Sync + 'static;

        fn fetch_all(&self, query: &SelectQuery) -> Result<Vec<Record>, RepositoryError<Self::Error>>;

        fn fetch_smart_list(&self, query: &SelectQuery) -> Result<SmartList<Record>, RepositoryError<Self::Error>>;

        fn fetch_smart_list_with_relation_aggregates(
            &self,
            query: &SelectQuery,
            relation_aggregates: &[RuntimeRelationAggregate],
        ) -> Result<SmartList<Record>, RepositoryError<Self::Error>>;
    }

    pub trait TeaqlEntityRepository: TeaqlRecordRepository {
        fn fetch_enhanced_entities<T>(&self, query: &SelectQuery) -> Result<SmartList<T>, RepositoryError<Self::Error>>
        where
            T: teaql_core::Entity;

        fn fetch_enhanced_entities_with_relation_aggregates<T>(
            &self,
            query: &SelectQuery,
            relation_aggregates: &[RuntimeRelationAggregate],
        ) -> Result<SmartList<T>, RepositoryError<Self::Error>>
        where
            T: teaql_core::Entity;

        fn save_entity_graph<T>(&self, entity: T) -> Result<GraphNode, RepositoryError<Self::Error>>
        where
            T: teaql_core::Entity;
    }

    impl<'a, D, E> TeaqlRecordRepository for teaql_runtime::ResolvedRepository<'a, D, E>
    where
        D: teaql_sql::SqlDialect,
        E: QueryExecutor,
    {
        type Error = E::Error;

        fn fetch_all(&self, query: &SelectQuery) -> Result<Vec<Record>, RepositoryError<Self::Error>> {
            teaql_runtime::ResolvedRepository::fetch_all(self, query)
        }

        fn fetch_smart_list(&self, query: &SelectQuery) -> Result<SmartList<Record>, RepositoryError<Self::Error>> {
            teaql_runtime::ResolvedRepository::fetch_smart_list(self, query)
        }

        fn fetch_smart_list_with_relation_aggregates(
            &self,
            query: &SelectQuery,
            relation_aggregates: &[RuntimeRelationAggregate],
        ) -> Result<SmartList<Record>, RepositoryError<Self::Error>> {
            teaql_runtime::ResolvedRepository::fetch_smart_list_with_relation_aggregates(
                self,
                query,
                relation_aggregates,
            )
        }
    }

    impl<'a, D, E> TeaqlEntityRepository for teaql_runtime::ResolvedRepository<'a, D, E>
    where
        D: teaql_sql::SqlDialect,
        E: QueryExecutor,
    {
        fn fetch_enhanced_entities<T>(&self, query: &SelectQuery) -> Result<SmartList<T>, RepositoryError<Self::Error>>
        where
            T: teaql_core::Entity,
        {
            teaql_runtime::ResolvedRepository::fetch_enhanced_entities(self, query)
        }

        fn fetch_enhanced_entities_with_relation_aggregates<T>(
            &self,
            query: &SelectQuery,
            relation_aggregates: &[RuntimeRelationAggregate],
        ) -> Result<SmartList<T>, RepositoryError<Self::Error>>
        where
            T: teaql_core::Entity,
        {
            teaql_runtime::ResolvedRepository::fetch_enhanced_entities_with_relation_aggregates(
                self,
                query,
                relation_aggregates,
            )
        }

        fn save_entity_graph<T>(&self, entity: T) -> Result<GraphNode, RepositoryError<Self::Error>>
        where
            T: teaql_core::Entity,
        {
            teaql_runtime::ResolvedRepository::save_entity_graph(self, entity)
        }
    }

    pub type TeaqlRepositoryError<R> = RepositoryError<<R as TeaqlRecordRepository>::Error>;

    pub trait TeaqlRuntime {
        fn user_context(&self) -> &UserContext;

        fn fetch_facet_smart_list(
            &self,
            entity: &str,
            query: &SelectQuery,
            relation_aggregates: &[RuntimeRelationAggregate],
            trace_context: Vec<teaql_core::TraceNode>,
        ) -> Result<SmartList<Record>, RuntimeError>;
    }

    /// Internal trait for repository access. Application code should not use this trait directly.
    #[doc(hidden)]
    pub trait TeaqlRepositoryProvider: TeaqlRuntime {
        type PlatformRepository<'a>: TeaqlEntityRepository + 'a
        where
            Self: 'a;

        fn platform_repository(&self) -> Result<Self::PlatformRepository<'_>, ContextError>;
        type TaskStatusRepository<'a>: TeaqlEntityRepository + 'a
        where
            Self: 'a;

        fn task_status_repository(&self) -> Result<Self::TaskStatusRepository<'_>, ContextError>;
        type TaskRepository<'a>: TeaqlEntityRepository + 'a
        where
            Self: 'a;

        fn task_repository(&self) -> Result<Self::TaskRepository<'_>, ContextError>;
        type TaskExecutionLogRepository<'a>: TeaqlEntityRepository + 'a
        where
            Self: 'a;

        fn task_execution_log_repository(&self) -> Result<Self::TaskExecutionLogRepository<'_>, ContextError>;
    }

    #[allow(async_fn_in_trait)]
    pub trait TeaqlUserContextExt {
        async fn commit_data(&self) -> Result<(), RepositoryError<teaql_provider_rusqlite::MutationExecutorError>>;

        async fn transaction_data<F, Fut>(&self, f: F) -> Result<(), RepositoryError<teaql_provider_rusqlite::MutationExecutorError>>
        where
            F: FnOnce() -> Fut,
            Fut: Future<Output = Result<(), RepositoryError<teaql_provider_rusqlite::MutationExecutorError>>>;
    }

    impl TeaqlUserContextExt for teaql_runtime::UserContext {
        async fn commit_data(&self) -> Result<(), RepositoryError<teaql_provider_rusqlite::MutationExecutorError>> {
            self.commit_changes::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>()
        }

        async fn transaction_data<F, Fut>(&self, f: F) -> Result<(), RepositoryError<teaql_provider_rusqlite::MutationExecutorError>>
        where
            F: FnOnce() -> Fut,
            Fut: Future<Output = Result<(), RepositoryError<teaql_provider_rusqlite::MutationExecutorError>>>,
        {
            let executor = self.require_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().map_err(|err| {
                RepositoryError::Runtime(RuntimeError::Graph(format!(
                    "cannot start transaction without executor: {err}"
                )))
            })?;
            let root = self.entity_root();

            executor.begin_transaction().map_err(RepositoryError::Executor)?;
            root.push_change_set();

            let result = f().await;
            match result {
                Ok(()) => {
                    root.pop_change_set();
                    executor.commit_transaction().map_err(RepositoryError::Executor)?;
                    Ok(())
                }
                Err(err) => {
                    root.pop_change_set();
                    executor.rollback_transaction().map_err(RepositoryError::Executor)?;
                    Err(err)
                }
            }
        }
    }

    impl TeaqlRuntime for teaql_runtime::UserContext {
        fn user_context(&self) -> &UserContext {
            self
        }

        fn fetch_facet_smart_list(
            &self,
            entity: &str,
            query: &SelectQuery,
            relation_aggregates: &[RuntimeRelationAggregate],
            trace_context: Vec<teaql_core::TraceNode>,
        ) -> Result<SmartList<Record>, RuntimeError> {
            self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>(entity.to_owned())
                .map_err(|err| RuntimeError::Graph(err.to_string()))?
                .with_trace_context(trace_context)
                .fetch_smart_list_with_relation_aggregates(query, relation_aggregates)
                .map_err(|err| RuntimeError::Graph(err.to_string()))
        }
    }

    impl TeaqlRepositoryProvider for teaql_runtime::UserContext {
    type PlatformRepository<'a> = teaql_runtime::ResolvedRepository<'a, teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor> where Self: 'a;

    fn platform_repository(&self) -> Result<Self::PlatformRepository<'_>, teaql_runtime::ContextError> {
        self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>("platform")
    }
    type TaskStatusRepository<'a> = teaql_runtime::ResolvedRepository<'a, teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor> where Self: 'a;

    fn task_status_repository(&self) -> Result<Self::TaskStatusRepository<'_>, teaql_runtime::ContextError> {
        self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>("task_status")
    }
    type TaskRepository<'a> = teaql_runtime::ResolvedRepository<'a, teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor> where Self: 'a;

    fn task_repository(&self) -> Result<Self::TaskRepository<'_>, teaql_runtime::ContextError> {
        self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>("task")
    }
    type TaskExecutionLogRepository<'a> = teaql_runtime::ResolvedRepository<'a, teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor> where Self: 'a;

    fn task_execution_log_repository(&self) -> Result<Self::TaskExecutionLogRepository<'_>, teaql_runtime::ContextError> {
        self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>("task_execution_log")
    }
}

#[derive(Clone, Debug, PartialEq)]
    pub struct QuerySelection {
        pub query: SelectQuery,
        pub relation_selections: Vec<RelationSelection>,
        pub relation_filters: Vec<RelationFilter>,
        pub child_enhancements: Vec<QuerySelection>,
        pub query_options: QueryOptions,
    }

    impl QuerySelection {
        pub fn new(query: impl Into<SelectQuery>) -> Self {
            Self {
                query: query.into(),
                relation_selections: Vec::new(),
                relation_filters: Vec::new(),
                child_enhancements: Vec::new(),
                query_options: QueryOptions::default(),
            }
        }

        pub fn into_query(self) -> SelectQuery {
            let query = apply_relation_selections(self.query, self.relation_selections);
            apply_runtime_metadata(query, &self.query_options, &self.child_enhancements)
        }
    }

    impl From<SelectQuery> for QuerySelection {
        fn from(query: SelectQuery) -> Self {
            QuerySelection::new(query)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct RelationSelection {
        pub name: String,
        pub query: SelectQuery,
        pub relation_selections: Vec<RelationSelection>,
        pub relation_filters: Vec<RelationFilter>,
        pub child_enhancements: Vec<QuerySelection>,
        pub query_options: QueryOptions,
    }

    impl RelationSelection {
        pub fn new(name: impl Into<String>, selection: impl Into<QuerySelection>) -> Self {
            let selection = selection.into();
            Self {
                name: name.into(),
                query: selection.query,
                relation_selections: selection.relation_selections,
                relation_filters: selection.relation_filters,
                child_enhancements: selection.child_enhancements,
                query_options: selection.query_options,
            }
        }

        pub fn into_query(self) -> SelectQuery {
            let query = apply_relation_selections(self.query, self.relation_selections);
            apply_runtime_metadata(query, &self.query_options, &self.child_enhancements)
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct RelationFilter {
        pub name: String,
        pub query: SelectQuery,
        pub relation_selections: Vec<RelationSelection>,
        pub relation_filters: Vec<RelationFilter>,
        pub child_enhancements: Vec<QuerySelection>,
        pub query_options: QueryOptions,
    }

    impl RelationFilter {
        pub fn new(name: impl Into<String>, selection: impl Into<QuerySelection>) -> Self {
            let selection = selection.into();
            Self {
                name: name.into(),
                query: selection.query,
                relation_selections: selection.relation_selections,
                relation_filters: selection.relation_filters,
                child_enhancements: selection.child_enhancements,
                query_options: selection.query_options,
            }
        }
    }

    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct QueryOptions {
        pub comment: Option<String>,
        pub raw_sql: Option<String>,
        pub raw_sql_search_criteria: Vec<String>,
        pub dynamic_properties: Vec<RawDynamicProperty>,
        pub raw_projections: Vec<RawProjection>,
        pub relation_aggregates: Vec<RelationAggregate>,
        pub object_group_bys: Vec<ObjectGroupBy>,
        pub facets: Vec<FacetRequest>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct UnsafeRawSqlSegment {
        sql: String,
    }

    impl UnsafeRawSqlSegment {
        pub fn trusted(sql: impl Into<String>) -> Self {
            Self { sql: sql.into() }
        }

        pub fn into_sql(self) -> String {
            self.sql
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RawDynamicProperty {
        pub property_name: String,
        pub raw_sql_segment: String,
    }

    impl RawDynamicProperty {
        pub fn new(property_name: impl Into<String>, raw_sql_segment: UnsafeRawSqlSegment) -> Self {
            Self {
                property_name: property_name.into(),
                raw_sql_segment: raw_sql_segment.into_sql(),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RawProjection {
        pub property_name: String,
        pub raw_sql_segment: String,
    }

    impl RawProjection {
        pub fn new(property_name: impl Into<String>, raw_sql_segment: UnsafeRawSqlSegment) -> Self {
            Self {
                property_name: property_name.into(),
                raw_sql_segment: raw_sql_segment.into_sql(),
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct RelationAggregate {
        pub relation_name: String,
        pub alias: String,
        pub query: QuerySelection,
        pub single_result: bool,
    }

    impl RelationAggregate {
        pub fn new(
            relation_name: impl Into<String>,
            alias: impl Into<String>,
            query: impl Into<QuerySelection>,
            single_result: bool,
        ) -> Self {
            Self {
                relation_name: relation_name.into(),
                alias: alias.into(),
                query: query.into(),
                single_result,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct FacetRequest {
        pub facet_name: String,
        pub relation_name: String,
        pub query: QuerySelection,
        pub include_all_facets: bool,
    }

    impl FacetRequest {
        pub fn new(
            facet_name: impl Into<String>,
            relation_name: impl Into<String>,
            query: impl Into<QuerySelection>,
            include_all_facets: bool,
        ) -> Self {
            Self {
                facet_name: facet_name.into(),
                relation_name: relation_name.into(),
                query: query.into(),
                include_all_facets,
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ObjectGroupBy {
        pub property_name: String,
        pub storage_field: String,
        pub query: QuerySelection,
    }

    impl ObjectGroupBy {
        pub fn new(
            property_name: impl Into<String>,
            storage_field: impl Into<String>,
            query: impl Into<QuerySelection>,
        ) -> Self {
            Self {
                property_name: property_name.into(),
                storage_field: storage_field.into(),
                query: query.into(),
            }
        }
    }

    pub(crate) fn apply_relation_selections(
        mut query: SelectQuery,
        relation_selections: Vec<RelationSelection>,
    ) -> SelectQuery {
        for selection in relation_selections {
            query = query.relation_query(selection.name.clone(), selection.into_query());
        }
        query
    }

    pub(crate) fn runtime_relation_aggregates(options: &QueryOptions) -> Vec<RuntimeRelationAggregate> {
        options
            .relation_aggregates
            .iter()
            .map(|aggregate| {
                RuntimeRelationAggregate::new(
                    aggregate.relation_name.clone(),
                    aggregate.alias.clone(),
                    aggregate.query.clone().into_query(),
                    aggregate.single_result,
                )
            })
            .collect()
    }

    pub(crate) fn execute_facets<C>(
        ctx: &C,
        outer_query: &SelectQuery,
        options: &QueryOptions,
    ) -> Result<BTreeMap<String, SmartList<Record>>, RuntimeError>
    where
        C: TeaqlRuntime + ?Sized,
    {
        let mut facets = BTreeMap::new();
        for facet in &options.facets {
            let mut selection = facet.query.clone();
            merge_outer_filter_into_facet_aggregates(&mut selection, outer_query);
            if !facet.include_all_facets {
                selection = restrict_facet_to_outer_query(ctx, selection, outer_query, &facet.relation_name)?;
            }
            let relation_aggregates = runtime_relation_aggregates(&selection.query_options);
            let query = apply_runtime_metadata(
                selection.query,
                &selection.query_options,
                &selection.child_enhancements,
            );
            let mut chain = outer_query.trace_chain.clone();
            chain.push(teaql_core::TraceNode { 
                entity_type: query.entity.clone(),
                entity_id: None,
                comment: facet.facet_name.clone(),
            });

            let facet_rows = ctx.fetch_facet_smart_list(&query.entity, &query, &relation_aggregates, chain)?;
            facets.insert(facet.facet_name.clone(), facet_rows);
        }
        Ok(facets)
    }

    pub(crate) fn merge_outer_filter_into_facet_aggregates(selection: &mut QuerySelection, outer_query: &SelectQuery) {
        let Some(filter) = outer_query.filter.clone() else {
            return;
        };
        for aggregate in &mut selection.query_options.relation_aggregates {
            if aggregate.query.query.entity == outer_query.entity {
                aggregate.query.query = aggregate.query.query.clone().and_filter(filter.clone());
            }
        }
    }

    pub(crate) fn restrict_facet_to_outer_query<C>(
        ctx: &C,
        mut selection: QuerySelection,
        outer_query: &SelectQuery,
        relation_name: &str,
    ) -> Result<QuerySelection, RuntimeError>
    where
        C: TeaqlRuntime + ?Sized,
    {
        let descriptor = ctx
            .user_context()
            .entity(&outer_query.entity)
            .cloned()
            .ok_or_else(|| RuntimeError::Graph(format!("missing entity: {}", outer_query.entity)))?;
        let relation = descriptor
            .relation_by_name(relation_name)
            .cloned()
            .ok_or_else(|| RuntimeError::MissingRelation {
                entity: outer_query.entity.clone(),
                relation: relation_name.to_owned(),
            })?;
        let mut subquery = outer_query.clone();
        subquery.projection.clear();
        subquery.expr_projection.clear();
        subquery.order_by.clear();
        subquery.slice = None;
        subquery.aggregates.clear();
        subquery.group_by.clear();
        subquery.relations.clear();
        selection.query = selection.query.and_filter(Expr::in_subquery(
            relation.foreign_key,
            descriptor,
            subquery,
            relation.local_key,
        ));
        Ok(selection)
    }

    pub(crate) fn attach_facets<T>(rows: &mut SmartList<T>, facets: BTreeMap<String, SmartList<Record>>) {
        for (name, facet) in facets {
            rows.add_facet(name, facet);
        }
    }

    pub(crate) fn apply_runtime_metadata(
        mut query: SelectQuery,
        options: &QueryOptions,
        child_enhancements: &[QuerySelection],
    ) -> SelectQuery {
        if let Some(c) = options.comment.clone() {
            query = query.comment(c);
        }
        query.raw_sql = options.raw_sql.clone();
        query.raw_sql_search_criteria = options.raw_sql_search_criteria.clone();
        query.dynamic_properties = options
            .dynamic_properties
            .iter()
            .map(|projection| {
                teaql_core::RawSqlProjection::new(
                    projection.property_name.clone(),
                    projection.raw_sql_segment.clone(),
                )
            })
            .collect();
        query.raw_projections = options
            .raw_projections
            .iter()
            .map(|projection| {
                teaql_core::RawSqlProjection::new(
                    projection.property_name.clone(),
                    projection.raw_sql_segment.clone(),
                )
            })
            .collect();
        query.object_group_bys = options
            .object_group_bys
            .iter()
            .map(|group_by| {
                teaql_core::ObjectGroupBy::new(
                    group_by.property_name.clone(),
                    group_by.storage_field.clone(),
                    group_by.query.clone().into_query(),
                )
            })
            .collect();
        query.child_enhancements = child_enhancements
            .iter()
            .cloned()
            .map(QuerySelection::into_query)
            .collect();
        query
    }

    pub(crate) fn field_operator_expr(
        field: &str,
        operator: FieldOperator,
        values: Vec<teaql_core::Value>,
    ) -> Expr {
        match operator {
            FieldOperator::Equal => Expr::eq(field, required_value(operator, &values, 0)),
            FieldOperator::NotEqual => Expr::ne(field, required_value(operator, &values, 0)),
            FieldOperator::GreaterThan => Expr::gt(field, required_value(operator, &values, 0)),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field, required_value(operator, &values, 0)),
            FieldOperator::LessThan => Expr::lt(field, required_value(operator, &values, 0)),
            FieldOperator::LessThanOrEqual => Expr::lte(field, required_value(operator, &values, 0)),
            FieldOperator::Between => Expr::between(
                field,
                required_value(operator, &values, 0),
                required_value(operator, &values, 1),
            ),
            FieldOperator::In => Expr::in_list(field, values),
            FieldOperator::NotIn => Expr::not_in_list(field, values),
            FieldOperator::Contain => Expr::contain(field, required_text(operator, &values, 0)),
            FieldOperator::NotContain => Expr::not_contain(field, required_text(operator, &values, 0)),
            FieldOperator::BeginWith => Expr::begin_with(field, required_text(operator, &values, 0)),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field, required_text(operator, &values, 0)),
            FieldOperator::EndWith => Expr::end_with(field, required_text(operator, &values, 0)),
            FieldOperator::NotEndWith => Expr::not_end_with(field, required_text(operator, &values, 0)),
            FieldOperator::SoundsLike => Expr::sound_like(field, required_value(operator, &values, 0)),
            FieldOperator::IsNull => Expr::is_null(field),
            FieldOperator::IsNotNull => Expr::is_not_null(field),
        }
    }

    pub(crate) fn field_operator_column_expr(field: &str, operator: FieldOperator, other_field: &str) -> Expr {
        let binary_op = match operator {
            FieldOperator::Equal => BinaryOp::Eq,
            FieldOperator::NotEqual => BinaryOp::Ne,
            FieldOperator::GreaterThan => BinaryOp::Gt,
            FieldOperator::GreaterThanOrEqual => BinaryOp::Gte,
            FieldOperator::LessThan => BinaryOp::Lt,
            FieldOperator::LessThanOrEqual => BinaryOp::Lte,
            FieldOperator::Contain => BinaryOp::Like,
            FieldOperator::NotContain => BinaryOp::NotLike,
            FieldOperator::BeginWith => BinaryOp::Like,
            FieldOperator::NotBeginWith => BinaryOp::NotLike,
            FieldOperator::EndWith => BinaryOp::Like,
            FieldOperator::NotEndWith => BinaryOp::NotLike,
            unsupported => panic!("{unsupported:?} is not supported for property-to-property filters"),
        };
        Expr::compare_columns(field, binary_op, other_field)
    }

    pub(crate) fn dynamic_json_value_to_teaql_value(value: &JsonValue) -> teaql_core::Value {
        match value {
            JsonValue::Null => teaql_core::Value::Null,
            JsonValue::Bool(value) => teaql_core::Value::Bool(*value),
            JsonValue::Number(value) => {
                if let Some(value) = value.as_i64() {
                    teaql_core::Value::I64(value)
                } else if let Some(value) = value.as_u64() {
                    teaql_core::Value::U64(value)
                } else if let Some(value) = value.as_f64() {
                    teaql_core::Value::F64(value)
                } else {
                    teaql_core::Value::Null
                }
            }
            JsonValue::String(value) => teaql_core::Value::Text(value.trim().to_owned()),
            JsonValue::Array(values) => teaql_core::Value::List(
                values
                    .iter()
                    .map(dynamic_json_value_to_teaql_value)
                    .collect(),
            ),
            JsonValue::Object(object) => object
                .get("id")
                .map(dynamic_json_value_to_teaql_value)
                .unwrap_or(teaql_core::Value::Null),
        }
    }

    pub(crate) fn dynamic_json_values(value: &JsonValue) -> Vec<teaql_core::Value> {
        match value {
            JsonValue::Array(values) => values
                .iter()
                .map(dynamic_json_value_to_teaql_value)
                .collect(),
            value => vec![dynamic_json_value_to_teaql_value(value)],
        }
    }

    pub(crate) fn dynamic_json_operator(value: &JsonValue) -> FieldOperator {
        match value {
            JsonValue::String(value) if value.eq_ignore_ascii_case("__is_null__") => FieldOperator::IsNull,
            JsonValue::String(value) if value.eq_ignore_ascii_case("__is_not_null__") => {
                FieldOperator::IsNotNull
            }
            JsonValue::String(_) => FieldOperator::Contain,
            JsonValue::Number(_) | JsonValue::Bool(_) => FieldOperator::Equal,
            JsonValue::Array(values)
                if values
                    .first()
                    .map(JsonValue::is_string)
                    .unwrap_or(false) =>
            {
                FieldOperator::In
            }
            JsonValue::Array(values)
                if values
                    .first()
                    .map(JsonValue::is_object)
                    .unwrap_or(false) =>
            {
                FieldOperator::In
            }
            JsonValue::Array(values) if values.len() == 2 => FieldOperator::Between,
            _ => FieldOperator::Equal,
        }
    }

    pub(crate) fn dynamic_json_filter_expr(field: &str, value: &JsonValue) -> Expr {
        let operator = dynamic_json_operator(value);
        field_operator_expr(field, operator, dynamic_json_values(value))
    }

    pub(crate) fn dynamic_json_u64_field(object: &serde_json::Map<String, JsonValue>, field: &str) -> Option<u64> {
        object.get(field).and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
        })
    }

    pub(crate) fn remove_default_live_filter(filter: Option<Expr>) -> Option<Expr> {
        let default_filter = Expr::gt("version", 0_i64);
        remove_filter_expr(filter?, &default_filter)
    }

    pub(crate) fn remove_filter_expr(filter: Expr, target: &Expr) -> Option<Expr> {
        if &filter == target {
            return None;
        }
        match filter {
            Expr::And(parts) => {
                let mut retained = parts
                    .into_iter()
                    .filter_map(|part| remove_filter_expr(part, target))
                    .collect::<Vec<_>>();
                match retained.len() {
                    0 => None,
                    1 => retained.pop(),
                    _ => Some(Expr::And(retained)),
                }
            }
            other => Some(other),
        }
    }

    pub(crate) fn required_value(
        operator: FieldOperator,
        values: &[teaql_core::Value],
        index: usize,
    ) -> teaql_core::Value {
        values.get(index).cloned().unwrap_or_else(|| {
            panic!("{operator:?} requires value at index {index}")
        })
    }

    pub(crate) fn required_text(operator: FieldOperator, values: &[teaql_core::Value], index: usize) -> String {
        match required_value(operator, values, index) {
            teaql_core::Value::Text(value) => value,
            value => panic!("{operator:?} requires text value, got {value:?}"),
        }
    }

    impl EntityReference for teaql_core::Value {
        fn entity_id_value(self) -> teaql_core::Value {
            self
        }
    }

    impl EntityReference for u64 {
        fn entity_id_value(self) -> teaql_core::Value {
            teaql_core::Value::U64(self)
        }
    }

}
use request_support::*;

pub struct Q;

impl Q {
    pub fn platforms() -> PlatformRequest {
        PlatformRequest::new()
    }
    pub fn platforms_minimal() -> PlatformRequest {
        PlatformRequest::new().select_self()
    }
    pub fn platforms_with_children() -> PlatformRequest {
        PlatformRequest::new().select_children()
    }
    pub fn task_status() -> TaskStatusRequest {
        TaskStatusRequest::new()
    }
    pub fn task_status_minimal() -> TaskStatusRequest {
        TaskStatusRequest::new().select_self()
    }
    pub fn task_status_with_children() -> TaskStatusRequest {
        TaskStatusRequest::new().select_children()
    }
    pub fn tasks() -> TaskRequest {
        TaskRequest::new()
    }
    pub fn tasks_minimal() -> TaskRequest {
        TaskRequest::new().select_self()
    }
    pub fn tasks_with_children() -> TaskRequest {
        TaskRequest::new().select_children()
    }
    pub fn task_execution_logs() -> TaskExecutionLogRequest {
        TaskExecutionLogRequest::new()
    }
    pub fn task_execution_logs_minimal() -> TaskExecutionLogRequest {
        TaskExecutionLogRequest::new().select_self()
    }
    pub fn task_execution_logs_with_children() -> TaskExecutionLogRequest {
        TaskExecutionLogRequest::new().select_children()
    }
}
pub struct PlatformRequest<R = crate::Platform> {
    pub query: teaql_core::SelectQuery,
    pub relation_selections: Vec<RelationSelection>,
    pub relation_filters: Vec<RelationFilter>,
    pub child_enhancements: Vec<QuerySelection>,
    pub query_options: QueryOptions,
    pub filter_id: Option<u64>,
    marker: PhantomData<R>,
}

impl<R> Into<QuerySelection> for PlatformRequest<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
        }
    }
}

impl<R> Clone for PlatformRequest<R> {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            relation_selections: self.relation_selections.clone(),
            relation_filters: self.relation_filters.clone(),
            child_enhancements: self.child_enhancements.clone(),
            query_options: self.query_options.clone(),
            filter_id: self.filter_id.clone(),
            marker: PhantomData,
        }
    }
}

impl<R> PlatformRequest<R> {
    pub(crate) fn new() -> Self {
        Self {
            query: teaql_core::SelectQuery::new("platform"),
            relation_selections: Vec::new(),
            relation_filters: Vec::new(),
            child_enhancements: Vec::new(),
            query_options: QueryOptions::default(),
            filter_id: None,
            marker: PhantomData,
        }
    }

    pub fn return_type<T>(self) -> PlatformRequest<T> {
        PlatformRequest {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
            filter_id: self.filter_id,
            marker: PhantomData,
        }
    }

    pub fn query(&self) -> &teaql_core::SelectQuery { &self.query }
    pub fn relation_selections(&self) -> &[RelationSelection] { &self.relation_selections }
    pub fn relation_filters(&self) -> &[RelationFilter] { &self.relation_filters }
    pub fn child_enhancements(&self) -> &[QuerySelection] { &self.child_enhancements }
    pub fn query_options(&self) -> &QueryOptions { &self.query_options }
    pub fn into_query(self) -> teaql_core::SelectQuery { self.query }

    pub fn new_entity(&self, _ctx: &teaql_runtime::UserContext) -> crate::Platform {
        crate::Platform::new()
    }


    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {
        self.query = self.query.count(alias);
        self
    }

    pub fn aggregate_count_field(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.count_field(field, alias);
        self
    }

    pub fn aggregate_sum(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.sum(field, alias);
        self
    }

    pub fn aggregate_avg(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.avg(field, alias);
        self
    }

    pub fn aggregate_min(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.min(field, alias);
        self
    }

    pub fn aggregate_max(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.max(field, alias);
        self
    }

    pub async fn execute_for_list<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<R>, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let repository = ctx
            .platform_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let mut query = self.query;
        query.filter = query.filter.map_or(
            Some(teaql_core::Expr::gt(
                "version",
                teaql_core::Value::I64(0),
            )),
            |f| {
                Some(teaql_core::Expr::And(vec![
                    f,
                    teaql_core::Expr::gt(
                        "version",
                        teaql_core::Value::I64(0),
                    ),
                ]))
            },
        );
        let query = request_support::apply_runtime_metadata(query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates::<R>(
            &query,
            &relation_aggregates,
        )?;
        let facets = request_support::execute_facets(ctx, &query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_first<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id<'a, C>(
        self,
        ctx: &'a C,
        id: impl Into<teaql_core::Value>,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.and_filter(teaql_core::Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<teaql_core::Record>, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .platform_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates)?;
        let facets = request_support::execute_facets(ctx, &outer_query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_record<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<teaql_core::Record>, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.data.into_iter().next())
    }

    pub async fn execute_for_count<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<u64, request_support::TeaqlRepositoryError<C::PlatformRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .platform_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let mut query = self.query.clone();
        query.projection.clear();
        query.expr_projection.clear();
        query.order_by.clear();
        query.slice = None;
        query.relations.clear();
        query = query.count("COUNT_ALIAS");
        let rows = repository.fetch_all(&query)?;
        rows.into_iter().next().and_then(|row| row.get("COUNT_ALIAS").cloned()).and_then(|v| v.try_u64()).ok_or_else(|| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph("count result is missing or not numeric".to_string())))
    }

    pub fn filter(mut self, filter: Expr) -> Self {
        self.query = self.query.filter(filter);
        self
    }
    
    pub fn and_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.and_filter(filter);
        self
    }
    
    pub fn or_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.or_filter(filter);
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.query_options.comment = Some(comment.into());
        self.query.comment = self.query_options.comment.clone();
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.query = self.query.limit(limit);
        self
    }

    pub fn skip(mut self, offset: u64) -> Self {
        self.query = self.query.offset(offset);
        self
    }

    pub fn page_offset(mut self, offset: u64, limit: u64) -> Self {
        self.query = self.query.page(offset, limit);
        self
    }

    pub fn group_by(mut self, field: impl Into<String>) -> Self {
        self.query = self.query.group_by(field);
        self
    }

    pub fn select_self(self) -> Self {
        self
    }
    
    pub fn select_self_fields(self) -> Self {
        self
    }
    
    pub fn unlimited(self) -> Self {
        self
    }
    
    pub fn enhance_children_if_needed(self) -> Self {
        self
    }
    
    pub fn select_children(self) -> Self {
        self
    }
    pub fn select_id(mut self) -> Self {
        self.query = self.query.project("id");
        self
    }
    
    pub fn group_by_id(self) -> Self { self.group_by("id") }
    

    pub fn with_id(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "id";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_id_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", min.into(), max.into()));
        self
    }

    pub fn with_id_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_id_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("id"));
        self
    }

    pub fn with_id_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("id"));
        self
    }
    
    pub fn with_id_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("id", value.into()));
        self
    }

    pub fn with_id_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("id", val.clone()));
        if "id" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_id_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("id", value));
        self
    }

    pub fn with_id_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("id", value));
        self
    }

    pub fn with_id_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("id", value));
        self
    }

    pub fn with_id_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("id", value));
        self
    }

    pub fn with_id_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("id", value));
        self
    }
    
    pub fn with_id_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_id_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_id_asc(mut self) -> Self {
        self.query = self.query.order_asc("id");
        self
    }

    pub fn order_by_id_desc(mut self) -> Self {
        self.query = self.query.order_desc("id");
        self
    }
    pub fn count_id(self) -> Self {
        self.count_id_as("id_count")
    }

    pub fn count_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("id", alias)
    }

    pub fn sum_id(self) -> Self {
        self.sum_id_as("sum_id")
    }

    pub fn sum_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("id", alias)
    }

    pub fn avg_id(self) -> Self {
        self.avg_id_as("avg_id")
    }

    pub fn avg_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("id", alias)
    }

    pub fn min_id(self) -> Self {
        self.min_id_as("min_id")
    }

    pub fn min_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("id", alias)
    }

    pub fn max_id(self) -> Self {
        self.max_id_as("max_id")
    }

    pub fn max_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("id", alias)
    }
    pub fn select_name(mut self) -> Self {
        self.query = self.query.project("name");
        self
    }
    
    pub fn group_by_name(self) -> Self { self.group_by("name") }
    

    pub fn with_name(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "name";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_name_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("name", min.into(), max.into()));
        self
    }

    pub fn with_name_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("name", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_name_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("name"));
        self
    }

    pub fn with_name_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("name"));
        self
    }
    
    pub fn with_name_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("name", value.into()));
        self
    }

    pub fn with_name_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("name", val.clone()));
        if "name" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_name_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("name", value));
        self
    }

    pub fn with_name_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("name", value));
        self
    }

    pub fn with_name_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("name", value));
        self
    }

    pub fn with_name_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("name", value));
        self
    }

    pub fn with_name_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("name", value));
        self
    }
    
    pub fn with_name_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("name", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_name_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("name", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_name_asc(mut self) -> Self {
        self.query = self.query.order_asc("name");
        self
    }

    pub fn order_by_name_desc(mut self) -> Self {
        self.query = self.query.order_desc("name");
        self
    }
    pub fn with_name_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("%{}%", value.into())));
        self
    }

    pub fn with_name_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("name", value.into()));
        self
    }
    
    pub fn with_name_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("name", value.into()));
        self
    }
    
    pub fn with_name_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("name", value.into()));
        self
    }
    
    pub fn with_name_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("name", value.into()));
        self
    }
    
    pub fn with_name_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("name", value.into()));
        self
    }

    pub fn with_name_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("{}%", value.into())));
        self
    }
    pub fn with_name_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("%{}", value.into())));
        self
    }
    pub fn count_name(self) -> Self {
        self.count_name_as("name_count")
    }

    pub fn count_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("name", alias)
    }

    pub fn sum_name(self) -> Self {
        self.sum_name_as("sum_name")
    }

    pub fn sum_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("name", alias)
    }

    pub fn avg_name(self) -> Self {
        self.avg_name_as("avg_name")
    }

    pub fn avg_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("name", alias)
    }

    pub fn min_name(self) -> Self {
        self.min_name_as("min_name")
    }

    pub fn min_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("name", alias)
    }

    pub fn max_name(self) -> Self {
        self.max_name_as("max_name")
    }

    pub fn max_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("name", alias)
    }
    pub fn select_founded(mut self) -> Self {
        self.query = self.query.project("founded");
        self
    }
    
    pub fn group_by_founded(self) -> Self { self.group_by("founded") }
    

    pub fn with_founded(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "founded";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_founded_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("founded", min.into(), max.into()));
        self
    }

    pub fn with_founded_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("founded", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_founded_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("founded"));
        self
    }

    pub fn with_founded_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("founded"));
        self
    }
    
    pub fn with_founded_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("founded", value.into()));
        self
    }

    pub fn with_founded_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("founded", val.clone()));
        if "founded" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_founded_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("founded", value));
        self
    }

    pub fn with_founded_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("founded", value));
        self
    }

    pub fn with_founded_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("founded", value));
        self
    }

    pub fn with_founded_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("founded", value));
        self
    }

    pub fn with_founded_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("founded", value));
        self
    }
    
    pub fn with_founded_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("founded", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_founded_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("founded", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_founded_asc(mut self) -> Self {
        self.query = self.query.order_asc("founded");
        self
    }

    pub fn order_by_founded_desc(mut self) -> Self {
        self.query = self.query.order_desc("founded");
        self
    }
    pub fn count_founded(self) -> Self {
        self.count_founded_as("founded_count")
    }

    pub fn count_founded_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("founded", alias)
    }

    pub fn sum_founded(self) -> Self {
        self.sum_founded_as("sum_founded")
    }

    pub fn sum_founded_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("founded", alias)
    }

    pub fn avg_founded(self) -> Self {
        self.avg_founded_as("avg_founded")
    }

    pub fn avg_founded_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("founded", alias)
    }

    pub fn min_founded(self) -> Self {
        self.min_founded_as("min_founded")
    }

    pub fn min_founded_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("founded", alias)
    }

    pub fn max_founded(self) -> Self {
        self.max_founded_as("max_founded")
    }

    pub fn max_founded_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("founded", alias)
    }
    pub fn with_task_list_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn count_tasks(self) -> Self {
        self.count_tasks_as("count_tasks")
    }

    pub fn count_tasks_as(self, alias: impl Into<String>) -> Self {
        self.count_tasks_with(alias, crate::Q::tasks().unlimited())
    }
    pub fn count_tasks_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new(
            "task_list",
            alias,
            selection,
            true,
        ));
        self
    }
}
pub struct TaskStatusRequest<R = crate::TaskStatus> {
    pub query: teaql_core::SelectQuery,
    pub relation_selections: Vec<RelationSelection>,
    pub relation_filters: Vec<RelationFilter>,
    pub child_enhancements: Vec<QuerySelection>,
    pub query_options: QueryOptions,
    pub filter_id: Option<u64>,
    marker: PhantomData<R>,
}

impl<R> Into<QuerySelection> for TaskStatusRequest<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
        }
    }
}

impl<R> Clone for TaskStatusRequest<R> {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            relation_selections: self.relation_selections.clone(),
            relation_filters: self.relation_filters.clone(),
            child_enhancements: self.child_enhancements.clone(),
            query_options: self.query_options.clone(),
            filter_id: self.filter_id.clone(),
            marker: PhantomData,
        }
    }
}

impl<R> TaskStatusRequest<R> {
    pub(crate) fn new() -> Self {
        Self {
            query: teaql_core::SelectQuery::new("task_status"),
            relation_selections: Vec::new(),
            relation_filters: Vec::new(),
            child_enhancements: Vec::new(),
            query_options: QueryOptions::default(),
            filter_id: None,
            marker: PhantomData,
        }
    }

    pub fn return_type<T>(self) -> TaskStatusRequest<T> {
        TaskStatusRequest {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
            filter_id: self.filter_id,
            marker: PhantomData,
        }
    }

    pub fn query(&self) -> &teaql_core::SelectQuery { &self.query }
    pub fn relation_selections(&self) -> &[RelationSelection] { &self.relation_selections }
    pub fn relation_filters(&self) -> &[RelationFilter] { &self.relation_filters }
    pub fn child_enhancements(&self) -> &[QuerySelection] { &self.child_enhancements }
    pub fn query_options(&self) -> &QueryOptions { &self.query_options }
    pub fn into_query(self) -> teaql_core::SelectQuery { self.query }

    pub fn new_entity(&self, _ctx: &teaql_runtime::UserContext) -> crate::TaskStatus {
        crate::TaskStatus::new()
    }


    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {
        self.query = self.query.count(alias);
        self
    }

    pub fn aggregate_count_field(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.count_field(field, alias);
        self
    }

    pub fn aggregate_sum(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.sum(field, alias);
        self
    }

    pub fn aggregate_avg(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.avg(field, alias);
        self
    }

    pub fn aggregate_min(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.min(field, alias);
        self
    }

    pub fn aggregate_max(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.max(field, alias);
        self
    }

    pub async fn execute_for_list<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<R>, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let repository = ctx
            .task_status_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let mut query = self.query;
        query.filter = query.filter.map_or(
            Some(teaql_core::Expr::gt(
                "version",
                teaql_core::Value::I64(0),
            )),
            |f| {
                Some(teaql_core::Expr::And(vec![
                    f,
                    teaql_core::Expr::gt(
                        "version",
                        teaql_core::Value::I64(0),
                    ),
                ]))
            },
        );
        let query = request_support::apply_runtime_metadata(query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates::<R>(
            &query,
            &relation_aggregates,
        )?;
        let facets = request_support::execute_facets(ctx, &query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_first<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id<'a, C>(
        self,
        ctx: &'a C,
        id: impl Into<teaql_core::Value>,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.and_filter(teaql_core::Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<teaql_core::Record>, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .task_status_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates)?;
        let facets = request_support::execute_facets(ctx, &outer_query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_record<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<teaql_core::Record>, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.data.into_iter().next())
    }

    pub async fn execute_for_count<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<u64, request_support::TeaqlRepositoryError<C::TaskStatusRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .task_status_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let mut query = self.query.clone();
        query.projection.clear();
        query.expr_projection.clear();
        query.order_by.clear();
        query.slice = None;
        query.relations.clear();
        query = query.count("COUNT_ALIAS");
        let rows = repository.fetch_all(&query)?;
        rows.into_iter().next().and_then(|row| row.get("COUNT_ALIAS").cloned()).and_then(|v| v.try_u64()).ok_or_else(|| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph("count result is missing or not numeric".to_string())))
    }

    pub fn filter(mut self, filter: Expr) -> Self {
        self.query = self.query.filter(filter);
        self
    }
    
    pub fn and_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.and_filter(filter);
        self
    }
    
    pub fn or_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.or_filter(filter);
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.query_options.comment = Some(comment.into());
        self.query.comment = self.query_options.comment.clone();
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.query = self.query.limit(limit);
        self
    }

    pub fn skip(mut self, offset: u64) -> Self {
        self.query = self.query.offset(offset);
        self
    }

    pub fn page_offset(mut self, offset: u64, limit: u64) -> Self {
        self.query = self.query.page(offset, limit);
        self
    }

    pub fn group_by(mut self, field: impl Into<String>) -> Self {
        self.query = self.query.group_by(field);
        self
    }

    pub fn select_self(self) -> Self {
        self
    }
    
    pub fn select_self_fields(self) -> Self {
        self
    }
    
    pub fn unlimited(self) -> Self {
        self
    }
    
    pub fn enhance_children_if_needed(self) -> Self {
        self
    }
    
    pub fn select_children(self) -> Self {
        self
    }
    pub fn select_id(mut self) -> Self {
        self.query = self.query.project("id");
        self
    }
    
    pub fn group_by_id(self) -> Self { self.group_by("id") }
    

    pub fn with_id(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "id";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_id_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", min.into(), max.into()));
        self
    }

    pub fn with_id_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_id_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("id"));
        self
    }

    pub fn with_id_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("id"));
        self
    }
    
    pub fn with_id_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("id", value.into()));
        self
    }

    pub fn with_id_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("id", val.clone()));
        if "id" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_id_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("id", value));
        self
    }

    pub fn with_id_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("id", value));
        self
    }

    pub fn with_id_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("id", value));
        self
    }

    pub fn with_id_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("id", value));
        self
    }

    pub fn with_id_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("id", value));
        self
    }
    
    pub fn with_id_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_id_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_id_asc(mut self) -> Self {
        self.query = self.query.order_asc("id");
        self
    }

    pub fn order_by_id_desc(mut self) -> Self {
        self.query = self.query.order_desc("id");
        self
    }
    pub fn count_id(self) -> Self {
        self.count_id_as("id_count")
    }

    pub fn count_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("id", alias)
    }

    pub fn sum_id(self) -> Self {
        self.sum_id_as("sum_id")
    }

    pub fn sum_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("id", alias)
    }

    pub fn avg_id(self) -> Self {
        self.avg_id_as("avg_id")
    }

    pub fn avg_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("id", alias)
    }

    pub fn min_id(self) -> Self {
        self.min_id_as("min_id")
    }

    pub fn min_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("id", alias)
    }

    pub fn max_id(self) -> Self {
        self.max_id_as("max_id")
    }

    pub fn max_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("id", alias)
    }
    pub fn select_name(mut self) -> Self {
        self.query = self.query.project("name");
        self
    }
    
    pub fn group_by_name(self) -> Self { self.group_by("name") }
    

    pub fn with_name(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "name";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_name_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("name", min.into(), max.into()));
        self
    }

    pub fn with_name_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("name", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_name_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("name"));
        self
    }

    pub fn with_name_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("name"));
        self
    }
    
    pub fn with_name_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("name", value.into()));
        self
    }

    pub fn with_name_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("name", val.clone()));
        if "name" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_name_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("name", value));
        self
    }

    pub fn with_name_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("name", value));
        self
    }

    pub fn with_name_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("name", value));
        self
    }

    pub fn with_name_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("name", value));
        self
    }

    pub fn with_name_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("name", value));
        self
    }
    
    pub fn with_name_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("name", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_name_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("name", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_name_asc(mut self) -> Self {
        self.query = self.query.order_asc("name");
        self
    }

    pub fn order_by_name_desc(mut self) -> Self {
        self.query = self.query.order_desc("name");
        self
    }
    pub fn with_name_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("%{}%", value.into())));
        self
    }

    pub fn with_name_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("name", value.into()));
        self
    }
    
    pub fn with_name_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("name", value.into()));
        self
    }
    
    pub fn with_name_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("name", value.into()));
        self
    }
    
    pub fn with_name_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("name", value.into()));
        self
    }
    
    pub fn with_name_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("name", value.into()));
        self
    }

    pub fn with_name_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("{}%", value.into())));
        self
    }
    pub fn with_name_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("%{}", value.into())));
        self
    }
    pub fn count_name(self) -> Self {
        self.count_name_as("name_count")
    }

    pub fn count_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("name", alias)
    }

    pub fn sum_name(self) -> Self {
        self.sum_name_as("sum_name")
    }

    pub fn sum_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("name", alias)
    }

    pub fn avg_name(self) -> Self {
        self.avg_name_as("avg_name")
    }

    pub fn avg_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("name", alias)
    }

    pub fn min_name(self) -> Self {
        self.min_name_as("min_name")
    }

    pub fn min_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("name", alias)
    }

    pub fn max_name(self) -> Self {
        self.max_name_as("max_name")
    }

    pub fn max_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("name", alias)
    }
    pub fn select_code(mut self) -> Self {
        self.query = self.query.project("code");
        self
    }
    
    pub fn group_by_code(self) -> Self { self.group_by("code") }
    

    pub fn with_code(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "code";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_code_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("code", min.into(), max.into()));
        self
    }

    pub fn with_code_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("code", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_code_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("code"));
        self
    }

    pub fn with_code_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("code"));
        self
    }
    
    pub fn with_code_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("code", value.into()));
        self
    }

    pub fn with_code_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("code", val.clone()));
        if "code" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_code_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("code", value));
        self
    }

    pub fn with_code_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("code", value));
        self
    }

    pub fn with_code_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("code", value));
        self
    }

    pub fn with_code_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("code", value));
        self
    }

    pub fn with_code_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("code", value));
        self
    }
    
    pub fn with_code_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("code", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_code_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("code", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_code_asc(mut self) -> Self {
        self.query = self.query.order_asc("code");
        self
    }

    pub fn order_by_code_desc(mut self) -> Self {
        self.query = self.query.order_desc("code");
        self
    }
    pub fn with_code_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("code", format!("%{}%", value.into())));
        self
    }

    pub fn with_code_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("code", value.into()));
        self
    }
    
    pub fn with_code_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("code", value.into()));
        self
    }
    
    pub fn with_code_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("code", value.into()));
        self
    }
    
    pub fn with_code_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("code", value.into()));
        self
    }
    
    pub fn with_code_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("code", value.into()));
        self
    }

    pub fn with_code_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("code", format!("{}%", value.into())));
        self
    }
    pub fn with_code_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("code", format!("%{}", value.into())));
        self
    }
    pub fn count_code(self) -> Self {
        self.count_code_as("code_count")
    }

    pub fn count_code_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("code", alias)
    }

    pub fn sum_code(self) -> Self {
        self.sum_code_as("sum_code")
    }

    pub fn sum_code_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("code", alias)
    }

    pub fn avg_code(self) -> Self {
        self.avg_code_as("avg_code")
    }

    pub fn avg_code_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("code", alias)
    }

    pub fn min_code(self) -> Self {
        self.min_code_as("min_code")
    }

    pub fn min_code_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("code", alias)
    }

    pub fn max_code(self) -> Self {
        self.max_code_as("max_code")
    }

    pub fn max_code_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("code", alias)
    }
    pub fn select_color(mut self) -> Self {
        self.query = self.query.project("color");
        self
    }
    
    pub fn group_by_color(self) -> Self { self.group_by("color") }
    

    pub fn with_color(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "color";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_color_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("color", min.into(), max.into()));
        self
    }

    pub fn with_color_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("color", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_color_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("color"));
        self
    }

    pub fn with_color_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("color"));
        self
    }
    
    pub fn with_color_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("color", value.into()));
        self
    }

    pub fn with_color_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("color", val.clone()));
        if "color" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_color_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("color", value));
        self
    }

    pub fn with_color_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("color", value));
        self
    }

    pub fn with_color_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("color", value));
        self
    }

    pub fn with_color_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("color", value));
        self
    }

    pub fn with_color_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("color", value));
        self
    }
    
    pub fn with_color_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("color", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_color_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("color", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_color_asc(mut self) -> Self {
        self.query = self.query.order_asc("color");
        self
    }

    pub fn order_by_color_desc(mut self) -> Self {
        self.query = self.query.order_desc("color");
        self
    }
    pub fn with_color_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("color", format!("%{}%", value.into())));
        self
    }

    pub fn with_color_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("color", value.into()));
        self
    }
    
    pub fn with_color_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("color", value.into()));
        self
    }
    
    pub fn with_color_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("color", value.into()));
        self
    }
    
    pub fn with_color_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("color", value.into()));
        self
    }
    
    pub fn with_color_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("color", value.into()));
        self
    }

    pub fn with_color_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("color", format!("{}%", value.into())));
        self
    }
    pub fn with_color_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("color", format!("%{}", value.into())));
        self
    }
    pub fn count_color(self) -> Self {
        self.count_color_as("color_count")
    }

    pub fn count_color_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("color", alias)
    }

    pub fn sum_color(self) -> Self {
        self.sum_color_as("sum_color")
    }

    pub fn sum_color_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("color", alias)
    }

    pub fn avg_color(self) -> Self {
        self.avg_color_as("avg_color")
    }

    pub fn avg_color_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("color", alias)
    }

    pub fn min_color(self) -> Self {
        self.min_color_as("min_color")
    }

    pub fn min_color_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("color", alias)
    }

    pub fn max_color(self) -> Self {
        self.max_color_as("max_color")
    }

    pub fn max_color_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("color", alias)
    }
    pub fn select_display_order(mut self) -> Self {
        self.query = self.query.project("display_order");
        self
    }
    
    pub fn group_by_display_order(self) -> Self { self.group_by("display_order") }
    

    pub fn with_display_order(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "display_order";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_display_order_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("display_order", min.into(), max.into()));
        self
    }

    pub fn with_display_order_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("display_order", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_display_order_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("display_order"));
        self
    }

    pub fn with_display_order_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("display_order"));
        self
    }
    
    pub fn with_display_order_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("display_order", value.into()));
        self
    }

    pub fn with_display_order_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("display_order", val.clone()));
        if "display_order" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_display_order_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("display_order", value));
        self
    }

    pub fn with_display_order_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("display_order", value));
        self
    }

    pub fn with_display_order_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("display_order", value));
        self
    }

    pub fn with_display_order_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("display_order", value));
        self
    }

    pub fn with_display_order_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("display_order", value));
        self
    }
    
    pub fn with_display_order_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("display_order", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_display_order_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("display_order", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_display_order_asc(mut self) -> Self {
        self.query = self.query.order_asc("display_order");
        self
    }

    pub fn order_by_display_order_desc(mut self) -> Self {
        self.query = self.query.order_desc("display_order");
        self
    }
    pub fn count_display_order(self) -> Self {
        self.count_display_order_as("display_order_count")
    }

    pub fn count_display_order_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("display_order", alias)
    }

    pub fn sum_display_order(self) -> Self {
        self.sum_display_order_as("sum_display_order")
    }

    pub fn sum_display_order_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("display_order", alias)
    }

    pub fn avg_display_order(self) -> Self {
        self.avg_display_order_as("avg_display_order")
    }

    pub fn avg_display_order_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("display_order", alias)
    }

    pub fn min_display_order(self) -> Self {
        self.min_display_order_as("min_display_order")
    }

    pub fn min_display_order_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("display_order", alias)
    }

    pub fn max_display_order(self) -> Self {
        self.max_display_order_as("max_display_order")
    }

    pub fn max_display_order_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("display_order", alias)
    }
    pub fn select_progress(mut self) -> Self {
        self.query = self.query.project("progress");
        self
    }
    
    pub fn group_by_progress(self) -> Self { self.group_by("progress") }
    

    pub fn with_progress(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "progress";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_progress_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("progress", min.into(), max.into()));
        self
    }

    pub fn with_progress_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("progress", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_progress_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("progress"));
        self
    }

    pub fn with_progress_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("progress"));
        self
    }
    
    pub fn with_progress_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("progress", value.into()));
        self
    }

    pub fn with_progress_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("progress", val.clone()));
        if "progress" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_progress_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("progress", value));
        self
    }

    pub fn with_progress_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("progress", value));
        self
    }

    pub fn with_progress_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("progress", value));
        self
    }

    pub fn with_progress_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("progress", value));
        self
    }

    pub fn with_progress_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("progress", value));
        self
    }
    
    pub fn with_progress_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("progress", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_progress_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("progress", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_progress_asc(mut self) -> Self {
        self.query = self.query.order_asc("progress");
        self
    }

    pub fn order_by_progress_desc(mut self) -> Self {
        self.query = self.query.order_desc("progress");
        self
    }
    pub fn count_progress(self) -> Self {
        self.count_progress_as("progress_count")
    }

    pub fn count_progress_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("progress", alias)
    }

    pub fn sum_progress(self) -> Self {
        self.sum_progress_as("sum_progress")
    }

    pub fn sum_progress_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("progress", alias)
    }

    pub fn avg_progress(self) -> Self {
        self.avg_progress_as("avg_progress")
    }

    pub fn avg_progress_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("progress", alias)
    }

    pub fn min_progress(self) -> Self {
        self.min_progress_as("min_progress")
    }

    pub fn min_progress_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("progress", alias)
    }

    pub fn max_progress(self) -> Self {
        self.max_progress_as("max_progress")
    }

    pub fn max_progress_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("progress", alias)
    }
    pub fn with_task_list_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn count_tasks(self) -> Self {
        self.count_tasks_as("count_tasks")
    }

    pub fn count_tasks_as(self, alias: impl Into<String>) -> Self {
        self.count_tasks_with(alias, crate::Q::tasks().unlimited())
    }
    pub fn count_tasks_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new(
            "task_list",
            alias,
            selection,
            true,
        ));
        self
    }
}
pub struct TaskRequest<R = crate::Task> {
    pub query: teaql_core::SelectQuery,
    pub relation_selections: Vec<RelationSelection>,
    pub relation_filters: Vec<RelationFilter>,
    pub child_enhancements: Vec<QuerySelection>,
    pub query_options: QueryOptions,
    pub filter_id: Option<u64>,
    marker: PhantomData<R>,
}

impl<R> Into<QuerySelection> for TaskRequest<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
        }
    }
}

impl<R> Clone for TaskRequest<R> {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            relation_selections: self.relation_selections.clone(),
            relation_filters: self.relation_filters.clone(),
            child_enhancements: self.child_enhancements.clone(),
            query_options: self.query_options.clone(),
            filter_id: self.filter_id.clone(),
            marker: PhantomData,
        }
    }
}

impl<R> TaskRequest<R> {
    pub(crate) fn new() -> Self {
        Self {
            query: teaql_core::SelectQuery::new("task"),
            relation_selections: Vec::new(),
            relation_filters: Vec::new(),
            child_enhancements: Vec::new(),
            query_options: QueryOptions::default(),
            filter_id: None,
            marker: PhantomData,
        }
    }

    pub fn return_type<T>(self) -> TaskRequest<T> {
        TaskRequest {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
            filter_id: self.filter_id,
            marker: PhantomData,
        }
    }

    pub fn query(&self) -> &teaql_core::SelectQuery { &self.query }
    pub fn relation_selections(&self) -> &[RelationSelection] { &self.relation_selections }
    pub fn relation_filters(&self) -> &[RelationFilter] { &self.relation_filters }
    pub fn child_enhancements(&self) -> &[QuerySelection] { &self.child_enhancements }
    pub fn query_options(&self) -> &QueryOptions { &self.query_options }
    pub fn into_query(self) -> teaql_core::SelectQuery { self.query }

    pub fn new_entity(&self, _ctx: &teaql_runtime::UserContext) -> crate::Task {
        crate::Task::new()
    }


    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {
        self.query = self.query.count(alias);
        self
    }

    pub fn aggregate_count_field(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.count_field(field, alias);
        self
    }

    pub fn aggregate_sum(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.sum(field, alias);
        self
    }

    pub fn aggregate_avg(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.avg(field, alias);
        self
    }

    pub fn aggregate_min(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.min(field, alias);
        self
    }

    pub fn aggregate_max(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.max(field, alias);
        self
    }

    pub async fn execute_for_list<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<R>, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let repository = ctx
            .task_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let mut query = self.query;
        query.filter = query.filter.map_or(
            Some(teaql_core::Expr::gt(
                "version",
                teaql_core::Value::I64(0),
            )),
            |f| {
                Some(teaql_core::Expr::And(vec![
                    f,
                    teaql_core::Expr::gt(
                        "version",
                        teaql_core::Value::I64(0),
                    ),
                ]))
            },
        );
        let query = request_support::apply_runtime_metadata(query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates::<R>(
            &query,
            &relation_aggregates,
        )?;
        let facets = request_support::execute_facets(ctx, &query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_first<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id<'a, C>(
        self,
        ctx: &'a C,
        id: impl Into<teaql_core::Value>,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.and_filter(teaql_core::Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<teaql_core::Record>, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .task_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates)?;
        let facets = request_support::execute_facets(ctx, &outer_query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_record<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<teaql_core::Record>, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.data.into_iter().next())
    }

    pub async fn execute_for_count<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<u64, request_support::TeaqlRepositoryError<C::TaskRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .task_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let mut query = self.query.clone();
        query.projection.clear();
        query.expr_projection.clear();
        query.order_by.clear();
        query.slice = None;
        query.relations.clear();
        query = query.count("COUNT_ALIAS");
        let rows = repository.fetch_all(&query)?;
        rows.into_iter().next().and_then(|row| row.get("COUNT_ALIAS").cloned()).and_then(|v| v.try_u64()).ok_or_else(|| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph("count result is missing or not numeric".to_string())))
    }

    pub fn filter(mut self, filter: Expr) -> Self {
        self.query = self.query.filter(filter);
        self
    }
    
    pub fn and_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.and_filter(filter);
        self
    }
    
    pub fn or_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.or_filter(filter);
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.query_options.comment = Some(comment.into());
        self.query.comment = self.query_options.comment.clone();
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.query = self.query.limit(limit);
        self
    }

    pub fn skip(mut self, offset: u64) -> Self {
        self.query = self.query.offset(offset);
        self
    }

    pub fn page_offset(mut self, offset: u64, limit: u64) -> Self {
        self.query = self.query.page(offset, limit);
        self
    }

    pub fn group_by(mut self, field: impl Into<String>) -> Self {
        self.query = self.query.group_by(field);
        self
    }

    pub fn select_self(self) -> Self {
        self
    }
    
    pub fn select_self_fields(self) -> Self {
        self
    }
    
    pub fn unlimited(self) -> Self {
        self
    }
    
    pub fn enhance_children_if_needed(self) -> Self {
        self
    }
    
    pub fn select_children(self) -> Self {
        self
    }
    pub fn select_id(mut self) -> Self {
        self.query = self.query.project("id");
        self
    }
    
    pub fn group_by_id(self) -> Self { self.group_by("id") }
    

    pub fn with_id(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "id";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_id_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", min.into(), max.into()));
        self
    }

    pub fn with_id_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_id_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("id"));
        self
    }

    pub fn with_id_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("id"));
        self
    }
    
    pub fn with_id_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("id", value.into()));
        self
    }

    pub fn with_id_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("id", val.clone()));
        if "id" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_id_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("id", value));
        self
    }

    pub fn with_id_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("id", value));
        self
    }

    pub fn with_id_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("id", value));
        self
    }

    pub fn with_id_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("id", value));
        self
    }

    pub fn with_id_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("id", value));
        self
    }
    
    pub fn with_id_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_id_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_id_asc(mut self) -> Self {
        self.query = self.query.order_asc("id");
        self
    }

    pub fn order_by_id_desc(mut self) -> Self {
        self.query = self.query.order_desc("id");
        self
    }
    pub fn count_id(self) -> Self {
        self.count_id_as("id_count")
    }

    pub fn count_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("id", alias)
    }

    pub fn sum_id(self) -> Self {
        self.sum_id_as("sum_id")
    }

    pub fn sum_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("id", alias)
    }

    pub fn avg_id(self) -> Self {
        self.avg_id_as("avg_id")
    }

    pub fn avg_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("id", alias)
    }

    pub fn min_id(self) -> Self {
        self.min_id_as("min_id")
    }

    pub fn min_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("id", alias)
    }

    pub fn max_id(self) -> Self {
        self.max_id_as("max_id")
    }

    pub fn max_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("id", alias)
    }
    pub fn select_name(mut self) -> Self {
        self.query = self.query.project("name");
        self
    }
    
    pub fn group_by_name(self) -> Self { self.group_by("name") }
    

    pub fn with_name(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "name";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_name_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("name", min.into(), max.into()));
        self
    }

    pub fn with_name_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("name", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_name_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("name"));
        self
    }

    pub fn with_name_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("name"));
        self
    }
    
    pub fn with_name_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("name", value.into()));
        self
    }

    pub fn with_name_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("name", val.clone()));
        if "name" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_name_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("name", value));
        self
    }

    pub fn with_name_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("name", value));
        self
    }

    pub fn with_name_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("name", value));
        self
    }

    pub fn with_name_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("name", value));
        self
    }

    pub fn with_name_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("name", value));
        self
    }
    
    pub fn with_name_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("name", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_name_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("name", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_name_asc(mut self) -> Self {
        self.query = self.query.order_asc("name");
        self
    }

    pub fn order_by_name_desc(mut self) -> Self {
        self.query = self.query.order_desc("name");
        self
    }
    pub fn with_name_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("%{}%", value.into())));
        self
    }

    pub fn with_name_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("name", value.into()));
        self
    }
    
    pub fn with_name_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("name", value.into()));
        self
    }
    
    pub fn with_name_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("name", value.into()));
        self
    }
    
    pub fn with_name_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("name", value.into()));
        self
    }
    
    pub fn with_name_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("name", value.into()));
        self
    }

    pub fn with_name_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("{}%", value.into())));
        self
    }
    pub fn with_name_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("name", format!("%{}", value.into())));
        self
    }
    pub fn count_name(self) -> Self {
        self.count_name_as("name_count")
    }

    pub fn count_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("name", alias)
    }

    pub fn sum_name(self) -> Self {
        self.sum_name_as("sum_name")
    }

    pub fn sum_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("name", alias)
    }

    pub fn avg_name(self) -> Self {
        self.avg_name_as("avg_name")
    }

    pub fn avg_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("name", alias)
    }

    pub fn min_name(self) -> Self {
        self.min_name_as("min_name")
    }

    pub fn min_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("name", alias)
    }

    pub fn max_name(self) -> Self {
        self.max_name_as("max_name")
    }

    pub fn max_name_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("name", alias)
    }
    pub fn with_status_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn count_status(self) -> Self {
        self.count_status_as("count_status")
    }

    pub fn count_status_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("status_id", alias)
    }
    pub fn facet_by_status_as(self, facet_name: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        self.facet_by_status_as_with_options(facet_name, request, true)
    }

    pub fn facet_by_status_as_with_options(
        mut self,
        facet_name: impl Into<String>,
        request: impl Into<QuerySelection>,
        include_all_facets: bool,
    ) -> Self {
        self.query_options.facets.push(FacetRequest::new(
            facet_name,
            "status",
            request,
            include_all_facets,
        ));
        self
    }
    pub fn with_platform_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn count_platform(self) -> Self {
        self.count_platform_as("count_platform")
    }

    pub fn count_platform_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("platform_id", alias)
    }
    pub fn facet_by_platform_as(self, facet_name: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        self.facet_by_platform_as_with_options(facet_name, request, true)
    }

    pub fn facet_by_platform_as_with_options(
        mut self,
        facet_name: impl Into<String>,
        request: impl Into<QuerySelection>,
        include_all_facets: bool,
    ) -> Self {
        self.query_options.facets.push(FacetRequest::new(
            facet_name,
            "platform",
            request,
            include_all_facets,
        ));
        self
    }
    pub fn with_task_execution_log_list_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn count_task_execution_logs(self) -> Self {
        self.count_task_execution_logs_as("count_task_execution_logs")
    }

    pub fn count_task_execution_logs_as(self, alias: impl Into<String>) -> Self {
        self.count_task_execution_logs_with(alias, crate::Q::task_execution_logs().unlimited())
    }
    pub fn count_task_execution_logs_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new(
            "task_execution_log_list",
            alias,
            selection,
            true,
        ));
        self
    }
}
pub struct TaskExecutionLogRequest<R = crate::TaskExecutionLog> {
    pub query: teaql_core::SelectQuery,
    pub relation_selections: Vec<RelationSelection>,
    pub relation_filters: Vec<RelationFilter>,
    pub child_enhancements: Vec<QuerySelection>,
    pub query_options: QueryOptions,
    pub filter_id: Option<u64>,
    marker: PhantomData<R>,
}

impl<R> Into<QuerySelection> for TaskExecutionLogRequest<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
        }
    }
}

impl<R> Clone for TaskExecutionLogRequest<R> {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            relation_selections: self.relation_selections.clone(),
            relation_filters: self.relation_filters.clone(),
            child_enhancements: self.child_enhancements.clone(),
            query_options: self.query_options.clone(),
            filter_id: self.filter_id.clone(),
            marker: PhantomData,
        }
    }
}

impl<R> TaskExecutionLogRequest<R> {
    pub(crate) fn new() -> Self {
        Self {
            query: teaql_core::SelectQuery::new("task_execution_log"),
            relation_selections: Vec::new(),
            relation_filters: Vec::new(),
            child_enhancements: Vec::new(),
            query_options: QueryOptions::default(),
            filter_id: None,
            marker: PhantomData,
        }
    }

    pub fn return_type<T>(self) -> TaskExecutionLogRequest<T> {
        TaskExecutionLogRequest {
            query: self.query,
            relation_selections: self.relation_selections,
            relation_filters: self.relation_filters,
            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
            filter_id: self.filter_id,
            marker: PhantomData,
        }
    }

    pub fn query(&self) -> &teaql_core::SelectQuery { &self.query }
    pub fn relation_selections(&self) -> &[RelationSelection] { &self.relation_selections }
    pub fn relation_filters(&self) -> &[RelationFilter] { &self.relation_filters }
    pub fn child_enhancements(&self) -> &[QuerySelection] { &self.child_enhancements }
    pub fn query_options(&self) -> &QueryOptions { &self.query_options }
    pub fn into_query(self) -> teaql_core::SelectQuery { self.query }

    pub fn new_entity(&self, _ctx: &teaql_runtime::UserContext) -> crate::TaskExecutionLog {
        crate::TaskExecutionLog::new()
    }


    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {
        self.query = self.query.count(alias);
        self
    }

    pub fn aggregate_count_field(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.count_field(field, alias);
        self
    }

    pub fn aggregate_sum(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.sum(field, alias);
        self
    }

    pub fn aggregate_avg(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.avg(field, alias);
        self
    }

    pub fn aggregate_min(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.min(field, alias);
        self
    }

    pub fn aggregate_max(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.max(field, alias);
        self
    }

    pub async fn execute_for_list<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<R>, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let repository = ctx
            .task_execution_log_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let mut query = self.query;
        query.filter = query.filter.map_or(
            Some(teaql_core::Expr::gt(
                "version",
                teaql_core::Value::I64(0),
            )),
            |f| {
                Some(teaql_core::Expr::And(vec![
                    f,
                    teaql_core::Expr::gt(
                        "version",
                        teaql_core::Value::I64(0),
                    ),
                ]))
            },
        );
        let query = request_support::apply_runtime_metadata(query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates::<R>(
            &query,
            &relation_aggregates,
        )?;
        let facets = request_support::execute_facets(ctx, &query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_first<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id<'a, C>(
        self,
        ctx: &'a C,
        id: impl Into<teaql_core::Value>,
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.and_filter(teaql_core::Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<teaql_core::Record>, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .task_execution_log_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates)?;
        let facets = request_support::execute_facets(ctx, &outer_query, &query_options)
            .map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        Ok(rows)
    }

    pub async fn execute_for_record<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<Option<teaql_core::Record>, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.data.into_iter().next())
    }

    pub async fn execute_for_count<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<u64, request_support::TeaqlRepositoryError<C::TaskExecutionLogRepository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .task_execution_log_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let mut query = self.query.clone();
        query.projection.clear();
        query.expr_projection.clear();
        query.order_by.clear();
        query.slice = None;
        query.relations.clear();
        query = query.count("COUNT_ALIAS");
        let rows = repository.fetch_all(&query)?;
        rows.into_iter().next().and_then(|row| row.get("COUNT_ALIAS").cloned()).and_then(|v| v.try_u64()).ok_or_else(|| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph("count result is missing or not numeric".to_string())))
    }

    pub fn filter(mut self, filter: Expr) -> Self {
        self.query = self.query.filter(filter);
        self
    }
    
    pub fn and_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.and_filter(filter);
        self
    }
    
    pub fn or_filter(mut self, filter: Expr) -> Self {
        self.query = self.query.or_filter(filter);
        self
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.query_options.comment = Some(comment.into());
        self.query.comment = self.query_options.comment.clone();
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.query = self.query.limit(limit);
        self
    }

    pub fn skip(mut self, offset: u64) -> Self {
        self.query = self.query.offset(offset);
        self
    }

    pub fn page_offset(mut self, offset: u64, limit: u64) -> Self {
        self.query = self.query.page(offset, limit);
        self
    }

    pub fn group_by(mut self, field: impl Into<String>) -> Self {
        self.query = self.query.group_by(field);
        self
    }

    pub fn select_self(self) -> Self {
        self
    }
    
    pub fn select_self_fields(self) -> Self {
        self
    }
    
    pub fn unlimited(self) -> Self {
        self
    }
    
    pub fn enhance_children_if_needed(self) -> Self {
        self
    }
    
    pub fn select_children(self) -> Self {
        self
    }
    pub fn select_id(mut self) -> Self {
        self.query = self.query.project("id");
        self
    }
    
    pub fn group_by_id(self) -> Self { self.group_by("id") }
    

    pub fn with_id(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "id";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_id_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", min.into(), max.into()));
        self
    }

    pub fn with_id_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("id", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_id_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("id"));
        self
    }

    pub fn with_id_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("id"));
        self
    }
    
    pub fn with_id_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("id", value.into()));
        self
    }

    pub fn with_id_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("id", val.clone()));
        if "id" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_id_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("id", value));
        self
    }

    pub fn with_id_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("id", value));
        self
    }

    pub fn with_id_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("id", value));
        self
    }

    pub fn with_id_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("id", value));
        self
    }

    pub fn with_id_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("id", value));
        self
    }
    
    pub fn with_id_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_id_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("id", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_id_asc(mut self) -> Self {
        self.query = self.query.order_asc("id");
        self
    }

    pub fn order_by_id_desc(mut self) -> Self {
        self.query = self.query.order_desc("id");
        self
    }
    pub fn count_id(self) -> Self {
        self.count_id_as("id_count")
    }

    pub fn count_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("id", alias)
    }

    pub fn sum_id(self) -> Self {
        self.sum_id_as("sum_id")
    }

    pub fn sum_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("id", alias)
    }

    pub fn avg_id(self) -> Self {
        self.avg_id_as("avg_id")
    }

    pub fn avg_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("id", alias)
    }

    pub fn min_id(self) -> Self {
        self.min_id_as("min_id")
    }

    pub fn min_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("id", alias)
    }

    pub fn max_id(self) -> Self {
        self.max_id_as("max_id")
    }

    pub fn max_id_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("id", alias)
    }
    pub fn select_action(mut self) -> Self {
        self.query = self.query.project("action");
        self
    }
    
    pub fn group_by_action(self) -> Self { self.group_by("action") }
    

    pub fn with_action(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "action";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_action_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("action", min.into(), max.into()));
        self
    }

    pub fn with_action_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("action", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_action_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("action"));
        self
    }

    pub fn with_action_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("action"));
        self
    }
    
    pub fn with_action_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("action", value.into()));
        self
    }

    pub fn with_action_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("action", val.clone()));
        if "action" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_action_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("action", value));
        self
    }

    pub fn with_action_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("action", value));
        self
    }

    pub fn with_action_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("action", value));
        self
    }

    pub fn with_action_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("action", value));
        self
    }

    pub fn with_action_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("action", value));
        self
    }
    
    pub fn with_action_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("action", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_action_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("action", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_action_asc(mut self) -> Self {
        self.query = self.query.order_asc("action");
        self
    }

    pub fn order_by_action_desc(mut self) -> Self {
        self.query = self.query.order_desc("action");
        self
    }
    pub fn with_action_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("action", format!("%{}%", value.into())));
        self
    }

    pub fn with_action_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("action", value.into()));
        self
    }
    
    pub fn with_action_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("action", value.into()));
        self
    }
    
    pub fn with_action_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("action", value.into()));
        self
    }
    
    pub fn with_action_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("action", value.into()));
        self
    }
    
    pub fn with_action_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("action", value.into()));
        self
    }

    pub fn with_action_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("action", format!("{}%", value.into())));
        self
    }
    pub fn with_action_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("action", format!("%{}", value.into())));
        self
    }
    pub fn count_action(self) -> Self {
        self.count_action_as("action_count")
    }

    pub fn count_action_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("action", alias)
    }

    pub fn sum_action(self) -> Self {
        self.sum_action_as("sum_action")
    }

    pub fn sum_action_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("action", alias)
    }

    pub fn avg_action(self) -> Self {
        self.avg_action_as("avg_action")
    }

    pub fn avg_action_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("action", alias)
    }

    pub fn min_action(self) -> Self {
        self.min_action_as("min_action")
    }

    pub fn min_action_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("action", alias)
    }

    pub fn max_action(self) -> Self {
        self.max_action_as("max_action")
    }

    pub fn max_action_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("action", alias)
    }
    pub fn select_detail(mut self) -> Self {
        self.query = self.query.project("detail");
        self
    }
    
    pub fn group_by_detail(self) -> Self { self.group_by("detail") }
    

    pub fn with_detail(mut self, operator: FieldOperator, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        let field_name = "detail";
        let expr = match operator {
            FieldOperator::Equal => Expr::eq(field_name, val.clone()),
            FieldOperator::NotEqual => Expr::ne(field_name, val.clone()),
            FieldOperator::GreaterThan => Expr::gt(field_name, val.clone()),
            FieldOperator::GreaterThanOrEqual => Expr::gte(field_name, val.clone()),
            FieldOperator::LessThan => Expr::lt(field_name, val.clone()),
            FieldOperator::LessThanOrEqual => Expr::lte(field_name, val.clone()),
            FieldOperator::Between => Expr::eq(field_name, val.clone()), // Approximation
            FieldOperator::In => Expr::in_list(field_name, vec![val.clone()]),
            FieldOperator::NotIn => Expr::not_in_list(field_name, vec![val.clone()]),
            FieldOperator::Contain => Expr::contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotContain => Expr::not_contain(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::BeginWith => Expr::begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotBeginWith => Expr::not_begin_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::EndWith => Expr::end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::NotEndWith => Expr::not_end_with(field_name, if let teaql_core::Value::Text(s) = &val { s.clone() } else { "".to_string() }),
            FieldOperator::SoundsLike => Expr::sound_like(field_name, val.clone()),
            FieldOperator::IsNull => Expr::is_null(field_name),
            FieldOperator::IsNotNull => Expr::is_not_null(field_name),
        };
        self.query = self.query.and_filter(expr);
        if field_name == "id" {
            if let FieldOperator::Equal = operator {
                if let teaql_core::Value::I64(v) = val {
                    self.filter_id = Some(v as u64);
                } else if let teaql_core::Value::U64(v) = val {
                    self.filter_id = Some(v);
                }
            }
        }
        self
    }
    
    pub fn with_detail_between(mut self, min: impl Into<teaql_core::Value>, max: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::between("detail", min.into(), max.into()));
        self
    }

    pub fn with_detail_between_range<T: Into<teaql_core::Value> + Clone>(mut self, range: DateRange<T>) -> Self {
        self.query = self.query.and_filter(Expr::between("detail", range.start.clone().into(), range.end.clone().into()));
        self
    }

    pub fn with_detail_is_unknown(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_null("detail"));
        self
    }

    pub fn with_detail_is_known(mut self) -> Self {
        self.query = self.query.and_filter(Expr::is_not_null("detail"));
        self
    }
    
    pub fn with_detail_sounding_like(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::sound_like("detail", value.into()));
        self
    }

    pub fn with_detail_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("detail", val.clone()));
        if "detail" == "id" {
            if let teaql_core::Value::I64(v) = val {
                self.filter_id = Some(v as u64);
            } else if let teaql_core::Value::U64(v) = val {
                self.filter_id = Some(v);
            }
        }
        self
    }
    
    pub fn with_detail_is_not(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::ne("detail", value));
        self
    }

    pub fn with_detail_greater_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("detail", value));
        self
    }

    pub fn with_detail_greater_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gte("detail", value));
        self
    }

    pub fn with_detail_less_than(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("detail", value));
        self
    }

    pub fn with_detail_less_than_or_equal_to(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lte("detail", value));
        self
    }
    
    pub fn with_detail_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::in_list("detail", values.into_iter().map(Into::into)));
        self
    }

    pub fn with_detail_not_in(mut self, values: impl IntoIterator<Item = impl Into<teaql_core::Value>>) -> Self {
        self.query = self.query.and_filter(Expr::not_in_list("detail", values.into_iter().map(Into::into)));
        self
    }

    pub fn order_by_detail_asc(mut self) -> Self {
        self.query = self.query.order_asc("detail");
        self
    }

    pub fn order_by_detail_desc(mut self) -> Self {
        self.query = self.query.order_desc("detail");
        self
    }
    pub fn with_detail_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("detail", format!("%{}%", value.into())));
        self
    }

    pub fn with_detail_not_containing(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_contain("detail", value.into()));
        self
    }
    
    pub fn with_detail_not_starting_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_begin_with("detail", value.into()));
        self
    }
    
    pub fn with_detail_not_ending_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::not_end_with("detail", value.into()));
        self
    }
    
    pub fn with_detail_before(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::lt("detail", value.into()));
        self
    }
    
    pub fn with_detail_after(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::gt("detail", value.into()));
        self
    }

    pub fn with_detail_starts_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("detail", format!("{}%", value.into())));
        self
    }
    pub fn with_detail_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("detail", format!("%{}", value.into())));
        self
    }
    pub fn count_detail(self) -> Self {
        self.count_detail_as("detail_count")
    }

    pub fn count_detail_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("detail", alias)
    }

    pub fn sum_detail(self) -> Self {
        self.sum_detail_as("sum_detail")
    }

    pub fn sum_detail_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("detail", alias)
    }

    pub fn avg_detail(self) -> Self {
        self.avg_detail_as("avg_detail")
    }

    pub fn avg_detail_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("detail", alias)
    }

    pub fn min_detail(self) -> Self {
        self.min_detail_as("min_detail")
    }

    pub fn min_detail_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("detail", alias)
    }

    pub fn max_detail(self) -> Self {
        self.max_detail_as("max_detail")
    }

    pub fn max_detail_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("detail", alias)
    }
    pub fn with_task_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn count_task(self) -> Self {
        self.count_task_as("count_task")
    }

    pub fn count_task_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("task_id", alias)
    }
    pub fn facet_by_task_as(self, facet_name: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        self.facet_by_task_as_with_options(facet_name, request, true)
    }

    pub fn facet_by_task_as_with_options(
        mut self,
        facet_name: impl Into<String>,
        request: impl Into<QuerySelection>,
        include_all_facets: bool,
    ) -> Self {
        self.query_options.facets.push(FacetRequest::new(
            facet_name,
            "task",
            request,
            include_all_facets,
        ));
        self
    }
}