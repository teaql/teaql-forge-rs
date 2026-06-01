import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    q_rs = f.read()

# 1. Add generic aggregation helpers if missing
agg_helpers = """    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {
        self.query = self.query.count(alias);
        self
    }

    pub fn aggregate_count_field(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.count_field(field, alias);
        self
    }

    pub fn aggregate_sum(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.sum(field, alias);
        self
    }

    pub fn aggregate_avg(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.avg(field, alias);
        self
    }

    pub fn aggregate_min(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.min(field, alias);
        self
    }

    pub fn aggregate_max(mut self, field: impl Into<String>, alias: impl Into<String>) -> Self {
        self.query = self.query.max(field, alias);
        self
    }"""

if "pub fn aggregate_count_field" not in q_rs:
    # Insert before the closing brace of the impl block
    # We can just put it at the end of the fields loop before relations
    pass # Wait, let's just insert it at the end of the impl block

field_aggs = """    pub fn count_{{ field.name }}(self) -> Self {
        self.count_{{ field.name }}_as("{{ field.name }}_count")
    }

    pub fn count_{{ field.name }}_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_count_field("{{ field.name }}", alias)
    }

    pub fn sum_{{ field.name }}(self) -> Self {
        self.sum_{{ field.name }}_as("sum_{{ field.name }}")
    }

    pub fn sum_{{ field.name }}_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_sum("{{ field.name }}", alias)
    }

    pub fn avg_{{ field.name }}(self) -> Self {
        self.avg_{{ field.name }}_as("avg_{{ field.name }}")
    }

    pub fn avg_{{ field.name }}_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_avg("{{ field.name }}", alias)
    }

    pub fn min_{{ field.name }}(self) -> Self {
        self.min_{{ field.name }}_as("min_{{ field.name }}")
    }

    pub fn min_{{ field.name }}_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_min("{{ field.name }}", alias)
    }

    pub fn max_{{ field.name }}(self) -> Self {
        self.max_{{ field.name }}_as("max_{{ field.name }}")
    }

    pub fn max_{{ field.name }}_as(self, alias: impl Into<String>) -> Self {
        self.aggregate_max("{{ field.name }}", alias)
    }"""

relation_aggs = """{%- set target_method = relation.target_module %}
{%- if target_method != "task_status" %}{%- set target_method = target_method ~ "s" %}{%- endif %}
{%- set plural_name = relation.target_module ~ "s" %}
{%- if relation.name == "task_execution_log_list" %}{%- set plural_name = "task_execution_logs" %}{%- endif %}
{%- if relation.name == "task_list" %}{%- set plural_name = "tasks" %}{%- endif %}
    pub fn count_{{ plural_name }}(self) -> Self {
        self.count_{{ plural_name }}_as("{{ plural_name }}_count")
    }

    pub fn count_{{ plural_name }}_as(self, alias: impl Into<String>) -> Self {
        self.count_{{ plural_name }}_with(alias, crate::Q::{{ target_method }}().unlimited())
    }

    pub fn count_{{ plural_name }}_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new(
            "{{ relation.name }}",
            alias,
            selection,
            true,
        ));
        self
    }"""

# I need to carefully replace the existing hardcoded count_tasks with relation_aggs
# But relation_aggs should be inside the `{%- for relation in entity.relations %}` block!

# Let's read the current template and do manual replacements.
