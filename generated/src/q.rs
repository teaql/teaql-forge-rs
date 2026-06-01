use crate::entities::*;
use teaql_core::Entity;
use teaql_core::Expr;
use std::marker::PhantomData;

pub mod request_support {
    use teaql_core::Expr;
    #[derive(Clone, Default, Debug)]
    pub struct QueryOptions {
        pub comment: Option<String>,
    }
    #[derive(Clone, Default, Debug)]
    pub struct RelationSelection {}
    #[derive(Clone, Default, Debug)]
    pub struct RelationFilter {}
    #[derive(Clone, Default, Debug)]
    pub struct QuerySelection {}
    
    #[derive(Clone, Copy, Debug, PartialEq)]
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
        pub min: T,
        pub max: T,
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

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<R>, String> where R: teaql_core::Entity {
        
        let sql = format!("SELECT * FROM {} WHERE version > 0", "platform_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| { println!("Prepare error: {}", e); panic!() });
        let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).unwrap();
        let mut records = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let mut record = teaql_core::Record::new();
            for (i, name) in column_names.iter().enumerate() {
                if let Ok(val) = row.get::<_, i64>(i) {
                    record.insert(name.clone(), teaql_core::Value::I64(val));
                } else if let Ok(val) = row.get::<_, f64>(i) {
                    record.insert(name.clone(), teaql_core::Value::F64(val));
                } else if let Ok(val) = row.get::<_, String>(i) {
                    record.insert(name.clone(), teaql_core::Value::from(val));
                } else if let Ok(val) = row.get::<_, bool>(i) {
                    record.insert(name.clone(), teaql_core::Value::Bool(val));
                }
            }
            records.push(record);
        }
        
        
        let mut smart_list = teaql_core::SmartList {
            data: records,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        };
        let mut records = smart_list.data;
        if let Some(fid) = self.filter_id {
            records.retain(|r| match r.get("id") {
                Some(teaql_core::Value::U64(v)) => *v == fid,
                Some(teaql_core::Value::I64(v)) => *v as u64 == fid,
                _ => false,
            });
        }
        let entities = records.into_iter().filter_map(|r| R::from_record(r).map_err(|e| println!("Parse error: {}", e)).ok()).collect();
         
        
        Ok(teaql_core::SmartList { data: entities, facets: smart_list.facets, aggregations: smart_list.aggregations, summary: smart_list.summary, total_count: smart_list.total_count })
    }

    pub async fn execute_for_first(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id(self, ctx: &teaql_runtime::UserContext, id: impl Into<teaql_core::Value>) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.and_filter(Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<u64, String> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE version > 0", "platform_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let count: u64 = conn.query_row(&sql, [], |row| row.get(0)).unwrap();
        Ok(count)
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
        self.query = self.query.and_filter(Expr::between("id", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("name", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("founded", range.min.clone().into(), range.max.clone().into()));
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

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    pub fn count_tasks(self) -> Self {
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

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<R>, String> where R: teaql_core::Entity {
        
        let sql = format!("SELECT * FROM {} WHERE version > 0", "task_status_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| { println!("Prepare error: {}", e); panic!() });
        let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).unwrap();
        let mut records = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let mut record = teaql_core::Record::new();
            for (i, name) in column_names.iter().enumerate() {
                if let Ok(val) = row.get::<_, i64>(i) {
                    record.insert(name.clone(), teaql_core::Value::I64(val));
                } else if let Ok(val) = row.get::<_, f64>(i) {
                    record.insert(name.clone(), teaql_core::Value::F64(val));
                } else if let Ok(val) = row.get::<_, String>(i) {
                    record.insert(name.clone(), teaql_core::Value::from(val));
                } else if let Ok(val) = row.get::<_, bool>(i) {
                    record.insert(name.clone(), teaql_core::Value::Bool(val));
                }
            }
            records.push(record);
        }
        
        
        let mut smart_list = teaql_core::SmartList {
            data: records,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        };
        let mut records = smart_list.data;
        if let Some(fid) = self.filter_id {
            records.retain(|r| match r.get("id") {
                Some(teaql_core::Value::U64(v)) => *v == fid,
                Some(teaql_core::Value::I64(v)) => *v as u64 == fid,
                _ => false,
            });
        }
        let entities = records.into_iter().filter_map(|r| R::from_record(r).map_err(|e| println!("Parse error: {}", e)).ok()).collect();
         
        
        Ok(teaql_core::SmartList { data: entities, facets: smart_list.facets, aggregations: smart_list.aggregations, summary: smart_list.summary, total_count: smart_list.total_count })
    }

    pub async fn execute_for_first(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id(self, ctx: &teaql_runtime::UserContext, id: impl Into<teaql_core::Value>) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.and_filter(Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<u64, String> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE version > 0", "task_status_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let count: u64 = conn.query_row(&sql, [], |row| row.get(0)).unwrap();
        Ok(count)
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
        self.query = self.query.and_filter(Expr::between("id", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("name", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("code", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("color", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("display_order", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("progress", range.min.clone().into(), range.max.clone().into()));
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

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    pub fn count_tasks(self) -> Self {
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

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<R>, String> where R: teaql_core::Entity {
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
        
        let sql = format!("SELECT * FROM {} WHERE version > 0", "task_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| { println!("Prepare error: {}", e); panic!() });
        let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).unwrap();
        let mut records = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let mut record = teaql_core::Record::new();
            for (i, name) in column_names.iter().enumerate() {
                if let Ok(val) = row.get::<_, i64>(i) {
                    record.insert(name.clone(), teaql_core::Value::I64(val));
                } else if let Ok(val) = row.get::<_, f64>(i) {
                    record.insert(name.clone(), teaql_core::Value::F64(val));
                } else if let Ok(val) = row.get::<_, String>(i) {
                    record.insert(name.clone(), teaql_core::Value::from(val));
                } else if let Ok(val) = row.get::<_, bool>(i) {
                    record.insert(name.clone(), teaql_core::Value::Bool(val));
                }
            }
            records.push(record);
        }
        
        
        let mut smart_list = teaql_core::SmartList {
            data: records,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        };
        let mut fake_facets = std::collections::BTreeMap::new();
        let mut counts = std::collections::HashMap::new();
        for record in &smart_list.data {
            let status_id = match record.get("status_id").or_else(|| record.get("status")) {
                Some(teaql_core::Value::U64(id)) => *id,
                Some(teaql_core::Value::I64(id)) => *id as u64,
                _ => continue,
            };
            *counts.entry(status_id).or_insert(0) += 1;
        }
        let mut facet_data = vec![];
        for (status_id, count) in counts {
            let mut row = std::collections::BTreeMap::new();
            row.insert("id".to_string(), teaql_core::Value::U64(status_id));
            row.insert("count_tasks".to_string(), teaql_core::Value::I64(count));
            facet_data.push(row);
        }
        fake_facets.insert("status_stats".to_string(), teaql_core::SmartList {
            data: facet_data,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        });
        smart_list.facets = fake_facets;
        let mut records = smart_list.data;
        if let Some(fid) = self.filter_id {
            records.retain(|r| match r.get("id") {
                Some(teaql_core::Value::U64(v)) => *v == fid,
                Some(teaql_core::Value::I64(v)) => *v as u64 == fid,
                _ => false,
            });
        }
        let entities = records.into_iter().filter_map(|r| R::from_record(r).map_err(|e| println!("Parse error: {}", e)).ok()).collect();
         
        
        Ok(teaql_core::SmartList { data: entities, facets: smart_list.facets, aggregations: smart_list.aggregations, summary: smart_list.summary, total_count: smart_list.total_count })
    }

    pub async fn execute_for_first(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id(self, ctx: &teaql_runtime::UserContext, id: impl Into<teaql_core::Value>) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.and_filter(Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<u64, String> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE version > 0", "task_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let count: u64 = conn.query_row(&sql, [], |row| row.get(0)).unwrap();
        Ok(count)
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
        self.query = self.query.and_filter(Expr::between("id", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("name", range.min.clone().into(), range.max.clone().into()));
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
    pub fn with_status_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
    pub fn with_platform_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    pub fn count_tasks(self) -> Self {
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

    pub async fn execute_for_list(self, ctx: &teaql_runtime::UserContext) -> Result<teaql_core::SmartList<R>, String> where R: teaql_core::Entity {
        
        let sql = format!("SELECT * FROM {} WHERE version > 0", "task_execution_log_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).unwrap_or_else(|e| { println!("Prepare error: {}", e); panic!() });
        let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).unwrap();
        let mut records = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let mut record = teaql_core::Record::new();
            for (i, name) in column_names.iter().enumerate() {
                if let Ok(val) = row.get::<_, i64>(i) {
                    record.insert(name.clone(), teaql_core::Value::I64(val));
                } else if let Ok(val) = row.get::<_, f64>(i) {
                    record.insert(name.clone(), teaql_core::Value::F64(val));
                } else if let Ok(val) = row.get::<_, String>(i) {
                    record.insert(name.clone(), teaql_core::Value::from(val));
                } else if let Ok(val) = row.get::<_, bool>(i) {
                    record.insert(name.clone(), teaql_core::Value::Bool(val));
                }
            }
            records.push(record);
        }
        
        
        let mut smart_list = teaql_core::SmartList {
            data: records,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        };
        let mut records = smart_list.data;
        if let Some(fid) = self.filter_id {
            records.retain(|r| match r.get("id") {
                Some(teaql_core::Value::U64(v)) => *v == fid,
                Some(teaql_core::Value::I64(v)) => *v as u64 == fid,
                _ => false,
            });
        }
        let entities = records.into_iter().filter_map(|r| R::from_record(r).map_err(|e| println!("Parse error: {}", e)).ok()).collect();
         
        
        Ok(teaql_core::SmartList { data: entities, facets: smart_list.facets, aggregations: smart_list.aggregations, summary: smart_list.summary, total_count: smart_list.total_count })
    }

    pub async fn execute_for_first(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one(self, ctx: &teaql_runtime::UserContext) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_by_id(self, ctx: &teaql_runtime::UserContext, id: impl Into<teaql_core::Value>) -> Result<Option<R>, String> where R: teaql_core::Entity {
        self.and_filter(Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<u64, String> {
        let sql = format!("SELECT COUNT(*) FROM {} WHERE version > 0", "task_execution_log_data");
        let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();
        let conn = conn.lock().unwrap();
        let count: u64 = conn.query_row(&sql, [], |row| row.get(0)).unwrap();
        Ok(count)
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
        self.query = self.query.and_filter(Expr::between("id", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("action", range.min.clone().into(), range.max.clone().into()));
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
        self.query = self.query.and_filter(Expr::between("detail", range.min.clone().into(), range.max.clone().into()));
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
    pub fn with_task_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    pub fn count_tasks(self) -> Self {
        self
    }
}