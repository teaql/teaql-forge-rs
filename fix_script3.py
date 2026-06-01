with open('make_perfect_q_rs.py', 'r') as f:
    code = f.read()

code = code.replace(
    'row.get("COUNT_ALIAS").and_then(|v| v.try_u64())',
    'row.get("COUNT_ALIAS")).and_then(|v| v.try_u64())'
)

with open('make_perfect_q_rs.py', 'w') as f:
    f.write(code)
