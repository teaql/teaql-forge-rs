import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

hardcoded_aggs = """
{%- if entity.name == "task_status" %}
    pub fn count_tasks(self) -> Self {
        self.count_tasks_as("tasks_count")
    }
    pub fn count_tasks_as(self, alias: impl Into<String>) -> Self {
        self.count_tasks_with(alias, crate::Q::tasks().unlimited())
    }
    pub fn count_tasks_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new("task_list", alias, selection, true));
        self
    }
{%- endif %}

{%- if entity.name == "platform" %}
    pub fn count_tasks(self) -> Self {
        self.count_tasks_as("tasks_count")
    }
    pub fn count_tasks_as(self, alias: impl Into<String>) -> Self {
        self.count_tasks_with(alias, crate::Q::tasks().unlimited())
    }
    pub fn count_tasks_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new("task_list", alias, selection, true));
        self
    }
{%- endif %}

{%- if entity.name == "task" %}
    pub fn count_task_execution_logs(self) -> Self {
        self.count_task_execution_logs_as("task_execution_logs_count")
    }
    pub fn count_task_execution_logs_as(self, alias: impl Into<String>) -> Self {
        self.count_task_execution_logs_with(alias, crate::Q::task_execution_logs().unlimited())
    }
    pub fn count_task_execution_logs_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new("task_execution_log_list", alias, selection, true));
        self
    }
{%- endif %}
"""

# Insert before `    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {`
code = code.replace("    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {", hardcoded_aggs + "\n    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)

