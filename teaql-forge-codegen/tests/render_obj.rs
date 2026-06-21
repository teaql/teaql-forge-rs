use minijinja::{context, Environment};
use std::fs;

#[test]
fn test_render() {
    let xml_content =
        fs::read_to_string("../../teaql-code-gen/ksml-test-cases/01_perfectly_valid.xml").unwrap();
    let domain =
        teaql_forge_model::parser::parse_model(&xml_content, "01_perfectly_valid.xml").unwrap();
    let render_domain = teaql_forge_codegen::context::build_render_context(&domain);

    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    env.add_template(
        "sample_data",
        include_str!("../templates/lib/sample_data/index.j2"),
    )
    .unwrap();

    let support_ticket = render_domain
        .entities
        .iter()
        .find(|e| e.rust_module == "support_ticket")
        .unwrap();
    println!(
        "support_ticket object_fields len: {}",
        support_ticket.object_fields.len()
    );

    let res = env
        .get_template("sample_data")
        .unwrap()
        .render(context! {
            domain => render_domain,
            businessObjectDescriptors => vec![support_ticket.clone()],
            constantObjectDescriptors => Vec::<teaql_forge_codegen::context::RenderEntity>::new(),
            rootObjectDescriptors => Vec::<teaql_forge_codegen::context::RenderEntity>::new()
        })
        .unwrap();

    println!("{}", res);
}
