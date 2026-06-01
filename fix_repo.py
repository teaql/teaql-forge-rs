import re

with open('generate_q_template.py', 'r') as f:
    content = f.read()

# Replace execute_for_list body
old_list = """        // FAKE IT TIL WE MAKE IT
        let records = Vec::new();"""

new_list = """        let sql = format!("SELECT * FROM {}_data", "{{ entity.name }}");
        let conn = ctx.get_resource::<std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>>().unwrap();
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql).unwrap();
        let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).unwrap();
        let mut records = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            let mut record = teaql_core::Record::new();
            for (i, name) in column_names.iter().enumerate() {
                if let Ok(val) = row.get::<_, i64>(i) {
                    record.insert(name, teaql_core::Value::I64(val));
                } else if let Ok(val) = row.get::<_, f64>(i) {
                    record.insert(name, teaql_core::Value::F64(val));
                } else if let Ok(val) = row.get::<_, String>(i) {
                    record.insert(name, teaql_core::Value::String(val));
                } else if let Ok(val) = row.get::<_, bool>(i) {
                    record.insert(name, teaql_core::Value::Bool(val));
                }
            }
            records.push(record);
        }"""

content = content.replace(old_list, new_list)

old_count = """    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<u64, String> {
        Ok(0)
    }"""

new_count = """    pub async fn execute_for_count(self, ctx: &teaql_runtime::UserContext) -> Result<u64, String> {
        let sql = format!("SELECT COUNT(*) FROM {}_data", "{{ entity.name }}");
        let conn = ctx.get_resource::<std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>>().unwrap();
        let conn = conn.lock().unwrap();
        let count: u64 = conn.query_row(&sql, [], |row| row.get(0)).unwrap();
        Ok(count)
    }"""

content = content.replace(old_count, new_count)

with open('generate_q_template.py', 'w') as f:
    f.write(content)

print("Restored rusqlite calls")
