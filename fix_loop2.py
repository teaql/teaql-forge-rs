import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

old_loop = """{%- for domain in schema.domains %}
{%- for entity in domain.entities %}
impl<R> Into<QuerySelection> for {{ entity.name|pascal_case }}Request<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            query_options: self.query_options,
        }
    }
}
{%- endfor %}
{%- endfor %}"""

new_loop = """{%- for entity in entities %}
impl<R> Into<QuerySelection> for {{ entity.name|pascal_case }}Request<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            query_options: self.query_options,
        }
    }
}
{%- endfor %}"""

content = content.replace(old_loop, new_loop)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)

print("fixed loop 2")
