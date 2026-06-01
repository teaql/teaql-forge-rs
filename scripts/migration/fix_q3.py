import re
with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

# Delete ALL remaining hardcoded count_tasks_as and count_tasks_with
code = re.sub(r'\n    pub fn count_tasks_as\(self, alias: impl Into<String>\) -> Self \{[\s\S]*?pub fn count_tasks_with[\s\S]*?\}\n', '', code)
with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)

