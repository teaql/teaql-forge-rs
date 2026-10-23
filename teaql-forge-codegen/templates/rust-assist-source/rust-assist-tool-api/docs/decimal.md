# TeaQL Rust Tool API: decimal

Use `teaql_tool_context::prelude::ContextDecimalExt`. `ctx.decimal()` publishes `of`, `zero`, `one`, `add`, `sub`, `mul`,
`div`, `round`, `ceil`, `floor`, `abs`, `min`, `max`, `percent`, and `ratio` with
`rust_decimal::Decimal` values. Fallible calls retain `teaql_tool_core::Result`; intent-bearing
values use `.comment("business reason")`. Preserve the Decimal response; never convert through
floating point and reject unknown operations.
