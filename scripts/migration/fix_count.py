import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

old_count = """    pub fn count_tasks(mut self) -> Self {
        self.query_options.relation_aggregates.push(RelationAggregate::new("tasks", "count", "id", false));
        self
    }
    
    pub fn count_task_execution_logs(mut self) -> Self {
        self.query_options.relation_aggregates.push(RelationAggregate::new("task_execution_logs", "count", "id", false));
        self
    }"""

new_count = """    pub fn count_tasks(self) -> Self {
        self.count_tasks_as("count_tasks")
    }

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

if old_count in code:
    code = code.replace(old_count, new_count)
else:
    print("old_count not found!")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)
