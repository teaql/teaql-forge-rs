with open("generate_q_template.py", "r") as f:
    text = f.read()

# We need to replace {{ entity.rust_module }} with pluralized version in the Q struct methods
old_q = """{%- if entity.rust_module.ends_with("s") %}{% set suffix = "" %}{%- endif %}"""

new_q = """{%- if entity.rust_module == "task_status" %}{% set suffix = "" %}{%- endif %}"""

text = text.replace(old_q, new_q)
with open("generate_q_template.py", "w") as f:
    f.write(text)
