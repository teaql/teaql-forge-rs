//! Application properties read by the pinned JVM generator at process startup.
use std::{collections::BTreeMap, sync::OnceLock};

struct Properties {
    values: BTreeMap<String, String>,
    arguments: Vec<String>,
    startup_stderr: String,
}

struct StartupError {
    message: String,
    stderr: String,
}

impl StartupError {
    fn launcher(message: String, notices: &str) -> Self {
        Self {
            stderr: format!("{notices}Error: {message}\n"),
            message,
        }
    }

    fn vm(message: String, notices: &str) -> Self {
        Self {
            stderr: format!("{notices}{message}\nError: Could not create the Java Virtual Machine.\nError: A fatal exception has occurred. Program will exit.\n"),
            message,
        }
    }
}

fn words(input: &str, variable: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for ch in input.chars() {
        if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            } else {
                word.push(ch);
            }
            started = true;
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            started = true;
        } else if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}') {
            if started {
                out.push(std::mem::take(&mut word));
            }
            started = false;
        } else {
            word.push(ch);
            started = true;
        }
    }
    if quote.is_some() {
        return Err(if variable == "JDK_JAVA_OPTIONS" {
            format!("Unmatched quote in environment variable {variable}")
        } else {
            format!("Unmatched quote in {variable}")
        });
    }
    if started {
        out.push(word);
    }
    Ok(out)
}

fn definitions(values: &mut BTreeMap<String, String>, options: impl IntoIterator<Item = String>) {
    for option in options {
        if let Some(property) = option.strip_prefix("-D") {
            let (name, value) = property.split_once('=').unwrap_or((property, ""));
            values.insert(name.into(), value.into());
        }
    }
}

fn argument_file(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if let Some(delimiter) = quote {
            match ch {
                '\r' | '\n' => {
                    if started {
                        out.push(std::mem::take(&mut word));
                    }
                    started = false;
                    quote = None;
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        if matches!(next, '\n' | '\r') {
                            while chars.peek().is_some_and(|ch| ch.is_ascii_whitespace()) {
                                chars.next();
                            }
                        } else {
                            word.push(match next {
                                'n' => '\n',
                                'r' => '\r',
                                't' => '\t',
                                'f' => '\u{c}',
                                ch => ch,
                            });
                        }
                    }
                }
                ch if ch == delimiter => quote = None,
                ch => word.push(ch),
            }
        } else if ch == '#' {
            word.clear();
            started = false;
            for ch in chars.by_ref() {
                if matches!(ch, '\n' | '\r') {
                    break;
                }
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            started = true;
        } else if ch.is_ascii_whitespace() {
            if started {
                out.push(std::mem::take(&mut word));
            }
            started = false;
        } else {
            word.push(ch);
            started = true;
        }
    }
    if started {
        out.push(word);
    }
    out
}

fn expand_files(options: Vec<String>) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut enabled = true;
    for option in options {
        if option == "--disable-@files" {
            enabled = false;
            continue;
        }
        if let Some(literal) = option.strip_prefix("@@") {
            result.push(format!("@{literal}"));
        } else if enabled && option.starts_with('@') && option.len() > 1 {
            let filename = &option[1..];
            let contents = std::fs::read_to_string(filename)
                .map_err(|_| format!("could not open `{filename}'"))?;
            result.extend(argument_file(&contents));
        } else {
            result.push(option);
        }
    }
    Ok(result)
}

fn validate_environment(options: &[String], name: &str) -> Result<(), String> {
    let mut takes_value = false;
    for option in options {
        if takes_value {
            takes_value = false;
            continue;
        }
        if name == "JDK_JAVA_OPTIONS"
            && [
                "-cp",
                "-classpath",
                "--class-path",
                "-p",
                "--module-path",
                "--upgrade-module-path",
                "--add-modules",
                "--limit-modules",
                "--add-exports",
                "--add-opens",
                "--add-reads",
                "--patch-module",
                "--enable-native-access",
                "--source",
            ]
            .contains(&option.as_str())
        {
            takes_value = true;
        } else if !option.starts_with('-') && !option.is_empty() {
            return Err(if name == "JDK_JAVA_OPTIONS" {
                "Cannot specify main class in environment variable JDK_JAVA_OPTIONS".into()
            } else {
                format!("Unrecognized option: {option}")
            });
        }
    }
    Ok(())
}

fn environment_words(name: &str, notices: &mut String) -> Result<Vec<String>, String> {
    match std::env::var(name) {
        Ok(value) => {
            if name == "JDK_JAVA_OPTIONS" {
                notices.push_str("NOTE: ");
            }
            notices.push_str(&format!("Picked up {name}: {value}\n"));
            words(&value, name)
        }
        Err(_) => Ok(Vec::new()),
    }
}

fn load(raw: impl IntoIterator<Item = String>) -> Result<Properties, StartupError> {
    let mut notices = String::new();
    // The launcher parses its environment and @files before starting the VM.
    // Property precedence differs from this error/diagnostic ordering.
    let jdk = environment_words("JDK_JAVA_OPTIONS", &mut notices)
        .and_then(expand_files)
        .and_then(|options| {
            validate_environment(&options, "JDK_JAVA_OPTIONS")?;
            Ok(options)
        })
        .map_err(|message| StartupError::launcher(message, &notices))?;
    let mut command_values = BTreeMap::new();
    let mut raw = raw.into_iter();
    let mut arguments = raw.next().into_iter().collect::<Vec<_>>();
    let mut leading = true;
    let mut expand = true;
    let mut pending = raw.collect::<std::collections::VecDeque<_>>();
    while let Some(mut argument) = pending.pop_front() {
        if leading && argument == "--disable-@files" {
            expand = false;
            continue;
        }
        if leading && expand {
            if argument.starts_with("@@") {
                argument.remove(0);
            } else if let Some(filename) =
                argument.strip_prefix('@').filter(|value| !value.is_empty())
            {
                let contents = std::fs::read_to_string(filename).map_err(|_| {
                    StartupError::launcher(format!("could not open `{filename}'"), &notices)
                })?;
                // JDK expands each launch argument file once. Its contents
                // are tokenized, but nested @ references remain literal.
                for token in argument_file(&contents) {
                    if leading && token.starts_with("-D") {
                        definitions(&mut command_values, [token]);
                    } else {
                        leading = false;
                        arguments.push(token);
                    }
                }
                continue;
            }
        }
        if leading && argument.starts_with("-D") {
            definitions(&mut command_values, [argument]);
        } else {
            leading = false;
            arguments.push(argument);
        }
    }
    let tool = environment_words("JAVA_TOOL_OPTIONS", &mut notices)
        .map_err(|message| StartupError::vm(message, &notices))?;
    let legacy = environment_words("_JAVA_OPTIONS", &mut notices)
        .map_err(|message| StartupError::vm(message, &notices))?;
    validate_environment(&tool, "JAVA_TOOL_OPTIONS")
        .and_then(|_| validate_environment(&legacy, "_JAVA_OPTIONS"))
        .map_err(|message| StartupError::vm(message, &notices))?;
    let mut values = BTreeMap::new();
    definitions(&mut values, tool);
    definitions(&mut values, jdk);
    values.extend(command_values);
    definitions(&mut values, legacy);
    Ok(Properties {
        values,
        arguments,
        startup_stderr: notices,
    })
}

fn properties() -> &'static Result<Properties, StartupError> {
    PROPERTIES.get_or_init(|| load(std::env::args()))
}

static PROPERTIES: OnceLock<Result<Properties, StartupError>> = OnceLock::new();

/// JVM option variables and leading -D arguments are consumed once per process.
pub fn arguments() -> Result<Vec<String>, String> {
    properties()
        .as_ref()
        .map(|p| p.arguments.clone())
        .map_err(|error| error.message.clone())
}

/// Process entry points emit the original launch diagnostics and exit before
/// reading a model or initializing application resources when parsing fails.
pub fn launch_arguments() -> Vec<String> {
    match properties() {
        Ok(properties) => {
            eprint!("{}", properties.startup_stderr);
            properties.arguments.clone()
        }
        Err(error) => {
            eprint!("{}", error.stderr);
            std::process::exit(1);
        }
    }
}

pub fn property(name: &str) -> Option<String> {
    properties().as_ref().ok()?.values.get(name).cloned()
}

pub fn user_home() -> String {
    property("user.home").unwrap_or_else(crate::input_path::os_user_home)
}

pub(crate) fn boxed_relations_threshold(attributes: &BTreeMap<String, String>) -> i32 {
    let integer =
        |value: &str| crate::hutool_json::parse_integer(value).and_then(|v| i32::try_from(v).ok());
    attributes
        .get("boxed_relations_cost_threshold")
        .and_then(|v| integer(v))
        .or_else(|| property("teaql.boxedRelationsCostThreshold").and_then(|v| integer(&v)))
        .unwrap_or(40)
}

#[cfg(any(test, feature = "parity-probes"))]
pub fn parity_report(read_demo: bool) -> serde_json::Value {
    use serde_json::json;
    let keys = [
        "teaql.boxedRelationsCostThreshold",
        "teaql.demo.model",
        "teaql.generator.templateRoot",
        "user.home",
        "parity.value",
    ];
    let properties = keys
        .into_iter()
        .map(|name| {
            (
                name,
                if name == "user.home" {
                    Some(user_home())
                } else {
                    property(name)
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let thresholds = [
        None,
        Some(""),
        Some("0"),
        Some("31"),
        Some("３１"),
        Some(" 1"),
        Some("+18"),
        Some("-1"),
        Some("2147483648"),
        Some("٢"),
    ]
    .into_iter()
    .map(|value| {
        let attributes = value
            .map(|value| BTreeMap::from([("boxed_relations_cost_threshold".into(), value.into())]))
            .unwrap_or_default();
        json!({"attribute":value,"effective":boxed_relations_threshold(&attributes)})
    })
    .collect::<Vec<_>>();
    let mut out = json!({"properties":properties,"thresholds":thresholds,
        "home_path":crate::input_path::normalize("~/models/../input.xml"),
        "demo_path":crate::demo_model::path(),"demo_filename":crate::demo_model::filename(),
        "template_folder":crate::templates::root("doc").unwrap().display().to_string()});
    if read_demo {
        out["demo_read"] = match crate::demo_model::read() {
            Ok((bytes, _)) => {
                json!({"hex":bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()})
            }
            Err(error) => json!({"exception_class":error.exception_class,"message":error.message}),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::process::Command;

    /// Each source case gets a fresh process, matching JVM static initialization
    /// without changing environment variables shared by other workspace tests.
    #[test]
    fn complete_process_reports_match_actual_jvm_launches() {
        if let Ok(case) = std::env::var("TEAQL_PROCESS_GOLDEN_CASE") {
            let case: Value = serde_json::from_str(&case).unwrap();
            let raw = std::iter::once("probe".to_owned()).chain(
                case["arguments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_owned()),
            );
            assert!(PROPERTIES.set(load(raw)).is_ok());
            let report = match arguments() {
                Ok(_) => parity_report(false),
                Err(error) => json!({"failure_reason":error}),
            };
            std::fs::write(
                std::env::var("TEAQL_PROCESS_GOLDEN_OUTPUT").unwrap(),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            return;
        }
        let reference: Value = serde_json::from_str(include_str!(
            "../tests/expected/process-properties-6f2d0a.json"
        ))
        .unwrap();
        assert_eq!(
            reference["source_commit"],
            "6f2d0a33699277b943148db948bdec2d5725df37"
        );
        let root =
            std::env::temp_dir().join(format!("teaql-process-regression-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        for original in reference["property_cases"].as_array().unwrap() {
            let mut case = original.clone();
            if let Some(contents) = case["argument_file"].as_str() {
                let path = root.join("launch.args");
                std::fs::write(&path, contents).unwrap();
                case["environment"]["JDK_JAVA_OPTIONS"] = json!(format!("@{}", path.display()));
            }
            let output = root.join("report.json");
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args([
                "process_properties::tests::complete_process_reports_match_actual_jvm_launches",
                "--exact",
                "--nocapture",
            ]);
            for name in [
                "JAVA_TOOL_OPTIONS",
                "JDK_JAVA_OPTIONS",
                "_JAVA_OPTIONS",
                "TEAQL_DEMO_MODEL",
                "TEAQL_GENERATOR_TEMPLATE_ROOT",
                "CLASSPATH",
            ] {
                command.env_remove(name);
            }
            for (key, value) in case["environment"].as_object().unwrap() {
                command.env(key, value.as_str().unwrap());
            }
            let child = command
                .env("TEAQL_PROCESS_GOLDEN_CASE", case.to_string())
                .env("TEAQL_PROCESS_GOLDEN_OUTPUT", &output)
                .output()
                .unwrap();
            assert!(
                child.status.success(),
                "{}: {}",
                case["name"],
                String::from_utf8_lossy(&child.stderr)
            );
            let actual: Value = serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
            if case["result"].is_null() {
                assert!(
                    actual["failure_reason"]
                        .as_str()
                        .unwrap()
                        .contains(case["failure_reason"].as_str().unwrap()),
                    "{}: {actual}",
                    case["name"]
                );
            } else {
                let mut expected = case["result"].clone();
                expected.as_object_mut().unwrap().remove("demo_read");
                let explicit_home = case["environment"]
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|v| v.as_str().unwrap().contains("-Duser.home"))
                    || case["arguments"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v.as_str().unwrap().starts_with("-Duser.home"));
                if !explicit_home {
                    expected["properties"]["user.home"] = json!(crate::input_path::os_user_home());
                    expected["home_path"] =
                        json!(crate::input_path::normalize("~/models/../input.xml"));
                }
                assert_eq!(actual, expected, "{}", case["name"]);
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
