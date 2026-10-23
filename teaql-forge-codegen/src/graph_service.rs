//! DataDesignGenerator's GraphServiceClient, including its response-body semantics.
use crate::context::RenderDomain;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(feature = "parity-probes")]
mod mtls;
mod proxy;
mod tls;
mod transport;
mod wrapper;

#[derive(Clone, Serialize)]
struct Request {
    uri: &'static str,
    method: &'static str,
    headers: BTreeMap<&'static str, Vec<&'static str>>,
    timeout_ms: u64,
    body_hex: String,
}

#[derive(Clone, Deserialize)]
struct Response {
    #[cfg(feature = "parity-probes")]
    #[serde(skip)]
    observation: Option<serde_json::Value>,
    #[serde(default)]
    headers: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    body_hex: String,
    #[serde(default)]
    failure: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn payload(domain: &RenderDomain) -> String {
    #[derive(Serialize)]
    struct Word {
        size: usize,
        text: String,
    }
    #[derive(Serialize)]
    struct Payload {
        words: Vec<Word>,
    }
    // Java calls physical getters here, bypassing attached STG attributes.
    // HashMap iteration visits "size" before "text" in the pinned JVM.
    let words = domain
        .entities
        .iter()
        .map(|entity| Word {
            size: entity.reverse_relations.len(),
            text: entity
                .name
                .trim_end_matches('_')
                .split('_')
                .map(crate::preview_domain::upper_first)
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect();
    String::from_utf8(crate::semantic::json_bytes(&Payload { words }).unwrap()).unwrap()
}

fn charset(headers: &BTreeMap<String, Vec<String>>) -> String {
    let value = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, values)| values.first())
        .map(String::as_str)
        .unwrap_or("");
    let value = value
        .split_once(';')
        .map_or(value, |(_, parameters)| parameters);
    // sun.net.www.HeaderParser separates on spaces/commas, but not semicolons,
    // and does not trim the key around '='. Preserve those observable quirks.
    let chars = value
        .trim_matches(|ch| ch <= '\u{20}')
        .chars()
        .collect::<Vec<_>>();
    let (mut begin, mut end) = (0, 0);
    let (mut in_key, mut quoted) = (true, false);
    let mut key = String::new();
    let mut entries = Vec::<(String, Option<String>)>::new();
    let text = |begin: usize, end: usize| chars[begin..end].iter().collect::<String>();
    while end < chars.len() {
        match chars[end] {
            '=' if !quoted => {
                key = text(begin, end).to_lowercase();
                in_key = false;
                end += 1;
                begin = end;
            }
            '"' if quoted => {
                entries.push((std::mem::take(&mut key), Some(text(begin, end))));
                quoted = false;
                end += 1;
                while end < chars.len() && matches!(chars[end], ' ' | ',') {
                    end += 1;
                }
                in_key = true;
                begin = end;
            }
            '"' => {
                quoted = true;
                end += 1;
                begin = end;
            }
            ' ' | ',' if !quoted => {
                if in_key {
                    entries.push((text(begin, end).to_lowercase(), None));
                } else {
                    entries.push((std::mem::take(&mut key), Some(text(begin, end))));
                }
                while end < chars.len() && matches!(chars[end], ' ' | ',') {
                    end += 1;
                }
                in_key = true;
                begin = end;
            }
            _ => end += 1,
        }
    }
    if end > begin {
        if in_key {
            entries.push((text(begin, end).to_lowercase(), None));
        } else {
            let finish = if chars[end - 1] == '"' { end - 1 } else { end };
            entries.push((key, Some(text(begin, finish))));
        }
    }
    entries
        .into_iter()
        .find(|(key, _)| key == "charset")
        .and_then(|(_, value)| value)
        .unwrap_or_else(|| "UTF-8".into())
}

fn decode(headers: &BTreeMap<String, Vec<String>>, bytes: &[u8]) -> String {
    let label = charset(headers);
    let normalized = label.to_ascii_uppercase().replace(['-', '_'], "");
    match normalized.as_str() {
        "ISO88591" | "ISOIR100" | "LATIN1" | "L1" | "IBM819" | "CP819" => {
            bytes.iter().map(|byte| char::from(*byte)).collect()
        }
        "USASCII" | "ASCII" | "ISO646US" => bytes
            .iter()
            .map(|byte| {
                if *byte < 128 {
                    char::from(*byte)
                } else {
                    '\u{fffd}'
                }
            })
            .collect(),
        "UTF16" | "UTF16BE" | "UTF16LE" => {
            let mut data = bytes;
            let mut little = normalized == "UTF16LE";
            if normalized == "UTF16" {
                if data.starts_with(&[0xfe, 0xff]) {
                    data = &data[2..];
                } else if data.starts_with(&[0xff, 0xfe]) {
                    data = &data[2..];
                    little = true;
                }
            }
            let unit = |at: usize| {
                if little {
                    u16::from_le_bytes([data[at], data[at + 1]])
                } else {
                    u16::from_be_bytes([data[at], data[at + 1]])
                }
            };
            let mut text = String::new();
            let mut at = 0;
            while at < data.len() {
                if at + 2 > data.len() {
                    text.push('\u{fffd}');
                    break;
                }
                let first = unit(at);
                at += 2;
                if (0xd800..=0xdbff).contains(&first) {
                    if at + 2 > data.len() {
                        text.push('\u{fffd}');
                        break;
                    }
                    let second = unit(at);
                    at += 2;
                    if (0xdc00..=0xdfff).contains(&second) {
                        text.push(
                            char::from_u32(
                                0x10000
                                    + ((first as u32 - 0xd800) << 10)
                                    + (second as u32 - 0xdc00),
                            )
                            .unwrap(),
                        );
                    } else {
                        text.push('\u{fffd}');
                    }
                } else {
                    text.push(char::from_u32(first as u32).unwrap_or('\u{fffd}'));
                }
            }
            text
        }
        "UTF8" => crate::java_xml::utf8_string(bytes),
        _ => match encoding_rs::Encoding::for_label(label.as_bytes()) {
            Some(encoding) => {
                let text = encoding.decode_without_bom_handling(bytes).0.into_owned();
                if encoding == encoding_rs::WINDOWS_1252 {
                    text.replace(
                        ['\u{81}', '\u{8d}', '\u{8f}', '\u{90}', '\u{9d}'],
                        "\u{fffd}",
                    )
                } else {
                    text
                }
            }
            None => crate::java_xml::utf8_string(bytes),
        },
    }
}

#[cfg(feature = "parity-probes")]
pub fn proxy_probe(cases: &serde_json::Value) -> serde_json::Value {
    let mut results = serde_json::Map::new();
    for case in cases.as_array().unwrap() {
        let url = reqwest::Url::parse(case["uri"].as_str().unwrap()).unwrap();
        let value = proxy::select(url.host_str().unwrap(), |name| {
            case["properties"][name].as_str().map(str::to_owned)
        });
        let result = match value {
            Ok(effective) => serde_json::json!({"effective_http_proxy":effective}),
            Err(message) => {
                serde_json::json!({"error":{"exception_class":"java.lang.IllegalArgumentException","message":message}})
            }
        };
        results.insert(case["name"].as_str().unwrap().to_string(), result);
    }
    serde_json::Value::Object(results)
}

#[derive(Debug)]
enum ClientError {
    Argument(String),
    Initialization,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Argument(message) => message,
            Self::Initialization => tls::INITIALIZATION_MESSAGE,
        })
    }
}

#[cfg(feature = "parity-probes")]
fn error_json(error: ClientError) -> serde_json::Value {
    let message = error.to_string();
    match error {
        ClientError::Argument(_) => {
            serde_json::json!({"exception_class":"java.lang.IllegalArgumentException","message":message})
        }
        ClientError::Initialization => {
            serde_json::json!({"exception_class":"java.io.UncheckedIOException","message":message})
        }
    }
}

#[cfg(feature = "parity-probes")]
pub fn trust_probe() -> serde_json::Value {
    match tls::config().and_then(|_| tls::certificates()) {
        Ok(certs) => {
            serde_json::json!({"certificates_der_hex":certs.iter().map(|v| hex(v)).collect::<Vec<_>>()})
        }
        Err(()) => serde_json::json!({"error":error_json(ClientError::Initialization)}),
    }
}

#[cfg(feature = "parity-probes")]
pub fn key_probe() -> serde_json::Value {
    tls::key_report()
        .unwrap_or_else(|_| serde_json::json!({"error":error_json(ClientError::Initialization)}))
}

#[cfg(feature = "parity-probes")]
pub fn mutual_tls_probe(
    peer: &std::path::Path,
    fixture: &std::path::Path,
    output: &std::path::Path,
) -> serde_json::Value {
    mtls::run(peer, fixture, output)
}

fn word_cloud(body: String) -> Result<String, ClientError> {
    let request = Request {
        uri: "https://mq.teaql.io/",
        method: "POST",
        timeout_ms: 10_000,
        headers: BTreeMap::from([
            ("Authorization", vec!["Bearer YOUR_API_KEY"]),
            ("Content-Type", vec!["application/json"]),
        ]),
        body_hex: hex(body.as_bytes()),
    };
    #[cfg(any(test, feature = "parity-probes"))]
    let controlled = CONTROLLED.with(|state| {
        state.borrow_mut().as_mut().map(|(response, requests)| {
            requests.lock().unwrap().push(serde_json::to_value(&request).unwrap());
            response.clone()
        })
    });
    #[cfg(not(any(test, feature = "parity-probes")))]
    let controlled: Option<Response> = None;
    let result = controlled.map_or_else(|| transport::transmit(request, body), Ok);
    #[cfg(feature = "parity-probes")]
    OBSERVATIONS.with(|observations| {
        if let Some(observations) = observations.borrow_mut().as_mut() {
            observations.push(
                result
                    .as_ref()
                    .ok()
                    .and_then(|response| response.observation.clone())
                    .unwrap_or_else(|| serde_json::json!({"transport_error":true})),
            );
        }
    });
    match result {
        Ok(response) if response.failure.is_empty() => {
            Ok(decode(&response.headers, &unhex(&response.body_hex)))
        }
        Err(transport::Failure::Argument(message)) => Err(ClientError::Argument(message)),
        Err(transport::Failure::Initialization) => Err(ClientError::Initialization),
        _ => Ok("error".into()),
    }
}

pub(crate) fn attach(
    domain: &RenderDomain,
    graph: &mut serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    wrapper::attach(domain, graph, payload(domain))
}

pub(crate) fn getter(value: serde_json::Value) -> serde_json::Value {
    if value.as_object().is_none_or(|object| object.len() != 1) {
        return value;
    }
    match value
        .get("__stg_graph_service_payload")
        .and_then(serde_json::Value::as_str)
    {
        Some(payload) => word_cloud(payload.into())
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        None => value,
    }
}

#[cfg(any(test, feature = "parity-probes"))]
thread_local! {
    static CONTROLLED: std::cell::RefCell<Option<(Response, std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>)>> = const { std::cell::RefCell::new(None) };
}

/// Preserve a controlled graph response when STG preview uses its larger
/// dedicated render stack. The request log is shared with the caller so its
/// existing parity probe still observes requests made in that worker.
#[cfg(any(test, feature = "parity-probes"))]
pub(crate) fn inherit_controlled<T>(action: impl FnOnce() -> T) -> impl FnOnce() -> T {
    let inherited = CONTROLLED.with(|state| state.borrow().clone());
    move || {
        let Some(inherited) = inherited else { return action() };
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                CONTROLLED.with(|state| *state.borrow_mut() = None);
            }
        }
        CONTROLLED.with(|state| {
            assert!(state.borrow().is_none());
            *state.borrow_mut() = Some(inherited);
        });
        let _reset = Reset;
        action()
    }
}

/// Test transport is compiled only into explicitly requested parity probes.
#[cfg(any(test, feature = "parity-probes"))]
pub fn with_response<T>(
    response: serde_json::Value,
    action: impl FnOnce() -> T,
) -> (T, Vec<serde_json::Value>) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            CONTROLLED.with(|state| *state.borrow_mut() = None);
        }
    }
    CONTROLLED.with(|state| {
        assert!(state.borrow().is_none());
        *state.borrow_mut() = Some((
            serde_json::from_value(response).unwrap(),
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        ));
    });
    let _reset = Reset;
    let result = action();
    let requests = CONTROLLED.with(|state| {
        let (_, requests) = state.borrow_mut().take().unwrap();
        let observed = std::mem::take(&mut *requests.lock().unwrap());
        observed
    });
    (result, requests)
}

#[cfg(feature = "parity-probes")]
pub fn client_probe(body: String) -> String {
    word_cloud(body).expect("source client argument error")
}

#[cfg(feature = "parity-probes")]
pub fn client_outcome(body: String) -> serde_json::Value {
    match word_cloud(body) {
        Ok(body) => serde_json::json!({"body":body}),
        Err(error) => {
            serde_json::json!({"error":error_json(error)})
        }
    }
}

#[cfg(feature = "parity-probes")]
thread_local! {
    static OBSERVATIONS: std::cell::RefCell<Option<Vec<serde_json::Value>>> = const { std::cell::RefCell::new(None) };
}

/// Observe the real transport without replacing it. Only parity builds expose it.
#[cfg(feature = "parity-probes")]
pub fn observe<T>(action: impl FnOnce() -> T) -> (T, Vec<serde_json::Value>) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            OBSERVATIONS.with(|state| *state.borrow_mut() = None);
        }
    }
    OBSERVATIONS.with(|state| {
        assert!(state.borrow().is_none());
        *state.borrow_mut() = Some(Vec::new());
    });
    let _reset = Reset;
    let result = action();
    let observations = OBSERVATIONS.with(|state| state.borrow_mut().take().unwrap());
    (result, observations)
}
