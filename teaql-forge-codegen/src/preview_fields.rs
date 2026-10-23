//! Physical FieldDescriptor beans, distinct from attached public properties.
use crate::context::RenderDomain;
use crate::preview_domain::{attribute_value, pascal, camel, upper_first};
use crate::preview_rust::{attribute_flag, raw_type, sample};
use serde_json::{Map, Value, json};
use teaql_forge_model::field_attributes::Attribute;

fn attribute(value: &Attribute) -> Value {
    json!({"__stg_java_class":"com.skynet.codegenerator.meta.Attribute", "__stg_attributes":{},
        "name":value.name,"value":value.value,"append":value.append,"override":value.override_value,
        "remove":value.remove,"deleted":value.deleted,"attachableToAdditional":!value.deleted && value.name!="sampleValue",
        "lineNumber":0,"xmlPath":null,"snakeName":crate::preview_domain::snake(&value.name),"displayName":value.name,
        "attributes":[],"attachableAttributes":[]})
}

pub(crate) fn enhance(domain: &RenderDomain, graph: &mut Map<String, Value>) {
    let builtins = teaql_forge_model::types::builtin_types();
    // Candidate is a plain bean; its fields stay available to the native
    // generators while the adaptor exposes the actual getter names and types.
    let package = graph["domain"]["fullpackage"].as_str().unwrap_or_default().to_string();
    let domain_name = graph["domain"]["camelName"].as_str().unwrap_or_default().to_string();
    for node in graph.values_mut() {
        if let Some(candidates) = node.get_mut("identityConstants").and_then(Value::as_array_mut) {
            for item in candidates {
                let value = item["value"].as_str().unwrap_or_default().to_string();
                let member = camel(&value).to_lowercase();
                let controller = format!("{}Controller",pascal(&value));
                item["__stg_java_bean_class"] = json!("com.skynet.codegenerator.meta.Candidate");
                item["candidateValue"] = json!(pascal(&value));
                item["fullpackage"] = json!(format!("{package}.{member}"));
                item["scenarioEntranceService"] = json!(format!("{package}.{member}.{controller}"));
                item["scenarioEntranceServiceName"] = json!(controller);
                item["scenarioEntranceServicePath"] = json!(format!("{domain_name}Services/{}Controller",camel(&value)));
            }
        }
    }
    let ids = graph.keys().filter(|id| id.starts_with("field:")).cloned().collect::<Vec<_>>();
    for id in ids {
        let mut parts = id.splitn(3, ':'); parts.next();
        let owner = parts.next().unwrap().parse::<usize>().unwrap();
        let name = parts.next().unwrap();
        let entity = &domain.entities[owner];
        let original = &graph[&id];
        let kind = raw_type(graph, original).to_ascii_lowercase();
        let own = domain.source_field_attributes.get(&entity.name).and_then(|fields| fields.get(name)).cloned().unwrap_or_default();
        let inherited = entity.fields.iter().find(|field| field.name == name).map(|field| {
            if !field.type_attributes.is_empty() { field.type_attributes.clone() }
            else { builtins.get(&field.source_type_name).cloned().unwrap_or_default() }
        }).unwrap_or_default();
        let object = original["objectProperty"] == true;
        let mut attrs = if object {
            original["referenceObject"]["__stg_ref"].as_str().and_then(|id| graph.get(id))
                .and_then(|target|target["__stg_attributes"].as_object()).cloned().unwrap_or_default()
        } else { inherited.iter().map(|item|(item.name.clone(),json!(item.value))).collect() };
        for item in &own { attrs.insert(item.name.clone(), json!(item.value)); }
        let flag = |name: &str| attrs.get(name).and_then(Value::as_str).is_some_and(attribute_flag);
        let target = original["referenceObject"]["__stg_ref"].as_str().and_then(|id| graph.get(id));
        let constant = object && target.is_some_and(|target| target["constant"] == true);
        let constants = target.map(|target| target["identityConstants"].clone()).unwrap_or_else(|| json!([]));
        let mut getters = original["__stg_java_getters"].as_object().cloned().unwrap_or_default();
        let string = matches!(kind.as_str(), "string" | "text") || flag("isString") || flag("isText");
        let boolean = matches!(kind.as_str(), "bool" | "boolean");
        for (logical, bean, value) in [
            ("string","isString",string),
            ("number","isNumber",matches!(kind.as_str(),"int"|"integer"|"number"|"double"|"money"|"long")),
            ("boolean","isBoolean",boolean),("date","isDate",kind=="date"),
            ("time","isTime",matches!(kind.as_str(),"datetime"|"time")),
            ("double","isDouble",kind=="double"),("float","isFloat",kind=="float"),
            ("decimal","isDecimal",matches!(kind.as_str(),"decimal"|"number")),
            ("money","isMoney",kind=="money"),("dateTime","isDateTime",kind=="datetime"),
            ("dateTimeWithZone","isDateTimeWithZone",kind=="datetimewithzone"),
            ("jSON","isJSON",matches!(kind.as_str(),"json"|"jsonme")),
            ("integer","isInteger",matches!(kind.as_str(),"int"|"integer")),
            ("long","isLong",kind=="long"),
            ("businessId","isBusinessId",matches!(kind.as_str(),"businessid"|"business_id")||flag("isBusinessId")),
        ] { getters.insert(logical.into(),json!(value)); getters.insert(bean.into(),json!(value)); }
        getters.insert("isBool".into(),json!(boolean));
        getters.insert("constantObjectField".into(),json!(constant));
        getters.insert("constantTargetValues".into(),constants.clone());
        getters.insert("candidateForConst".into(),if constant { constants } else { json!([]) });
        getters.insert("contextRoot".into(),json!(flag("contextRoot")));
        getters.insert("type".into(),entity.fields.iter().find(|field| field.name==name && field.line_number==0)
            .map(|field|json!(if field.is_id {"id"} else {"version"})).unwrap_or(Value::Null));
        getters.insert("entityType".into(),Value::Null);
        getters.insert("attributes".into(),json!(own.iter().map(attribute).collect::<Vec<_>>()));
        let mut names = own.iter().filter(|item| item.name!="sampleValue" && !item.deleted).map(|item|item.name.clone()).collect::<Vec<_>>();
        if !object { names.extend(inherited.iter().filter(|item|item.name!="sampleValue").map(|item|item.name.clone())); }
        let attachable = crate::java_hash_set::scalars(names).into_iter().filter_map(|name| {
            own.iter().find(|item|item.name==name).cloned().or_else(||inherited.iter().find(|item|item.name==name).map(|item|Attribute {
                name:item.name.clone(),value:item.value.clone(),append:item.append,override_value:item.override_value,remove:false,deleted:false,
            })).map(|item|attribute(&item))
        }).collect::<Vec<_>>();
        getters.insert("attachableAttributes".into(),json!(attachable));
        let mut physical = original.clone(); physical["__stg_attributes"] = json!(attrs);
        let raw = sample(&physical,1);
        getters.insert("defaultSampleValue".into(),json!(raw));
        getters.insert("rustSafeDefaultSampleValue".into(),json!(raw.replace('\\',"\\\\").replace('"',"\\\"")));
        getters.insert("jsonLiteral".into(),json!(raw.trim_matches(|ch| ch<='\u{20}').starts_with('{') && raw.trim_matches(|ch| ch<='\u{20}').ends_with('}')));
        getters.extend(crate::business_id::values(&entity.name,|name|attrs.get(name).and_then(Value::as_str)));
        let node = graph.get_mut(&id).unwrap().as_object_mut().unwrap();
        for (name,value) in &getters {
            for name in [name.clone(), upper_first(name)] {
                // FieldDescriptorNaming.required is an enhancer and precedes
                // the distinct isRequired JavaBean getter.
                if matches!(name.as_str(), "required" | "isDecimal") { continue }
                if let Some(attached) = attrs.get(&name).and_then(Value::as_str).filter(|value|!value.is_empty()) {
                    node.insert(name, attribute_value(attached));
                } else {
                    node.insert(name, value.clone());
                }
            }
        }
        node.insert("__stg_java_getters".into(),json!(getters));
    }

}
