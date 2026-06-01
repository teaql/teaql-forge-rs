import re

with open('/home/philip/githome/robot-task-board/expanded.rs', 'r') as f:
    lines = f.readlines()

request_support = "".join(lines[83:1313])

import subprocess
subprocess.run(["python3", "generate_q_template.py"], check=True)

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'r') as f:
    content = f.read()

# Add use request_support::*; to the top
content = content.replace("use std::marker::PhantomData;", "use std::marker::PhantomData;\nuse request_support::*;")

parts = content.split("}\n\n{%- for entity in entities %}")

new_content = parts[0] + "}\n\n" + request_support + "\n{%- for entity in entities %}" + parts[1]

with open('crates/teaql-forge-codegen/templates/src/q.rs.j2', 'w') as f:
    f.write(new_content)

print("Injected request_support into q.rs.j2")
