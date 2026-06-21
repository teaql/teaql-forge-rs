use std::fs;

#[test]
fn test_debug_obj() {
    let xml_content =
        fs::read_to_string("../../teaql-code-gen/ksml-test-cases/01_perfectly_valid.xml").unwrap();
    let domain =
        teaql_forge_model::parser::parse_model(&xml_content, "01_perfectly_valid.xml").unwrap();
    let render_domain = teaql_forge_codegen::context::build_render_context(&domain);
    for e in &render_domain.entities {
        if e.name == "Support Ticket" {
            println!("Support Ticket object fields: {}", e.object_fields.len());
            for f in &e.object_fields {
                println!("  - {:?} (constant: {})", f.target, f.constant_object_field);
            }
        }
    }
}
