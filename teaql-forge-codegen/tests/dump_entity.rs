use teaql_forge_codegen::context::*;

#[test]
fn dump() {
    let e = RenderEntity {
        name: "Test".to_string(),
        line_number: 1,
        xml_path: "".to_string(),
        rust_struct: "Test".to_string(),
        rust_module: "test".to_string(),
        rust_plural: "tests".to_string(),
        table_name: "tests".to_string(),
        data_service: None,
        audit_mask_fields: vec![],
        audit_value_max_len: None,
        fields: vec![],
        object_fields: vec![],
        constant_object_fields: vec![],
        one_to_one_relations: vec![],
        one_to_many_relations: vec![],
        relations: vec![],
        reverse_relations: vec![],
        is_human: false,
        attribute_predicate_prefix: "".to_string(),
        attribute_predicate_suffix: "".to_string(),
        bool_predicate_prefix: "".to_string(),
        constant_seed_graphs: vec![],
        seed_values: vec![],
        depth: 0,
        constant: false,
        rust_needs_smart_list: false,
        rust_has_select_all_relations: false,
        rust_has_select_any_relations: false,
    };
    println!("{}", serde_json::to_string_pretty(&e).unwrap());
    panic!("look at me");
}
