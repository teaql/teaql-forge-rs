import re

with open('/home/philip/githome/robot-task-board/expanded.rs', 'r') as f:
    lines = f.readlines()

expanded = "".join(lines)
start_idx = expanded.find("    pub trait TeaqlRecordRepository")
end_idx = expanded.find("    mod request {")
support_code = expanded[start_idx:end_idx]

support_code = support_code.replace("crate::runtime::DataServiceDialect", "teaql_provider_rusqlite::RusqliteDialect")
support_code = support_code.replace("crate::runtime::DataServiceExecutor", "teaql_provider_rusqlite::RusqliteMutationExecutor")
support_code = support_code.replace("crate::runtime::DataServiceMutationError", "teaql_provider_rusqlite::RusqliteMutationError")
support_code = support_code.replace("crate::runtime::DataServiceIdGenerator", "teaql_provider_rusqlite::RusqliteIdSpaceGenerator")
support_code = support_code.replace("crate::runtime::DataServicePool", "rusqlite::Connection")

# INSTEAD of regex, we can just replace the specific strings!
# Or we can just find the exact block using string find!
# Wait, let's just generate the TeaqlRuntime and impl TeaqlRuntime Manually and append it, and DELETE the old ones!
trait_start = support_code.find("    pub trait TeaqlRuntime {")
trait_end = support_code.find("    #[allow(async_fn_in_trait)]") # Next item
if trait_start != -1 and trait_end != -1:
    support_code = support_code[:trait_start] + "/* TEAQL_RUNTIME_PLACEHOLDER */\n" + support_code[trait_end:]

impl_start = support_code.find("    impl TeaqlRuntime for teaql_runtime::UserContext {")
impl_end = support_code.find("    pub struct QueryOptions {") # Next item
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
    q_content = f.read()

q_content = re.sub(r'pub mod request_support \{.*?\n\}\n', '', q_content, flags=re.DOTALL)
q_content = re.sub(r'pub mod request_support \{.*?\}\n', '', q_content, flags=re.DOTALL)

# Also remove everything between use teaql_core::SelectQuery; and pub struct Q;
import_end = q_content.find("use teaql_core::SelectQuery;\n")
if import_end != -1:
    q_start = q_content.find("pub struct Q;")
    if q_start != -1:
        q_content = q_content[:import_end + len("use teaql_core::SelectQuery;\n")] + "\n" + q_content[q_start:]

header = """pub mod request_support {
"""
q_content = q_content[:import_end + len("use teaql_core::SelectQuery;\n")] + "\n" + header + support_code + "}\n" + q_content[import_end + len("use teaql_core::SelectQuery;\n"):]

# Replace the execute methods
# Wait, they are ALREADY replaced by the previous run! Let's just restore q.rs.j2 and run again!

old_exec = re.search(r'    pub async fn execute_for_list\(self, ctx: &teaql_runtime::UserContext\).*?pub async fn execute_for_count\(self, ctx: &teaql_runtime::UserContext\) -> Result<u64, String> \{.*?\n    \}', q_content, re.DOTALL)
if old_exec:
    new_exec = """    pub async fn execute_for_list<'a, C>(self, ctx: &'a C) -> Result<teaql_core::SmartList<R>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized, R: teaql_core::Entity {
        let mut repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = runtime_relation_aggregates(&query_options);
        let query = apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates(&query, &relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        
        let facets = execute_facets(ctx, &outer_query, &query_options).await.map_err(teaql_runtime::RepositoryError::Runtime)?;
        attach_facets(&mut rows, facets);
        
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
        let mut repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = runtime_relation_aggregates(&query_options);
        let query = apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        Ok(rows.data.into_iter().map(teaql_core::Entity::into_record).collect())
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
    
# Remove duplicate imports just in case
q_content = q_content.replace('use crate::entities::*;\nuse teaql_core::Entity;\nuse teaql_core::Expr;\nuse std::marker::PhantomData;\n', '')

# Prepend header
header = """use crate::entities::*;
use teaql_core::Entity;
use teaql_core::Expr;
use std::marker::PhantomData;
use teaql_runtime::*;
use teaql_core::{Record, SmartList};
use std::future::Future;
use std::collections::BTreeMap;
use teaql_core::SelectQuery;

"""
q_content = header + q_content

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(q_content)

print("Re-ran fix_replace.py")
