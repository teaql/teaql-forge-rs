with open('clean_replace.py', 'r') as f:
    code = f.read()

code = code.replace("Ok(rows.data.into_iter().map(teaql_core::Entity::into_record).collect())", "Ok(rows.data)")
# Wait! In execute_for_records, I can just write it using execute_for_list(ctx).await? No, execute_for_list(ctx).await? requires R: Entity, which is fine!
# But fetch_smart_list_with_relation_aggregates is fine too, just Ok(rows.data)

with open('clean_replace.py', 'w') as f:
    f.write(code)
