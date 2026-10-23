//! Small StringTemplate subset used by the pinned Java generator templates.
//! The templates stay in this crate; rendering does not invoke the source JVM.

use crate::java_stg_expr as expression;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
mod beans;
mod declarations;
mod delimiters;
mod dictionary;
mod eval;
mod group_parser;
mod mapping;
mod operand_stack;
mod operands;
mod properties;
mod syntax;
mod text;
mod utf16;
mod writer;
pub(crate) use writer::inline_value;
use text::Text;

const COMPILE_ERROR: &str = "__stg_compile_error:";
// In an independent default-stack JVM per preview, the pinned source renders
// through layer1224 but fails at layer1225. Same-process HTTP has a different,
// order-dependent stack boundary. The index adds one call to the chain.
const MAX_RENDER_DEPTH: usize = 1226;
const PREVIEW_RENDER_STACK_BYTES: usize = 32 * 1024 * 1024;

/// Remove one template-only blank line before a rendered method signature.
/// AutoIndentWriter inserts spaces after embedded line breaks in a value, so
/// the source signature cannot be matched as a plain substring in that case.
pub(crate) fn collapse_gap_before_signature(content: String, signature: &str) -> String {
    let normalized = signature.replace('\r', "");
    let rendered = normalized
        .split('\n')
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join("\n[ \\t]*");
    let pattern = Regex::new(&format!("\n(?P<rest>\n{rendered})"))
        .expect("escaped method signature must compile");
    pattern.replace_all(&content, "${rest}").into_owned()
}

#[derive(Clone, Debug)]
struct Definition {
    args: Vec<FormalArgument>,
    tokens: Vec<Token>,
    no_newlines: bool,
    missing_implicit: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RegionKind {
    Implicit,
    Embedded,
    Explicit,
}

#[derive(Clone, Debug)]
struct FormalArgument {
    name: String,
    default: Option<String>,
    default_state: DefaultState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DefaultState {
    Ready,
    EmptyTemplate,
    Null,
}

#[derive(Clone, Debug)]
enum Token {
    Text(Text),
    IndentedText {
        indent: String,
        text: Text,
    },
    Newline,
    Tag {
        expr: String,
        line_indent: Option<String>,
        inline_branch_indent: Option<String>,
    },
}

#[derive(Clone, Default)]
struct WriterState {
    missing_implicit: bool,
    no_newlines: bool,
    written_since_newline: usize,
    previous_was_newline: bool,
    writer: Option<writer::AutoIndentWriter>,
    operands: std::rc::Rc<std::cell::RefCell<operand_stack::OperandStack>>,
}

impl WriterState {
    fn preview() -> Self {
        Self {
            writer: Some(writer::AutoIndentWriter::default()),
            ..Default::default()
        }
    }
    fn write(&mut self, output: &mut Text, value: &Text) {
        if let Some(writer) = &mut self.writer {
            self.written_since_newline += writer.write(output, value);
            self.previous_was_newline = false;
            return;
        }
        let start = output.len();
        output.0.extend(value.0.iter().copied().filter(|unit| *unit != 13));
        self.written_since_newline += output.len() - start;
        self.previous_was_newline = false;
    }

    fn write_output(&mut self, output: &mut Text, value: &Text) {
        if let Some(writer) = &mut self.writer {
            writer.write(output, value);
        } else {
            output.0.extend(value.0.iter().copied().filter(|unit| *unit != 13));
        }
    }

    fn newline(&mut self, output: &mut Text) {
        if self.written_since_newline > 0 || self.previous_was_newline {
            if let Some(writer) = &mut self.writer {
                writer.write(output, &Text::from("\n"));
            } else {
                output.push('\n');
            }
        }
        self.written_since_newline = 0;
        self.previous_was_newline = true;
    }
}

pub(crate) struct StgTemplate {
    delimiters: delimiters::Delimiters,
    definitions: HashMap<String, Definition>,
    dictionaries: Map<String, Value>,
    graph: Map<String, Value>,
    source_whitespace: bool,
}

impl StgTemplate {
    fn bind_domain_attributes(vars: &mut Map<String, Value>) {
        let Some(domain) = vars.get_mut("domain").and_then(Value::as_object_mut) else { return };
        // These are attached values read by SKYNETObjectAdaptor, after the
        // generator has already used the physical descriptor collection.
        if let Some(value) = domain.get("rootAttributes")
            .and_then(|attributes| attributes.get("objectDescriptors"))
            .and_then(Value::as_str).filter(|value| !value.is_empty())
            .map(crate::preview_domain::attribute_value)
        {
            domain.insert("objectDescriptors".into(), value);
        }
    }
    pub(crate) fn parse_preview(source: &str) -> Result<Self, String> {
        if source.contains("::=") {
            return Self::parse_with_layout(source, true);
        }
        let mut definitions = HashMap::new();
        let tokens =
            tokenize(source, true, delimiters::Delimiters::default()).map_err(|error| {
                if error.starts_with(COMPILE_ERROR) {
                    // ST's anonymous constructor propagates STException with a
                    // null message; the source controller supplies this fallback.
                    "Unknown Server Error".to_owned()
                } else {
                    error
                }
            })?;
        if tokens.iter().any(|token| {
            matches!(token, Token::Tag { expr, .. }
            if expr.starts_with('@') && expr != "@end")
        }) {
            // ST4's anonymous ST has no enclosing template name. Its region
            // compiler dereferences that null before a preview can render.
            return Err(
                "Cannot invoke \"String.charAt(int)\" because \"enclosingTemplateName\" is null"
                    .into(),
            );
        }
        definitions.insert(
            "index".into(),
            Definition {
                args: vec![FormalArgument {
                    name: "domain".into(),
                    default: None,
                    default_state: DefaultState::Ready,
                }],
                tokens,
                no_newlines: false,
                missing_implicit: false,
            },
        );
        Ok(Self {
            delimiters: delimiters::Delimiters::default(),
            definitions,
            dictionaries: Map::new(),
            graph: Map::new(),
            source_whitespace: true,
        })
    }

    pub(crate) fn render_preview(self, vars: Map<String, Value>) -> Result<String, String> {
        // An external ST4 group can contain hundreds of valid nested named
        // calls. Render on a known stack size so CLI, test, and HTTP workers
        // expose the same output instead of depending on their caller stacks.
        let render = move || self.render_preview_inner(vars);
        #[cfg(any(test, feature = "parity-probes"))]
        let render = crate::graph_service::inherit_controlled(render);
        std::thread::Builder::new()
            .name("teaql-stg-preview".into())
            .stack_size(PREVIEW_RENDER_STACK_BYTES)
            .spawn(render)
            .map_err(|error| error.to_string())?
            .join()
            .map_err(|_| "STG preview render thread panicked".to_owned())?
    }

    fn render_preview_inner(&self, mut vars: Map<String, Value>) -> Result<String, String> {
        self.render_named(
            "index",
            &[],
            &mut vars,
            "",
            &mut BTreeSet::new(),
            &mut 0,
            &mut WriterState::preview(),
        )
        .map(|text| text.utf8())
    }

    pub(crate) fn render_graph_preview(
        mut self,
        vars: Map<String, Value>,
        graph: Map<String, Value>,
    ) -> Result<String, String> {
        self.graph = graph;
        self.render_preview(vars)
    }

    pub(crate) fn render_graph(
        mut self,
        vars: Map<String, Value>,
        graph: Map<String, Value>,
        own_package: &str,
    ) -> Result<String, String> {
        self.graph = graph;
        self.render(vars, own_package)
    }

    pub(crate) fn render_template(
        &self,
        name: &str,
        mut vars: Map<String, Value>,
    ) -> Result<String, String> {
        Self::bind_domain_attributes(&mut vars);
        let mut imports = BTreeSet::new();
        let mut depth = 0;
        self.render_named(
            name,
            &[],
            &mut vars,
            "",
            &mut imports,
            &mut depth,
            &mut if self.source_whitespace { WriterState::preview() } else { WriterState::default() },
        )
        .map(|value| if self.source_whitespace { value.utf8() } else { value.trim_end_newlines().utf8() })
    }

    pub(crate) fn parse_for_vars(source: &str, vars: &Map<String, Value>) -> Result<Self, String> {
        if vars.get("domain").and_then(|domain| domain.get("rootAttributes"))
            .and_then(|attributes| attributes.get("objectDescriptors"))
            .and_then(Value::as_str).is_some_and(|value| !value.is_empty())
        { Self::parse_preview(source) } else { Self::parse(source) }
    }

    pub(crate) fn parse(source: &str) -> Result<Self, String> {
        // Bundled generators also have output adapters for their established
        // token layout. External previews use ST4's source whitespace rules.
        Self::parse_with_layout(source, false)
    }

    fn parse_with_layout(source: &str, source_whitespace: bool) -> Result<Self, String> {
        let group = if source_whitespace {
            Some(declarations::scan(source)?)
        } else {
            None
        };
        let group_values = group.as_ref().map(|g| g.ranges.clone()).unwrap_or_default();
        let delimiters = group.as_ref().map(|g| g.delimiters).unwrap_or_default();
        let headers = if let Some(group) = &group {
            group.headers.clone()
        } else {
            declarations::legacy_headers(source)?
        };
        let mut definitions = HashMap::new();
        let mut regions = HashMap::new();
        let mut occupied = Vec::new();
        let alias_declarations = group.as_ref().map(|g| g.aliases.as_slice()).unwrap_or(&[]);
        let mut next_alias = 0;
        for header in headers {
            if dictionary::contains(&group_values, header.position)
                || occupied
                    .iter()
                    .any(|(start, end)| header.position >= *start && header.position < *end)
            {
                continue;
            }
            let name = header.name.as_str();
            let arguments = header.arguments.as_str();
            let opening = header.opening;
            let suffix = &source[opening..];
            let no_newlines = source_whitespace && !header.missing_body && suffix.starts_with("<%");
            let (remainder, quoted) = if header.missing_body {
                ("", false)
            } else if let Some(body) = suffix.strip_prefix("<<") {
                (body, false)
            } else if no_newlines {
                (&suffix[2..], false)
            } else if let Some(body) = suffix.strip_prefix('"') {
                (body, true)
            } else {
                continue;
            };
            let (body, end) = if header.missing_body {
                (String::new(), header.end)
            } else if source_whitespace {
                (
                    declarations::body(&source[opening..header.end])?,
                    header.end,
                )
            } else if quoted {
                let mut escaped = false;
                let end = remainder
                    .char_indices()
                    .find_map(|(position, ch)| {
                        if escaped {
                            escaped = false;
                            None
                        } else if ch == '\\' {
                            escaped = true;
                            None
                        } else if ch == '"' {
                            Some(position)
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| format!("unterminated template {name}"))?;
                let quoted = &source[opening..opening + end + 2];
                (
                    if source_whitespace {
                        remainder[..end].replace("\\\"", "\"")
                    } else {
                        serde_json::from_str::<String>(quoted).map_err(|error| error.to_string())?
                    },
                    opening + end + 2,
                )
            } else {
                let end = if remainder.starts_with('\n') {
                    remainder.find("\n>>").map(|offset| offset + 1)
                } else {
                    remainder.find(">>")
                }
                .ok_or_else(|| format!("unterminated template {name}"))?;
                let body = remainder[..end]
                    .strip_prefix('\n')
                    .unwrap_or(&remainder[..end]);
                (
                    body.strip_suffix('\n').unwrap_or(body).to_string(),
                    opening + 2 + end + 2,
                )
            };
            occupied.push((header.position, end));
            declarations::apply_before(
                alias_declarations,
                &mut next_alias,
                header.position,
                &mut definitions,
            );
            let mut args: Vec<FormalArgument> = header.parsed_args.clone().unwrap_or_else(|| {
                expression::split_top_level(arguments, ",")
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .map(|part| {
                        let (name, default) = part
                            .split_once('=')
                            .map(|(name, value)| (name.trim(), Some(value.trim().to_string())))
                            .unwrap_or((part, None));
                        FormalArgument {
                            name: name.into(),
                            default,
                            default_state: DefaultState::Ready,
                        }
                    })
                    .collect()
            });
            let explicit = name.strip_prefix('@').and_then(|name| name.split_once('.'));
            let scope = explicit.map(|(scope, _)| scope).unwrap_or(name);
            let name = explicit
                .map(|(scope, region)| region_name(scope, region))
                .unwrap_or_else(|| name.to_string());
            if explicit.is_some() && regions.get(&name) != Some(&RegionKind::Implicit) {
                // ST4 keeps embedded/already explicit regions, and ignores an
                // explicit region declared before its enclosing template.
                continue;
            }
            let tokens = if no_newlines {
                dictionary::tokens(&body, true, true, delimiters)
            } else {
                tokenize(&body, source_whitespace, delimiters)
            };
            let tokens = match tokens {
                // STGroup.defineTemplateOrRegion catches STException at this
                // declaration. Later definitions and explicit retries proceed.
                Err(error) if source_whitespace && error.starts_with(COMPILE_ERROR) => continue,
                other => other?,
            };
            let mut missing_implicit = false;
            if source_whitespace {
                for argument in &mut args {
                    let Some(default) = &argument.default else {
                        continue;
                    };
                    let anonymous = default.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
                    if missing_implicit {
                        argument.default_state = if anonymous.is_some() {
                            DefaultState::EmptyTemplate
                        } else {
                            DefaultState::Null
                        };
                    } else if let Some(body) = anonymous {
                        match tokenize(body, true, delimiters) {
                            Err(error) if error.starts_with(COMPILE_ERROR) => {
                                argument.default_state = DefaultState::EmptyTemplate;
                                missing_implicit = true;
                            }
                            other => {
                                other?;
                            }
                        }
                    }
                }
            }
            let tokens = if missing_implicit {
                let mut pending_definitions = definitions.clone();
                let mut pending_regions = regions.clone();
                compile_regions(
                    &tokens,
                    scope,
                    &mut pending_definitions,
                    &mut pending_regions,
                    no_newlines,
                )?
            } else {
                compile_regions(&tokens, scope, &mut definitions, &mut regions, no_newlines)?
            };
            if explicit.is_some() {
                regions.insert(name.clone(), RegionKind::Explicit);
            }
            let definition = Definition {
                args,
                tokens,
                no_newlines,
                missing_implicit,
            };
            if source_whitespace && explicit.is_none() {
                definitions.entry(name).or_insert(definition);
            } else {
                definitions.insert(name, definition);
            }
        }
        let mut dictionaries = Map::new();
        let dictionary_headers = group
            .as_ref()
            .map(|g| g.dictionaries.clone())
            .unwrap_or_else(|| declarations::legacy_dictionaries(source));
        for header in dictionary_headers {
            if dictionary::contains(&group_values, header.position)
                || occupied
                    .iter()
                    .any(|(start, end)| header.position >= *start && header.position < *end)
            {
                continue;
            }
            if source_whitespace
                && header.parsed_entries.is_none()
                && !source[header.opening..header.end].ends_with(']')
            {
                // An anonymous group token consumed EOF: GroupParser cannot
                // finish/register this dictionary, but prior templates remain.
                continue;
            }
            let end = if source_whitespace {
                header.end - 1
            } else {
                expression::matching_end(source, header.opening, '[', ']')
                    .ok_or("unterminated dictionary")?
            };
            let mut values = Map::new();
            let mut keys = Vec::new();
            let parsed = header.parsed_entries.is_some();
            let entries = if let Some(entries) = &header.parsed_entries {
                entries.clone()
            } else {
                let raw_entries = if source_whitespace {
                    dictionary::entries(&source[header.opening + 1..end], delimiters)?
                } else {
                    expression::split_top_level(&source[header.opening + 1..end], ",")
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .map(|entry| {
                            let p = expression::split_top_level(entry, ":");
                            if p.len() != 2 {
                                Err(format!("invalid dictionary entry: {entry}"))
                            } else {
                                Ok((p[0], p[1]))
                            }
                        })
                        .collect::<Result<Vec<_>, String>>()?
                };
                raw_entries
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), Some(value.to_owned())))
                    .collect()
            };
            for (key_input, value_input) in entries {
                let key = if parsed {
                    key_input.clone()
                } else if key_input == "default" {
                    "default".to_string()
                } else {
                    expression::string(&if source_whitespace {
                        properties::dictionary_literal(&key_input)
                    } else {
                        expression::value(&key_input, &Map::new())
                    })
                };
                let value = if value_input.is_none() {
                    Value::Null
                } else if value_input.as_deref() == Some("key") {
                    serde_json::json!({"__stg_key":true,"__string":"key"})
                } else {
                    if source_whitespace {
                        dictionary::literal(value_input.as_deref().unwrap())?
                    } else {
                        expression::value(value_input.as_deref().unwrap(), &Map::new())
                    }
                };
                if !values.contains_key(&key) {
                    keys.push(Value::String(key.clone()));
                }
                values.insert(key, value);
            }
            let value = serde_json::json!({"__stg_dictionary":values,"__stg_keys":keys});
            if source_whitespace {
                dictionaries.entry(header.name).or_insert(value);
            } else {
                dictionaries.insert(header.name, value);
            }
        }
        if source_whitespace {
            declarations::apply_before(
                alias_declarations,
                &mut next_alias,
                usize::MAX,
                &mut definitions,
            );
        } else {
            let aliases =
                Regex::new(r"(?m)^([A-Za-z_][A-Za-z0-9_]*)\s*::=\s*([A-Za-z_][A-Za-z0-9_]*)\s*$")
                    .unwrap();
            let mut unresolved = aliases
                .captures_iter(source)
                .filter(|capture| {
                    !dictionary::contains(&group_values, capture.get(0).unwrap().start())
                        && !occupied.iter().any(|(start, end)| {
                            capture.get(0).unwrap().start() >= *start
                                && capture.get(0).unwrap().start() < *end
                        })
                })
                .map(|capture| (capture[1].to_string(), capture[2].to_string()))
                .collect::<Vec<_>>();
            for _ in 0..unresolved.len() {
                unresolved.retain(|(name, target)| {
                    if let Some(definition) = definitions.get(target).cloned() {
                        definitions.insert(name.clone(), definition);
                        false
                    } else {
                        true
                    }
                });
            }
        }
        if !definitions.contains_key("index") {
            return Err("missing index template".to_string());
        }
        Ok(Self {
            delimiters,
            definitions,
            dictionaries,
            graph: Map::new(),
            source_whitespace,
        })
    }

    pub(crate) fn render(
        &self,
        mut vars: Map<String, Value>,
        own_package: &str,
    ) -> Result<String, String> {
        Self::bind_domain_attributes(&mut vars);
        let mut imports = BTreeSet::new();
        let mut depth = 0;
        self.render_named(
            "index",
            &[],
            &mut vars,
            own_package,
            &mut imports,
            &mut depth,
            &mut if self.source_whitespace { WriterState::preview() } else { WriterState::default() },
        )?;
        let mut depth = 0;
        self.render_named(
            "index",
            &[],
            &mut vars,
            own_package,
            &mut imports,
            &mut depth,
            &mut if self.source_whitespace { WriterState::preview() } else { WriterState::default() },
        )
        .map(|rendered| if self.source_whitespace { rendered.utf8() } else { rendered.trim_end_newlines().utf8() })
    }

    fn definition(&self, name: &str) -> Option<&Definition> {
        self.definitions.get(name).or_else(|| {
            if self.source_whitespace {
                name.strip_prefix('/')
                    .and_then(|name| self.definitions.get(name))
            } else {
                None
            }
        })
    }

    fn render_named(
        &self,
        name: &str,
        args: &[Option<Value>],
        vars: &mut Map<String, Value>,
        own_package: &str,
        imports: &mut BTreeSet<String>,
        depth: &mut usize,
        state: &mut WriterState,
    ) -> Result<Text, String> {
        let Some(definition) = self.definition(name) else {
            // ST4 logs NO_SUCH_TEMPLATE for an embedded call and supplies an
            // empty CompiledST, so the public preview continues rendering.
            return Ok(Text::new());
        };
        let checkpoint = state.writer.as_ref().map(|writer| writer.checkpoint());
        let entry = *depth == 0;
        if entry {
            for (name, value) in &self.dictionaries {
                vars.entry(name.clone()).or_insert_with(|| value.clone());
            }
        }
        *depth += 1;
        if *depth > MAX_RENDER_DEPTH {
            return Err(format!("template recursion limit in {name}"));
        }
        let previous_no_newlines =
            std::mem::replace(&mut state.no_newlines, definition.no_newlines);
        let previous_missing_implicit =
            std::mem::replace(&mut state.missing_implicit, definition.missing_implicit);
        state.previous_was_newline = true;
        let mut previous = Vec::new();
        for (i, param) in definition.args.iter().enumerate() {
            let value = args.get(i).and_then(|value| value.clone()).or_else(|| {
                if entry {
                    vars.get(&param.name).cloned()
                } else {
                    None
                }
            });
            previous.push((
                param.name.clone(),
                vars.insert(param.name.clone(), value.unwrap_or(Value::Null)),
            ));
        }
        for (i, param) in definition.args.iter().enumerate() {
            if args.get(i).is_some_and(Option::is_some) || (entry && previous[i].1.is_some()) {
                continue;
            }
            if let Some(default) = &param.default {
                let value = if param.default_state == DefaultState::EmptyTemplate {
                    serde_json::json!({"__stg_anonymous_default":""})
                } else if param.default_state == DefaultState::Null {
                    Value::Null
                } else if self.source_whitespace
                    && default.starts_with('"')
                    && default.ends_with('"')
                {
                    let token = default.replace("\\\"", "\"");
                    Value::String(token[1..token.len() - 1].to_owned())
                } else if let Some(body) = default
                    .strip_prefix('{')
                    .and_then(|body| body.strip_suffix('}'))
                {
                    if self.delimiters.immediate(body) {
                        // ST4 explicitly evaluates {<(expression)>} now. Other
                        // anonymous defaults are templates, dynamically scoped
                        // at each use, including use in a nested callee/map.
                        if self.source_whitespace {
                            self.isolated_string(
                                &serde_json::json!({"__stg_anonymous_default":body}),
                                &mut eval::Scope {
                                    conditional: false,
                                    vars,
                                    own_package,
                                    imports,
                                    depth,
                                    state,
                                },
                            )?
                            .into_value()
                        } else {
                            self.render_range(
                                &tokenize(body, self.source_whitespace, self.delimiters)?,
                                0,
                                vars,
                                own_package,
                                imports,
                                depth,
                                &mut WriterState::default(),
                            )?
                            .0
                            .into_value()
                        }
                    } else {
                        serde_json::json!({"__stg_anonymous_default": body})
                    }
                } else {
                    expression::value_with_graph(default, vars, &self.graph)
                };
                vars.insert(param.name.clone(), value);
            }
        }
        let (result, stopped) = self.render_range(
            &definition.tokens,
            0,
            vars,
            own_package,
            imports,
            depth,
            state,
        )?;
        if stopped == usize::MAX {
            if let Some(checkpoint) = checkpoint {
                state.writer.as_mut().unwrap().failed_since(checkpoint);
            }
        }
        for (key, value) in previous.into_iter().rev() {
            if let Some(value) = value {
                vars.insert(key, value);
            } else {
                vars.remove(&key);
            }
        }
        *depth -= 1;
        state.no_newlines = previous_no_newlines;
        state.missing_implicit = previous_missing_implicit;
        Ok(result)
    }

    fn render_range(
        &self,
        tokens: &[Token],
        mut cursor: usize,
        vars: &mut Map<String, Value>,
        own_package: &str,
        imports: &mut BTreeSet<String>,
        depth: &mut usize,
        state: &mut WriterState,
    ) -> Result<(Text, usize), String> {
        let mut output = Text::new();
        while cursor < tokens.len() {
            match &tokens[cursor] {
                Token::Text(value) => state.write(&mut output, value),
                Token::IndentedText { indent, text } => {
                    state.writer.as_mut().unwrap().indents.push(indent.clone());
                    state.write(&mut output, text);
                    state.writer.as_mut().unwrap().indents.pop();
                }
                Token::Newline => state.newline(&mut output),
                Token::Tag {
                    expr,
                    line_indent,
                    inline_branch_indent,
                } => {
                    if expr == "else" || expr == "endif" || expr.starts_with("elseif(") {
                        break;
                    }
                    if let Some(condition) = expr
                        .strip_prefix("if(")
                        .and_then(|expr| expr.strip_suffix(')'))
                    {
                        let mut branches = Vec::new();
                        let mut next = cursor + 1;
                        let mut nesting = 0;
                        let mut start = next;
                        let mut branch_condition = Some(condition);
                        let end = loop {
                            match tokens.get(next) {
                                None => return Err(format!("if without endif: {condition}")),
                                Some(Token::Tag { expr, .. }) => {
                                    if expr.starts_with("if(") {
                                        nesting += 1;
                                    } else if expr == "endif" && nesting > 0 {
                                        nesting -= 1;
                                    } else if nesting == 0 {
                                        if expr == "endif" {
                                            branches.push((branch_condition, start));
                                            break next;
                                        }
                                        if expr == "else" || expr.starts_with("elseif(") {
                                            branches.push((branch_condition, start));
                                            branch_condition = expr
                                                .strip_prefix("elseif(")
                                                .and_then(|s| s.strip_suffix(')'));
                                            start = next + 1;
                                        }
                                    }
                                }
                                _ => {}
                            }
                            next += 1;
                        };
                        let mut selected = None;
                        for (condition, start) in branches {
                            let mut scope = eval::Scope {
                                conditional: false,
                                vars,
                                own_package,
                                imports,
                                depth,
                                state,
                            };
                            let enabled = match condition {
                                None => true,
                                Some(condition) => {
                                    match self.evaluate_condition(condition, &mut scope) {
                                        Ok(value) => value,
                                        Err(error) if error.starts_with("__stg_getter_fault:") => {
                                            return Ok((output, usize::MAX));
                                        }
                                        Err(error) => return Err(error),
                                    }
                                }
                            };
                            if enabled {
                                selected = Some(start);
                                break;
                            }
                        }
                        if let Some(start) = selected {
                            if self.source_whitespace {
                                if let Some(indent) = inline_branch_indent
                                    .as_ref()
                                    .filter(|value| !value.is_empty())
                                {
                                    state.writer.as_mut().unwrap().indents.push(indent.clone());
                                }
                                state.previous_was_newline = false;
                                let (rendered, stopped) = self.render_range(
                                    tokens,
                                    start,
                                    vars,
                                    own_package,
                                    imports,
                                    depth,
                                    state,
                                )?;
                                output.append(&rendered);
                                if stopped == usize::MAX {
                                    return Ok((output, stopped));
                                }
                                if inline_branch_indent
                                    .as_ref()
                                    .is_some_and(|value| !value.is_empty())
                                {
                                    state.writer.as_mut().unwrap().indents.pop();
                                }
                                cursor = end;
                                cursor += 1;
                                continue;
                            }
                            let mut branch_imports = imports.clone();
                            let mut branch_state = state.clone();
                            branch_state.previous_was_newline = false;
                            let (rendered, stopped) = self.render_range(
                                tokens,
                                start,
                                vars,
                                own_package,
                                &mut branch_imports,
                                depth,
                                &mut branch_state,
                            )?;
                            if let Some(indent) = inline_branch_indent {
                                for (index, line) in rendered.lines().enumerate() {
                                    if index > 0 {
                                        output.push('\n');
                                    }
                                    if !line.is_empty() {
                                        output.push_str(indent);
                                    }
                                    output.append(&line);
                                }
                            } else {
                                output.append(&rendered);
                            }
                            *imports = branch_imports;
                            *state = branch_state;
                            if stopped == usize::MAX {
                                return Ok((output, stopped));
                            }
                        } else {
                            state.previous_was_newline = false;
                        }
                        cursor = end;
                    } else if expr.starts_with('!') && expr.ends_with('!') {
                    } else {
                        if self.source_whitespace {
                            let checkpoint = state.writer.as_ref().unwrap().checkpoint();
                            if let Some(indent) =
                                line_indent.as_ref().filter(|value| !value.is_empty())
                            {
                                state.writer.as_mut().unwrap().indents.push(indent.clone());
                            }
                            let rendered =
                                self.eval_stream(expr, vars, own_package, imports, depth, state);
                            let rendered = match rendered {
                                Ok(value) => value,
                                Err(error) if error.starts_with("__stg_getter_fault:") => {
                                    return Ok((output, usize::MAX));
                                }
                                Err(error) => return Err(error),
                            };
                            if line_indent.as_ref().is_some_and(|value| !value.is_empty()) {
                                state.writer.as_mut().unwrap().indents.pop();
                            }
                            output.append(&rendered);
                            state.written_since_newline +=
                                state.writer.as_ref().unwrap().count_since(checkpoint);
                            state.previous_was_newline = false;
                            cursor += 1;
                            continue;
                        }
                        let rendered = Text::from(
                            (match self.eval(expr, vars, own_package, imports, depth, state) {
                                Ok(value) => value,
                                Err(error) if error.starts_with("__stg_getter_fault:") => {
                                    return Ok((output, usize::MAX));
                                }
                                Err(error) => return Err(error),
                            }).replace('\r', ""),
                        );
                        if let Some(indent) = line_indent {
                            if !rendered.is_empty() {
                                for (index, line) in rendered.lines().enumerate() {
                                    if index > 0 {
                                        output.push('\n');
                                    }
                                    if !line.is_empty() {
                                        output.push_str(indent);
                                    }
                                    output.append(&line);
                                }
                            }
                        } else {
                            output.append(&rendered);
                        }
                        state.written_since_newline += rendered.len();
                        state.previous_was_newline = false;
                    }
                }
            }
            cursor += 1;
        }
        Ok((output, cursor))
    }

    fn eval(
        &self,
        expr: &str,
        vars: &mut Map<String, Value>,
        own_package: &str,
        imports: &mut BTreeSet<String>,
        depth: &mut usize,
        state: &mut WriterState,
    ) -> Result<String, String> {
        let pieces = expression::split_top_level(expr, ";");
        let expr = pieces[0];
        if expr == "naming.imports" {
            return Ok(imports
                .iter()
                .map(|name| format!("import {name};"))
                .collect::<Vec<_>>()
                .join("\n"));
        }
        if let Some(inner) = expr.strip_prefix("naming.package.(").and_then(|value| value.strip_suffix(')')) {
            let value = expression::value_with_graph(inner, vars, &self.graph);
            return Ok(format!("package {};", value.as_str().unwrap_or_default()));
        }
        if let Some(inner) = expr
            .strip_prefix("naming.(")
            .and_then(|value| value.strip_suffix(')'))
        {
            let value = expression::value_with_graph(inner, vars, &self.graph);
            if value.is_null() {
                return Err("__stg_getter_fault:Cannot invoke \"String.lastIndexOf(int)\" because \"className\" is null".into());
            }
            if let Some(message) = expression::getter_fault(&value) {
                return Err(format!("__stg_getter_fault:{message}"));
            }
            let name = value.as_str().unwrap_or_default();
            if name.contains('.')
                && !name.starts_with("java.lang.")
                && !name.starts_with(&format!("{own_package}."))
            {
                imports.insert(name.to_string());
            } else if name.starts_with(&format!("{own_package}.")) {
                let sibling_package = name
                    .rsplit_once('.')
                    .map(|(package, _)| package)
                    .unwrap_or("");
                if sibling_package != own_package {
                    imports.insert(name.to_string());
                }
            }
            return Ok(name.rsplit('.').next().unwrap_or(name).to_string());
        }
        let mut scope = eval::Scope {
            conditional: false,
            vars,
            own_package,
            imports,
            depth,
            state,
        };
        let mut options = HashMap::new();
        for option in pieces
            .iter()
            .skip(1)
            .flat_map(|piece| expression::split_top_level(piece, ","))
        {
            if let Some((key, value)) = option.split_once('=') {
                options.insert(key.trim(), self.evaluate(value.trim(), &mut scope)?);
            }
        }
        let separator = if let Some(value) = options.get("separator") {
            self.render_value(value, "", None, &mut scope)?
        } else {
            String::new()
        };
        let value = self.evaluate(expr, &mut scope)?;
        self.render_value(&value, &separator, options.get("null"), &mut scope)
    }

    fn render_value(
        &self,
        value: &Value,
        separator: &str,
        null: Option<&Value>,
        scope: &mut eval::Scope<'_>,
    ) -> Result<String, String> {
        if let Some(body) = value.get("__stg_anonymous_default").and_then(Value::as_str) {
            *scope.depth += 1;
            if *scope.depth > MAX_RENDER_DEPTH {
                return Err("template recursion limit in anonymous default".into());
            }
            let rendered = self
                .render_range(
                    &tokenize(body, self.source_whitespace, self.delimiters)?,
                    0,
                    scope.vars,
                    scope.own_package,
                    scope.imports,
                    scope.depth,
                    scope.state,
                )?
                .0;
            *scope.depth -= 1;
            return Ok(rendered.utf8());
        }
        if let Some(keys) = expression::dictionary_keys(value) {
            return self.render_value(&Value::Array(keys), separator, null, scope);
        }
        if let Some(items) = expression::java_sequence(value) {
            return self.render_value(&Value::Array(items.clone()), separator, null, scope);
        }
        match value {
            Value::Array(items) => {
                let mut result = String::new();
                let mut written = false;
                for item in items {
                    if written && (!item.is_null() || null.is_some()) {
                        result.push_str(separator);
                    }
                    let text = self.render_value(item, separator, null, scope)?;
                    if !text.is_empty() {
                        written = true;
                    }
                    result.push_str(&text);
                }
                Ok(result)
            }
            Value::Null => {
                if let Some(null) = null {
                    self.render_value(null, "", None, scope)
                } else {
                    Ok(String::new())
                }
            }
            _ => Ok(value_string(value)),
        }
    }
}

fn region_name(scope: &str, name: &str) -> String {
    // STGroup.getMangledRegionName: /region__/index__slot.
    format!("/region__/{scope}__{name}")
}

fn compile_regions(
    tokens: &[Token],
    scope: &str,
    definitions: &mut HashMap<String, Definition>,
    regions: &mut HashMap<String, RegionKind>,
    no_newlines: bool,
) -> Result<Vec<Token>, String> {
    fn identifier(name: &str) -> bool {
        name.as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
            && name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    }
    let mut output = Vec::new();
    let mut cursor = 0;
    while cursor < tokens.len() {
        let Token::Tag {
            expr, line_indent, ..
        } = &tokens[cursor]
        else {
            output.push(tokens[cursor].clone());
            cursor += 1;
            continue;
        };
        let head = expression::split_top_level(expr, ";")[0];
        let Some(region) = head.strip_prefix('@') else {
            output.push(tokens[cursor].clone());
            cursor += 1;
            continue;
        };
        let invoked = region.ends_with("()");
        let region = region.strip_suffix("()").unwrap_or(region);
        if !identifier(region) || region == "end" {
            output.push(tokens[cursor].clone());
            cursor += 1;
            continue;
        }
        let name = region_name(scope, region);
        if invoked {
            if !regions.contains_key(&name) {
                regions.insert(name.clone(), RegionKind::Implicit);
                definitions.insert(
                    name.clone(),
                    Definition {
                        args: Vec::new(),
                        tokens: Vec::new(),
                        no_newlines,
                        missing_implicit: false,
                    },
                );
            }
            cursor += 1;
        } else {
            let mut start = cursor + 1;
            if line_indent.is_some() && matches!(tokens.get(start), Some(Token::Newline)) {
                start += 1;
            }
            let mut end = start;
            let mut nesting = 1;
            while end < tokens.len() {
                if let Token::Tag { expr, .. } = &tokens[end] {
                    if expr == "@end" {
                        nesting -= 1;
                        if nesting == 0 {
                            break;
                        }
                    } else if expr.strip_prefix('@').is_some_and(identifier) {
                        nesting += 1;
                    }
                }
                end += 1;
            }
            if end == tokens.len() {
                return Err(format!("unterminated region {scope}.{region}"));
            }
            let body = compile_regions(
                &tokens[start..end],
                scope,
                definitions,
                regions,
                no_newlines,
            )?;
            if matches!(regions.get(&name), None | Some(RegionKind::Implicit)) {
                regions.insert(name.clone(), RegionKind::Embedded);
                definitions.insert(
                    name.clone(),
                    Definition {
                        args: Vec::new(),
                        tokens: body,
                        no_newlines,
                        missing_implicit: false,
                    },
                );
            }
            cursor = end + 1;
        }
        output.push(Token::Tag {
            expr: format!("{name}(){}", &expr[head.len()..]),
            line_indent: if invoked { line_indent.clone() } else { None },
            inline_branch_indent: None,
        });
    }
    Ok(output)
}

fn value_string(value: &Value) -> String {
    expression::string(value)
}
fn restore_var(vars: &mut Map<String, Value>, name: &str, previous: Option<Value>) {
    if let Some(previous) = previous {
        vars.insert(name.into(), previous);
    } else {
        vars.remove(name);
    }
}
fn map_items(value: Value) -> Vec<Value> {
    if let Some(items) = expression::java_sequence(&value) {
        return items.clone();
    }
    if let Some(keys) = expression::dictionary_keys(&value) {
        return keys;
    }
    match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items,
        other => vec![other],
    }
}
fn tokenize(
    input: &str,
    source_whitespace: bool,
    delimiters: delimiters::Delimiters,
) -> Result<Vec<Token>, String> {
    tokenize_at(input, source_whitespace, true, delimiters)
}

fn tokenize_at(
    input: &str,
    source_whitespace: bool,
    initial_line_start: bool,
    delimiters: delimiters::Delimiters,
) -> Result<Vec<Token>, String> {
    if source_whitespace {
        return utf16::tokenize(input, initial_line_start, delimiters);
    }
    let mut result = Vec::new();
    let mut literal = String::new();
    let mut literal_at_line_start = initial_line_start;
    let mut cursor = 0;
    let bytes = input.as_bytes();
    let mut conditional_starts = Vec::new();
    let mut suppressed_indent_end = 0;
    let linebreak_escape = format!("{}\\\\{}", delimiters.start, delimiters.stop);
    let comment_start = format!("{}!", delimiters.start);
    let comment_end = format!("!{}", delimiters.stop);
    while cursor < bytes.len() {
        if source_whitespace && input[cursor..].starts_with(&linebreak_escape) {
            let mut end = cursor + linebreak_escape.len();
            while matches!(bytes.get(end), Some(b' ' | b'\t')) {
                end += 1;
            }
            if bytes.get(end) == Some(&b'\r') {
                end += 1;
            }
            if bytes.get(end) != Some(&b'\n') {
                return Err("missing newline after ST4 linebreak escape".into());
            }
            end += 1;
            while matches!(bytes.get(end), Some(b' ' | b'\t')) {
                end += 1;
            }
            cursor = end;
            suppressed_indent_end = end;
            continue;
        }
        let escaped = input[cursor..]
            .strip_prefix('\\')
            .and_then(|s| s.chars().next());
        if let Some(ch) = escaped.filter(|ch| {
            if source_whitespace {
                *ch == delimiters.start || matches!(ch, '}' | '\\')
            } else {
                matches!(ch, '<' | '>' | '{' | '}' | '\\')
            }
        }) {
            if literal.is_empty() {
                literal_at_line_start =
                    cursor == 0 && initial_line_start || cursor > 0 && bytes[cursor - 1] == b'\n';
            }
            literal.push(ch);
            cursor += 1 + ch.len_utf8();
            continue;
        }
        if !input[cursor..].starts_with(delimiters.start) {
            if literal.is_empty() {
                literal_at_line_start =
                    cursor == 0 && initial_line_start || cursor > 0 && bytes[cursor - 1] == b'\n';
            }
            let character = input[cursor..].chars().next().ok_or("bad UTF-8 boundary")?;
            literal.push(character);
            cursor += character.len_utf8();
            continue;
        }
        let tag_start = cursor;
        if source_whitespace
            && input[cursor..].starts_with(&comment_start)
            && !input[cursor + comment_start.len()..].contains(&comment_end)
        {
            let start = input[..cursor].rfind('\n').map(|p| p + 1).unwrap_or(0);
            if input[start..cursor]
                .chars()
                .all(|c| matches!(c, ' ' | '\t'))
            {
                literal.truncate(literal.len().saturating_sub(cursor - start));
            }
            break;
        }
        if source_whitespace
            && delimiters.stop_is_syntax()
            && !input[cursor..].starts_with(&comment_start)
            && !input[cursor + delimiters.start.len_utf8()..].starts_with('\\')
        {
            return Err(format!("{COMPILE_ERROR}expression has no RDELIM token"));
        }
        // UNICODE emits its TEXT token even when match(RDELIM) reports a
        // mismatch/EOF. In particular, a compact <<...>>> group can consume
        // the tag's '>' as part of the group's closing token.
        let unicode_end = source_whitespace
            .then(|| {
                let start = cursor + delimiters.start.len_utf8();
                let hex = input[start..].strip_prefix("\\u")?.get(..4)?;
                hex.bytes()
                    .all(|c| c.is_ascii_hexdigit())
                    .then_some(start + 6)
            })
            .flatten();
        let end = if let Some(end) = unicode_end {
            end
        } else if input[cursor..].starts_with(&comment_start) {
            input[cursor + comment_start.len()..]
                .find(&comment_end)
                .ok_or("unterminated STG comment")?
                + cursor
                + comment_start.len()
                + 1
        } else {
            let mut position = cursor + delimiters.start.len_utf8();
            let mut braces = 0usize;
            let mut quoted = false;
            let mut tags = 1usize;
            loop {
                let character = input[position..].chars().next().ok_or_else(|| {
                    format!(
                        "{}unterminated STG tag at byte {tag_start}: {}",
                        if source_whitespace { COMPILE_ERROR } else { "" },
                        input[tag_start..].chars().take(90).collect::<String>()
                    )
                })?;
                let expression = tags > braces;
                if character == '\\' && !(expression && !quoted && character == delimiters.stop) {
                    position += character.len_utf8();
                    if position < input.len() {
                        position += input[position..].chars().next().unwrap().len_utf8();
                    }
                    continue;
                }
                match character {
                    '"' if expression => quoted = !quoted,
                    '{' if expression && !quoted => braces += 1,
                    '}' if !expression => braces = braces.saturating_sub(1),
                    ch if ch == delimiters.start && !expression => tags += 1,
                    ch if ch == delimiters.stop && expression && !quoted => {
                        if tags == 1 && braces == 0 {
                            break position;
                        }
                        tags = tags.saturating_sub(1);
                    }
                    _ => {}
                }
                position += character.len_utf8();
            }
        };
        let closing_width = if unicode_end.is_some() {
            input[end..].chars().next().map(char::len_utf8).unwrap_or(0)
        } else {
            delimiters.stop.len_utf8()
        };
        let raw_expr = &input[cursor + delimiters.start.len_utf8()..end];
        let expr = if source_whitespace && !raw_expr.starts_with('\\') && !raw_expr.starts_with('!')
        {
            syntax::tag(raw_expr, delimiters)?
        } else {
            raw_expr.trim().to_string()
        };
        let multiline_conditional_end = if expr == "endif" {
            conditional_starts
                .pop()
                .is_some_and(|start| input[start..cursor].contains('\n'))
        } else {
            if expr.starts_with("if(") {
                conditional_starts.push(cursor);
            }
            false
        };
        let source_line_start = input[..cursor]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let before = &input[source_line_start..cursor];
        let after = &input[end + closing_width..];
        let after_line = after.split('\n').next().unwrap_or(after);
        let leading = source_line_start >= suppressed_indent_end
            && (source_line_start > 0 || initial_line_start)
            && before.chars().all(|c| c == ' ' || c == '\t');
        let standalone =
            leading && after_line.chars().all(|c| c == ' ' || c == '\t') && after.contains('\n');
        let control = expr.starts_with("if(") || expr == "else" || expr == "endif";
        let inline_branch_indent =
            (source_whitespace && leading && !standalone && expr.starts_with("if("))
                .then(|| before.to_string());
        let leading_indent =
            source_whitespace && leading && (!control || inline_branch_indent.is_some());
        let indent = if standalone || leading_indent {
            Some(before.to_string())
        } else {
            None
        };
        if standalone || leading_indent {
            let trailing_indent = before.len();
            literal.truncate(literal.len().saturating_sub(trailing_indent));
        }
        if standalone
            || (source_whitespace
                && multiline_conditional_end
                && after_line.chars().all(|c| c == ' ' || c == '\t')
                && after.contains('\n'))
        {
            cursor = end + closing_width + after_line.len() + 1;
        } else {
            cursor = end + closing_width;
        }
        flush_literal(
            &mut result,
            &mut literal,
            source_whitespace,
            literal_at_line_start,
        );
        if source_whitespace && raw_expr.starts_with('\\') {
            // These lexer escapes emit TEXT, including an actual newline.
            // A newline TEXT is not the interpreter's NEWLINE instruction.
            let text = match raw_expr {
                "\\n" => Some(Text::from("\n")),
                "\\t" => Some(Text::from("\t")),
                "\\ " => Some(Text::from(" ")),
                _ => raw_expr
                    .strip_prefix("\\u")
                    .filter(|s| s.len() == 4)
                    .and_then(|s| u16::from_str_radix(s, 16).ok())
                    .map(|code| Text(vec![code])),
            };
            if let Some(text) = text {
                result.push(if let Some(indent) = indent.filter(|s| !s.is_empty()) {
                    Token::IndentedText { indent, text }
                } else {
                    Token::Text(text)
                });
                if standalone {
                    result.push(Token::Newline);
                }
            }
        } else if !expr.starts_with('!') || !expr.ends_with('!') {
            let closing = expr == "endif" || expr == "@end";
            result.push(Token::Tag {
                expr,
                line_indent: indent,
                inline_branch_indent,
            });
            if standalone && !closing && !control || standalone && control && !closing {
                result.push(Token::Newline);
            }
        }
        if cursor <= tag_start {
            return Err("STG tokenizer did not advance".into());
        }
    }
    flush_literal(
        &mut result,
        &mut literal,
        source_whitespace,
        literal_at_line_start,
    );
    if source_whitespace {
        syntax::structure(&result)?;
    }
    Ok(result)
}

fn flush_literal(
    result: &mut Vec<Token>,
    literal: &mut String,
    source_whitespace: bool,
    at_line_start: bool,
) {
    if literal.is_empty() {
        return;
    }
    let value = std::mem::take(literal);
    for (index, part) in value.split('\n').enumerate() {
        if index > 0 {
            result.push(Token::Newline);
        }
        if !part.is_empty() {
            if source_whitespace && (index > 0 || at_line_start) {
                let indent = part.len() - part.trim_start_matches([' ', '\t']).len();
                if indent > 0 && indent < part.len() {
                    result.push(Token::IndentedText {
                        indent: part[..indent].into(),
                        text: part[indent..].into(),
                    });
                    continue;
                }
            }
            result.push(Token::Text(part.into()));
        }
    }
}
