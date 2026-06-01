import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

impl_start = content.find("impl TeaqlRepositoryProvider for teaql_runtime::UserContext {")
impl_end = content.find("#[derive(Clone, Debug, PartialEq)]\n    pub struct QuerySelection", impl_start)

if impl_start != -1 and impl_end != -1:
    content = content[:impl_start] + content[impl_end:]

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)
