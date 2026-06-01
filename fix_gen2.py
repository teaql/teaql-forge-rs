import re

with open('generate_q_template.py', 'r') as f:
    content = f.read()

# Fix initializer of Request
init_match = """    pub fn {{ func_name }}() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new()
    }
    pub fn {{ func_name }}_minimal() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request {
            query: teaql_core::SelectQuery::new("{{ entity.name }}"),
            relation_selections: Vec::new(),
            relation_filters: Vec::new(),
            child_enhancements: Vec::new(),
            query_options: QueryOptions::default(),
            marker: PhantomData,
        }
    }"""
init_repl = """    pub fn {{ func_name }}() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new()
    }
    pub fn {{ func_name }}_minimal() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request {
            query: teaql_core::SelectQuery::new("{{ entity.name }}"),
            relation_selections: Vec::new(),
            relation_filters: Vec::new(),
            child_enhancements: Vec::new(),
            query_options: QueryOptions::default(),
            filter_id: None,
            marker: PhantomData,
        }
    }"""
content = content.replace(init_match, init_repl)

# Fix filter logic
logic_match = """        let mut entities: Vec<R> = smart_list.data.into_iter().filter_map(|r| R::from_record(r).map_err(|e| println!("Parse error: {}", e)).ok()).collect();
        if let Some(fid) = self.filter_id {
            entities.retain(|e| e.id() == fid);
        }"""
logic_repl = """        let mut records = smart_list.data;
        if let Some(fid) = self.filter_id {
            records.retain(|r| match r.get("id") {
                Some(teaql_core::Value::U64(v)) => *v == fid,
                Some(teaql_core::Value::I64(v)) => *v as u64 == fid,
                _ => false,
            });
        }
        let entities = records.into_iter().filter_map(|r| R::from_record(r).map_err(|e| println!("Parse error: {}", e)).ok()).collect();"""
content = content.replace(logic_match, logic_repl)

with open('generate_q_template.py', 'w') as f:
    f.write(content)

print("generate_q_template.py fixed logic and initializer")
