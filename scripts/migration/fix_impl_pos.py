import re

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

impl_block = """{%- for entity in entities %}
impl<R> Into<QuerySelection> for {{ entity.rust_struct }}Request<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            query_options: self.query_options,
        }
    }
}
{%- endfor %}"""

content = content.replace(impl_block, "")

# insert it inside the entity loop where it's defined
entity_loop = """impl<R> Clone for {{ entity.rust_struct }}Request<R> {"""
new_entity_loop = """impl<R> Into<QuerySelection> for {{ entity.rust_struct }}Request<R> {
    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            query_options: self.query_options,
        }
    }
}

impl<R> Clone for {{ entity.rust_struct }}Request<R> {"""

content = content.replace(entity_loop, new_entity_loop)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(content)

print("fixed impl position")
