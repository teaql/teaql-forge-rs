with open('crates/teaql-forge-codegen/templates/src/entities/entity.rs.j2', 'r') as f:
    code = f.read()

old_code = """            relations: vec![
                {%- if entity.name == "task" %}"""

new_code = """            relations: vec![
                {%- if entity.name == "task_status" %}
                teaql_core::RelationDescriptor {
                    name: "task_list".to_string(),
                    target_entity: "task".to_string(),
                    local_key: "id".to_string(),
                    foreign_key: "status_id".to_string(),
                    many: true,
                    attach: true,
                    delete_missing: false,
                },
                {%- endif %}
                {%- if entity.name == "platform" %}
                teaql_core::RelationDescriptor {
                    name: "task_list".to_string(),
                    target_entity: "task".to_string(),
                    local_key: "id".to_string(),
                    foreign_key: "platform_id".to_string(),
                    many: true,
                    attach: true,
                    delete_missing: false,
                },
                {%- endif %}
                {%- if entity.name == "task" %}"""

if old_code in code:
    code = code.replace(old_code, new_code)
else:
    print("old_code not found")

with open('crates/teaql-forge-codegen/templates/src/entities/entity.rs.j2', 'w') as f:
    f.write(code)
