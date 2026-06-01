import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

# Delete the remaining hardcoded count_tasks_as and count_tasks_with
old_hardcoded = """
    pub fn count_tasks_as(self, alias: impl Into<String>) -> Self {
        self.count_tasks_with(alias, crate::Q::tasks().unlimited())
    }
    pub fn count_tasks_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new(
            "task_list",
            alias,
            selection,
            true,
        ));
        self
    }"""
code = code.replace(old_hardcoded, "")

# And we also need to make sure count_{{ plural_name }} actually exists!
# Wait, why was count_tasks(self) missing?
# Because my previous regex might have accidentally eaten `count_{{ plural_name }}` if I put it wrong?
# No, my regex was: re.sub(r'\s*pub fn count_tasks\(self\).*?\n    \}\n', '\n', code)
# `count_{{ plural_name }}` doesn't literally match `count_tasks` in the template!

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)

