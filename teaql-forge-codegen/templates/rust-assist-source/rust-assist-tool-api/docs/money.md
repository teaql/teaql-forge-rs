# TeaQL Rust Tool API: money

Use `teaql_tool_context::prelude::ContextMoneyExt`. `ctx.money()` publishes `of`, `zero`, `add`, `sub`, `mul`, `div`,
`round`, `allocate`, `same_currency`, and `format`. Fallible calls retain
`teaql_tool_core::Result`; intent-bearing values use `.comment("business reason")`. Preserve the
native Money value, reject currency mismatch and unknown operations explicitly.
