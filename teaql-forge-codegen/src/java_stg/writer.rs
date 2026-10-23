//! ST4 4.3.3's AutoIndentWriter and deferred template values for previews.
use super::*;

#[derive(Clone)]
pub(super) struct AutoIndentWriter {
    pub indents: Vec<String>,
    pub anchors: Vec<usize>,
    anchor_limit: usize,
    position: usize,
    start: bool,
    index: usize,
    discarded: usize,
}

impl Default for AutoIndentWriter {
    fn default() -> Self {
        Self {
            indents: Vec::new(),
            anchors: Vec::new(),
            anchor_limit: 10,
            position: 0,
            start: true,
            index: 0,
            discarded: 0,
        }
    }
}

impl AutoIndentWriter {
    fn push_anchor(&mut self) {
        if self.anchors.len() == self.anchor_limit {
            // 4.3.3 copies length-1 entries when growing its Java int array.
            if let Some(last) = self.anchors.last_mut() {
                *last = 0;
            }
            self.anchor_limit *= 2;
        }
        self.anchors.push(self.position);
    }

    pub fn checkpoint(&self) -> (usize, usize) {
        (self.index, self.discarded)
    }
    pub fn count_since(&self, point: (usize, usize)) -> usize {
        (self.index - point.0) - (self.discarded - point.1)
    }
    pub fn failed_since(&mut self, point: (usize, usize)) {
        self.discarded += self.count_since(point);
    }

    fn indent(&mut self, output: &mut Text) -> usize {
        let mut count = 0;
        for value in &self.indents {
            output.push_str(value);
            count += value.encode_utf16().count();
        }
        if let Some(anchor) = self.anchors.last() {
            if *anchor > count {
                output.push_str(&" ".repeat(*anchor - count));
                count = *anchor;
            }
        }
        self.position += count;
        count
    }

    pub fn write(&mut self, output: &mut Text, value: &Text) -> usize {
        let mut written = 0;
        for unit in &value.0 {
            if unit == &13 {
                continue;
            }
            if unit == &10 {
                output.0.push(10);
                self.start = true;
                // Preserve 4.3.3's cumulative count within a single write.
                self.position = written;
                written += 1;
                continue;
            }
            if self.start {
                written += self.indent(output);
                self.start = false;
            }
            output.0.push(*unit);
            written += 1;
            self.position += 1;
        }
        self.index += written;
        written
    }
}

// Manual generation paths share ST's character and continuation handling.
// The surrounding literal has already started the first line and continues
// immediately after the value, including when the value ends with a newline.
pub(crate) fn inline_value(value: &str, indent: usize) -> String {
    let mut writer = AutoIndentWriter { start: false, ..Default::default() };
    writer.indents.push(" ".repeat(indent));
    let mut output = Text::new();
    writer.write(&mut output, &Text::from(value));
    if writer.start { writer.indent(&mut output); }
    output.utf8()
}

pub(super) fn named(name: &str, args: &[Option<Value>]) -> Value {
    serde_json::json!({"__stg_named_call": name, "args": args.iter().map(|value|
        value.as_ref().map(|value|serde_json::json!({"value":value}))).collect::<Vec<_>>()})
}

impl StgTemplate {
    pub(super) fn isolated_string(
        &self,
        value: &Value,
        scope: &mut eval::Scope<'_>,
    ) -> Result<Text, String> {
        if let Some(value) = Text::from_value(value) {
            return Ok(value);
        }
        let mut state = WriterState::preview();
        state.operands = scope.state.operands.clone();
        state.missing_implicit = scope.state.missing_implicit;
        state.written_since_newline = scope.state.written_since_newline;
        let mut isolated = eval::Scope {
            conditional: false,
            vars: scope.vars,
            own_package: scope.own_package,
            imports: scope.imports,
            depth: scope.depth,
            state: &mut state,
        };
        let result = self.render_stream(value, &Text::new(), None, &mut isolated);
        scope.state.written_since_newline = state.written_since_newline;
        result
    }

    pub(super) fn eval_stream(
        &self,
        input: &str,
        vars: &mut Map<String, Value>,
        own_package: &str,
        imports: &mut BTreeSet<String>,
        depth: &mut usize,
        state: &mut WriterState,
    ) -> Result<Text, String> {
        let pieces = expression::split_top_level(input, ";");
        let mut scope = eval::Scope {
            conditional: false,
            vars,
            own_package,
            imports,
            depth,
            state,
        };
        self.evaluate_operand(pieces[0], &mut scope)?;
        let has_options = pieces.len() > 1;
        if has_options {
            scope
                .state
                .operands
                .borrow_mut()
                .push(operands::Operand::Options(HashMap::new()))?;
        }
        for option in pieces
            .iter()
            .skip(1)
            .flat_map(|piece| expression::split_top_level(piece, ","))
        {
            let (name, expr) = option
                .split_once('=')
                .map(|(a, b)| (a.trim(), b.trim()))
                .unwrap_or((option.trim(), ""));
            if !matches!(name, "anchor" | "format" | "null" | "separator" | "wrap") {
                continue;
            }
            let result = if expr.is_empty() {
                match name {
                    "anchor" => Value::Bool(true),
                    "wrap" => Value::String("\n".into()),
                    _ => continue,
                }
            } else {
                self.evaluate_operand(expr, &mut scope)?;
                operands::pop_value(&mut scope)?
            };
            let mut stack = scope.state.operands.borrow_mut();
            let operands::Operand::Options(options) = stack.top_mut()? else {
                return Err(operands::fault());
            };
            options.insert(name.to_owned(), result);
        }
        let options = if has_options {
            match scope.state.operands.borrow_mut().pop()? {
                operands::Operand::Options(options) => options,
                operands::Operand::Value(Value::Null) => HashMap::new(),
                _ => return Err(operands::fault()),
            }
        } else {
            HashMap::new()
        };
        let value = operands::pop_value(&mut scope)?;
        // ST4 converts options in enum order with independent writers, before
        // pushing the anchor. The original value is evaluated first.
        let mut strings = HashMap::new();
        for name in ["anchor", "format", "null", "separator", "wrap"] {
            if let Some(value) = options.get(name).filter(|value| !value.is_null()) {
                strings.insert(name, self.isolated_string(value, &mut scope)?);
            }
        }
        let anchor = options.get("anchor").is_some_and(|value| !value.is_null());
        if anchor {
            let writer = scope.state.writer.as_mut().unwrap();
            writer.push_anchor();
        }
        // TemplatePreviewService calls ST.render() with NO_WRAP and registers
        // no AttributeRenderer: wrap/format strings are evaluated but inert.
        let rendered = self.render_stream(
            &value,
            strings.get("separator").unwrap_or(&Text::new()),
            strings.get("null"),
            &mut scope,
        );
        if anchor {
            scope.state.writer.as_mut().unwrap().anchors.pop();
        }
        rendered
    }

    fn render_stream(
        &self,
        value: &Value,
        separator: &Text,
        null: Option<&Text>,
        scope: &mut eval::Scope<'_>,
    ) -> Result<Text, String> {
        if let Some(name) = value.get("__stg_named_call") {
            let Some(name) = Text::from_value(name).and_then(|text| text.valid_string()) else {
                return Ok(Text::new());
            };
            let args = value["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.get("value").cloned())
                .collect::<Vec<_>>();
            return self.render_named(
                &name,
                &args,
                scope.vars,
                scope.own_package,
                scope.imports,
                scope.depth,
                scope.state,
            );
        }
        let body = value
            .get("__stg_inline_template")
            .or_else(|| value.get("__stg_anonymous_default"))
            .and_then(Value::as_str);
        if let Some(body) = body {
            let checkpoint = scope.state.writer.as_ref().unwrap().checkpoint();
            *scope.depth += 1;
            if *scope.depth > MAX_RENDER_DEPTH {
                return Err("template recursion limit in anonymous template".into());
            }
            let no_newlines = value.get("__stg_no_newlines") == Some(&Value::Bool(true));
            let initial_line_start = value
                .get("__stg_initial_line_start")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let previous_no_newlines = std::mem::replace(&mut scope.state.no_newlines, no_newlines);
            let previous_missing_implicit = std::mem::replace(
                &mut scope.state.missing_implicit,
                value.get("__stg_missing_implicit") == Some(&Value::Bool(true)),
            );
            let mut previous = Vec::new();
            if let Some(bindings) = value.get("bindings").and_then(Value::as_object) {
                for (name, value) in bindings {
                    previous.push((name.clone(), scope.vars.insert(name.clone(), value.clone())));
                }
            }
            scope.state.previous_was_newline = false;
            let (rendered, stopped) = self.render_range(
                &if value.get("__stg_singleton") == Some(&Value::Bool(true))
                    || no_newlines
                    || !initial_line_start
                {
                    dictionary::tokens(
                        body,
                        value.get("__stg_no_newlines") == Some(&Value::Bool(true)),
                        initial_line_start,
                        self.delimiters,
                    )?
                } else {
                    tokenize(body, self.source_whitespace, self.delimiters)?
                },
                0,
                scope.vars,
                scope.own_package,
                scope.imports,
                scope.depth,
                scope.state,
            )?;
            if stopped == usize::MAX {
                scope
                    .state
                    .writer
                    .as_mut()
                    .unwrap()
                    .failed_since(checkpoint);
            }
            for (name, previous) in previous {
                restore_var(scope.vars, &name, previous);
            }
            *scope.depth -= 1;
            scope.state.no_newlines = previous_no_newlines;
            scope.state.missing_implicit = previous_missing_implicit;
            return Ok(rendered);
        }
        if let Some(keys) = expression::dictionary_keys(value) {
            return self.render_stream(&Value::Array(keys), separator, null, scope);
        }
        if let Some(items) = expression::java_sequence(value) {
            return self.render_stream(&Value::Array(items.clone()), separator, null, scope);
        }
        let mut output = Text::new();
        match value {
            Value::Array(values) => {
                let mut written = false;
                for value in values {
                    if written && (!value.is_null() || null.is_some()) {
                        scope.state.write_output(&mut output, separator);
                    }
                    let checkpoint = scope.state.writer.as_ref().unwrap().checkpoint();
                    let text = self.render_stream(value, separator, null, scope)?;
                    if scope.state.writer.as_ref().unwrap().count_since(checkpoint) > 0 {
                        written = true;
                    }
                    output.append(&text);
                }
            }
            Value::Null => {
                if let Some(null) = null {
                    scope.state.write_output(&mut output, null);
                }
            }
            _ => scope.state.write_output(
                &mut output,
                &Text::from_value(value).unwrap_or_else(|| Text::from(value_string(value))),
            ),
        }
        Ok(output)
    }
}
