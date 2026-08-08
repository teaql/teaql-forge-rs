use teaql_forge_codegen::context::build_render_context;
use teaql_forge_model::parser::parse_model;

#[test]
fn test_recursive_object_references_panic() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <root name="recursive-service" org="example" data_service="sqlite">
        <customer account="account()" name="string()"/>
        <account customer="customer()" name="string()"/>
    </root>
    "#;

    let domain = parse_model(xml, "main.xml").expect("Should parse");
    
    // We expect the creation of WorkspaceContext to panic due to cycle
    let result = std::panic::catch_unwind(|| {
        build_render_context(&domain);
    });

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        String::new()
    };
    assert!(msg.contains("Cycle detected") || msg.contains("reference cycle detected"));
}

#[test]
fn test_excessive_object_depth_panic() {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root name=\"deep-service\" org=\"example\" data_service=\"sqlite\">\n  <node0 name=\"string()\"/>\n"
    );
    for i in 1..=21 {
        xml.push_str(&format!("  <node{} parent=\"node{}()\" name=\"string()\"/>\n", i, i - 1));
    }
    xml.push_str("</root>\n");

    let domain = parse_model(&xml, "main.xml").expect("Should parse");
    
    let result = std::panic::catch_unwind(|| {
        build_render_context(&domain);
    });

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        String::new()
    };
    assert!(msg.contains("depth exceeded") || msg.contains("too deep") || msg.contains("Cycle detected") || msg.contains("depth"));
}
