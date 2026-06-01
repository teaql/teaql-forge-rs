with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    code = f.read()

# Fix range fields
code = code.replace('range.min', 'range.start')
code = code.replace('range.max', 'range.end')

# Fix QuerySelection into
old_into = """    fn into(self) -> QuerySelection {
        QuerySelection {
            query: self.query,
            query_options: self.query_options,
        }
    }"""
new_into = """    fn into(self) -> request_support::QuerySelection {
        request_support::QuerySelection {
            query: self.query,
            query_options: self.query_options,
            child_enhancements: self.child_enhancements,
            relation_filters: self.relation_filters,
            relation_selections: self.relation_selections,
        }
    }"""
code = code.replace(old_into, new_into)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(code)

with open('make_perfect_q_rs.py', 'r') as f:
    make_code = f.read()

make_code = make_code.replace(
    'row.get("COUNT_ALIAS")).and_then(|v| v.try_u64())',
    'row.get("COUNT_ALIAS").and_then(|v| v.try_u64())'
)
make_code = make_code.replace(
    'row.get("COUNT_ALIAS").and_then(|v| v.try_u64())',
    'row.get("COUNT_ALIAS").cloned()).and_then(|v| v.try_u64())'
)

# And fix teaql_runtime::TeaqlRepositoryError to request_support::TeaqlRepositoryError
make_code = make_code.replace('teaql_runtime::TeaqlRepositoryError', 'request_support::TeaqlRepositoryError')

with open('make_perfect_q_rs.py', 'w') as f:
    f.write(make_code)
