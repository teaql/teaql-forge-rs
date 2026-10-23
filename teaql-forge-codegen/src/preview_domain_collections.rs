//! Stable collections returned by the original Domain JavaBean getters.
use crate::preview_domain::upper_first;
use serde_json::{Map, Value, json};
use std::collections::HashSet;

type Graph = Map<String, Value>;

fn node<'a>(graph: &'a Graph, value: &'a Value) -> &'a Value {
    value["__stg_ref"].as_str().and_then(|id| graph.get(id)).unwrap_or(value)
}

fn collection(kind: &str, values: Vec<Value>) -> Value {
    json!({"__stg_java_sequence":values,"__stg_java_sequence_type":kind})
}

fn hash(value: &str) -> u32 {
    value.encode_utf16().fold(0u32, |hash, unit| hash.wrapping_mul(31).wrapping_add(u32::from(unit)))
}

fn spread(hash: u32) -> u32 {
    hash ^ (hash >> 16)
}

fn hash_order(mut entries: Vec<(u32, Value)>, initial: usize) -> Vec<(u32, Value)> {
    let mut capacity = initial.max(16).next_power_of_two();
    while entries.len() > capacity * 3 / 4 {
        capacity *= 2;
    }
    entries.sort_by_key(|(hash, _)| spread(*hash) as usize & (capacity - 1));
    entries
}

fn scalars(graph: &Graph) -> Value {
    let mut values = Vec::new();
    for item in graph["domain"]["types"].as_array().into_iter().flatten() {
        let ty = node(graph, item);
        let Some(value) = ty["__stg_attributes"]["graphqlType"].as_str() else { continue };
        if !matches!(value, "Int" | "Boolean" | "String" | "Float" | "ID") {
            values.push(value.to_string());
        }
    }
    collection("java.util.HashSet", crate::java_hash_set::scalars(values).into_iter().map(Value::String).collect())
}

fn auto_functions(graph: &Graph) -> Value {
    let mut seen = HashSet::new();
    let mut functions = Vec::new();
    for object in graph["domain"]["objectDescriptors"].as_array().into_iter().flatten() {
        for field in node(graph, object)["properties"].as_array().into_iter().flatten() {
            let field = node(graph, field);
            for attribute in ["createFunction", "updateFunction"] {
                let Some(name) = field["__stg_attributes"][attribute].as_str() else { continue };
                if name == "now" || !seen.insert(name.to_string()) { continue }
                let class = "com.skynet.codegenerator.meta.AutoFunction";
                let code = hash(name).wrapping_add(31);
                // Domain invokes the real methods directly. Attached attributes
                // named like those methods must not replace the argument lists.
                let arguments = field["__stg_attributes"]["createFunctionArgNames"].as_str()
                    .map(|names| names.split(',').map(str::to_owned).collect::<Vec<_>>())
                    .unwrap_or_default();
                let values = arguments.iter().map(|name| {
                    field["__stg_attributes"][name].as_str().unwrap_or("").to_owned()
                }).collect::<Vec<_>>();
                functions.push((code, json!({
                    "__stg_java_bean_class":class,
                    "__string":format!("{class}@{code:x}"),
                    "functionName":name,
                    "returnType":node(graph,&field["fieldType"])["fullName"],
                    "argNames":arguments,
                    "argValues":values,
                })));
            }
        }
    }
    let names = functions.iter().map(|(_, value)| value["functionName"].as_str().unwrap())
        .collect::<Vec<_>>();
    let values = if let Some(order) = crate::java_hash_set::auto_function_order(&names) {
        let mut entries = functions.into_iter().map(|(_, value)| Some(value)).collect::<Vec<_>>();
        order.into_iter().map(|index| entries[index].take().unwrap()).collect()
    } else {
        hash_order(functions,16).into_iter().map(|(_, value)| value).collect()
    };
    collection("java.util.ArrayList", values)
}

pub(crate) fn enhance(graph: &mut Graph) {
    let catalog: Value = serde_json::from_str(include_str!("data/source-domain-collections-6f2d0a.json")).unwrap();
    let matchers = catalog["typeMatchers"].as_array().unwrap().iter().map(|item| {
        let mut value = item.as_object().unwrap().clone();
        value.insert("__stg_java_bean_class".into(),json!("com.skynet.codegenerator.meta.TypeMatcher"));
        Value::Object(value)
    }).collect();
    let physical = [
        ("scalars",scalars(graph)),
        ("typeMatchers",collection("java.util.ArrayList",matchers)),
        // BaseDomainManager's sole internal XML has no libraries, and its SAX
        // input parser does not construct Library entries from user elements.
        ("libraries",collection("java.util.ArrayList",vec![])),
        ("inUseLibraries",collection("java.util.HashSet",vec![])),
        ("customAutoFunctions",auto_functions(graph)),
    ];
    let root = graph.get_mut("domain").unwrap().as_object_mut().unwrap();
    for (name,value) in physical {
        root["__stg_java_getters"][name] = value.clone();
        for key in [name.to_string(),upper_first(name)] {
            if root["__stg_attributes"][&key].as_str().is_none_or(str::is_empty) {
                root.insert(key,value.clone());
            }
        }
    }
}
