with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

code = code.replace("{{ relation.rust_field }}", "{{ relation.name }}")
code = code.replace('teaql_core::Expr::in_subquery(\n            "{{ relation.name }}",\n            "{{ relation.target_module }}",', 'teaql_core::Expr::in_subquery(\n            "{{ relation.name }}",\n            teaql_core::EntityDescriptor::new("{{ relation.target_module }}"),')

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)
