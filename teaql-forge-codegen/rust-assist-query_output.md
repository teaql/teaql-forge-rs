Please help me complete the query service business code for the `Platform` object.

To ensure absolute correctness of the API, please refer to and strictly imitate the following **real code example for `Platform`**.

### Standard Query Example (Reference)
Please carefully observe the `select_xxx()` methods and the strictly required `.purpose()` and `.comment()` cascade in the example code:

```rust
use teaql_core::{Entity, SmartList};
use crm_erp_service::{Q, Platform, TeaqlRepositoryProvider};

pub async fn query_example(
    ctx: &impl TeaqlRepositoryProvider, 
) -> Result<SmartList<Platform>, Box<dyn std::error::Error>>
{
    // [BEST PRACTICE]: Always start with _minimal() to avoid over-fetching and ensure compliance.
    let rows = Q::platforms_minimal()
        .select_id()
        .select_name()
        .select_create_time()
        .select_last_update_time()
        .select_version()
        // --- Filter and Order Options (Uncomment and customize if needed) ---
        // .with_id_is(/* value */)
        // .order_by_id_desc() // or _asc()
        // .with_name_is(/* value */)
        // .order_by_name_desc() // or _asc()
        // .with_create_time_is(/* value */)
        // .order_by_create_time_desc() // or _asc()
        // .with_last_update_time_is(/* value */)
        // .order_by_last_update_time_desc() // or _asc()
        // .with_version_is(/* value */)
        // .order_by_version_desc() // or _asc()
        // --------------------------------------------------------------------

        // --- Advanced Options: Fetch nested objects and tailor fields (uncomment and crop as needed) ---
        // -----------------------------------------------------------------------------------------------

        .limit(20)
        // [CRITICAL]: All execute_for_list / execute must cascade these two methods!
        .comment("why: Please explain why this query is needed here")
        .purpose("what: Please explain the action details of this query here")
        .execute_for_list(ctx)
        .await?;

    Ok(rows)
}
```

### Your Task
Please completely imitate the framework, imports, and syntax features of the above code to implement the real query logic for `Platform` based on my specific business needs. Please output the Rust source code directly.