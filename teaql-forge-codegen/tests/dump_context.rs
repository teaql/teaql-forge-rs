use teaql_forge_core::modeling::ModelBuilder;
use teaql_forge_codegen::generator::EntityDescriptor;

#[test]
fn dump_object_descriptor() {
    let builder = ModelBuilder::from_xml_string(include_str!("../tests/data/01_perfectly_valid.xml")).unwrap();
    let model = builder.build().unwrap();
    for object in model.objects() {
        let desc = EntityDescriptor::new(object, "sqlite");
        println!("{}", serde_json::to_string_pretty(&desc).unwrap());
    }
    panic!("look at me");
}
