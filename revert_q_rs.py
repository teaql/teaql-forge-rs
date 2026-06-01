import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

# Replace execute_for_list
old_exec_list = re.search(r'pub async fn execute_for_list\(self, ctx: &teaql_runtime::UserContext\) -> Result<teaql_core::SmartList<crate::{{ entity.rust_struct }}>, String> \{.*?\n    \}', content, re.DOTALL)

new_exec_list = """pub async fn execute_for_list<'a, C>(self, ctx: &'a C) -> Result<teaql_core::SmartList<crate::{{ entity.rust_struct }}>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: teaql_runtime::TeaqlRuntime + ?Sized {
        let mut repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = teaql_runtime::runtime_relation_aggregates(&query_options);
        let query = teaql_runtime::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        // Facet calculation simulated out as requested
        // let facets = teaql_runtime::execute_facets(ctx, &outer_query, &query_options).await?;
        // teaql_runtime::attach_facets(&mut rows, facets);
        Ok(rows)
    }"""

content = content.replace(old_exec_list.group(0), new_exec_list)

# Replace execute_for_one
old_exec_one = re.search(r'pub async fn execute_for_one\(self, ctx: &teaql_runtime::UserContext\) -> Result<Option<crate::{{ entity.rust_struct }}>, String> \{.*?\n    \}', content, re.DOTALL)

new_exec_one = """pub async fn execute_for_one<'a, C>(self, ctx: &'a C) -> Result<Option<crate::{{ entity.rust_struct }}>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: teaql_runtime::TeaqlRuntime + ?Sized {
        let records = self.limit(1).execute_for_list(ctx).await?;
        Ok(records.data.into_iter().next())
    }"""

content = content.replace(old_exec_one.group(0), new_exec_one)

# Replace execute_for_records
old_exec_records = re.search(r'pub async fn execute_for_records\(self, ctx: &teaql_runtime::UserContext\) -> Result<Vec<teaql_core::Record>, String> \{.*?\n    \}', content, re.DOTALL)

new_exec_records = """pub async fn execute_for_records<'a, C>(self, ctx: &'a C) -> Result<Vec<teaql_core::Record>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: teaql_runtime::TeaqlRuntime + ?Sized {
        let mut repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = teaql_runtime::runtime_relation_aggregates(&query_options);
        let query = teaql_runtime::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        // let facets = teaql_runtime::execute_facets(ctx, &outer_query, &query_options).await?;
        // teaql_runtime::attach_facets(&mut rows, facets);
        Ok(rows.data.into_iter().map(teaql_core::Entity::into_record).collect())
    }"""

content = content.replace(old_exec_records.group(0), new_exec_records)

# Replace execute_for_record
old_exec_record = re.search(r'pub async fn execute_for_record\(self, ctx: &teaql_runtime::UserContext\) -> Result<Option<teaql_core::Record>, String> \{.*?\n    \}', content, re.DOTALL)

new_exec_record = """pub async fn execute_for_record<'a, C>(self, ctx: &'a C) -> Result<Option<teaql_core::Record>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: teaql_runtime::TeaqlRuntime + ?Sized {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.into_iter().next())
    }"""

content = content.replace(old_exec_record.group(0), new_exec_record)

# Replace execute_for_count
old_exec_count = re.search(r'pub async fn execute_for_count\(self, ctx: &teaql_runtime::UserContext\) -> Result<u64, String> \{.*?\n    \}', content, re.DOTALL)

new_exec_count = """pub async fn execute_for_count<'a, C>(self, ctx: &'a C) -> Result<u64, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: teaql_runtime::TeaqlRuntime + ?Sized {
        let repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let mut query = self.query.clone();
        query.projection.clear();
        query.expr_projection.clear();
        query.order_by.clear();
        query.slice = None;
        query.relations.clear();
        query = query.count("COUNT_ALIAS");
        let rows = repository.fetch_all(&query)?;
        rows.first().and_then(|row| row.get("COUNT_ALIAS")).and_then(teaql_core::Value::try_u64).ok_or_else(|| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph("count result is missing or not numeric".to_string())))
    }"""

content = content.replace(old_exec_count.group(0), new_exec_count)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)

print("Reverted execute methods")
