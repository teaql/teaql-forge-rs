//! ObjectDescriptor collection getters retain the original model references.
use crate::context::RenderDomain;
use crate::preview_domain::{camel, pascal, snake, upper_first};
use crate::preview_rust::{attribute_flag, sample};
use serde_json::{Map, Value, json};

type Graph = Map<String, Value>;

fn reference(id: impl Into<String>) -> Value {
    json!({"__stg_ref":id.into()})
}
fn node<'a>(graph: &'a Graph, value: &'a Value) -> &'a Value {
    value["__stg_ref"]
        .as_str()
        .and_then(|id| graph.get(id))
        .unwrap_or(value)
}
fn list(value: &Value, name: &str) -> Vec<Value> {
    value[name].as_array().cloned().unwrap_or_default()
}
fn flag(value: &Value, name: &str) -> bool {
    value["__stg_attributes"][name]
        .as_str()
        .is_some_and(attribute_flag)
}
fn object_field(value: &Value) -> bool {
    value["fieldType"]["__stg_ref"]
        .as_str()
        .is_some_and(|id| id.starts_with("object-type:"))
}
fn bean(value: &mut Value, name: &str, result: Value) {
    for key in [name.to_string(), upper_first(name)] {
        if value["__stg_attributes"][&key]
            .as_str()
            .is_none_or(str::is_empty)
        {
            value[&key] = result.clone();
        }
    }
}
fn object_index(value: &Value) -> Option<usize> {
    value["__stg_ref"]
        .as_str()?
        .strip_prefix("object:")?
        .parse()
        .ok()
}
fn ancestors(graph: &Graph, index: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut current = object_index(&graph[&format!("object:{index}")]["parentOd"]);
    while let Some(index) = current.filter(|index| !out.contains(index)) {
        out.push(index);
        current = object_index(&graph[&format!("object:{index}")]["parentOd"]);
    }
    out
}
fn normalized_model_name(value: &str) -> String {
    let split = regex::Regex::new(r"([a-z0-9])([A-Z])")
        .unwrap()
        .replace_all(value, "${1}_${2}");
    regex::Regex::new(r"[^A-Za-z0-9]+")
        .unwrap()
        .replace_all(&split, "_")
        .trim_matches('_')
        .to_ascii_lowercase()
}

fn constant_rows(graph: &Graph, fields: &[Value]) -> Vec<Value> {
    let size = fields
        .first()
        .map(|field| {
            node(graph, field)["__stg_attributes"]["candidates"]
                .as_str()
                .map_or(0, |value| {
                    if value.is_empty() {
                        0
                    } else {
                        value.split(',').count()
                    }
                })
        })
        .unwrap_or(0);
    (1..=size).map(|line| {
        let value = |name: &str| fields.iter().map(|field| node(graph, field)).find(|field| field["originalName"] == name).map(|field| sample(field, line));
        let name = value("name");
        json!({"id":value("id"),"name":name,"code":value("code"),
            "methodName":name.as_ref().map(|name| pascal(name).replace(' ', "")).unwrap_or_default(),
            "snakeMethodName":name.as_ref().map(|name| snake(name).to_lowercase().replace(' ', "")).unwrap_or_default(),
            "camelMethodName":name.as_ref().map(|name| camel(name).replace(' ', "")).unwrap_or_default()})
    }).collect()
}

fn mark_depth(
    domain: &RenderDomain,
    index: usize,
    depth: usize,
    depths: &mut [usize],
    path: &mut Vec<usize>,
) {
    if path.contains(&index) {
        return;
    }
    path.push(index);
    let entity = &domain.entities[index];
    let parent = entity.ancestors(domain).first().and_then(|parent| {
        domain.entities.iter().position(|item| item.name == parent.name)
    });
    depths[index] = parent.map_or(depth.max(depths[index]), |parent| depths[parent] + 1);
    for (child, candidate) in domain.entities.iter().enumerate() {
        for relation in &candidate.object_fields {
            if relation.target_struct == entity.rust_struct && child != index {
                mark_depth(domain, child, depth + 1, depths, path);
            }
        }
    }
    path.pop();
}

// BaseDomainManager visits object descriptors in XML order and follows their
// reverse relations in child/field order. Inherited depth is read from the
// parent's depth at the time of each visit, rather than solved to a fixed point.
pub(crate) fn source_depths(domain: &RenderDomain) -> Vec<usize> {
    let mut depths = vec![0; domain.entities.len()];
    for index in 0..depths.len() {
        mark_depth(domain, index, 0, &mut depths, &mut Vec::new());
    }
    depths
}

fn sorted_visit(
    graph: &Graph,
    index: usize,
    sorted: &mut Vec<usize>,
    path: &mut Vec<usize>,
) -> bool {
    if sorted.contains(&index) {
        return true;
    }
    if path.contains(&index) {
        return false;
    }
    path.push(index);
    let object = &graph[&format!("object:{index}")];
    let dependencies = object_index(&object["parentOd"]).into_iter().chain(
        list(object, "fields")
            .iter()
            .filter_map(|field| object_index(&node(graph, field)["referenceObject"]))
            .collect::<Vec<_>>(),
    );
    for dependency in dependencies {
        if !sorted_visit(graph, dependency, sorted, path) {
            return false;
        }
    }
    path.pop();
    sorted.push(index);
    true
}

fn unified_children(graph: &Graph, index: usize, out: &mut Vec<Value>, extra: &mut Graph) {
    for child in list(&graph[&format!("object:{index}")], "childrenOds") {
        let Some(child_index) = object_index(&child) else {
            continue;
        };
        let field = format!("synthetic-field:{index}:{child_index}");
        extra.insert(field.clone(), json!({"name":null,"Name":null,"originalName":null,"objectDescriptor":null,"fieldType":null,"__stg_attributes":{}}));
        let relation = format!("synthetic-reverse:{index}:{child_index}");
        extra.insert(relation.clone(), json!({"parent":reference(format!("object:{index}")),"child":child,"property":reference(field)}));
        out.push(reference(relation));
        unified_children(graph, child_index, out, extra);
    }
}

pub(crate) fn enhance(domain: &RenderDomain, graph: &mut Graph) {
    let original = graph.clone();
    let root = &original["domain"];
    let threshold = crate::process_properties::boxed_relations_threshold(&domain.root_attributes);
    let depths = source_depths(domain);
    let mut aggregates = Vec::new();
    let mut many = Vec::new();
    let mut one = Vec::new();
    for index in 0..domain.entities.len() {
        let object = &original[&format!("object:{index}")];
        aggregates.push(
            list(object, "fields")
                .into_iter()
                .flat_map(|field| {
                    let functions = if flag(node(&original, &field), "isNumber") {
                        list(node(&original, &field), "aggregationFunctions")
                    } else {
                        Vec::new()
                    };
                    functions
                        .into_iter()
                        .map(move |function| json!({"fd":field,"func":function}))
                })
                .collect::<Vec<_>>(),
        );
        let is_one = |relation: &Value| {
            node(&original, &node(&original, relation)["property"])["__stg_attributes"]["relationType"]
                == "oneToOne"
        };
        let relations = list(object, "reverseRelations");
        many.push(
            relations
                .iter()
                .filter(|relation| !is_one(relation))
                .cloned()
                .collect::<Vec<_>>(),
        );
        one.push(relations.into_iter().filter(is_one).collect::<Vec<_>>());
    }
    let entries = many
        .iter()
        .map(|relations| {
            relations
                .iter()
                .flat_map(|relation| {
                    object_index(&node(&original, relation)["child"])
                        .map(|child| {
                            aggregates[child].iter().map(
                                |aggregate| json!({"relation":relation,"aggProperty":aggregate}),
                            ).collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (index, entity) in domain.entities.iter().enumerate() {
        let key = format!("object:{index}");
        let object = &original[&key];
        let fields = list(object, "fields");
        let parents = ancestors(&original, index);
        let relations = list(object, "reverseRelations");
        let cost: usize = relations
            .iter()
            .map(|relation| {
                list(
                    node(&original, &node(&original, relation)["child"]),
                    "fields",
                )
                .len()
            })
            .sum();
        let objects = fields
            .iter()
            .filter(|field| object_field(node(&original, field)))
            .cloned()
            .collect::<Vec<_>>();
        let scalars = fields
            .iter()
            .filter(|field| !object_field(node(&original, field)))
            .cloned()
            .collect::<Vec<_>>();
        let constants = objects
            .iter()
            .filter(|field| {
                let target = node(&original, &node(&original, field)["referenceObject"]);
                flag(target, "constant") || flag(target, "_constant")
            })
            .cloned()
            .collect::<Vec<_>>();
        let mask = object["__stg_attributes"]["audit_mask_fields"]
            .as_str()
            .unwrap_or("")
            .split(',')
            .map(|name| name.trim_matches(|ch| ch <= '\u{20}'))
            .filter(|name| !name.is_empty())
            .map(normalized_model_name)
            .collect::<Vec<_>>();
        let audit = fields
            .iter()
            .filter(|field| {
                mask.contains(&normalized_model_name(
                    node(&original, field)["originalName"]
                        .as_str()
                        .unwrap_or(""),
                ))
            })
            .cloned()
            .collect::<Vec<_>>();
        let parent_fields = parents
            .iter()
            .flat_map(|parent| list(&original[&format!("object:{parent}")], "fields"))
            .collect::<Vec<_>>();
        let mut parent_filters = parent_fields
            .iter()
            .filter(|field| !flag(node(&original, field), "isId"))
            .cloned()
            .collect::<Vec<_>>();
        if !parents.is_empty() {
            parent_filters.push(
                fields
                    .iter()
                    .find(|field| node(&original, field)["originalName"] == "id")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        let parent_pojo = parent_fields
            .into_iter()
            .filter(|field| {
                !flag(node(&original, field), "isId") && !flag(node(&original, field), "isVersion")
            })
            .collect::<Vec<_>>();
        let mut unified = relations.clone();
        unified_children(&original, index, &mut unified, graph);
        let is_root = !entity.has_parent() && objects.is_empty();
        let is_constant = flag(object, "constant") || flag(object, "_constant");
        let value_object = flag(object, "valueObject");
        let view_object = flag(object, "viewObject");
        let log = flag(object, "log")
            || object["__stg_attributes"]["features"]
                .as_str()
                .unwrap_or("")
                .split(',')
                .any(|feature| {
                    feature
                        .trim_matches(|ch| ch <= '\u{20}')
                        .eq_ignore_ascii_case("log")
                });
        let values = [
            ("fields", json!(fields)),
            ("properties", object["properties"].clone()),
            ("domain", reference("domain")),
            ("parentOd", object["parentOd"].clone()),
            ("childrenOds", object["childrenOds"].clone()),
            ("allChildrenOds", object["allChildrenOds"].clone()),
            ("identityConstants", object["identityConstants"].clone()),
            ("tableName", object["tableName"].clone()),
            ("dataService", object["dataService"].clone()),
            (
                "supportFullTextSearch",
                json!(
                    entity
                        .data_service
                        .as_deref()
                        .is_some_and(|name| name.eq_ignore_ascii_case("meilisearch"))
                ),
            ),
            ("humanOd", json!(entity.is_human)),
            (
                "attributePredicatePrefix",
                json!(if entity.is_human { "whose" } else { "with" }),
            ),
            (
                "attributePredicateSuffix",
                json!(if entity.is_human { "s" } else { "ing" }),
            ),
            (
                "booleanPredicatePrefix",
                json!(if entity.is_human {
                    "whoAre"
                } else {
                    "whichAre"
                }),
            ),
            ("root", json!(is_root)),
            ("constant", json!(is_constant)),
            ("transient", json!(flag(object, "transient"))),
            ("valueObject", json!(value_object)),
            ("viewObject", json!(view_object)),
            ("repository", json!(!value_object && !view_object)),
            ("logObject", json!(log)),
            ("depth", json!(depths[index])),
            ("reverseRelationsCost", json!(cost)),
            (
                "boxedReverseRelations",
                json!(if threshold == 0 {
                    !relations.is_empty()
                } else {
                    cost as i64 > threshold as i64
                }),
            ),
            ("objectProperties", json!(objects)),
            ("objectFields", json!(objects)),
            ("scalarFields", json!(scalars)),
            ("auditMaskFields", json!(audit)),
            ("constantObjectFields", json!(constants)),
            (
                "requestFilterProperties",
                json!(
                    fields
                        .iter()
                        .filter(|field| parents.is_empty() || !flag(node(&original, field), "isId"))
                        .cloned()
                        .collect::<Vec<_>>()
                ),
            ),
            ("parentFilterProperties", json!(parent_filters)),
            ("parentPojoProperties", json!(parent_pojo)),
            ("reverseRelations", json!(relations)),
            ("unifiedRelations", json!(unified)),
            ("oneToManyRelations", json!(many[index])),
            ("oneToOneRelations", json!(one[index])),
            (
                "parentReverseRelations",
                json!(
                    parents
                        .iter()
                        .flat_map(|parent| list(
                            &original[&format!("object:{parent}")],
                            "reverseRelations"
                        ))
                        .collect::<Vec<_>>()
                ),
            ),
            (
                "parentOneToManyRelations",
                json!(
                    parents
                        .iter()
                        .flat_map(|parent| many[*parent].clone())
                        .collect::<Vec<_>>()
                ),
            ),
            (
                "parentOneToOneRelations",
                json!(
                    parents
                        .iter()
                        .flat_map(|parent| one[*parent].clone())
                        .collect::<Vec<_>>()
                ),
            ),
            ("aggregationProperties", json!(aggregates[index])),
            ("aggregationEntries", json!(entries[index])),
            (
                "parentAggregationProperties",
                json!(
                    parents
                        .iter()
                        .flat_map(|parent| aggregates[*parent].clone())
                        .collect::<Vec<_>>()
                ),
            ),
            (
                "parentAggregationEntries",
                json!(
                    parents
                        .iter()
                        .flat_map(|parent| entries[*parent].clone())
                        .collect::<Vec<_>>()
                ),
            ),
            ("constantItemList", json!(constant_rows(&original, &fields))),
        ];
        let object = graph.get_mut(&key).unwrap();
        for (name, result) in values {
            bean(object, name, result);
        }
        bean(object, "HasParent", json!(!parents.is_empty()));
    }
    let indexes = (0..domain.entities.len()).collect::<Vec<_>>();
    let object_refs = |indexes: Vec<usize>| {
        json!(
            indexes
                .into_iter()
                .map(|index| reference(format!("object:{index}")))
                .collect::<Vec<_>>()
        )
    };
    let mut sorted = Vec::new();
    let sortable = indexes
        .iter()
        .all(|index| sorted_visit(&original, *index, &mut sorted, &mut Vec::new()));
    let properties = [
        ("boxedRelationsCostThreshold", json!(threshold)),
        (
            "haveVerb",
            json!(if flag(root, "use_has") { "has" } else { "have" }),
        ),
        (
            "haveNoVerb",
            json!(if flag(root, "use_has") {
                "hasNo"
            } else {
                "haveNo"
            }),
        ),
        ("commonDomain", json!(domain.name.is_empty())),
        (
            "objectDescriptorsHasList",
            crate::java_stg_expr::java_collection("java.util.ImmutableCollections$ListN",
                indexes
                    .iter()
                    .filter(|index| {
                        !list(&original[&format!("object:{index}")], "reverseRelations").is_empty()
                    })
                    .map(|index|reference(format!("object:{index}")))
                    .collect(),
            ),
        ),
        (
            "rootDescriptors",
            object_refs(
                indexes
                    .iter()
                    .filter(|index| {
                        !domain.entities[**index].has_parent()
                            && domain.entities[**index].object_fields.is_empty()
                            && !flag(&original[&format!("object:{index}")], "constant")
                            && !flag(&original[&format!("object:{index}")], "_constant")
                    })
                    .copied()
                    .collect(),
            ),
        ),
        (
            "constantDescriptors",
            object_refs(
                indexes
                    .iter()
                    .filter(|index| {
                        flag(&original[&format!("object:{index}")], "constant")
                            || flag(&original[&format!("object:{index}")], "_constant")
                    })
                    .copied()
                    .collect(),
            ),
        ),
        (
            "sortedObjectDescriptors",
            if sortable {
                object_refs(sorted)
            } else {
                // ObjectModelAdaptor returns a missing bean value when this
                // getter overflows on a self reference; the outer ST continues.
                Value::Null
            },
        ),
    ];
    let root = graph.get_mut("domain").unwrap();
    for (name, result) in properties {
        bean(root, name, result);
    }
}
