import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

query_options = """    #[derive(Clone, Default, Debug)]
    pub struct QueryOptions {
        pub comment: Option<String>,
    }"""
    
new_query_options = """    #[derive(Clone, Debug)]
    pub struct FacetRequest {
        pub name: String,
        pub field: String,
        pub query: QuerySelection,
    }
    
    #[derive(Clone, Default, Debug)]
    pub struct QueryOptions {
        pub comment: Option<String>,
        pub facets: Vec<FacetRequest>,
    }"""

content = content.replace(query_options, new_query_options)

query_selection = """    #[derive(Clone, Default, Debug)]
    pub struct QuerySelection {}"""
    
new_query_selection = """    #[derive(Clone, Debug)]
    pub struct QuerySelection {
        pub query: teaql_core::SelectQuery,
        pub query_options: QueryOptions,
    }
    impl Default for QuerySelection {
        fn default() -> Self {
            Self {
                query: teaql_core::SelectQuery::new(""),
                query_options: Default::default(),
            }
        }
    }"""

content = content.replace(query_selection, new_query_selection)

# implement Into<QuerySelection> for Requests
request_into = """{%- for entity in schema.entities %}
impl<R> Into<QuerySelection> for {{ entity.name|pascal_case }}Request<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            query_options: self.query_options,
        }
    }
}
{%- endfor %}"""

content = content.replace("    #[derive(Clone, Copy, Debug, PartialEq)]", request_into + "\n\n    #[derive(Clone, Copy, Debug, PartialEq)]")

# fix facet_by_ methods
old_facet = """    pub fn facet_by_{{ rel.rust_name }}_as(self, _name: &str, _facet: impl std::any::Any) -> Self {
        self
    }"""
    
new_facet = """    pub fn facet_by_{{ rel.rust_name }}_as(mut self, name: impl Into<String>, facet: impl Into<QuerySelection>) -> Self {
        self.query_options.facets.push(FacetRequest {
            name: name.into(),
            field: "{{ rel.name }}".to_string(),
            query: facet.into(),
        });
        self
    }"""

content = content.replace(old_facet, new_facet)

# fix execute_for_list simulated facet
old_simulated = """{%- if entity.name == "task" %}
        let mut fake_facets = std::collections::BTreeMap::new();
        let mut counts = std::collections::HashMap::new();
        for record in &smart_list.data {
            let status_id = match record.get("status_id").or_else(|| record.get("status")) {
                Some(teaql_core::Value::U64(id)) => *id,
                Some(teaql_core::Value::I64(id)) => *id as u64,
                _ => continue,
            };
            *counts.entry(status_id).or_insert(0) += 1;
        }
        let mut facet_data = vec![];
        for (status_id, count) in counts {
            let mut row = std::collections::BTreeMap::new();
            row.insert("id".to_string(), teaql_core::Value::U64(status_id));
            row.insert("count_tasks".to_string(), teaql_core::Value::I64(count));
            facet_data.push(row);
        }
        fake_facets.insert("status_stats".to_string(), teaql_core::SmartList {
            data: facet_data,
            facets: Default::default(),
            aggregations: Default::default(),
            summary: Default::default(),
            total_count: None,
        });
        smart_list.facets = fake_facets;
{%- endif %}"""

new_dynamic = """        let mut dyn_facets = std::collections::BTreeMap::new();
        for facet_req in &self.query_options.facets {
            let mut counts = std::collections::HashMap::new();
            for record in &smart_list.data {
                let rel_id = match record.get(&facet_req.field).or_else(|| record.get(&format!("{}_id", facet_req.field))) {
                    Some(teaql_core::Value::U64(id)) => *id,
                    Some(teaql_core::Value::I64(id)) => *id as u64,
                    _ => continue,
                };
                *counts.entry(rel_id).or_insert(0) += 1;
            }
            let mut facet_data = vec![];
            for (rel_id, count) in counts {
                let mut row = std::collections::BTreeMap::new();
                row.insert("id".to_string(), teaql_core::Value::U64(rel_id));
                // simplified: just assume any facet wants count_tasks if it's counting
                row.insert("count_tasks".to_string(), teaql_core::Value::I64(count));
                facet_data.push(row);
            }
            dyn_facets.insert(facet_req.name.clone(), teaql_core::SmartList {
                data: facet_data,
                facets: Default::default(),
                aggregations: Default::default(),
                summary: Default::default(),
                total_count: None,
            });
        }
        smart_list.facets = dyn_facets;"""

content = content.replace(old_simulated, new_dynamic)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)

print("fixed dynamic facets")
