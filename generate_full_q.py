import re

with open('/home/philip/githome/robot-task-board/expanded.rs', 'r') as f:
    expanded = f.read()

# Extract from pub mod request_support (or just the traits)
# Let's extract from "pub trait TeaqlRecordRepository" to the end of "impl TeaqlRuntime for teaql_runtime::UserContext"
traits_match = re.search(r'(    pub trait TeaqlRecordRepository \{.*?    impl TeaqlRuntime for teaql_runtime::UserContext \{.*?    \})', expanded, re.DOTALL)
if not traits_match:
    print("Could not find traits")
    exit(1)
traits_str = traits_match.group(1)

# Template the traits
# In TeaqlRuntime:
#         type PlatformRepository<'a>: TeaqlEntityRepository + 'a where
#             Self: 'a;
#         fn platform_repository(&self)
#         -> Result<Self::PlatformRepository<'_>, ContextError>;
# We can replace this with a Jinja loop
runtime_trait_loop = """
    pub trait TeaqlRuntime {
{%- for entity in entities %}
        type {{ entity.rust_struct }}Repository<'a>: TeaqlEntityRepository + 'a where Self: 'a;
        fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, ContextError>;
{%- endfor %}
        fn user_context(&self) -> &UserContext;
        fn fetch_facet_smart_list(&self, entity: &str, query: &SelectQuery, relation_aggregates: &[RuntimeRelationAggregate]) -> Result<SmartList<Record>, RuntimeError>;
    }
"""
traits_str = re.sub(r'    pub trait TeaqlRuntime \{.*?    \}', runtime_trait_loop, traits_str, flags=re.DOTALL)

# In impl TeaqlRuntime for teaql_runtime::UserContext:
#         type PlatformRepository<'a> =
#             teaql_runtime::ResolvedRepository<'a,
#             crate::runtime::DataServiceDialect,
#             crate::runtime::DataServiceExecutor> where Self: 'a;
#         fn platform_repository(&self)
#             -> Result<Self::PlatformRepository<'_>, ContextError> {
#             self.resolve_repository::<crate::runtime::DataServiceDialect,
#                 crate::runtime::DataServiceExecutor>("Platform")
#         }
runtime_impl_loop = """
    impl TeaqlRuntime for teaql_runtime::UserContext {
{%- for entity in entities %}
        type {{ entity.rust_struct }}Repository<'a> = teaql_runtime::ResolvedRepository<'a, crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor> where Self: 'a;
        fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, ContextError> {
            self.resolve_repository::<crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor>("{{ entity.rust_struct }}")
        }
{%- endfor %}
        fn user_context(&self) -> &UserContext {
            self
        }
        fn fetch_facet_smart_list(&self, entity: &str, query: &SelectQuery, relation_aggregates: &[RuntimeRelationAggregate]) -> Result<SmartList<Record>, RuntimeError> {
            self.resolve_repository::<crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor>(entity.to_owned()).map_err(|err| RuntimeError::Graph(err.to_string()))?.fetch_smart_list_with_relation_aggregates(query, relation_aggregates).map_err(|err| RuntimeError::Graph(err.to_string()))
        }
    }
"""
traits_str = re.sub(r'    impl TeaqlRuntime for teaql_runtime::UserContext \{.*?    \}', runtime_impl_loop, traits_str, flags=re.DOTALL)


# Now extract QuerySelection, FacetRequest etc.
# we can just use the ones I already have, but we need to extract execute_facets
facets_match = re.search(r'(    pub\(crate\) fn execute_facets<C>\(ctx: &C.*?    \})', expanded, re.DOTALL)
if facets_match:
    execute_facets_str = facets_match.group(1)
else:
    execute_facets_str = ""
    
attach_facets_match = re.search(r'(    pub\(crate\) fn attach_facets.*?    \})', expanded, re.DOTALL)
if attach_facets_match:
    attach_facets_str = attach_facets_match.group(1)
else:
    attach_facets_str = ""

# Now build the full q.rs.j2
with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    q_content = f.read()

# Find where to insert traits
import_insert = """use teaql_core::Entity;
use teaql_core::Expr;
use std::marker::PhantomData;
use teaql_runtime::*;
use teaql_core::{Record, SmartList};
use std::future::Future;

"""
q_content = q_content.replace('use std::marker::PhantomData;', import_insert)
q_content = q_content.replace('pub mod request_support {', traits_str + '\n' + execute_facets_str + '\n' + attach_facets_str + '\npub mod request_support {')

# Replace the execute methods
# Wait, let's just write the replacement for the execute methods inside the loop!

old_exec = re.search(r'    pub async fn execute_for_list\(self, ctx: &teaql_runtime::UserContext\).*?pub async fn execute_for_count\(self, ctx: &teaql_runtime::UserContext\) -> Result<u64, String> \{.*?\n    \}', q_content, re.DOTALL)

new_exec = """    pub async fn execute_for_list<'a, C>(self, ctx: &'a C) -> Result<teaql_core::SmartList<crate::{{ entity.rust_struct }}>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
        let repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let outer_query = self.query.clone();
        let relation_aggregates = teaql_runtime::runtime_relation_aggregates(&query_options);
        let query = teaql_runtime::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let mut rows = repository.fetch_enhanced_entities_with_relation_aggregates(&query, &relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
        
        // Since the user wants facet to be "usable except facet", we just mock the facet output for now if we can't compile execute_facets
        // BUT wait, we CAN compile execute_facets if we include it! We already included execute_facets!
        // Let's use it!
        let facets = execute_facets(ctx, &outer_query, &query_options).await.map_err(teaql_runtime::RepositoryError::Runtime)?;
        attach_facets(&mut rows, facets);
        
        Ok(rows)
    }

    pub async fn execute_for_first<'a, C>(self, ctx: &'a C) -> Result<Option<crate::{{ entity.rust_struct }}>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
        let rows = self.limit(1).execute_for_list(ctx).await?;
        Ok(rows.data.into_iter().next())
    }

    pub async fn execute_for_one<'a, C>(self, ctx: &'a C) -> Result<Option<crate::{{ entity.rust_struct }}>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
        self.execute_for_first(ctx).await
    }

    pub async fn execute_for_records<'a, C>(self, ctx: &'a C) -> Result<Vec<teaql_core::Record>, teaql_runtime::TeaqlRepositoryError<C::{{ entity.rust_struct }}Repository<'a>>> where C: TeaqlRuntime + ?Sized {
        let repository = ctx.{{ entity.rust_module }}_repository().map_err(|err| teaql_runtime::RepositoryError::Runtime(teaql_runtime::RuntimeError::Graph(err.to_string())))?;
        let query_options = self.query_options.clone();
        let relation_aggregates = teaql_runtime::runtime_relation_aggregates(&query_options);
        let query = teaql_runtime::apply_runtime_metadata(self.query, &query_options, &self.child_enhancements);
        let rows = repository.fetch_smart_list_with_relation_aggregates(&query, &relation_aggregates).map_err(|err| teaql_runtime::RuntimeError::Graph(err.to_string()))?;
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

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(q_content)

print("Generated full q.rs.j2!")
