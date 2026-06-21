use teaql_forge_codegen::context::build_render_context;
use teaql_forge_model::parser::parse_model;

#[test]
fn test_platform_preview() {
    let xml = include_str!("../../teaql-forge-server/src/demo.xml");
    let domain = parse_model(xml, "demo.xml").unwrap();
    let render_domain = build_render_context(&domain);
    std::fs::write("domain_output.json", serde_json::to_string_pretty(&render_domain).unwrap()).unwrap();
    let templates = vec![
        "rust-assist-create",
        "rust-assist-delete",
        "rust-assist-update",
        "rust-assist-query",
        "rust-assist-list-page",
        "rust-assist-expression",
        "rust-assist-runtime-custom",
        "rust-assist-tool-api",
    ];

    for tpl in templates {
        let output = teaql_forge_codegen::engine::render_preview(&render_domain, tpl, Some("platform".to_string())).unwrap();
        std::fs::write(format!("{}_output.md", tpl), output).unwrap();
    }
}
