//! Shared ST4 operand values survive an exception caught by a nested exec.
use super::*;

#[derive(Clone)]
pub(super) enum Operand {
    Value(Value),
    Arguments(Map<String, Value>),
    Options(HashMap<String, Value>),
}

pub(super) fn fault() -> String {
    "__stg_getter_fault:invalid interpreter operand".into()
}
pub(super) fn push_value(scope: &mut eval::Scope<'_>, value: Value) -> Result<(), String> {
    scope
        .state
        .operands
        .borrow_mut()
        .push(Operand::Value(value))
}
pub(super) fn pop_value(scope: &mut eval::Scope<'_>) -> Result<Value, String> {
    Ok(operand_value(scope.state.operands.borrow_mut().pop()?))
}
pub(super) fn operand_value(operand: Operand) -> Value {
    match operand {
        Operand::Value(value) => value,
        Operand::Options(values) => Value::Array(
            ["anchor", "format", "null", "separator", "wrap"]
                .iter()
                .map(|name| values.get(*name).cloned().unwrap_or(Value::Null))
                .collect(),
        ),
        Operand::Arguments(values) => {
            let mut keys = values.keys().cloned().collect::<Vec<_>>();
            let mut capacity = 16;
            while values.len() > capacity * 3 / 4 {
                capacity *= 2;
            }
            keys.sort_by_key(|key| {
                let h = key
                    .encode_utf16()
                    .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(u32::from(c)));
                (h ^ (h >> 16)) & (capacity as u32 - 1)
            });
            serde_json::json!({"__stg_dictionary":values,"__stg_keys":keys})
        }
    }
}

impl StgTemplate {
    fn stringify_top(&self, scope: &mut eval::Scope<'_>) -> Result<(), String> {
        let position = scope.state.operands.borrow().pointer;
        let value = operand_value(scope.state.operands.borrow().at(position)?.clone());
        let value = if value.is_null() {
            Value::Null
        } else {
            self.isolated_string(&value, scope)?.into_value()
        };
        scope
            .state
            .operands
            .borrow_mut()
            .set(position, Operand::Value(value))?;
        Ok(())
    }
    pub(super) fn condition_operand(
        &self,
        input: &str,
        scope: &mut eval::Scope<'_>,
    ) -> Result<(), String> {
        let input = expression::unparenthesize(input.trim());
        for operator in ["||", "&&"] {
            let parts = expression::split_top_level(input, operator);
            if parts.len() > 1 {
                self.condition_operand(parts[0], scope)?;
                for part in parts.iter().skip(1) {
                    self.condition_operand(part, scope)?;
                    let right = expression::truthy(&pop_value(scope)?);
                    let left = expression::truthy(&pop_value(scope)?);
                    push_value(
                        scope,
                        Value::Bool(if operator == "||" {
                            left || right
                        } else {
                            left && right
                        }),
                    )?;
                }
                return Ok(());
            }
        }
        if let Some(inner) = input.strip_prefix('!') {
            self.condition_operand(inner, scope)?;
            let position = scope.state.operands.borrow().pointer;
            let value = operand_value(scope.state.operands.borrow().at(position)?.clone());
            scope.state.operands.borrow_mut().set(
                position,
                Operand::Value(Value::Bool(!expression::truthy(&value))),
            )?;
            return Ok(());
        }
        self.evaluate_operand(input, scope)
    }

    pub(super) fn evaluate_operand(
        &self,
        input: &str,
        scope: &mut eval::Scope<'_>,
    ) -> Result<(), String> {
        let input = input.trim();
        if scope.vars.contains_key("__stg_generator_naming")
            && (input == "naming.imports" || input.starts_with("naming.(") || input.starts_with("naming.package.("))
        {
            let value = self.eval(input, scope.vars, scope.own_package, scope.imports, scope.depth, scope.state)?;
            return push_value(scope, Value::String(if input == "naming.imports" && value.is_empty() {
                "__stg_empty_generator_imports__".into()
            } else { value }));
        }
        let grouped = expression::unparenthesize(input);
        if !scope.conditional && grouped != input {
            self.evaluate_operand(&input[1..input.len() - 1], scope)?;
            // Java captures the LHS array index before the RHS executes. A
            // failed nested template may leave newer operands above this slot.
            return self.stringify_top(scope);
        }
        let input = grouped;
        if input.starts_with('"') && input.ends_with('"') {
            let value = Text::from_json_literal(input).ok_or_else(fault)?;
            return push_value(scope, value.into_value());
        }
        if input.starts_with('{') && input.ends_with('}') {
            push_value(
                scope,
                self.anonymous_value(input, scope.state.no_newlines, scope.state.missing_implicit)?,
            )?;
            return Ok(());
        }
        if expression::split_top_level(input, ":").len() > 1 {
            return self.mapping_operand(input, scope);
        }
        if input.starts_with('[')
            && expression::matching_end(input, 0, '[', ']') == Some(input.len() - 1)
        {
            push_value(
                scope,
                beans::collection("org.stringtemplate.v4.Interpreter$ObjectList", Vec::new()),
            )?;
            let body = &input[1..input.len() - 1];
            for item in expression::split_top_level(body, ",")
                .into_iter()
                .filter(|_| !body.trim().is_empty())
            {
                if item.is_empty() {
                    push_value(scope, Value::Null)?;
                } else {
                    self.evaluate_operand(item, scope)?;
                }
                let value = pop_value(scope)?;
                let mut stack = scope.state.operands.borrow_mut();
                let Operand::Value(value_list) = stack.top_mut()? else {
                    return Err(fault());
                };
                let values = value_list
                    .get_mut("__stg_java_sequence")
                    .and_then(Value::as_array_mut)
                    .ok_or_else(fault)?;
                if let Some(items) = expression::java_sequence(&value) {
                    values.extend(items.iter().cloned());
                } else if let Value::Array(items) = value {
                    values.extend(items);
                } else {
                    values.push(value);
                }
            }
            return Ok(());
        }
        if let Some((name, arguments)) = eval::call(input) {
            if expression::builtin(name) {
                if !arguments.is_empty() {
                    self.evaluate_operand(arguments, scope)?;
                }
                if !matches!(name, "trim" | "strlen") {
                    let position = scope.state.operands.borrow().pointer;
                    let argument = scope.state.operands.borrow().at(position)?.clone();
                    // These opcodes replace operands[sp], without a pop.
                    let value = operand_value(argument);
                    let result = expression::apply_builtin(name, value);
                    if let Some(message) = expression::getter_fault(&result) {
                        return Err(format!("__stg_getter_fault:{message}"));
                    }
                    scope
                        .state
                        .operands
                        .borrow_mut()
                        .set(position, Operand::Value(result))?;
                    return Ok(());
                }
                let argument = scope.state.operands.borrow_mut().pop()?;
                if matches!(&argument, Operand::Value(Value::Null)) {
                    return Err("__stg_getter_fault:Cannot invoke \"Object.getClass()\" because \"o\" is null".into());
                }
                let value = if let Some(text) = Text::from_value(&operand_value(argument.clone())) {
                    Operand::Value(if name == "strlen" {
                        Value::from(text.len())
                    } else {
                        text.trim().into_value()
                    })
                } else if name == "trim" {
                    argument
                } else {
                    Operand::Value(Value::from(0))
                };
                scope.state.operands.borrow_mut().push(value)?;
                return Ok(());
            }
            if name.starts_with('(') {
                return self.indirect_operand(name, arguments, 0, scope);
            }
            let name = name.to_owned();
            let arguments = self.arguments_operand(&name, arguments, &[], scope)?;
            push_value(scope, writer::named(&name, &arguments))?;
            return Ok(());
        }
        if expression::split_top_level(input, ".").len() > 1 {
            return self.property_operand(input, scope);
        }
        let value = expression::value_with_graph(input, scope.vars, &self.graph);
        if let Some(message) = expression::getter_fault(&value) {
            return Err(format!("__stg_getter_fault:{message}"));
        }
        push_value(scope, value)?;
        Ok(())
    }

    pub(super) fn indirect_operand(
        &self,
        name: &str,
        input: &str,
        mapped_count: usize,
        scope: &mut eval::Scope<'_>,
    ) -> Result<(), String> {
        self.evaluate_operand(&name[1..name.len() - 1], scope)?;
        self.stringify_top(scope)?;
        for _ in 0..mapped_count {
            push_value(scope, Value::Null)?;
        }
        let expressions = expression::split_top_level(input, ",")
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        for argument in &expressions {
            self.evaluate_operand(argument, scope)?;
        }
        let count = mapped_count + expressions.len();
        let position = scope.state.operands.borrow().pointer - count as isize;
        let actual_name = match scope.state.operands.borrow().at(position)? {
            Operand::Value(value) => Text::from_value(value).ok_or_else(fault)?,
            _ => return Err(fault()),
        };
        let mut values = Vec::new();
        let formals = self
            .definition(&actual_name.valid_string().unwrap_or_default())
            .map(|d| d.args.len())
            .unwrap_or(0);
        {
            let mut stack = scope.state.operands.borrow_mut();
            for i in 0..count.min(formals) {
                values.push(operand_value(stack.at(position + 1 + i as isize)?.clone()));
            }
            stack.drop(count + 1);
        }
        let mut arguments = vec![None; formals];
        for (i, value) in values.into_iter().enumerate() {
            if let Some(slot) = arguments.get_mut(i) {
                *slot = Some(value);
            }
        }
        let mut call = writer::named("", &arguments);
        call["__stg_named_call"] = actual_name.into_value();
        push_value(scope, call)?;
        Ok(())
    }

    pub(super) fn arguments_operand(
        &self,
        name: &str,
        input: &str,
        mapped: &[Value],
        scope: &mut eval::Scope<'_>,
    ) -> Result<Vec<Option<Value>>, String> {
        let arguments = expression::split_top_level(input, ",")
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let boxed = arguments
            .iter()
            .any(|value| *value == "..." || expression::split_top_level(value, "=").len() == 2);
        let definition = self.definition(name);
        let mut result = vec![None; definition.map(|value| value.args.len()).unwrap_or(0)];
        // The compiler emits implicit null map arguments before ARGS, even
        // when NEW_BOX_ARGS subsequently consumes only the argument map.
        for value in mapped {
            push_value(scope, value.clone())?;
        }
        if boxed {
            scope
                .state
                .operands
                .borrow_mut()
                .push(Operand::Arguments(Map::new()))?;
            for argument in arguments {
                if argument == "..." {
                    continue;
                }
                let assignment = expression::split_top_level(argument, "=");
                if assignment.len() != 2 {
                    self.evaluate(argument, scope)?;
                    continue;
                }
                self.evaluate_operand(assignment[1], scope)?;
                let value = pop_value(scope)?;
                let mut stack = scope.state.operands.borrow_mut();
                let Operand::Arguments(values) = stack.top_mut()? else {
                    return Err(fault());
                };
                values.insert(assignment[0].into(), value);
            }
            let values = match scope.state.operands.borrow_mut().pop()? {
                Operand::Arguments(values) => values,
                _ => return Err(fault()),
            };
            if let Some(definition) = definition {
                let passthrough = expression::split_top_level(input, ",").contains(&"...");
                for (i, formal) in definition.args.iter().enumerate() {
                    result[i] = values.get(&formal.name).cloned().or_else(|| {
                        if passthrough {
                            scope.vars.get(&formal.name).cloned()
                        } else {
                            None
                        }
                    });
                }
            }
        } else {
            let count = mapped.len() + arguments.len();
            for argument in arguments {
                self.evaluate_operand(argument, scope)?;
            }
            let mut values = Vec::new();
            {
                let mut stack = scope.state.operands.borrow_mut();
                let start = stack.pointer - count as isize + 1;
                for i in 0..count.min(result.len()) {
                    values.push(operand_value(stack.at(start + i as isize)?.clone()));
                }
                stack.drop(count);
            }
            for (i, value) in values.into_iter().enumerate() {
                if let Some(slot) = result.get_mut(i) {
                    *slot = Some(value);
                }
            }
        }
        Ok(result)
    }
}
