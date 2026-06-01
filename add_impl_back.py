import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

impl_code = """
impl TeaqlRepositoryProvider for teaql_runtime::UserContext {
{%- for entity in entities %}
    type {{ entity.rust_struct }}Repository<'a> = teaql_runtime::ResolvedRepository<'a, teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor> where Self: 'a;

    fn {{ entity.rust_module }}_repository(&self) -> Result<Self::{{ entity.rust_struct }}Repository<'_>, teaql_runtime::ContextError> {
        self.resolve_repository::<teaql_provider_rusqlite::RusqliteDialect, teaql_provider_rusqlite::RusqliteMutationExecutor>("{{ entity.rust_struct }}")
    }
{%- endfor %}
}

#[derive(Clone, Debug, PartialEq)]
    pub struct QuerySelection
"""

content = content.replace("#[derive(Clone, Debug, PartialEq)]\n    pub struct QuerySelection", impl_code.strip())

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)
