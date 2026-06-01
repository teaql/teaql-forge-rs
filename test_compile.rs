fn main() {
    let query = teaql_core::Query::new("task");
    let dialect = teaql_provider_rusqlite::RusqliteDialect;
    let compiled = teaql_core::SqlDialect::compile_query(&dialect, &query);
}
