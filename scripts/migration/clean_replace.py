import re

with open('/home/philip/githome/robot-task-board/expanded.rs', 'r') as f:
    expanded = f.read()

start_idx = expanded.find("pub mod request_support {")
if start_idx == -1:
    print("Could not find start")
    exit(1)

# Stop exactly where request_support ends, which is right before pub mod platform
end_idx = expanded.find("\npub mod runtime {")
if end_idx == -1:
    print("Could not find end")
    exit(1)

support_code = expanded[start_idx:end_idx]

support_code = support_code.replace("crate::runtime::DataServiceDialect", "teaql_provider_rusqlite::RusqliteDialect")
support_code = support_code.replace("crate::runtime::DataServiceExecutor", "teaql_provider_rusqlite::RusqliteMutationExecutor")
support_code = support_code.replace("crate::runtime::DataServiceMutationError", "teaql_provider_rusqlite::RusqliteMutationError")
support_code = support_code.replace("crate::runtime::DataServiceIdGenerator", "teaql_provider_rusqlite::RusqliteIdSpaceGenerator")
support_code = support_code.replace("crate::runtime::DataServicePool", "rusqlite::Connection")

trait_start = support_code.find("    pub trait TeaqlRuntime {")
trait_end = support_code.find("    #[allow(async_fn_in_trait)]")
if trait_start != -1 and trait_end != -1:
    support_code = support_code[:trait_start] + "/* TEAQL_RUNTIME_PLACEHOLDER */\n" + support_code[trait_end:]

impl_start = support_code.find("    impl TeaqlRuntime for teaql_runtime::UserContext {")
impl_end = support_code.find("    pub struct QuerySelection {")
if impl_start != -1 and impl_end != -1:
    support_code = support_code[:impl_start] + "/* TEAQL_IMPL_PLACEHOLDER */\n" + support_code[impl_end:]

runtime_trait_template = """    pub trait TeaqlRuntime {
{%- for entity in entities %}
        type {{ entity.rust_struct }}Repository<'a>: TeaqlEntityRepository + 'a where Self: 'a;
        fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, ContextError>;
{%- endfor %}
        fn user_context(&self) -> &UserContext;
        fn fetch_facet_smart_list(&self, entity: &str, query: &SelectQuery, relation_aggregates: &[RuntimeRelationAggregate]) -> Result<SmartList<Record>, RuntimeError>;
    }"""
support_code = support_code.replace("/* TEAQL_RUNTIME_PLACEHOLDER */", runtime_trait_template)

runtime_impl_template = """    impl TeaqlRuntime for teaql_runtime::UserContext {
{%- for entity in entities %}
        type {{ entity.rust_struct }}Repository<'a> = teaql_runtime::ResolvedRepository<'a, teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor> where Self: 'a;
        fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, ContextError> {
            self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>("{{ entity.rust_struct }}")
        }
{%- endfor %}
        fn user_context(&self) -> &UserContext { self }
        fn fetch_facet_smart_list(&self, entity: &str, query: &SelectQuery, relation_aggregates: &[RuntimeRelationAggregate]) -> Result<SmartList<Record>, RuntimeError> {
            self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>(entity.to_owned()).map_err(|err| RuntimeError::Graph(err.to_string()))?.fetch_smart_list_with_relation_aggregates(query, relation_aggregates).map_err(|err| RuntimeError::Graph(err.to_string()))
        }
    }"""
support_code = support_code.replace("/* TEAQL_IMPL_PLACEHOLDER */", runtime_impl_template)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    lines = f.readlines()

new_lines = []
in_support = False
for line in lines:
    if line.startswith("pub mod request_support {"):
        in_support = True
        new_lines.append(support_code + "\n")
    elif line.startswith("use request_support::*;"):
        in_support = False
        new_lines.append("use request_support::*;\n")
    elif not in_support:
        new_lines.append(line)

q_content = "".join(new_lines)

old_exec = re.search(r'    pub async fn execute_for_list\(self, ctx: &teaql_runtime::UserContext\).*?pub async fn execute_for_count\(self, ctx: &teaql_runtime::UserContext\) -> Result<u64, String> \{.*?\n    \}', q_content, re.DOTALL)
if old_exec:
    new_exec = """    pub async fn execute_for_list<'a, C>(self, ctx: &'a C) -> Result<teaql_core::SmartList<R>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized, R: teaql_core::Entity {
        let repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates(&query, &relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        
        let facets = request_support::execute_facets(ctx, &outer_query, &query_options).map_err(teaql_runtime::RepositoryError::Runtime)?;
        request_support::attach_facets(&mut rows, facets);
        
        Ok(rows)
    }

    pub async fn execute_for_first<'a, C>(self, ctx: &'a C) -> Result<Option<R>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized, R: teaql_core::Entity {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one<'a, C>(self, ctx: &'a C) -> Result<Option<R>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized, R: teaql_core::Entity {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(self, ctx: &'a C) -> Result<Vec<teaql_core::Record>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
        let repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = request_support::runtime_relation_aggregates(&query_options);
        let query = request_support::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        Ok(rows.data)
    }

    pub async fn execute_for_record<'a, C>(self, ctx: &'a C) -> Result<Option<teaql_core::Record>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
        let records = self.limit(1).execute_for_records(ctx).await?;
        Ok(records.into_iter().next())
    }

    pub async fn execute_for_count<'a, C>(self, ctx: &'a C) -> Result<u64, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
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
    q_content = q_content.replace(old_exec.group(0), new_exec)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(q_content)

print("Generated clean_replace.py!")
