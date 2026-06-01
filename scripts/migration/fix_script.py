import re

with open('make_perfect_q_rs.py', 'r') as f:
    code = f.read()

code = code.replace(
    'impl_end = request_support.find("}", impl_start) + 1',
    'impl_end = request_support.find("#[derive(Clone, Debug, PartialEq)]\\npub struct QuerySelection", impl_start)'
)

with open('make_perfect_q_rs.py', 'w') as f:
    f.write(code)
