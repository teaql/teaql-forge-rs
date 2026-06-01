import re

with open('/tmp/request_support.rs', 'r') as f:
    rs_content = f.read()

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    q_content = f.read()

start_idx = q_content.find("pub mod request_support {")
end_idx = q_content.find("use request_support::*;")

new_module = "pub mod request_support {\n" + "\n".join("    " + line if line else "" for line in rs_content.split("\n")) + "\n}\n"

q_content = q_content[:start_idx] + new_module + q_content[end_idx:]

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(q_content)
