with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

old_code = """    pub fn with_{{ relation.rust_field }}_matching(mut self, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query = self.query.relation_filter("{{ relation.name }}", selection.clone().into_query());
        self.relation_filters.push(RelationFilter::new("{{ relation.name }}", selection));
        self
    }"""

new_code = """    pub fn with_{{ relation.rust_field }}_matching(mut self, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query = self.query.and_filter(teaql_core::Expr::in_subquery(
            "{{ relation.name }}",
            "{{ relation.target_module }}",
            selection.query.clone(),
            "id",
        ));
        self.relation_filters.push(RelationFilter::new("{{ relation.name }}", selection));
        self
    }"""

code = code.replace(old_code, new_code)

old_aggregate1 = 'RelationAggregate::new("tasks", "count", "id")'
new_aggregate1 = 'RelationAggregate::new("tasks", "count", "id", false)'
code = code.replace(old_aggregate1, new_aggregate1)

old_aggregate2 = 'RelationAggregate::new("task_execution_logs", "count", "id")'
new_aggregate2 = 'RelationAggregate::new("task_execution_logs", "count", "id", false)'
code = code.replace(old_aggregate2, new_aggregate2)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)
