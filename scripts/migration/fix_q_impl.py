with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

old_q_impl = """impl Q {
{%- for entity in entities %}
{%- set suffix = "s" %}
{%- if entity.rust_module == "task_status" %}{% set suffix = "" %}{%- endif %}
{%- set func_name = entity.rust_module ~ suffix %}
    pub fn {{ func_name }}() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new()
    }
    pub fn {{ func_name }}_minimal() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new().select_self()
    }
    pub fn {{ func_name }}_with_children() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new().select_children()
    }
{%- endfor %}
}"""

new_q_impl = """impl Q {
{%- for entity in entities %}
{%- set suffix = "s" %}
{%- if entity.rust_module == "task_status" %}{% set suffix = "" %}{%- endif %}
{%- set func_name = entity.rust_module ~ suffix %}
    pub fn {{ func_name }}() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new()
            .select_self()
            .and_filter(teaql_core::Expr::gt("version", 0_i64))
    }
    pub fn {{ func_name }}_minimal() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new()
            .and_filter(teaql_core::Expr::gt("version", 0_i64))
    }
    pub fn {{ func_name }}_with_children() -> {{ entity.rust_struct }}Request {
        {{ entity.rust_struct }}Request::new()
            .unlimited()
            .select_self_fields()
            .enhance_children_if_needed()
    }
{%- endfor %}
}"""

if old_q_impl in code:
    code = code.replace(old_q_impl, new_q_impl)
else:
    print("Could not find old Q impl!")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)
