use teaql_forge_codegen::context::{build_render_context, RenderDomain, RenderEntity, RenderField};
use teaql_forge_model::parser::parse_model;

fn get_domain(xml: &str) -> RenderDomain {
    let domain = parse_model(xml, "main.xml").expect("Should parse");
    build_render_context(&domain)
}

fn get_entity<'a>(dom: &'a RenderDomain, name: &str) -> &'a RenderEntity {
    dom.entities.iter().find(|e| e.name == name).expect("Entity not found")
}

fn get_field<'a>(entity: &'a RenderEntity, name: &str) -> &'a RenderField {
    entity.fields.iter().find(|p| p.name == name).expect("Field not found")
}

#[test]
fn test_maps_built_in_types_to_rust_types() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <root name="rust-test" org="example" data_service="postgres">
        <shipment
                shipment_number="SHP00001"
                scheduled_date="date()"
                scheduled_at="createTime()"
                total_value="decimal()" />
    </root>
    "#;
    let dom = get_domain(xml);
    let shipment = get_entity(&dom, "shipment");
    for f in &shipment.fields {
        println!("DEBUG FIELD: {}", f.name);
    }
    let num = get_field(shipment, "shipment_number");
    assert_eq!(num.rust_type, "String");
    
    let date = get_field(shipment, "scheduled_date");
    assert_eq!(date.rust_type, "chrono::NaiveDate");
    
    let time = get_field(shipment, "scheduled_at");
    assert_eq!(time.rust_type, "teaql_core::time::Timestamp");
    
    let val = get_field(shipment, "total_value");
    assert_eq!(val.rust_type, "f64");
}

#[test]
fn test_wraps_optional_fields_in_option() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <root name="rust-test" org="example" data_service="postgres">
        <employee email="string()?" />
    </root>
    "#;
    let dom = get_domain(xml);
    let employee = get_entity(&dom, "employee");
    let email = get_field(employee, "email");

    assert!(!email.required);
}

#[test]
fn test_derives_rust_object_names() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <root name="rust-test" org="example" data_service="postgres">
        <user_account login_id="u001" />
    </root>
    "#;
    let dom = get_domain(xml);
    let user_account = get_entity(&dom, "user_account");

    assert_eq!(user_account.rust_struct, "UserAccount");
    assert_eq!(user_account.name, "user_account");
}

#[test]
fn test_derives_rust_plural_names_with_english_rules() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <root name="rust-test" org="example" data_service="postgres">
        <shipment />
        <business />
        <category />
        <merchant_status />
    </root>
    "#;
    let dom = get_domain(xml);
    
    let shipment = get_entity(&dom, "shipment");
    assert_eq!(shipment.rust_plural, "shipments");
    
    let business = get_entity(&dom, "business");
    assert_eq!(business.rust_plural, "businesses");
    
    let category = get_entity(&dom, "category");
    assert_eq!(category.rust_plural, "categories");
    
    let status = get_entity(&dom, "merchant_status");
    assert_eq!(status.rust_plural, "merchant_statuses");
}
