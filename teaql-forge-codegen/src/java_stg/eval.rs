//! Template calls and mapping operate on values before output options are applied.
use super::*;

pub(super) struct Scope<'a> {
    pub vars: &'a mut Map<String, Value>,
    pub own_package: &'a str,
    pub imports: &'a mut BTreeSet<String>,
    pub depth: &'a mut usize,
    pub state: &'a mut WriterState,
    pub conditional: bool,
}

impl StgTemplate {
    pub(super) fn evaluate(&self, input: &str, scope: &mut Scope<'_>) -> Result<Value, String> {
        if self.source_whitespace {
            self.evaluate_operand(input, scope)?;
            return operands::pop_value(scope);
        }
        self.evaluate_inner(input, scope)
    }

    pub(super) fn evaluate_inner(
        &self,
        input: &str,
        scope: &mut Scope<'_>,
    ) -> Result<Value, String> {
        let input = input.trim();
        let grouped = expression::unparenthesize(input);
        if self.source_whitespace && !scope.conditional && grouped != input {
            let value = self.evaluate(&input[1..input.len() - 1], scope)?;
            return if value.is_null() {
                Ok(Value::Null)
            } else {
                self.isolated_string(&value, scope).map(Text::into_value)
            };
        }
        let input = grouped;
        let stages = expression::split_top_level(input, ":");
        if stages.len() > 1 {
            let sources = expression::split_top_level(stages[0], ",")
                .into_iter()
                .map(|source| self.evaluate(source, scope))
                .collect::<Result<Vec<_>, _>>()?;
            let mut value = if sources.len() > 1 {
                let lists = sources.into_iter().map(map_items).collect::<Vec<_>>();
                let length = lists.iter().map(Vec::len).max().unwrap_or(0);
                let mut mapped = Vec::new();
                for index in 0..length {
                    let values = lists
                        .iter()
                        .map(|list| list.get(index).cloned().unwrap_or(Value::Null))
                        .collect::<Vec<_>>();
                    mapped.push(self.map_template(stages[1], &values, index, scope)?);
                }
                Value::Array(mapped)
            } else {
                sources.into_iter().next().unwrap_or(Value::Null)
            };
            for stage in
                stages
                    .iter()
                    .skip(if expression::split_top_level(stages[0], ",").len() > 1 {
                        2
                    } else {
                        1
                    })
            {
                let prototypes = expression::split_top_level(stage, ",");
                // ST4 4.3.3 leaves the implicit map argument on its operand
                // stack when NEW_BOX_ARGS consumes named arguments. MAP then
                // reads that null instead of the input collection. Preserve
                // the observable null (including the output null option).
                if prototypes.len() == 1 && boxed_map(prototypes[0]) {
                    value = Value::Null;
                    continue;
                }
                if value.is_null() {
                    continue;
                }
                let mut mapped = Vec::new();
                let mut index = 0;
                for item in map_items(value) {
                    if item.is_null() {
                        mapped.push(Value::Null);
                        continue;
                    }
                    mapped.push(self.map_template(
                        prototypes[index % prototypes.len()],
                        &[item],
                        index,
                        scope,
                    )?);
                    index += 1;
                }
                value = Value::Array(mapped);
            }
            return Ok(value);
        }
        if input.starts_with('[')
            && expression::matching_end(input, 0, '[', ']') == Some(input.len() - 1)
        {
            let mut values = Vec::new();
            for item in expression::split_top_level(&input[1..input.len() - 1], ",") {
                if item.is_empty() {
                    continue;
                }
                let value = self.evaluate(item, scope)?;
                if let Some(items) = expression::java_sequence(&value) {
                    values.extend(items.iter().cloned());
                } else if let Value::Array(items) = value {
                    values.extend(items);
                } else {
                    values.push(value);
                }
            }
            return Ok(beans::collection(
                "org.stringtemplate.v4.Interpreter$ObjectList",
                values,
            ));
        }
        if let Some((name, arguments)) = call(input) {
            if expression::builtin(name) {
                let argument = self.evaluate(arguments, scope)?;
                if self.source_whitespace && argument.is_null() && matches!(name, "trim" | "strlen")
                {
                    return Err("__stg_getter_fault:Cannot invoke \"Object.getClass()\" because \"o\" is null".into());
                }
                if self.source_whitespace && !argument.is_string() && !argument.is_null() {
                    if name == "strlen" {
                        return Ok(Value::from(0));
                    }
                    if name == "trim" {
                        return Ok(argument);
                    }
                }
                let result = expression::apply_builtin(name, argument);
                if let Some(message) = expression::getter_fault(&result) {
                    return Err(format!("__stg_getter_fault:{message}"));
                }
                return Ok(result);
            }
            let name = if name.starts_with('(') {
                let value = self.evaluate(name, scope)?;
                if self.source_whitespace {
                    self.isolated_string(&value, scope)?.utf8()
                } else {
                    value_string(&value)
                }
            } else {
                name.to_string()
            };
            let args = self.arguments(&name, arguments, &[], scope)?;
            if self.source_whitespace {
                return Ok(super::writer::named(&name, &args));
            }
            return self
                .render_named(
                    &name,
                    &args,
                    scope.vars,
                    scope.own_package,
                    scope.imports,
                    scope.depth,
                    scope.state,
                )
                .map(Text::into_value);
        }
        let value = expression::value_with_graph(input, scope.vars, &self.graph);
        if let Some(message) = expression::getter_fault(&value) {
            return Err(format!("__stg_getter_fault:{message}"));
        }
        Ok(value)
    }

    pub(super) fn evaluate_condition(
        &self,
        input: &str,
        scope: &mut Scope<'_>,
    ) -> Result<bool, String> {
        let previous = scope.conditional;
        scope.conditional = true;
        let result = self.condition(input, scope);
        scope.conditional = previous;
        result
    }

    fn condition(&self, input: &str, scope: &mut Scope<'_>) -> Result<bool, String> {
        if self.source_whitespace {
            self.condition_operand(input, scope)?;
            return Ok(expression::truthy(&operands::pop_value(scope)?));
        }
        let input = expression::unparenthesize(input.trim());
        let alternatives = expression::split_top_level(input, "||");
        if alternatives.len() > 1 {
            if self.source_whitespace {
                let mut result = false;
                for part in alternatives {
                    result |= self.evaluate_condition(part, scope)?;
                }
                return Ok(result);
            }
            for part in alternatives {
                if self.evaluate_condition(part, scope)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }
        let conjuncts = expression::split_top_level(input, "&&");
        if conjuncts.len() > 1 {
            if self.source_whitespace {
                let mut result = true;
                for part in conjuncts {
                    result &= self.evaluate_condition(part, scope)?;
                }
                return Ok(result);
            }
            for part in conjuncts {
                if !self.evaluate_condition(part, scope)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        if let Some(inner) = input.strip_prefix('!') {
            return self.evaluate_condition(inner, scope).map(|v| !v);
        }
        self.evaluate(input, scope)
            .map(|value| expression::truthy(&value))
    }

    fn arguments(
        &self,
        name: &str,
        input: &str,
        mapped: &[Value],
        scope: &mut Scope<'_>,
    ) -> Result<Vec<Option<Value>>, String> {
        if self.source_whitespace {
            return self.arguments_operand(name, input, mapped, scope);
        }
        let Some(definition) = self.definition(name) else {
            // Arguments are evaluated before ST4 creates the empty fallback
            // instance for a missing embedded template.
            for argument in expression::split_top_level(input, ",") {
                if argument.is_empty() || argument == "..." {
                    continue;
                }
                let named = expression::split_top_level(argument, "=");
                self.evaluate(if named.len() == 2 { named[1] } else { argument }, scope)?;
            }
            return Ok(Vec::new());
        };
        let mut values = vec![None; definition.args.len()];
        let mut cursor = mapped.len();
        let mut passthrough = false;
        for argument in expression::split_top_level(input, ",") {
            if argument.is_empty() {
                continue;
            }
            if argument == "..." {
                passthrough = true;
                continue;
            }
            let named = expression::split_top_level(argument, "=");
            let (index, expr) = if named.len() == 2 {
                let Some(index) = definition
                    .args
                    .iter()
                    .position(|formal| formal.name == named[0])
                else {
                    continue;
                };
                (index, named[1])
            } else {
                let index = cursor;
                cursor += 1;
                (index, argument)
            };
            let value = self.evaluate(expr, scope)?;
            if let Some(slot) = values.get_mut(index) {
                *slot = Some(value);
            }
        }
        if passthrough {
            for (index, formal) in definition.args.iter().enumerate() {
                if values[index].is_none() {
                    values[index] = scope.vars.get(&formal.name).cloned();
                }
            }
        }
        for (index, value) in mapped.iter().enumerate() {
            if let Some(slot) = values.get_mut(index) {
                *slot = Some(value.clone());
            }
        }
        Ok(values)
    }

    fn map_template(
        &self,
        prototype: &str,
        values: &[Value],
        index: usize,
        scope: &mut Scope<'_>,
    ) -> Result<Value, String> {
        let prototype = prototype.trim();
        if let Some(body) = prototype
            .strip_prefix('{')
            .and_then(|body| body.strip_suffix('}'))
        {
            let (params, body) = body
                .split_once('|')
                .ok_or_else(|| format!("invalid anonymous map: {prototype}"))?;
            if self.source_whitespace {
                let mut bindings = Map::new();
                for (position, param) in expression::split_top_level(params, ",")
                    .into_iter()
                    .enumerate()
                {
                    bindings.insert(
                        param.into(),
                        values.get(position).cloned().unwrap_or(Value::Null),
                    );
                }
                bindings.insert("i".into(), Value::from(index + 1));
                bindings.insert("i0".into(), Value::from(index));
                return Ok(
                    serde_json::json!({"__stg_inline_template":body.strip_prefix(' ').unwrap_or(body),"bindings":bindings}),
                );
            }
            let mut previous = Vec::new();
            for (position, param) in expression::split_top_level(params, ",")
                .into_iter()
                .enumerate()
            {
                previous.push((
                    param.to_string(),
                    scope.vars.insert(
                        param.into(),
                        values.get(position).cloned().unwrap_or(Value::Null),
                    ),
                ));
            }
            previous.push((
                "i".into(),
                scope.vars.insert("i".into(), Value::from(index + 1)),
            ));
            previous.push((
                "i0".into(),
                scope.vars.insert("i0".into(), Value::from(index)),
            ));
            let tokens = tokenize(
                body.strip_prefix(' ').unwrap_or(body),
                self.source_whitespace,
                self.delimiters,
            )?;
            let rendered = self
                .render_range(
                    &tokens,
                    0,
                    scope.vars,
                    scope.own_package,
                    scope.imports,
                    scope.depth,
                    scope.state,
                )?
                .0;
            for (name, value) in previous.into_iter().rev() {
                restore_var(scope.vars, &name, value);
            }
            return Ok(rendered.into_value());
        }
        let (name, args) =
            call(prototype).ok_or_else(|| format!("invalid map template: {prototype}"))?;
        let name = if name.starts_with('(') {
            let value = self.evaluate(name, scope)?;
            if self.source_whitespace {
                self.isolated_string(&value, scope)?.utf8()
            } else {
                value_string(&value)
            }
        } else {
            name.to_string()
        };
        let args = self.arguments(&name, args, values, scope)?;
        if self.source_whitespace {
            return Ok(super::writer::named(&name, &args));
        }
        self.render_named(
            &name,
            &args,
            scope.vars,
            scope.own_package,
            scope.imports,
            scope.depth,
            scope.state,
        )
        .map(Text::into_value)
    }
}

fn boxed_map(prototype: &str) -> bool {
    let Some((name, arguments)) = call(prototype.trim()) else {
        return false;
    };
    !name.starts_with('(')
        && expression::split_top_level(arguments, ",")
            .into_iter()
            .any(|argument| {
                argument == "..." || expression::split_top_level(argument, "=").len() == 2
            })
}

pub(super) fn call(input: &str) -> Option<(&str, &str)> {
    let opening = if input.starts_with('(') {
        expression::matching_end(input, 0, '(', ')')? + 1
    } else {
        input.find('(')?
    };
    if input.as_bytes().get(opening) != Some(&b'(')
        || input[..opening].trim_end().ends_with('.')
        || expression::matching_end(input, opening, '(', ')')? + 1 != input.len()
    {
        return None;
    }
    Some((
        input[..opening].trim(),
        &input[opening + 1..input.len() - 1],
    ))
}
