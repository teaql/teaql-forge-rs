//! Physical DataDesign DomainWrapper and ObjectDescriptorWrapper getters.
use crate::context::RenderDomain;
use serde_json::{Map, Value, json};
type Graph = Map<String, Value>;

fn node<'a>(graph: &'a Graph, value: &'a Value) -> &'a Value {
    value["__stg_ref"]
        .as_str()
        .and_then(|id| graph.get(id))
        .unwrap_or(value)
}
fn physical<'a>(value: &'a Value, name: &str) -> &'a Value {
    value["__stg_java_getters"]
        .get(name)
        .unwrap_or(&value[name])
}
fn items(value: &Value) -> Vec<Value> {
    crate::java_stg_expr::sequence_items(value)
        .cloned()
        .unwrap_or_default()
}
fn collection(kind: &str, values: Vec<Value>) -> Value {
    crate::java_stg_expr::java_collection(kind, values)
}
fn put(object: &mut Value, name: &str, value: Value) {
    object["__stg_java_getters"][name] = value.clone();
    for key in [name.to_string(), crate::preview_domain::upper_first(name)] {
        if object["__stg_attributes"][&key]
            .as_str()
            .is_none_or(str::is_empty)
        {
            object[&key] = value.clone();
        }
    }
}
fn bean(kind: &str, values: Value) -> Value {
    let mut object = values;
    object["__stg_java_bean_class"] = json!(format!("com.skynet.codegenerator.doc.{kind}"));
    object
}
fn index(table: &str, name: &str, columns: &str, kind: &str, remark: &str) -> Value {
    bean(
        "DatabaseIndex",
        json!({"tableName":table,"name":name,"columns":columns,"type":kind,"remark":remark}),
    )
}
fn table(graph: &Graph, value: &Value) -> String {
    physical(node(graph, value), "tableName")
        .as_str()
        .unwrap_or("")
        .to_uppercase()
}
fn column(graph: &Graph, value: &Value) -> String {
    node(graph, value)["SQLPresentName"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

fn object_wrapper(graph: &Graph, object_index: usize, original: &Value) -> Value {
    let mut wrapped = original.clone();
    wrapped["__stg_java_class"] = json!("com.skynet.codegenerator.doc.DataDesignObjectWrapper");
    put(&mut wrapped, "lineNumber", json!(0));
    put(&mut wrapped, "xmlPath", Value::Null);
    put(
        &mut wrapped,
        "rawDescriptor",
        json!({"__stg_ref":format!("object:{object_index}")}),
    );
    let fields = items(physical(original, "fields"));
    let properties = items(physical(original, "objectProperties"));
    let reverse = items(physical(original, "reverseRelations"));
    let name = physical(original, "name")
        .as_str()
        .unwrap_or("")
        .to_uppercase();
    let table_name = physical(original, "tableName").as_str().unwrap_or("");
    put(
        &mut wrapped,
        "columns",
        physical(original, "fields").clone(),
    );
    let rows = if let Some(first) = fields.first() {
        let candidates = &node(graph, first)["__stg_attributes"]["candidates"];
        let count = candidates
            .as_str()
            .filter(|value| !value.is_empty())
            .map_or(0, |value| value.split(',').count());
        collection("java.util.ArrayList",(1..=count).map(|line| {
            bean("DataRow",json!({"values":collection("java.util.ArrayList",fields.iter()
                .map(|field|json!(crate::preview_rust::sample(node(graph,field),line))).collect())}))
        }).collect())
    } else {
        crate::preview_rust::fault("Index 0 out of bounds for length 0")
    };
    put(&mut wrapped, "rows", rows);
    let mut indexes = vec![
        index(
            table_name,
            &format!("PK_{name}_ID"),
            "ID",
            "PRIMARY KEY",
            "",
        ),
        index(
            table_name,
            &format!("PK_{name}_ID_VERSION"),
            "ID,VERSION",
            "UNIQUE KEY",
            "",
        ),
    ];
    let mut constraints = vec![bean(
        "DatabaseConstraint",
        json!({"tableName":"-","name":format!("PK_{name}_ID"),
        "columns":"ID","type":"PRIMARY KEY","referenceColumns":"-","remark":"PRIMARY KEY"}),
    )];
    let mut depends = Vec::new();
    for field in &properties {
        let col = column(graph, field);
        let target = &node(graph, &node(graph, field)["fieldType"])["refer"];
        let target_table = table(graph, target);
        indexes.push(index(
            table_name,
            &format!("IDX_{name}_{col}"),
            &col,
            "INDEX",
            "",
        ));
        constraints.push(bean("DatabaseConstraint",json!({"tableName":target_table,"name":format!("FK_{name}_{col}"),
            "columns":col,"type":"","referenceColumns":"ID","remark":"CASCADE ON DELETE and UPDATE"})));
        depends.push(index(&target_table, "-", &col, "-", "-"));
    }
    for field in &fields {
        if crate::preview_rust::attribute_flag(
            node(graph, field)["__stg_attributes"]["isDate"]
                .as_str()
                .unwrap_or(""),
        ) {
            let col = column(graph, field);
            indexes.push(index(
                table_name,
                &format!("IDX_{name}_{col}"),
                &col,
                "INDEX",
                "",
            ));
        }
    }
    let depended = reverse
        .iter()
        .map(|relation| {
            let relation = node(graph, relation);
            index(
                &table(graph, &relation["child"]),
                "-",
                &column(graph, &relation["property"]),
                "-",
                "-",
            )
        })
        .collect();
    for (name, values) in [
        ("indexes", indexes),
        ("constrains", constraints),
        ("depends", depends),
        ("dependedBy", depended),
    ] {
        put(
            &mut wrapped,
            name,
            collection("java.util.ArrayList", values),
        );
    }
    wrapped
}

fn sorted_visit(
    graph: &Graph,
    object: &Value,
    sorted: &mut Vec<Value>,
    path: &mut Vec<Value>,
) -> bool {
    // Wrapper.equals(raw) delegates to raw.equals(raw), while raw.equals(wrapper)
    // is false. LinkedHashSet tests the lookup key's equals, not a symmetric key.
    let raw = object["__stg_ref"]
        .as_str()
        .and_then(|id| id.strip_prefix("doc-object:"))
        .map(|i| json!({"__stg_ref":format!("object:{i}")}));
    if sorted.contains(object) || raw.as_ref().is_some_and(|raw| sorted.contains(raw)) {
        return true;
    }
    if path.contains(object) {
        return false;
    }
    path.push(object.clone());
    let value = node(graph, object);
    let parent = physical(value, "parentOd");
    if !parent.is_null() && !sorted_visit(graph, parent, sorted, path) {
        return false;
    }
    for field in items(physical(value, "fields")) {
        let field = node(graph, &field);
        if let Some(target) = physical(field, "referenceObject").get("__stg_ref") {
            if !sorted_visit(graph, &json!({"__stg_ref":target}), sorted, path) {
                return false;
            }
        }
    }
    path.pop();
    sorted.push(object.clone());
    true
}

pub(super) fn attach(domain: &RenderDomain, graph: &mut Graph, payload: String) -> Value {
    // Keep the original Domain graph intact: fields, relations and nonwrapped
    // Domain getters still return the original object descriptors and domain.
    let mut wrapped = graph["domain"].clone();
    wrapped["__stg_java_class"] = json!("com.skynet.codegenerator.doc.DataDesignGenerator$1");
    put(&mut wrapped, "lineNumber", json!(0));
    put(&mut wrapped, "xmlPath", Value::Null);
    let mut objects = Vec::new();
    for i in 0..domain.entities.len() {
        let id = format!("doc-object:{i}");
        let value = object_wrapper(graph, i, &graph[&format!("object:{i}")]);
        graph.insert(id.clone(), value);
        objects.push(json!({"__stg_ref":id}));
    }
    for i in 0..domain.entities.len() {
        let mut unified = items(physical(&graph[&format!("object:{i}")], "unifiedRelations"));
        for relation in &mut unified {
            let Some(id) = relation["__stg_ref"]
                .as_str()
                .filter(|id| id.starts_with(&format!("synthetic-reverse:{i}:")))
            else {
                continue;
            };
            let mut value = graph[id].clone();
            put(
                &mut value,
                "parent",
                json!({"__stg_ref":format!("doc-object:{i}")}),
            );
            let new_id = format!("doc-{id}");
            graph.insert(new_id.clone(), value);
            *relation = json!({"__stg_ref":new_id});
        }
        put(
            graph.get_mut(&format!("doc-object:{i}")).unwrap(),
            "unifiedRelations",
            collection("java.util.ArrayList", unified),
        );
    }
    for name in ["rootDescriptors", "constantDescriptors"] {
        let values = items(physical(&graph["domain"], name))
            .into_iter()
            .map(|value| {
                let id = value["__stg_ref"].as_str().unwrap();
                json!({"__stg_ref":format!("doc-object:{}",id.strip_prefix("object:").unwrap())})
            })
            .collect();
        put(
            &mut wrapped,
            name,
            collection("java.util.ArrayList", values),
        );
    }
    // This inherited Domain getter reads its private, unpopulated field.
    put(
        &mut wrapped,
        "objectDescriptorsHasList",
        collection("java.util.ImmutableCollections$ListN", vec![]),
    );
    let mut sorted = Vec::new();
    let sortable = objects
        .iter()
        .all(|object| sorted_visit(graph, object, &mut sorted, &mut Vec::new()));
    put(
        &mut wrapped,
        "sortedObjectDescriptors",
        if sortable {
            collection("java.util.ArrayList", sorted)
        } else {
            Value::Null
        },
    );
    put(
        &mut wrapped,
        "objectDescriptors",
        collection("java.util.ImmutableCollections$ListN", objects),
    );
    put(
        &mut wrapped,
        "wordCloudSVG",
        json!({"__stg_graph_service_payload":payload}),
    );
    graph.insert("doc-domain".into(), wrapped);
    json!({"__stg_ref":"doc-domain"})
}
