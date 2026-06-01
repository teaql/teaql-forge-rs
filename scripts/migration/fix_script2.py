with open('make_perfect_q_rs.py', 'r') as f:
    code = f.read()

code = code.replace(
    'teaql_runtime::TeaqlRepositoryError',
    'request_support::TeaqlRepositoryError'
)
code = code.replace(
    'row.get("COUNT_ALIAS").cloned().and_then(teaql_core::Value::try_u64)',
    'row.get("COUNT_ALIAS").and_then(|v| v.try_u64())'
)
code = code.replace(
    'row.get("COUNT_ALIAS")).and_then(teaql_core::Value::try_u64)',
    'row.get("COUNT_ALIAS").and_then(|v| v.try_u64())'
)

with open('make_perfect_q_rs.py', 'w') as f:
    f.write(code)
