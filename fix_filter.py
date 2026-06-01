import re

with open('generate_q_template.py', 'r') as f:
    content = f.read()

# Add filter_id to Request struct
struct_match = """pub struct {{ entity.name|capitalize }}Request {
    pub query: teaql_core::Query,
    pub query_options: teaql_core::QueryOptions,"""
struct_repl = """pub struct {{ entity.name|capitalize }}Request {
    pub query: teaql_core::Query,
    pub query_options: teaql_core::QueryOptions,
    pub filter_id: Option<u64>,"""
content = content.replace(struct_match, struct_repl)

# Add self.filter_id update to with_id_is
id_match = """    pub fn with_id_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        self.query = self.query.and_filter(Expr::eq("id", value));
        self
    }"""
id_repl = """    pub fn with_id_is(mut self, value: impl Into<teaql_core::Value>) -> Self {
        let val = value.into();
        self.query = self.query.and_filter(Expr::eq("id", val.clone()));
        if let teaql_core::Value::I64(v) = val {
            self.filter_id = Some(v as u64);
        } else if let teaql_core::Value::U64(v) = val {
            self.filter_id = Some(v);
        }
        self
    }"""
content = content.replace(id_match, id_repl)

# Add memory filtering to execute_for_list
exec_match = """        let entities: Vec<R> = smart_list.data.into_iter().filter_map(|r| R::from_record(r).ok()).collect();"""
exec_repl = """        let mut entities: Vec<R> = smart_list.data.into_iter().filter_map(|r| R::from_record(r).ok()).collect();
        if let Some(fid) = self.filter_id {
            entities.retain(|e| e.id() == fid);
        }"""
content = content.replace(exec_match, exec_repl)

with open('generate_q_template.py', 'w') as f:
    f.write(content)

print("Applied filter_id fix")
