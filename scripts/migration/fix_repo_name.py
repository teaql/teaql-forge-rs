with open('make_perfect_q_rs.py', 'r') as f:
    code = f.read()

code = code.replace(
    'self.resolve_repository::<crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor>("{{ entity.rust_struct }}")',
    'self.resolve_repository::<crate::runtime::DataServiceDialect, crate::runtime::DataServiceExecutor>("{{ entity.rust_module }}")'
)

with open('make_perfect_q_rs.py', 'w') as f:
    f.write(code)
