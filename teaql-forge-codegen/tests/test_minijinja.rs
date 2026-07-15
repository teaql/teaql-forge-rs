use minijinja::Environment;
use teaql_forge_codegen::context::*;
use teaql_forge_model::ir::Domain;

#[test]
fn test_render() {
    let xml = std::fs::read_to_string("../teaql-forge-server/src/demo.xml").unwrap();
    let domain: Domain = teaql_forge_model::parser::parse_model(&xml, "demo.xml").unwrap();
    let render_domain = build_render_context(&domain);
    let mut env = Environment::new();
    env.add_template(
        "test",
        "{{ domain.businessDescriptors[0].rust_plural_name }}",
    )
    .unwrap();
    let result = env
        .get_template("test")
        .unwrap()
        .render(minijinja::context! { domain => render_domain })
        .unwrap();
    println!("RESULT: '{}'", result);
}

#[test]
fn test_data_design_react() {
    let xml = std::fs::read_to_string("../teaql-forge-server/src/demo.xml").unwrap();
    let domain: Domain = teaql_forge_model::parser::parse_model(&xml, "demo.xml").unwrap();
    let render_domain = build_render_context(&domain);
    let mut env = Environment::new();
    let template_content = std::fs::read_to_string("templates/doc/data-design-react.j2").unwrap();
    env.add_template("data-design-react", &template_content)
        .unwrap();
    let result = env
        .get_template("data-design-react")
        .unwrap()
        .render(minijinja::context! { domain => render_domain })
        .unwrap();

    if let Some(start) = result.find("window.domainData = {") {
        let json_part = &result[start..];
        if let Some(end) = json_part.find("};\n") {
            println!("--- JSON START ---");
            let snippet = &json_part[..std::cmp::min(1000, end + 1)];
            println!("{}", snippet);
            println!("--- JSON END ---");
        }
    }
    // removed panic
}
