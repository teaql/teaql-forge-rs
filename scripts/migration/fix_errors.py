import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

content = content.replace("crate::runtime::DataServiceMutationError", "teaql_provider_rusqlite::MutationExecutorError")
content = content.replace("teaql_provider_rusqlite::RusqliteExecutor", "teaql_provider_rusqlite::RusqliteMutationExecutor")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)
