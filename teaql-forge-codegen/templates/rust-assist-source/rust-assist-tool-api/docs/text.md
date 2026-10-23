# TeaQL Rust Tool API: text

Use `teaql_tool_context::prelude::ContextTextExt`. `ctx.text()` exposes `trim`, `lowercase`, `uppercase`, `capitalize`,
`to_snake_case`, `to_camel_case`, `to_pascal_case`, `to_kebab_case`,
`normalize_whitespace`, `contains`, `starts_with`, and `ends_with`. Every argument is `&str`.
Transforms return `String`; predicates return `bool` after the mandatory
`.comment("business reason")`. Preserve these native types and reject unknown operations.
