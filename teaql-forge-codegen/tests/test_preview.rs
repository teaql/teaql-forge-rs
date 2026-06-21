use teaql_forge_codegen::context::build_render_context;
use teaql_forge_model::parser::parse_model;

#[test]
fn test_platform_preview() {
    let xml = include_str!("../../teaql-forge-server/src/demo.xml");
    let domain = parse_model(xml, "demo.xml").unwrap();
    let render_domain = build_render_context(&domain);
    let output = teaql_forge_codegen::engine::render_preview(&render_domain, "rust-assist-update", Some("platform".to_string())).unwrap();
    std::fs::write("test_preview_output.md", output).unwrap();
}
