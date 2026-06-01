import re

with open('generate_q_template.py', 'r') as f:
    content = f.read()

# Replace get_resource connection code
old_code = """let conn = ctx.get_resource::<std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>>().unwrap();"""

new_code = """let executor = ctx.get_resource::<teaql_provider_rusqlite::RusqliteMutationExecutor>().expect("Failed to get RusqliteMutationExecutor");
        let conn = executor.connection();"""

content = content.replace(old_code, new_code)

with open('generate_q_template.py', 'w') as f:
    f.write(content)

print("Fixed to use RusqliteMutationExecutor")
