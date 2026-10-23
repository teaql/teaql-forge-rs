//! LOAD_PROP/LOAD_PROP_IND render names while retaining their operand slot.
use super::*;
use operands::{Operand, fault, pop_value};

pub(super) fn dictionary_literal(input: &str) -> Value {
    let token = input.replace("\\\"", "\"");
    if let Some(text) = token.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        // Group.g STRING removes escaped quotes. Misc.replaceEscapes escapes
        // actual control characters; it does not decode backslash sequences.
        return Value::String(
            text.replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t"),
        );
    }
    expression::value(input, &Map::new())
}

impl StgTemplate {
    pub(super) fn property_operand(
        &self,
        input: &str,
        scope: &mut eval::Scope<'_>,
    ) -> Result<(), String> {
        let parts = expression::split_top_level(input, ".");
        self.evaluate_operand(parts[0], scope)?;
        for part in parts.iter().skip(1) {
            let property = if part.starts_with('(') {
                let previous = scope.conditional;
                scope.conditional = false;
                let result = self.evaluate_operand(&part[1..part.len() - 1], scope);
                scope.conditional = previous;
                result?;
                pop_value(scope)?
            } else {
                Value::String((*part).into())
            };
            let (position, object) = if part.starts_with('(') {
                let stack = scope.state.operands.borrow();
                (
                    stack.pointer,
                    operands::operand_value(stack.at(stack.pointer)?.clone()),
                )
            } else {
                // LOAD_PROP pops first, then captures ++sp before the getter.
                let mut stack = scope.state.operands.borrow_mut();
                let object = operands::operand_value(stack.pop()?);
                (stack.reserve(), object)
            };
            // Java captures the array position before getObjectProperty calls
            // toString. Failed name templates can leave newer operands above it.
            let value = self.lookup_property(&object, &property, scope)?;
            scope
                .state
                .operands
                .borrow_mut()
                .set(position, Operand::Value(value))?;
        }
        Ok(())
    }

    fn lookup_property(
        &self,
        object: &Value,
        property: &Value,
        scope: &mut eval::Scope<'_>,
    ) -> Result<Value, String> {
        let object = expression::dereference(object, &self.graph).unwrap_or(object);
        if object.is_null() {
            return Ok(Value::Null);
        }
        let name = if property.is_null() {
            Text::new()
        } else {
            self.isolated_string(property, scope)?
        };
        let name_string = name.valid_string();
        let name_str = name_string.as_deref();
        let value = if let Some(keys) = expression::dictionary_keys(object) {
            let dictionary = &object["__stg_dictionary"];
            let found = if property.is_null() {
                dictionary.get("default").cloned()
            } else if let Some(found) = name_str.and_then(|name| dictionary.get(name)) {
                Some(found.clone())
            } else if property.as_str() == Some("keys") {
                Some(beans::collection(
                    "java.util.LinkedHashMap$LinkedKeySet",
                    keys,
                ))
            } else if property.as_str() == Some("values") {
                Some(beans::collection(
                    "java.util.LinkedHashMap$LinkedValues",
                    keys.iter()
                        .map(|key| dictionary[key.as_str().unwrap()].clone())
                        .collect(),
                ))
            } else {
                dictionary.get("default").cloned()
            };
            match found {
                Some(value) if value.get("__stg_key") == Some(&Value::Bool(true)) => {
                    property.clone()
                }
                Some(value) => value,
                None => Value::Null,
            }
        } else if let Some(template) = object.get("__stg_named_call").and_then(Value::as_str) {
            self.definition(template)
                .and_then(|d| {
                    d.args
                        .iter()
                        .position(|p| Some(p.name.as_str()) == name_str)
                })
                .and_then(|position| object["args"].get(position))
                .and_then(|v| v.get("value"))
                .cloned()
                .unwrap_or(Value::Null)
        } else if object.get("__stg_inline_template").is_some()
            || object.get("__stg_anonymous_default").is_some()
        {
            name_str
                .and_then(|name| object["bindings"].get(name))
                .cloned()
                .unwrap_or(Value::Null)
        } else if property.is_null() {
            if object.get("__stg_attributes").is_some() {
                // SKYNETObjectAdaptor calls propertyName.startsWith even
                // when the indirect property supplied a null name.
                return Err(fault());
            }
            Value::Null
        } else if name.is_empty() {
            // ObjectModelAdaptor capitalizes charAt(0) before searching getters.
            return Err(fault());
        } else if name_str.is_some_and(|name| {
            matches!(
                name,
                "getClass"
                    | "hashCode"
                    | "toString"
                    | "equals"
                    | "clone"
                    | "finalize"
                    | "notify"
                    | "notifyAll"
                    | "wait"
            )
        }) && object.get("__stg_java_class").is_some()
        {
            // ReflectUtil finds inherited Object.getClass on the enhancer and
            // invokeStatic supplies null to this instance method.
            return Err(
                "__stg_getter_fault:Cannot invoke \"Object.getClass()\" because \"o\" is null"
                    .into(),
            );
        } else if object.get("__stg_java_class").is_some()
            && name_str
                .is_some_and(|name| !name.starts_with("__stg_") && object.get(name).is_some())
        {
            object.get(name_str.unwrap()).cloned().unwrap()
        } else if let Some(value) = name_str.and_then(|name| beans::property(object, name)) {
            // Attached attributes precede inherited model getters. Wrapper
            // representations themselves never expose their internal keys.
            if object.get("__stg_attributes").is_some() {
                name_str
                    .and_then(|name| object.get(name))
                    .cloned()
                    .unwrap_or(value)
            } else {
                value
            }
        } else if name_str.is_some_and(|name| name.starts_with("__stg_java_")) {
            Value::Null
        } else if name_str == Some("DELTA") && object.get("__stg_java_class").is_some() {
            object
                .get("DELTA")
                .cloned()
                .unwrap_or_else(|| Value::String("_delta".into()))
        } else {
            name_str
                .and_then(|name| object.get(name))
                .cloned()
                .or_else(|| {
                    name_str.and_then(|name| crate::preview_domain::has_attribute(object, name))
                })
                .unwrap_or(Value::Null)
        };
        let value = expression::dereference(&value, &self.graph)
            .cloned()
            .unwrap_or(value);
        let value = crate::graph_service::getter(value);
        if let Some(message) = expression::getter_fault(&value) {
            return Err(format!("__stg_getter_fault:{message}"));
        }
        Ok(value)
    }
}
