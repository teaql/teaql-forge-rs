Please help me complete the paginated list (List Page) service business code for the `Platform` object.

To ensure absolute correctness of the API, please refer to and strictly imitate the following **real list page code example for `Platform`**.

### Standard Paginated List Example (Reference)
Please carefully observe the `execute_for_page()` method with offset/limit parameters, the `_minimal()` query start, and the strictly required `.purpose()` and `.comment()` cascade:

```rust
use teaql_core::Entity;
use crm_erp_service::{Q, Platform, TeaqlRepositoryProvider};
use teaql_core::SmartList;
use serde_json::Value as JsonValue;

pub async fn list_page_example(
    ctx: &impl TeaqlRepositoryProvider, 
    offset: u64, 
    limit: u64, 
    ui_filters_json: &JsonValue
) -> Result<SmartList<Platform>, Box<dyn std::error::Error>>
{
    // [BEST PRACTICE]: Always start with _minimal() to avoid over-fetching
    let page = Q::platforms_minimal()
        .select_id()
        .select_name()
        .select_create_time()
        .select_last_update_time()
        .select_version()
        .filter_with_json(ui_filters_json.clone()) // Dynamic UI Search Support

        // --- Drill-Down Options: Fetch nested objects and tailor fields (uncomment as needed) ---
        // -----------------------------------------------------------------------------------------------

        // --- Facet Options: Aggregate referenced objects directly (uncomment as needed) ---
        // -----------------------------------------------------------------------------------------------

        // [CRITICAL]: All execute methods must cascade these two methods!
        .comment("why: Need to display a paginated list on the frontend")
        .purpose("what: Query paginated list of Platform")
        .execute_for_page(ctx, offset, limit)
        .await?;

    Ok(page)
}
```

### Your Task
Please completely imitate the framework, imports, and syntax features of the above code to implement the real paginated list logic for `Platform` based on my specific business needs. Please output the Rust source code directly.