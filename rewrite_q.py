import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

# 1. Add `aggregate_*` helpers at the end of the Request block (right before `pub async fn execute_for_list`)
helpers = """
    pub fn aggregate_count(mut self, alias: impl Into<String>) -> Self {
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
    }
"""
if "pub fn aggregate_count(" not in code:
    code = code.replace("    pub async fn execute_for_list", helpers + "\n    pub async fn execute_for_list")

# 2. Add property aggregations inside `{%- for field in entity.fields %}`
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
    }
{%- endfor %}"""
code = re.sub(r'\{%- endfor %\}(?=\n\n\s*\{%- for relation in entity.relations %\})', field_aggs, code, count=1)


# 3. Handle relations. In the relations loop, we should add relation counts. BUT we need `target_method` logic.
relation_aggs = """
{%- set target_method = relation.target_method %}
{%- set plural_name = relation.rust_name %}
    pub fn count_{{ plural_name }}(self) -> Self {
        self.count_{{ plural_name }}_as("count_{{ plural_name }}")
    }

    pub fn count_{{ plural_name }}_as(self, alias: impl Into<String>) -> Self {
{%- if relation.many %}
        self.count_{{ plural_name }}_with(alias, crate::Q::{{ target_method }}().unlimited())
{%- else %}
        self.aggregate_count_field("{{ relation.local_key }}", alias)
{%- endif %}
    }

{%- if relation.many %}
    pub fn count_{{ plural_name }}_with(mut self, alias: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query_options.relation_aggregates.push(RelationAggregate::new(
            "{{ relation.name }}",
            alias,
            selection,
            true,
        ));
        self
    }
{%- else %}
    pub fn facet_by_{{ relation.rust_name }}_as(self, facet_name: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        self.facet_by_{{ relation.rust_name }}_as_with_options(facet_name, request, true)
    }

    pub fn facet_by_{{ relation.rust_name }}_as_with_options(
        mut self,
        facet_name: impl Into<String>,
        request: impl Into<QuerySelection>,
        include_all_facets: bool,
    ) -> Self {
        self.query_options.facets.push(FacetRequest::new(
            facet_name,
            "{{ relation.name }}",
            request,
            include_all_facets,
        ));
        self
    }
{%- endif %}
{%- endfor %}"""

# Replace the endfor of relations with the new relation aggs
code = re.sub(r'\{%- endfor %\}(?=\n\n\s*pub fn facet_by_status_as)', relation_aggs, code, count=1)

# Remove the hardcoded facet_by_status_as and count_tasks
code = re.sub(r'\s*pub fn facet_by_status_as\(self, _name: &str, _facet: impl std::any::Any\) -> Self \{.*?\n    \}\n', '\n', code, flags=re.DOTALL)
code = re.sub(r'\s*pub fn count_tasks\(self\) -> Self \{.*?\n    \}\n', '\n', code, flags=re.DOTALL)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)

