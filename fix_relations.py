with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

old_mock_code = """{%- for relation in entity.relations %}
    pub fn with_{{ relation.name }}_matching(mut self, filter: impl Into<teaql_core::Expr>) -> Self {
        // Relation filter is unsupported in string AST natively without joins, so we mock it for now
        self
    }
{%- endfor %}

    pub fn facet_by_status_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }
    pub fn count_tasks(self) -> Self {
        self
    }"""

new_relation_code = """{%- for relation in entity.relations %}
    pub fn select_{{ relation.rust_field }}(mut self) -> Self {
        self.query = self.query.relation("{{ relation.name }}");
        self
    }

    pub fn select_{{ relation.rust_field }}_with(mut self, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query = self.query.relation_query("{{ relation.name }}", selection.clone().into_query());
        self.relation_selections.push(RelationSelection::new("{{ relation.name }}", selection));
        self
    }

    pub fn facet_by_{{ relation.rust_field }}_as(self, facet_name: impl Into<String>, request: impl Into<QuerySelection>) -> Self {
        self.facet_by_{{ relation.rust_field }}_as_with_options(facet_name, request, true)
    }

    pub fn facet_by_{{ relation.rust_field }}_as_with_options(
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

    pub fn with_{{ relation.rust_field }}_matching(mut self, request: impl Into<QuerySelection>) -> Self {
        let selection = request.into();
        self.query = self.query.relation_filter("{{ relation.name }}", selection.clone().into_query());
        self.relation_filters.push(RelationFilter::new("{{ relation.name }}", selection));
        self
    }
    
    pub fn have_{{ relation.rust_field }}(self) -> Self {
        self.with_{{ relation.rust_field }}_matching(teaql_core::SelectQuery::new("{{ relation.target }}"))
    }
{%- endfor %}

    pub fn count_tasks(mut self) -> Self {
        self.query_options.relation_aggregates.push(RelationAggregate::new("tasks", "count", "id"));
        self
    }
    
    pub fn count_task_execution_logs(mut self) -> Self {
        self.query_options.relation_aggregates.push(RelationAggregate::new("task_execution_logs", "count", "id"));
        self
    }
"""

if old_mock_code in code:
    code = code.replace(old_mock_code, new_relation_code)
else:
    print("Could not find old mock code!")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)
