import re

files = ['generate_q_template.py', 'crates/teaql-forge-codegen/templates/src/q.rs.j2']

for file in files:
    with open(file, 'r') as f:
        content = f.read()

    # Fix clone
    clone_match = """            child_enhancements: self.child_enhancements.clone(),
            query_options: self.query_options.clone(),
            marker: PhantomData,"""
    clone_repl = """            child_enhancements: self.child_enhancements.clone(),
            query_options: self.query_options.clone(),
            filter_id: self.filter_id.clone(),
            marker: PhantomData,"""
    content = content.replace(clone_match, clone_repl)

    # Fix return_type
    ret_match = """            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
            marker: PhantomData,"""
    ret_repl = """            child_enhancements: self.child_enhancements,
            query_options: self.query_options,
            filter_id: self.filter_id,
            marker: PhantomData,"""
    content = content.replace(ret_match, ret_repl)

    with open(file, 'w') as f:
        f.write(content)

print("Fixed clone and return_type")
