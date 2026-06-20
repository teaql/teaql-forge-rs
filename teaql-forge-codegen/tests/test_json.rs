use std::fs;
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_model::parser::parse_model;

#[test]
fn dump_json() {
    let xml_content =
        fs::read_to_string("../../teaql-code-gen/ksml-test-cases/01_perfectly_valid.xml").unwrap();
    let domain = parse_model(&xml_content, "01_perfectly_valid.xml").unwrap();
    let render_domain = build_render_context(&domain);
    let json = serde_json::to_string_pretty(&render_domain).unwrap();
    fs::write("domain_output.json", json).unwrap();
}
