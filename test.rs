use teaql_forge_model::parser::parse_model;
use teaql_forge_codegen::context::build_render_context;

fn main() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <root name="rust-test" org="example" data_service="postgres">
        <shipment
                shipment_number="SHP00001"
                scheduled_date="date()"
                scheduled_at="createTime()"
                total_value="decimal()" />
    </root>
    "#;
    let dom = parse_model(xml, "main.xml").unwrap();
    let render_dom = build_render_context(&dom);
    for e in &render_dom.entities {
        for f in &e.fields {
            println!("Field: {}", f.name);
        }
    }
}
