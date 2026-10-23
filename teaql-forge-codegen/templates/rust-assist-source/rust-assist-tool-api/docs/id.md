# TeaQL Rust Tool API: id

Use `teaql_tool_context::prelude::ContextIdExt`. `ctx.id()` exposes `uuid()`, `uuid_v7()`, `ulid()`, `nanoid()`, and
`with_prefix(&str)`, each returning `String`. Generated IDs requiring intent use
`.comment("business reason")`. No `snowflake()` or `short()` method is published by this version.
Preserve the native response shape and reject unknown operations.
