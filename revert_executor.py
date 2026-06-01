import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

content = content.replace("crate::runtime::ServiceRuntimeExecutor>", "teaql_provider_rusqlite::RusqliteMutationExecutor>")
content = content.replace("crate::runtime::ServiceRuntimeExecutor>(", "teaql_provider_rusqlite::RusqliteMutationExecutor>(")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)
