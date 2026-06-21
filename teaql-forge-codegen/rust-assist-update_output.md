Please help me complete the update (Update/Modify) service business code for the `Platform` object.

To ensure absolute correctness of the API, please refer to and strictly imitate the following **real update code example for `Platform`**.

### Standard Update Example (Reference)
Please carefully observe the querying mechanism, the `update_xxx()` property setting methods, and the strictly required `.audit_as()` and `.save()` cascade in the example code:

```rust
use teaql_core::Entity;
use crm_erp_service::{Q, Platform, TeaqlRepositoryProvider, AuditedSave};

pub async fn update_example(
    ctx: &impl TeaqlRepositoryProvider, 
    entity_id: i64,
    /* YOUR DTO / PARAMS HERE */
) -> Result<Option<Platform>, Box<dyn std::error::Error>>
{
    // 1. Fetch the entity first (Use _minimal to avoid over-fetching during updates)
    if let Some(mut existing_entity) = Q::platforms_minimal()
        .with_id_is(entity_id)
        .purpose("what: Find entity for modification")
        .comment("why: Need to update properties")
        .execute(ctx)
        .await?
    {
        // 2. Set updated property values (replace dummy data with actual inputs)
        // existing_entity.update_id(/* input data */);
        // existing_entity.update_name(/* input data */);
        // existing_entity.update_create_time(/* input data */);
        // existing_entity.update_last_update_time(/* input data */);
        // existing_entity.update_version(/* input data */);
        // 3. CRITICAL: Security audit constraints must be attached before calling save()!
        existing_entity.audit_as("Why this update operation was executed (audit record)")
            .save(ctx).await?;

        return Ok(Some(existing_entity));
    }

    Ok(None)
}
```

### Your Task
Please completely imitate the framework, imports, and syntax features of the above code to implement the real update logic for `Platform` based on my specific business needs. Please output the Rust source code directly.