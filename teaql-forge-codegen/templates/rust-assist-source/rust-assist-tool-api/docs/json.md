# TeaQL Rust Tool API: JSON

Use `teaql_tool_context::prelude::ContextJsonExt` and `ctx.json()`.
The conformance surface includes `parse`, `stringify`, `stringify_pretty`, `get`, `set`,
`remove`, `has`, `merge`, `diff`, and `patch`. Fallible calls retain
`teaql_tool_core::Result`; intent-bearing values use `.comment("business reason")`.
Preserve serde JSON values and reject unknown operations or pointer paths.
