use minijinja::Environment;
use teaql_forge_codegen::context::*;
use teaql_forge_model::ir::Domain;

#[test]
fn test_render() {
    let xml = std::fs::read_to_string("../teaql-forge-server/src/demo.xml").unwrap();
    let domain: Domain = teaql_forge_model::parser::parse_ksml(&xml).unwrap();
    let render_domain = build_render_context(&domain);
    let mut env = Environment::new();
    env.add_template("test", "{{ domain.businessDescriptors[0].rust_plural_name }}").unwrap();
    let result = env.get_template("test").unwrap().render(minijinja::context! { domain => render_domain }).unwrap();
    println!("RESULT: '{}'", result);
    panic!("look at me");
}
