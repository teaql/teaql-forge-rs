# TeaQL Rust Tool API: time

Dependency: `teaql-tool-context = { version = "2.0.0", features = ["std"] }`.
Import `teaql_tool_context::prelude::ContextTimeExt`.

Exact facade: `ctx.time()`. Verified methods are `now() -> MustComment<DateTime<Utc>>`,
`today() -> NaiveDate`, `parse_date(&str) -> Result<NaiveDate>`,
`parse_datetime(&str) -> Result<DateTime<Utc>>`, `add_days(DateTime<Utc>, i64)`,
`add_months(DateTime<Utc>, u32)`, `start_of_day(DateTime<Utc>)`,
`end_of_day(DateTime<Utc>)`, `days_between(DateTime<Utc>, DateTime<Utc>) -> i64`, and
`to_timezone(DateTime<Utc>, &str) -> Result<DateTime<Tz>>`.

Values requiring intent are unwrapped with `.comment("business reason")`; preserve the native
chrono value or `teaql_tool::Result` and reject unknown operations.
