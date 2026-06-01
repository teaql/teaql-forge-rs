import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

content = content.replace("crate::runtime::DataServiceDialect", "teaql_provider_rusqlite::RusqliteDialect")
content = content.replace("crate::runtime::DataServiceExecutor", "teaql_provider_rusqlite::RusqliteExecutor")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)
