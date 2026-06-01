import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

# Replace the field loop start
old_loop = """{%- for field in entity.fields %}
    pub fn select_{{ field.rust_name }}(mut self) -> Self {"""

new_loop = """{%- for field in entity.fields %}
{%- set is_relation_field = false %}
{%- for relation in entity.relations %}
{%- if relation.name == field.name %}{% set is_relation_field = true %}{% endif %}
{%- endfor %}
{%- if not is_relation_field %}
    pub fn select_{{ field.rust_name }}(mut self) -> Self {"""

if old_loop in code:
    code = code.replace(old_loop, new_loop)
else:
    print("old_loop not found!")

# Replace the field loop end
old_end = """    pub fn with_{{ field.rust_name }}_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("{{ field.name }}", format!("%{}", value.into())));
        self
    }
    {%- endif %}
{%- endfor %}"""

new_end = """    pub fn with_{{ field.rust_name }}_ends_with(mut self, value: impl Into<String>) -> Self {
        self.query = self.query.and_filter(Expr::like("{{ field.name }}", format!("%{}", value.into())));
        self
    }
    {%- endif %}
{%- endif %}
{%- endfor %}"""

if old_end in code:
    code = code.replace(old_end, new_end)
else:
    print("old_end not found!")

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)
