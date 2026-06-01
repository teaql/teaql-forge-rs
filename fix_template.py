import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

# Replace with_id_is
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

# Replace execute_for_list entities creation
exec_match = """        let mut entities: Vec<R> = smart_list.data.into_iter().filter_map(|r| R::from_record(r).ok()).collect();"""
exec_repl = """        let mut entities: Vec<R> = smart_list.data.into_iter().filter_map(|r| R::from_record(r).ok()).collect();
        if let Some(fid) = self.filter_id {
            entities.retain(|e| e.id() == fid);
        }"""
content = content.replace(exec_match, exec_repl)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)

print("Template fixed")
