import re

with open('/home/philip/githome/robot-task-board/generate-lib/lib/src/request_support.rs', 'r') as f:
    request_support = f.read()

# Template TeaqlRepositoryProvider trait
trait_start = request_support.find("pub trait TeaqlRepositoryProvider: TeaqlRuntime {")
trait_end = request_support.find("}", trait_start) + 1

trait_template = """pub trait TeaqlRepositoryProvider: TeaqlRuntime {
{%- for entity in entities %}
    type {{ entity.rust_struct }}Repository<'a>: TeaqlEntityRepository + 'a
    where
        Self: 'a;

    fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, ContextError>;
{%- endfor %}
}"""
request_support = request_support[:trait_start] + trait_template + request_support[trait_end:]

# Template impl TeaqlRepositoryProvider
impl_start = request_support.find("impl TeaqlRepositoryProvider for teaql_runtime::UserContext {")
impl_end = request_support.find("#[derive(Clone, Debug, PartialEq)]\npub struct QuerySelection", impl_start)

impl_template = """impl TeaqlRepositoryProvider for teaql_runtime::UserContext {
{%- for entity in entities %}
    type {{ entity.rust_struct }}Repository<'a> = teaql_runtime::ResolvedRepository<'a, crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor>
    where
        Self: 'a;

    fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, ContextError> {
        self.resolve_repository::<crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor>("{{ entity.rust_module }}")
    }
{%- endfor %}
}"""
request_support = request_support[:impl_start] + impl_template + request_support[impl_end:]

# Indent request_support by 4 spaces
request_support = "\n".join("    " + line if line else line for line in request_support.split("\n"))
request_support = "pub mod request_support {\n" + request_support + "\n}\n"

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    q_rs = f.read()

# Replace the old request_support block
old_support_start = q_rs.find("pub mod request_support {")
old_support_end = q_rs.find("use request_support::*;")
q_rs = q_rs[:old_support_start] + request_support + q_rs[old_support_end:]

# Add crate::runtime::* to the top
q_rs = "use crate::runtime::*;\n" + q_rs

# Now replace execute_for_* methods
exec_methods = """    pub async fn execute_for_list<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<R>, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        let repository = ctx
            .{{ entity.rust_module }}_repository()
            .map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
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
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
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
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
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
    ) -> Result<Option<R>, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
        R: teaql_core::Entity,
    {
        self.and_filter(teaql_core::Expr::eq("id", id)).execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<teaql_core::SmartList<teaql_core::Record>, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .{{ entity.rust_module }}_repository()
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
    ) -> Result<Option<teaql_core::Record>, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.data.into_iter().next())
    }

    pub async fn execute_for_count<'a, C>(
        self,
        ctx: &'a C,
    ) -> Result<u64, request_support::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>>
    where
        C: request_support::TeaqlRepositoryProvider + ?Sized,
    {
        let repository = ctx
            .{{ entity.rust_module }}_repository()
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
    }"""

old_exec = re.search(r'    pub async fn execute_for_list\(self, ctx: &teaql_runtime::UserContext\).*?pub async fn execute_for_count\(self, ctx: &teaql_runtime::UserContext\) -> Result<u64, String> \{.*?\n    \}', q_rs, re.DOTALL)
if old_exec:
    q_rs = q_rs.replace(old_exec.group(0), exec_methods)
else:
    print("Could not find old execute methods!")
    exit(1)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(q_rs)

print("Generated perfect q.rs.j2!")
