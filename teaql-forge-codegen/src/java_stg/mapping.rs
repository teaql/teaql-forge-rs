//! ST4 constructs map prototypes once, while source values remain on the stack.
use super::*;
use operands::{fault, pop_value, push_value};

impl StgTemplate {
    pub(super) fn mapping_operand(
        &self,
        input: &str,
        scope: &mut eval::Scope<'_>,
    ) -> Result<(), String> {
        let stages = expression::split_top_level(input, ":");
        let sources = expression::split_top_level(stages[0], ",");
        for source in &sources {
            self.evaluate_operand(source, scope)?;
        }
        let zipped = sources.len() > 1;
        if zipped {
            self.prototype_operand(stages[1], sources.len(), scope)?;
            let prototype = pop_template(scope)?;
            let mut values = Vec::new();
            {
                let mut stack = scope.state.operands.borrow_mut();
                let start = stack.pointer - sources.len() as isize + 1;
                for i in 0..sources.len() {
                    values.push(operands::operand_value(
                        stack.at(start + i as isize)?.clone(),
                    ));
                }
                stack.drop(sources.len());
            }
            let position = scope.state.operands.borrow_mut().reserve();
            let value = self.zip_prototype(values, prototype)?;
            scope
                .state
                .operands
                .borrow_mut()
                .set(position, operands::Operand::Value(value))?;
        }
        for stage in stages.iter().skip(if zipped { 2 } else { 1 }) {
            let prototypes = expression::split_top_level(stage, ",");
            for prototype in &prototypes {
                self.prototype_operand(prototype, 1, scope)?;
            }
            let mut values = Vec::new();
            // ROT_MAP casts every prototype before it adjusts the stack.
            if prototypes.len() > 1 {
                let stack = scope.state.operands.borrow();
                let start = stack.pointer - prototypes.len() as isize + 1;
                for i in 0..prototypes.len() {
                    let value = operands::operand_value(stack.at(start + i as isize)?.clone());
                    if !is_template(&value) {
                        return Err(fault());
                    }
                    values.push(value);
                }
                drop(stack);
                scope.state.operands.borrow_mut().drop(prototypes.len());
            } else {
                values.push(pop_template(scope)?);
            }
            let source = pop_value(scope)?;
            // 4.3.3's ROT_MAP has no push in its null branch. A caller may
            // therefore consume a value retained by an earlier instruction.
            if source.is_null() && prototypes.len() > 1 {
                continue;
            }
            let value = self.rotate_prototypes(source, &values)?;
            push_value(scope, value)?;
        }
        Ok(())
    }

    fn prototype_operand(
        &self,
        input: &str,
        count: usize,
        scope: &mut eval::Scope<'_>,
    ) -> Result<(), String> {
        let input = input.trim();
        if input.starts_with('{') && input.ends_with('}') {
            push_value(
                scope,
                self.anonymous_value(input, scope.state.no_newlines, scope.state.missing_implicit)?,
            )?;
        } else {
            let (name, args) = eval::call(input).ok_or_else(fault)?;
            if name.starts_with('(') {
                return self.indirect_operand(name, args, count, scope);
            }
            let args = self.arguments_operand(name, args, &vec![Value::Null; count], scope)?;
            push_value(scope, writer::named(name, &args))?;
        }
        Ok(())
    }

    pub(super) fn anonymous_value(
        &self,
        input: &str,
        no_newlines: bool,
        missing: bool,
    ) -> Result<Value, String> {
        if missing {
            // Missing compiled names produce ST4's empty CompiledST fallback.
            return Ok(serde_json::json!({"__stg_inline_template":"","bindings":{},
                "__stg_parameters":[],"__stg_missing_implicit":true}));
        }
        let value = syntax::anonymous(input)?;
        let mut bindings = Map::new();
        for name in &value.parameters {
            bindings.insert(name.clone(), Value::Null);
        }
        bindings.insert("i".into(), Value::Null);
        bindings.insert("i0".into(), Value::Null);
        Ok(
            serde_json::json!({"__stg_inline_template":value.body,"bindings":bindings,
            "__stg_parameters":value.parameters,"__stg_no_newlines":no_newlines,
            "__stg_initial_line_start":value.initial_line_start}),
        )
    }

    fn parameters(&self, prototype: &Value) -> Vec<String> {
        if let Some(values) = prototype.get("__stg_parameters").and_then(Value::as_array) {
            return values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
        }
        prototype
            .get("__stg_named_call")
            .and_then(Value::as_str)
            .and_then(|name| self.definition(name))
            .map(|d| d.args.iter().map(|arg| arg.name.clone()).collect())
            .unwrap_or_default()
    }

    fn fill_prototype(
        &self,
        prototype: &Value,
        values: &[Value],
        index: usize,
        zipped: bool,
    ) -> Result<Value, String> {
        if prototype.is_null() {
            return Err(fault());
        }
        let mut result = prototype.clone();
        if result.get("__stg_inline_template").is_some() {
            let parameters = self.parameters(prototype);
            let bindings = result["bindings"].as_object_mut().ok_or_else(fault)?;
            if prototype.get("__stg_missing_implicit") == Some(&Value::Bool(true)) {
                if let Some(value) = values.first() {
                    bindings.insert("it".into(), value.clone());
                }
                return Ok(result);
            }
            bindings.insert("i".into(), Value::from(index + 1));
            bindings.insert("i0".into(), Value::from(index));
            for (name, value) in parameters.iter().zip(values) {
                bindings.insert(name.clone(), value.clone());
            }
        } else {
            let parameters = self.parameters(prototype);
            let arguments = result["args"].as_array_mut().ok_or_else(fault)?;
            if zipped {
                // zip_map unconditionally rawSetAttribute(i0/i) on named
                // templates too. Missing formals throw before iteration.
                for (name, value) in [("i0", index), ("i", index + 1)] {
                    let position = parameters
                        .iter()
                        .position(|p| p == name)
                        .ok_or_else(fault)?;
                    arguments[position] = serde_json::json!({"value":value});
                }
            }
            for (slot, value) in arguments.iter_mut().zip(values) {
                *slot = serde_json::json!({"value":value});
            }
        }
        Ok(result)
    }

    fn rotate_prototypes(&self, source: Value, prototypes: &[Value]) -> Result<Value, String> {
        if source.is_null() {
            return Ok(Value::Null);
        }
        if source.is_array()
            || expression::java_sequence(&source).is_some()
            || expression::dictionary_keys(&source).is_some()
        {
            let mut values = Vec::new();
            let mut index = 0;
            for value in map_items(source) {
                if value.is_null() {
                    values.push(Value::Null);
                    continue;
                }
                values.push(self.fill_prototype(
                    &prototypes[index % prototypes.len()],
                    &[value],
                    index,
                    false,
                )?);
                index += 1;
            }
            Ok(Value::Array(values))
        } else {
            self.fill_prototype(&prototypes[0], &[source], 0, false)
        }
    }

    fn zip_prototype(&self, sources: Vec<Value>, prototype: Value) -> Result<Value, String> {
        if prototype.is_null()
            || prototype.get("__stg_missing_implicit") == Some(&Value::Bool(true))
        {
            return Ok(Value::Null);
        }
        let parameters = self.parameters(&prototype);
        if parameters.is_empty() {
            return Ok(Value::Null);
        }
        let sources = sources
            .into_iter()
            .take(parameters.len())
            .map(map_items)
            .collect::<Vec<_>>();
        // Attribute setup occurs even when every input iterator is empty.
        self.fill_prototype(&prototype, &[], 0, true)?;
        let length = sources.iter().map(Vec::len).max().unwrap_or(0);
        let mut values = Vec::new();
        for index in 0..length {
            let row = sources
                .iter()
                .map(|list| list.get(index).cloned().unwrap_or(Value::Null))
                .collect::<Vec<_>>();
            // An exhausted iterator leaves the prototype's default slot
            // untouched, whereas an explicit null replaces it.
            let mut value = self.fill_prototype(&prototype, &[], index, true)?;
            if value.get("__stg_inline_template").is_some() {
                for ((parameter, list), item) in parameters.iter().zip(&sources).zip(row) {
                    if index < list.len() {
                        value["bindings"][parameter] = item;
                    }
                }
            } else {
                let arguments = value["args"].as_array_mut().ok_or_else(fault)?;
                for ((slot, list), item) in arguments.iter_mut().zip(&sources).zip(row) {
                    if index < list.len() {
                        *slot = serde_json::json!({"value":item});
                    }
                }
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }
}

fn is_template(value: &Value) -> bool {
    value.is_null()
        || value.get("__stg_named_call").is_some()
        || value.get("__stg_inline_template").is_some()
}
fn pop_template(scope: &mut eval::Scope<'_>) -> Result<Value, String> {
    let value = pop_value(scope)?;
    if is_template(&value) {
        Ok(value)
    } else {
        Err(fault())
    }
}
