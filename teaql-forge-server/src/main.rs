use axum::{
    extract::Multipart,
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::io::{Read, Write};
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_codegen::engine::generate_for_scope;
use teaql_forge_codegen::evaluation::render_markdown_report;
use teaql_forge_codegen::input::ModelInput;
use teaql_forge_codegen::semantic;
use zip::write::SimpleFileOptions;

mod errors;
mod eval;
mod framework_errors;
mod semantic_identity;
mod startup;
mod telemetry;
use teaql_forge_codegen::templates;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value_t = 8081)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let arguments = teaql_forge_codegen::process_properties::launch_arguments();
    let args = Args::parse_from(arguments);

    let sender = telemetry::UsageSender::initialize_configured();
    let app = match app_with_telemetry(sender) {
        Ok(app) => app,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        },
    };

    if args.host == "0.0.0.0" {
        println!("Warning: You are exposing TeaQL Local Server to the network.");
        println!("Use Enterprise Mode or configure TLS/auth for production.");
    }

    let addr = format!("{}:{}", args.host, args.port);
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

fn app_with_telemetry(sender: telemetry::UsageSender) -> Result<Router, startup::Failure> {
    startup::check()?;
    Ok(Router::new()
        .route("/", get(index_handler))
        .route("/version", get(version_handler).post(ignored_post_handler))
        .route("/version/", get(version_handler).post(ignored_post_handler))
        .route("/services", get(services_handler).post(ignored_post_handler))
        .route(
            "/.well-known/teaql/usage-key.md",
            get(telemetry::key_handler)
                .post(usage_key_post_handler)
                .with_state(sender.clone()),
        )
        .route("/generate", post(generate_handler).get(|| async { StatusCode::OK }))
        .route(
            "/evaluate",
            get(eval::evaluate_get_handler).post(eval::evaluate_handler),
        )
        .route(
            "/evaluation",
            get(eval::evaluate_get_handler).post(eval::evaluate_handler),
        )
        .route(
            "/semantic-identity/initialize",
            post(semantic_identity::initialize).get(semantic_get_handler),
        )
        .route(
            "/semantic-identity/reconcile",
            post(semantic_identity::reconcile).get(semantic_get_handler),
        )
        .route(
            "/semantic-identity/rename",
            post(semantic_identity::mutate).get(semantic_get_handler),
        )
        .route(
            "/semantic-identity/delete",
            post(semantic_identity::mutate).get(semantic_get_handler),
        )
        .route(
            "/semantic-identity/restore",
            post(semantic_identity::mutate).get(semantic_get_handler),
        )
        .route(
            "/*path",
            get(preview_get_handler).post(preview_post_handler),
        )
        .method_not_allowed_fallback(|| async {
            (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "POST, GET")])
        })
        // Spring's telemetry filter also observes OPTIONS responses.
        .layer(axum::middleware::from_fn(source_method_parity))
        .layer(axum::middleware::from_fn_with_state(
            sender,
            telemetry::filter,
        )))
}

async fn source_method_parity(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if request.method() == Method::GET
        || request.method() == Method::HEAD
        || request.method() == Method::POST
    {
        spring_sort_accept_before_matching(request.headers());
    }
    if request.method() == Method::OPTIONS {
        return (
            StatusCode::OK,
            [(header::ALLOW, "POST,GET,HEAD,OPTIONS")],
        )
            .into_response();
    }
    // Spring selects the wildcard GET when a dedicated Markdown mapping's
    // produces condition cannot satisfy Accept. Those paths are ignored there.
    let markdown_mapping = ((request.method() == Method::GET || request.method() == Method::HEAD)
        && matches!(
            request.uri().path(),
            "/services" | "/version" | "/version/" | "/evaluate" | "/evaluation"
        ))
        || (request.method() == Method::POST
            && matches!(request.uri().path(), "/evaluate" | "/evaluation"));
    if markdown_mapping {
        let Some(content_type) = markdown_response_type(request.headers()) else {
            return StatusCode::OK.into_response();
        };
        let mut response = next.run(request).await;
        // Spring's content converter can fail after the route ran, including
        // when Axum's multipart extractor would otherwise return an error.
        if content_type.contains('"') {
            if let Some(reason) = spring_mime_failure_reason(&content_type) {
                std::panic::panic_any(SpringMvcMediaTypeFailure::invalid_media(&content_type, &reason));
            }
        }
        if response.status() == StatusCode::OK
            && response
                .headers()
                .get(header::CONTENT_TYPE)
                .is_some_and(|value| value.as_bytes().starts_with(b"text/markdown"))
        {
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_bytes(content_type.as_bytes()).expect("valid Markdown response type"),
            );
            if let Some(charset) = spring_response_charset(&content_type) {
                if !matches!(charset.to_ascii_lowercase().as_str(), "utf-8" | "utf8" | "unicode-1-1-utf-8") {
                    let (mut parts, body) = response.into_parts();
                    let utf8 = axum::body::to_bytes(body, usize::MAX)
                        .await
                        .expect("complete Markdown response body");
                    let encoded = spring_encode_string_body(&utf8, &charset);
                    if parts.headers.contains_key(header::CONTENT_LENGTH) {
                        parts.headers.insert(
                            header::CONTENT_LENGTH,
                            HeaderValue::from_str(&encoded.len().to_string()).unwrap(),
                        );
                    }
                    response = axum::response::Response::from_parts(
                        parts,
                        axum::body::Body::from(encoded),
                    );
                }
            }
        }
        return response;
    }
    next.run(request).await
}

/// RequestMappingInfo evaluates `produces` before checking a path. Spring's
/// header strategy therefore sorts every parsed Accept member even when the
/// eventual route cannot produce that member. An invalid uppercase Q can fail
/// here, before either the route or the body converter runs.
fn spring_sort_accept_before_matching(headers: &HeaderMap) {
    struct Media {
        kind: String,
        subtype: String,
        quality: Option<String>,
        parameter_count: usize,
    }
    impl Media {
        fn quality(&self) -> f64 {
            self.quality.as_deref().map_or(1.0, |value| {
                let unquoted = value
                    .strip_prefix('"')
                    .and_then(|part| part.strip_suffix('"'))
                    .or_else(|| value.strip_prefix('\'').and_then(|part| part.strip_suffix('\'')))
                    .unwrap_or(value);
                java_quality(unquoted).unwrap_or_else(|| {
                    std::panic::panic_any(SpringMvcMediaTypeFailure::number_format(unquoted))
                })
            })
        }
        fn wildcard_subtype(&self) -> bool {
            self.subtype == "*" || self.subtype.starts_with("*+")
        }
    }
    let mut media = Vec::new();
    for value in headers.get_all(header::ACCEPT) {
        let Ok(value) = std::str::from_utf8(value.as_bytes()) else { return };
        for candidate in split_outside_quotes(value, ',', true) {
            if candidate.trim().is_empty() { continue; }
            // MediaType.parseMediaTypes builds the whole list before sorting.
            // A constructor error is caught by Spring's mapping condition.
            if spring_mime_failure_reason(candidate).is_some() { return; }
            let parts = split_outside_quotes(candidate, ';', false);
            let base = if parts[0].trim() == "*" { "*/*" } else { parts[0].trim() };
            let Some((kind, subtype)) = base.split_once('/') else { return };
            let mut exact = Vec::<(&str, &str)>::new();
            for part in parts.into_iter().skip(1) {
                if let Some((name, value)) = part.split_once('=') {
                    let (name, value) = (name.trim(), value.trim());
                    if let Some((_, old)) = exact.iter_mut().find(|(old_name, _)| *old_name == name) {
                        *old = value;
                    } else {
                        exact.push((name, value));
                    }
                }
            }
            let mut folded = Vec::<(&str, &str)>::new();
            for (name, value) in exact {
                if let Some((old_name, old_value)) = folded.iter_mut()
                    .find(|(old_name, _)| old_name.eq_ignore_ascii_case(name)) {
                    *old_name = name;
                    *old_value = value;
                } else {
                    folded.push((name, value));
                }
            }
            media.push(Media {
                kind: kind.to_ascii_lowercase(),
                subtype: subtype.to_ascii_lowercase(),
                quality: folded.iter().find(|(name, _)| name.eq_ignore_ascii_case("q"))
                    .map(|(_, value)| (*value).to_string()),
                parameter_count: folded.len(),
            });
        }
    }
    if media.len() < 2 { return; }
    media.sort_by(|left, right| {
        let wildcard_type = (left.kind == "*").cmp(&(right.kind == "*"));
        if !wildcard_type.is_eq() { return wildcard_type; }
        if left.kind == right.kind {
            let wildcard_subtype = left.wildcard_subtype().cmp(&right.wildcard_subtype());
            if !wildcard_subtype.is_eq() { return wildcard_subtype; }
        }
        let quality = right.quality().total_cmp(&left.quality());
        if !quality.is_eq() { return quality; }
        if left.kind == right.kind && left.subtype == right.subtype {
            right.parameter_count.cmp(&left.parameter_count)
        } else {
            std::cmp::Ordering::Equal
        }
    });
}

fn markdown_response_type(headers: &HeaderMap) -> Option<String> {
    const DEFAULT: &str = "text/markdown;charset=UTF-8";
    // Spring parses the complete Accept list before considering a produced
    // type. One malformed member invalidates the list, even if another member
    // would satisfy the Markdown mapping.
    for value in headers.get_all(header::ACCEPT) {
        let Ok(value) = std::str::from_utf8(value.as_bytes()) else { continue };
        if split_outside_quotes(value, ',', true)
            .into_iter()
            .filter(|candidate| !candidate.trim().is_empty())
            .any(|candidate| spring_mime_failure_reason(candidate).is_some())
        {
            return None;
        }
    }
    let has_compatible_candidate = headers.get_all(header::ACCEPT).into_iter().any(|value| {
        std::str::from_utf8(value.as_bytes()).is_ok_and(|value| {
            split_outside_quotes(value, ',', true).into_iter().any(|candidate| {
                let parts = split_outside_quotes(candidate, ';', false);
                let base = parts[0].trim().to_ascii_lowercase();
                let compatible_type = matches!(base.as_str(), "text/markdown" | "text/*" | "*/*" | "*");
                let mut exact_parameters = Vec::<(&str, &str)>::new();
                for (name, value) in parts.into_iter().skip(1)
                    .filter_map(|part| part.split_once('='))
                {
                    let (name, value) = (name.trim(), value.trim());
                    if let Some((_, old_value)) = exact_parameters.iter_mut()
                        .find(|(old_name, _)| *old_name == name)
                    {
                        *old_value = value;
                    } else {
                        exact_parameters.push((name, value));
                    }
                }
                let charset = exact_parameters.into_iter()
                    .filter(|(name, _)| name.eq_ignore_ascii_case("charset"))
                    .map(|(_, value)| value).last();
                compatible_type && charset.is_none_or(|value| value.eq_ignore_ascii_case("UTF-8"))
            })
        })
    });
    let mut specified = false;
    let mut selected: Option<(f64, usize, u8, usize, String)> = None;
    let mut wildcard_charset_fallback = false;
    for value in headers.get_all(header::ACCEPT) {
        specified = true;
        let Ok(value) = std::str::from_utf8(value.as_bytes()) else { continue };
        for candidate in split_outside_quotes(value, ',', true) {
            // Spring's comma tokenizer skips escaped quotes; its parameter
            // scanner still toggles quoting at every quote character.
            let parts = split_outside_quotes(candidate, ';', false);
            let normalized = parts
                .iter()
                .enumerate()
                .filter_map(|(index, part)| {
                    if index == 0 {
                        Some(part.trim().to_string())
                    } else if let Some((name, value)) = part.split_once('=') {
                        Some(format!("{}={}", name.trim(), value.trim()))
                    } else {
                        // MimeTypeUtils skips fragments without an equals sign.
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(";");
            let media = normalized.parse::<mime::Mime>().or_else(|error| {
                if spring_mime_failure_reason(candidate).is_none() {
                    (if parts[0].trim() == "*" { "*/*" } else { parts[0].trim() })
                        .parse::<mime::Mime>()
                } else {
                    Err(error)
                }
            });
            let Ok(media) = media else { continue };
            let wildcard = media.type_().as_str() == "*" && media.subtype().as_str() == "*";
            let text_wildcard = media.type_() == mime::TEXT && media.subtype().as_str() == "*";
            let markdown = media.type_() == mime::TEXT && media.subtype().as_str() == "markdown";
            if !wildcard && !text_wildcard && !markdown {
                continue;
            }
            // Spring first collects parameters by their exact spelling in a
            // LinkedHashMap. MimeType then folds those entries into a case
            // insensitive map. An exact spelling update keeps its first
            // position; a later spelling with different case replaces and
            // moves the folded entry to the end.
            let mut exact_parameters: Vec<(&str, String)> = Vec::new();
            for parameter in parts.into_iter().skip(1) {
                let Some((name, value)) = parameter.split_once('=') else {
                    // MimeTypeUtils silently skips parameter fragments without '='.
                    continue;
                };
                let (name, value) = (name.trim(), value.trim());
                put_exact_media_parameter(
                    &mut exact_parameters,
                    name,
                    format!("{name}={value}"),
                );
            }
            // A solely uppercase CHARSET does not populate MimeType's
            // lowercase charset field, so Spring's converter adds its own
            // canonical charset. An earlier exact `charset` does populate
            // that field even when a later casing wins the parameter map.
            let has_lowercase_charset = exact_parameters
                .iter()
                .any(|(name, _)| *name == "charset");
            let mut parameters: Vec<(&str, String)> = Vec::new();
            for (name, rendered) in exact_parameters {
                let rendered = if name.eq_ignore_ascii_case("charset")
                    && name != "charset"
                    && !has_lowercase_charset
                {
                    let value = rendered.split_once('=').map_or("", |(_, value)| value);
                    let canonical = if value.eq_ignore_ascii_case("UTF-8") {
                        "UTF-8"
                    } else {
                        value
                    };
                    format!("charset={canonical}")
                } else {
                    rendered
                };
                put_media_parameter(&mut parameters, name, rendered);
            }
            let mut quality = 1.0;
            if let Some((_, rendered)) = parameters
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("q"))
            {
                let value = rendered.split_once('=').map_or("", |(_, value)| value);
                let unquoted = value
                    .strip_prefix('"')
                    .and_then(|part| part.strip_suffix('"'))
                    .or_else(|| {
                        value
                            .strip_prefix('\'')
                            .and_then(|part| part.strip_suffix('\''))
                    })
                    .unwrap_or(value);
                match java_quality(unquoted) {
                    Some(parsed) if (0.0..=1.0).contains(&parsed) => quality = parsed,
                    _ if parameters.iter().any(|(name, _)| *name == "q") => continue,
                    _ => std::panic::panic_any(SpringMvcMediaTypeFailure::number_format(unquoted)),
                }
            }
            let charset = parameters
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("charset"))
                .and_then(|(_, rendered)| rendered.split_once('=').map(|(_, value)| value.to_owned()));
            if charset.as_deref().is_some_and(|value| {
                let value = value.trim_matches(['"', '\'']);
                !java_charset_supported(value)
            }) {
                continue;
            }
            // Spring compares the most specific of the requested and produced
            // media types. The produced Markdown type has one charset parameter;
            // copyQualityValue adds q before the comparison, even when q=1.
            let request_parameter_count = parameters.len();
            let has_quality_parameter = parameters
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case("q"));
            let has_lowercase_quality_parameter = parameters.iter().any(|(name, _)| *name == "q");
            let selection_parameter_count = if markdown {
                parameters.len().max(1 + usize::from(has_quality_parameter))
            } else {
                1 + usize::from(has_quality_parameter)
            };
            if has_lowercase_quality_parameter {
                parameters.retain(|(name, _)| !name.eq_ignore_ascii_case("q"));
            }
            if parameters.len() == 1 && parameters[0].0.eq_ignore_ascii_case("q") {
                parameters.clear();
            }
            if charset
                .as_deref()
                .is_some_and(|value| !value.eq_ignore_ascii_case("UTF-8"))
            {
                if wildcard {
                    wildcard_charset_fallback = true;
                }
                if !markdown || !has_compatible_candidate {
                    continue;
                }
            }
            let request_specificity = if markdown { 2 } else if text_wildcard { 1 } else { 0 };
            let content_type = if wildcard || text_wildcard {
                DEFAULT.to_string()
            } else {
                // MimeType keeps an uppercase-only CHARSET parameter for
                // matching, but its cached charset field remains unset.
                // StringHttpMessageConverter therefore writes its declared
                // UTF-8 charset when this candidate wins negotiation.
                if !has_lowercase_charset {
                    for (name, rendered) in &mut parameters {
                        if name.eq_ignore_ascii_case("charset") {
                            *rendered = "charset=UTF-8".to_string();
                        }
                    }
                }
                if charset.is_none() {
                    put_media_parameter(&mut parameters, "charset", "charset=UTF-8".to_string());
                }
                format!(
                    "text/markdown;{}",
                    parameters
                        .iter()
                        .map(|(_, rendered)| rendered.as_str())
                        .collect::<Vec<_>>()
                        .join(";")
                )
            };
            if selected
                .as_ref()
                .is_none_or(|(best_quality, best_parameter_count, best_specificity, best_request_parameters, _)| {
                    quality.total_cmp(best_quality)
                        .then_with(|| selection_parameter_count.cmp(best_parameter_count))
                        // Spring sorts the Accept list first. When the merged
                        // candidates tie, that original sort breaks the tie.
                        .then_with(|| request_specificity.cmp(best_specificity))
                        .then_with(|| request_parameter_count.cmp(best_request_parameters))
                        .is_gt()
                })
            {
                selected = Some((quality, selection_parameter_count, request_specificity, request_parameter_count, content_type));
            }
        }
    }
    if wildcard_charset_fallback && !has_compatible_candidate {
        return spring_empty_produces_string_type(headers);
    }
    selected.map(|(_, _, _, _, content_type)| content_type).or_else(|| {
        if wildcard_charset_fallback {
            Some("text/plain;charset=UTF-8".to_string())
        } else {
            (!specified).then(|| DEFAULT.to_string())
        }
    })
}

/// A mismatched wildcard still matches Spring's `produces` condition through
/// its special ALL branch, but that branch drops the declared Markdown type.
/// StringHttpMessageConverter then exposes text/plain and */* as producible.
fn spring_empty_produces_string_type(headers: &HeaderMap) -> Option<String> {
    let mut selected: Option<(f64, u8, usize, String, String)> = None;
    for value in headers.get_all(header::ACCEPT) {
        let Ok(value) = std::str::from_utf8(value.as_bytes()) else { continue };
        for candidate in split_outside_quotes(value, ',', true) {
            let parts = split_outside_quotes(candidate, ';', false);
            let base = if parts[0].trim() == "*" { "*/*" } else { parts[0].trim() };
            let Ok(media) = base.parse::<mime::Mime>() else { continue };
            let kind = media.type_().as_str();
            let subtype = media.subtype().as_str();
            let wildcard = kind == "*" || subtype == "*" || subtype.starts_with("*+");
            let specificity = if kind == "*" { 0 } else if wildcard { 1 } else { 2 };
            let mut exact = Vec::<(&str, String)>::new();
            for part in parts.into_iter().skip(1) {
                if let Some((name, value)) = part.split_once('=') {
                    let (name, value) = (name.trim(), value.trim());
                    put_exact_media_parameter(&mut exact, name, format!("{name}={value}"));
                }
            }
            let mut parameters = Vec::<(&str, String)>::new();
            for (name, rendered) in exact {
                put_media_parameter(&mut parameters, name, rendered);
            }
            let parameter_count = parameters.len();
            let mut quality = 1.0;
            if let Some((_, rendered)) = parameters.iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("q")) {
                let value = rendered.split_once('=').map_or("", |(_, value)| value);
                let unquoted = value.strip_prefix('"').and_then(|part| part.strip_suffix('"'))
                    .or_else(|| value.strip_prefix('\'').and_then(|part| part.strip_suffix('\'')))
                    .unwrap_or(value);
                match java_quality(unquoted) {
                    Some(parsed) if (0.0..=1.0).contains(&parsed) => quality = parsed,
                    _ if parameters.iter().any(|(name, _)| *name == "q") => continue,
                    _ => std::panic::panic_any(SpringMvcMediaTypeFailure::number_format(unquoted)),
                }
            }
            let has_lowercase_quality = parameters.iter().any(|(name, _)| *name == "q");
            if has_lowercase_quality {
                parameters.retain(|(name, _)| !name.eq_ignore_ascii_case("q"));
            }
            if parameters.len() == 1 && parameters[0].0.eq_ignore_ascii_case("q") {
                parameters.clear();
            }
            let effective_base = if wildcard { "text/plain".to_string() }
                else { format!("{kind}/{subtype}") };
            let content_type = if wildcard {
                "text/plain;charset=UTF-8".to_string()
            } else {
                let json = kind == "application" && (subtype == "json" || subtype.ends_with("+json"));
                if !json && !parameters.iter().any(|(name, _)| name.eq_ignore_ascii_case("charset")) {
                    put_media_parameter(&mut parameters, "charset", "charset=UTF-8".to_string());
                }
                if parameters.is_empty() { effective_base.clone() }
                else { format!("{effective_base};{}", parameters.iter()
                    .map(|(_, rendered)| rendered.as_str()).collect::<Vec<_>>().join(";")) }
            };
            let better = selected.as_ref().is_none_or(|(best_quality, best_specificity,
                best_parameters, best_base, _)| {
                quality.total_cmp(best_quality)
                    .then_with(|| specificity.cmp(best_specificity))
                    .then_with(|| if effective_base == *best_base {
                        parameter_count.cmp(best_parameters)
                    } else { std::cmp::Ordering::Equal })
                    .is_gt()
            });
            if better {
                selected = Some((quality, specificity, parameter_count, effective_base, content_type));
            }
        }
    }
    selected.map(|(_, _, _, _, content_type)| content_type)
        .or_else(|| Some("text/plain;charset=UTF-8".to_string()))
}

fn spring_response_charset(content_type: &str) -> Option<String> {
    split_outside_quotes(content_type, ';', false)
        .into_iter()
        .skip(1)
        .filter_map(|part| part.split_once('='))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("charset"))
        .map(|(_, value)| {
            let value = value.trim();
            value
                .strip_prefix('"')
                .and_then(|part| part.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|part| part.strip_suffix('\'')))
                .unwrap_or(value)
                .to_owned()
        })
        .last()
}

/// Spring's StringHttpMessageConverter encodes the Java String after media
/// negotiation. The selected Content-Type alone does not change body bytes.
fn spring_encode_string_body(utf8: &[u8], charset: &str) -> Vec<u8> {
    let text = std::str::from_utf8(utf8).expect("Markdown body is a UTF-8 String");
    let name = java_canonical_charset(charset).unwrap_or(charset).to_ascii_lowercase();
    if matches!(name.as_str(), "iso-2022-cn" | "x-jisautodetect") {
        std::panic::panic_any(SpringMvcMediaTypeFailure::unsupported_operation());
    }
    if name == "cesu-8" {
        let mut result = Vec::with_capacity(text.len() * 2);
        for unit in text.encode_utf16() {
            if unit <= 0x7f {
                result.push(unit as u8);
            } else if unit <= 0x7ff {
                result.push(0xc0 | (unit >> 6) as u8);
                result.push(0x80 | (unit & 0x3f) as u8);
            } else {
                result.push(0xe0 | (unit >> 12) as u8);
                result.push(0x80 | ((unit >> 6) & 0x3f) as u8);
                result.push(0x80 | (unit & 0x3f) as u8);
            }
        }
        return result;
    }
    if let Some(index) = include_str!("data/java-single-byte-names-graalvm25.txt")
        .lines()
        .position(|candidate| candidate.eq_ignore_ascii_case(&name))
    {
        static TABLES: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        let tables = TABLES.get_or_init(|| {
            let mut decoded = Vec::new();
            flate2::read::GzDecoder::new(&include_bytes!("data/java-single-byte-maps-graalvm25.bin.gz")[..])
                .read_to_end(&mut decoded)
                .expect("pinned JVM single-byte charset tables");
            assert_eq!(decoded.len(), 104 * 65536);
            decoded
        });
        let table = &tables[index * 65536..(index + 1) * 65536];
        return text.chars().map(|character| {
            let codepoint = character as usize;
            table[if codepoint < 65536 { codepoint } else { 0xfffd }]
        }).collect();
    }
    let mut offset = 0;
    for line in include_str!("data/java-multibyte-map-names-graalvm25.tsv").lines() {
        let (mapped_name, width) = line.split_once('\t').expect("pinned charset table name");
        let width = width.parse::<usize>().expect("pinned charset table width");
        let table_size = 65536 * (width + 1);
        if mapped_name.eq_ignore_ascii_case(&name) {
            static TABLES: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
            let tables = TABLES.get_or_init(|| {
                let mut decoded = Vec::new();
                flate2::read::GzDecoder::new(&include_bytes!("data/java-multibyte-maps-graalvm25.bin.gz")[..])
                    .read_to_end(&mut decoded)
                    .expect("pinned JVM multibyte charset tables");
                assert_eq!(decoded.len(), 8_978_432);
                decoded
            });
            let table = &tables[offset..offset + table_size];
            let mut result = Vec::with_capacity(text.len() * width);
            for character in text.chars() {
                let codepoint = character as usize;
                if name == "x-euc-tw" && codepoint >= 65536 {
                    if let Some(encoded) = java_euc_tw_supplementary(codepoint as u32) {
                        result.extend_from_slice(encoded);
                        continue;
                    }
                }
                if name == "gb18030" && codepoint >= 65536 {
                    result.extend_from_slice(java_gb18030_supplementary(codepoint as u32));
                    continue;
                }
                let index = if codepoint < 65536 { codepoint } else { 0xfffd } * (width + 1);
                let length = table[index] as usize;
                result.extend_from_slice(&table[index + 1..index + 1 + length]);
            }
            return result;
        }
        offset += table_size;
    }
    let mut offset = 0;
    for line in include_str!("data/java-stateful-map-names-graalvm25.tsv").lines() {
        let (mapped_name, width) = line.split_once('\t').expect("pinned charset table name");
        let width = width.parse::<usize>().expect("pinned charset table width");
        let table_size = 65536 * (width + 1);
        if mapped_name.eq_ignore_ascii_case(&name) {
            static TABLES: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
            let tables = TABLES.get_or_init(|| {
                let mut decoded = Vec::new();
                flate2::read::GzDecoder::new(&include_bytes!("data/java-stateful-maps-graalvm25.bin.gz")[..])
                    .read_to_end(&mut decoded)
                    .expect("pinned JVM stateful charset tables");
                assert_eq!(decoded.len(), 7_274_496);
                decoded
            });
            let table = &tables[offset..offset + table_size];
            let mut result = Vec::with_capacity(text.len() * width);
            let japanese_iso_2022 = matches!(name.as_str(),
                "iso-2022-jp" | "iso-2022-jp-2" | "x-windows-50220"
                | "x-windows-50221" | "x-windows-iso2022jp");
            let ibm_mixed = name.starts_with("x-ibm");
            let iso_2022_shift = match name.as_str() {
                "iso-2022-kr" => Some(&b"\x1b$)C\x0e"[..]),
                "x-iso-2022-cn-gb" => Some(&b"\x1b$)A\x0e"[..]),
                "x-iso-2022-cn-cns" => Some(&b"\x1b$)G\x0e"[..]),
                _ => None,
            };
            let replacement_at = 0xfffd * (width + 1);
            let replacement = &table[replacement_at + 1..replacement_at + 1 + table[replacement_at] as usize];
            let mut shifted = false;
            let mut designated = false;
            let mut cns_plane_two_designated = false;
            let mut cns_plane_three_designated = false;
            for character in text.chars() {
                let codepoint = character as usize;
                let index = if codepoint < 65536 { codepoint } else { 0xfffd } * (width + 1);
                let length = table[index] as usize;
                let bytes = &table[index + 1..index + 1 + length];
                if japanese_iso_2022
                    && result.ends_with(b"\x1b(B")
                    && bytes.starts_with(b"\x1b$B")
                {
                    result.truncate(result.len() - 3);
                    result.extend_from_slice(&bytes[3..]);
                } else if name == "x-iso-2022-cn-cns" && bytes.starts_with(b"\x1b$*H\x1bN") {
                    // CNS plane 2 is a single shift: it neither leaves nor
                    // enters the plane 1 SO/SI state. Its designation is
                    // emitted only once until a newline resets the encoder.
                    result.extend_from_slice(if cns_plane_two_designated { &bytes[4..] } else { bytes });
                    cns_plane_two_designated = true;
                } else if name == "x-iso-2022-cn-cns" && bytes.starts_with(b"\x1b$+I\x1bO") {
                    result.extend_from_slice(if cns_plane_three_designated { &bytes[4..] } else { bytes });
                    cns_plane_three_designated = true;
                } else if ibm_mixed && bytes.first() == Some(&0x0e) && bytes.last() == Some(&0x0f) {
                    if !shifted {
                        result.push(0x0e);
                        shifted = true;
                    }
                    result.extend_from_slice(&bytes[1..bytes.len() - 1]);
                } else if iso_2022_shift.is_some_and(|prefix| bytes.starts_with(prefix)) {
                    let prefix = iso_2022_shift.unwrap();
                    if shifted {
                        result.extend_from_slice(&bytes[prefix.len()..]);
                    } else if designated {
                        result.push(0x0e);
                        result.extend_from_slice(&bytes[prefix.len()..]);
                        shifted = true;
                    } else {
                        result.extend_from_slice(bytes);
                        shifted = true;
                        designated = true;
                    }
                } else if shifted && character != '?' && bytes == replacement {
                    result.extend_from_slice(bytes);
                } else {
                    if shifted {
                        result.push(0x0f);
                        shifted = false;
                    }
                    result.extend_from_slice(bytes);
                    if iso_2022_shift.is_some() && character == '\n' {
                        designated = false;
                        cns_plane_two_designated = false;
                        cns_plane_three_designated = false;
                    }
                }
            }
            if shifted {
                result.push(0x0f);
            }
            return result;
        }
        offset += table_size;
    }
    let (width, little_endian, bom) = match name.as_str() {
        "utf-16" | "unicode" => (2, false, &[0xfe, 0xff][..]),
        "utf-16be" | "x-utf-16be" => (2, false, &[][..]),
        "utf-16le" | "x-utf-16le" => (2, true, &[][..]),
        "x-utf-16le-bom" => (2, true, &[0xff, 0xfe][..]),
        "utf-32" | "utf-32be" | "x-utf-32be" => (4, false, &[][..]),
        "utf-32le" | "x-utf-32le" => (4, true, &[][..]),
        "utf-32be-bom" | "x-utf-32be-bom" => (4, false, &[0x00, 0x00, 0xfe, 0xff][..]),
        "utf-32le-bom" | "x-utf-32le-bom" => (4, true, &[0xff, 0xfe, 0x00, 0x00][..]),
        _ => (0, false, &[][..]),
    };
    if width != 0 {
        let mut encoded = Vec::with_capacity(bom.len() + text.len() * width);
        encoded.extend_from_slice(bom);
        if width == 2 {
            for unit in text.encode_utf16() {
                encoded.extend_from_slice(&if little_endian { unit.to_le_bytes() } else { unit.to_be_bytes() });
            }
        } else {
            for character in text.chars() {
                let unit = character as u32;
                encoded.extend_from_slice(&if little_endian { unit.to_le_bytes() } else { unit.to_be_bytes() });
            }
        }
        return encoded;
    }
    if let Some(encoding) = encoding_rs::Encoding::for_label(name.as_bytes()) {
        let mut encoder = encoding.new_encoder();
        let mut result = Vec::with_capacity(text.len());
        let mut remaining = text;
        let mut buffer = [0u8; 8192];
        loop {
            let (status, read, written) = encoder.encode_from_utf8_without_replacement(
                remaining,
                &mut buffer,
                true,
            );
            result.extend_from_slice(&buffer[..written]);
            remaining = &remaining[read..];
            match status {
                encoding_rs::EncoderResult::InputEmpty => return result,
                encoding_rs::EncoderResult::OutputFull => continue,
                encoding_rs::EncoderResult::Unmappable(_) => {
                    // Java's String.getBytes(charset) writes the charset's
                    // encoded question mark for each unmappable character.
                    loop {
                        let (status, _, written) = encoder.encode_from_utf8_without_replacement(
                            "?",
                            &mut buffer,
                            false,
                        );
                        result.extend_from_slice(&buffer[..written]);
                        if status == encoding_rs::EncoderResult::InputEmpty {
                            break;
                        }
                        assert_eq!(status, encoding_rs::EncoderResult::OutputFull);
                    }
                }
            }
        }
    }
    utf8.to_vec()
}

fn java_canonical_charset(label: &str) -> Option<&'static str> {
    static ALIASES: std::sync::OnceLock<std::collections::HashMap<String, &'static str>> =
        std::sync::OnceLock::new();
    let aliases = ALIASES.get_or_init(|| {
        let mut aliases = std::collections::HashMap::new();
        for line in include_str!("data/java-charsets-catalog-graalvm25.tsv").lines() {
            let mut fields = line.splitn(3, '\t');
            let canonical = fields.next().expect("charset name");
            let _width = fields.next().expect("charset width");
            let names = fields.next().expect("charset aliases");
            aliases.insert(canonical.to_ascii_lowercase(), canonical);
            for alias in names.split(',').filter(|alias| !alias.is_empty()) {
                aliases.insert(alias.to_ascii_lowercase(), canonical);
            }
        }
        aliases
    });
    aliases.get(&label.to_ascii_lowercase()).copied()
}

fn java_euc_tw_supplementary(codepoint: u32) -> Option<&'static [u8]> {
    static POINTS: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let points = POINTS.get_or_init(|| {
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(&include_bytes!("data/java-euc-tw-supplementary-graalvm25.bin.gz")[..])
            .read_to_end(&mut decoded)
            .expect("pinned JVM EUC-TW supplementary table");
        assert_eq!(decoded.len(), 276_408);
        decoded
    });
    let mut low = 0;
    let mut high = points.len() / 9;
    while low < high {
        let middle = (low + high) / 2;
        let at = middle * 9;
        let current = u32::from_be_bytes(points[at..at + 4].try_into().unwrap());
        match current.cmp(&codepoint) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => {
                let length = points[at + 4] as usize;
                return Some(&points[at + 5..at + 5 + length]);
            }
        }
    }
    None
}

fn java_gb18030_supplementary(codepoint: u32) -> &'static [u8] {
    static POINTS: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let points = POINTS.get_or_init(|| {
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(&include_bytes!("data/java-gb18030-supplementary-graalvm25.bin.gz")[..])
            .read_to_end(&mut decoded)
            .expect("pinned JVM GB18030 supplementary table");
        assert_eq!(decoded.len(), 9_437_184);
        decoded
    });
    let at = (codepoint as usize - 0x10000) * 9;
    assert_eq!(u32::from_be_bytes(points[at..at + 4].try_into().unwrap()), codepoint);
    let length = points[at + 4] as usize;
    &points[at + 5..at + 5 + length]
}

#[derive(Debug)]
struct SpringMvcMediaTypeFailure {
    message: String,
    #[cfg_attr(not(test), allow(dead_code))]
    cause_class: &'static str,
}

impl SpringMvcMediaTypeFailure {
    const CLASS: &'static str = "org.springframework.web.util.NestedServletException";
    const INVALID_MEDIA_CAUSE: &'static str = "org.springframework.http.InvalidMediaTypeException";
    const NUMBER_FORMAT_CAUSE: &'static str = "java.lang.NumberFormatException";
    const UNSUPPORTED_OPERATION_CAUSE: &'static str = "java.lang.UnsupportedOperationException";

    fn invalid_media(content_type: &str, reason: &str) -> Self {
        Self {
            message: format!(
                "Request processing failed; nested exception is {}: Invalid mime type \"{content_type}\": {reason}",
                Self::INVALID_MEDIA_CAUSE,
            ),
            cause_class: Self::INVALID_MEDIA_CAUSE,
        }
    }

    fn number_format(value: &str) -> Self {
        Self {
            message: format!(
                "Request processing failed; nested exception is {}: For input string: \"{value}\"",
                Self::NUMBER_FORMAT_CAUSE,
            ),
            cause_class: Self::NUMBER_FORMAT_CAUSE,
        }
    }

    fn unsupported_operation() -> Self {
        Self {
            message: format!(
                "Request processing failed; nested exception is {}",
                Self::UNSUPPORTED_OPERATION_CAUSE,
            ),
            cause_class: Self::UNSUPPORTED_OPERATION_CAUSE,
        }
    }
}

impl std::fmt::Display for SpringMvcMediaTypeFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", Self::CLASS, self.message)
    }
}

/// Spring 5.3.23 MimeTypeUtils scans semicolon parameters by toggling at each
/// quote, including a quote preceded by a backslash. Its MimeType constructor
/// accepts a value when its first and last characters are matching quotes.
fn spring_mime_failure_reason(media: &str) -> Option<String> {
    let start = media.find(';').unwrap_or(media.len());
    let full_type = media[..start].trim();
    if full_type.is_empty() {
        return Some("'mimeType' must not be empty".into());
    }
    let full_type = if full_type == "*" { "*/*" } else { full_type };
    let Some((kind, subtype)) = full_type.split_once('/') else {
        return Some("does not contain '/'".into());
    };
    if kind.is_empty() {
        return Some("'type' must not be empty".into());
    }
    if subtype.is_empty() {
        return Some("does not contain subtype after '/'".into());
    }
    if kind == "*" && subtype != "*" {
        return Some("wildcard type is legal only in '*/*' (all mime types)".into());
    }
    if let Some(reason) = spring_token_failure(kind).or_else(|| spring_token_failure(subtype)) {
        return Some(reason);
    }
    let mut cursor = start;
    let mut parameters: Vec<(&str, &str)> = Vec::new();
    while cursor < media.len() {
        let begin = cursor + 1;
        let mut end = media.len();
        let mut quoted = false;
        for (offset, character) in media[begin..].char_indices() {
            if character == '"' {
                quoted = !quoted;
            } else if character == ';' && !quoted {
                end = begin + offset;
                break;
            }
        }
        let parameter = media[begin..end].trim();
        if let Some((name, value)) = parameter.split_once('=') {
            let (name, value) = (name.trim(), value.trim());
            if let Some((_, old_value)) = parameters.iter_mut().find(|(old_name, _)| *old_name == name) {
                *old_value = value;
            } else {
                parameters.push((name, value));
            }
        }
        cursor = end;
    }
    for (name, value) in &parameters {
        if name.is_empty() {
            return Some("'parameter' must not be empty".into());
        }
        if value.is_empty() {
            return Some("'value' must not be empty".into());
        }
        if let Some(reason) = spring_token_failure(name) {
            return Some(reason);
        }
        let quoted = (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''));
        if *name == "charset" {
            let charset = if quoted { &value[1..value.len() - 1] } else { value };
            if !java_charset_supported(charset) {
                return Some(format!("unsupported charset '{charset}'"));
            }
        } else if !quoted {
            if let Some(reason) = spring_token_failure(value) {
                return Some(reason);
            }
        }
    }
    // MimeType folds parameter names case-insensitively before MediaType
    // checks its exact lowercase q parameter. A later Q can replace q.
    let mut folded: Vec<(&str, &str)> = Vec::new();
    for (name, value) in parameters {
        if let Some((old_name, old_value)) = folded.iter_mut()
            .find(|(old_name, _)| old_name.eq_ignore_ascii_case(name)) {
            *old_name = name;
            *old_value = value;
        } else {
            folded.push((name, value));
        }
    }
    for (name, value) in folded {
        if name == "q" {
            let quoted = (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''));
            let number = if quoted { &value[1..value.len() - 1] } else { value };
            if !java_quality(number).is_some_and(|quality| (0.0..=1.0).contains(&quality)) {
                return Some(format!("Invalid quality value \"{number}\": should be between 0.0 and 1.0"));
            }
        }
    }
    None
}

fn spring_token_failure(token: &str) -> Option<String> {
    token.chars().find(|character| {
        !character.is_ascii_graphic()
            || "()<>@,;:\\\"/[]?={}".contains(*character)
    }).map(|character| {
        format!("Invalid token character '{character}' in token \"{token}\"")
    })
}

fn java_charset_supported(name: &str) -> bool {
    static NAMES: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
        std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| {
        include_str!("data/java-charsets-graalvm25.txt")
            .lines()
            .collect()
    });
    names.contains(name.to_ascii_lowercase().as_str())
}

fn java_quality(raw: &str) -> Option<f64> {
    let raw = raw.trim_matches(|character: char| character <= '\u{20}');
    let raw = raw.strip_suffix(['d', 'D', 'f', 'F']).unwrap_or(raw);
    let (negative, unsigned) = if let Some(value) = raw.strip_prefix('-') {
        (true, value)
    } else {
        (false, raw.strip_prefix('+').unwrap_or(raw))
    };
    if let Some(hex) = unsigned.strip_prefix("0x").or_else(|| unsigned.strip_prefix("0X")) {
        return java_hex_quality(hex, negative);
    }
    raw.parse().ok()
}

/// Convert Java's hexadecimal Double syntax without rounding the mantissa or
/// scaling factor separately. A tiny mantissa and a huge positive exponent can
/// have a finite result even when `2.0_f64.powi(exponent)` overflows.
fn java_hex_quality(hex: &str, negative: bool) -> Option<f64> {
    let at = hex.find(['p', 'P'])?;
    let exponent_text = &hex[at + 1..];
    let (exponent_negative, exponent_digits) = if let Some(digits) = exponent_text.strip_prefix('-') {
        (true, digits)
    } else {
        (false, exponent_text.strip_prefix('+').unwrap_or(exponent_text))
    };
    if exponent_digits.is_empty() { return None; }
    let exponent_magnitude = exponent_digits.bytes().try_fold(0_i64, |value, digit| {
        digit.is_ascii_digit().then(|| value.saturating_mul(10).saturating_add(i64::from(digit - b'0')))
    })?;
    let exponent = if exponent_negative { -exponent_magnitude } else { exponent_magnitude };
    let mut digits = Vec::new();
    let mut before_dot = 0usize;
    let mut dotted = false;
    for character in hex[..at].chars() {
        if character == '.' {
            if dotted { return None; }
            dotted = true;
        } else {
            digits.push(character.to_digit(16)? as u8);
            if !dotted { before_dot += 1; }
        }
    }
    if digits.is_empty() { return None; }
    let sign = if negative { 1_u64 << 63 } else { 0 };
    let Some(first) = digits.iter().position(|digit| *digit != 0) else {
        return Some(f64::from_bits(sign));
    };
    let highest_bit = digits[first].ilog2();
    let mut binary_exponent = i128::from(exponent)
        + 4 * (before_dot as i128 - first as i128 - 1)
        + i128::from(highest_bit);

    // The first 53 bits determine an IEEE 754 significand. Keep one guard bit
    // and whether any later bit is set for round-to-nearest, ties-to-even.
    let mut bits = Vec::with_capacity(54);
    let mut beyond = false;
    for (index, digit) in digits.iter().enumerate().skip(first) {
        let top = if index == first { highest_bit } else { 3 };
        for shift in (0..=top).rev() {
            let set = (digit >> shift) & 1 != 0;
            if bits.len() < 54 {
                bits.push(set);
            } else {
                beyond |= set;
            }
        }
    }
    let prefix = |count: usize| -> u64 {
        (0..count).fold(0_u64, |value, index| {
            (value << 1) | u64::from(bits.get(index).copied().unwrap_or(false))
        })
    };
    let round_up = |count: usize, value: u64| -> bool {
        bits.get(count).copied().unwrap_or(false)
            && (value & 1 != 0 || beyond || bits.iter().skip(count + 1).any(|bit| *bit))
    };
    if binary_exponent > 1023 {
        return Some(f64::from_bits(sign | (0x7ff_u64 << 52)));
    }
    if binary_exponent < -1075 {
        return Some(f64::from_bits(sign));
    }
    if binary_exponent >= -1022 {
        let mut significand = prefix(53);
        significand += u64::from(round_up(53, significand));
        if significand == 1_u64 << 53 {
            significand >>= 1;
            binary_exponent += 1;
        }
        if binary_exponent > 1023 {
            return Some(f64::from_bits(sign | (0x7ff_u64 << 52)));
        }
        let exponent_bits = ((binary_exponent + 1023) as u64) << 52;
        return Some(f64::from_bits(sign | exponent_bits | (significand & ((1_u64 << 52) - 1))));
    }
    let count = (binary_exponent + 1075) as usize;
    let mut significand = prefix(count);
    significand += u64::from(round_up(count, significand));
    Some(f64::from_bits(sign | significand))
}

fn split_outside_quotes(value: &str, separator: char, honor_escape: bool) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if honor_escape && quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == separator && !quoted {
            parts.push(&value[start..index]);
            start = index + character.len_utf8();
        }
    }
    parts.push(&value[start..]);
    parts
}

fn put_media_parameter<'a>(
    parameters: &mut Vec<(&'a str, String)>,
    name: &'a str,
    rendered: String,
) {
    if let Some(index) = parameters
        .iter()
        .position(|(key, _)| key.eq_ignore_ascii_case(name))
    {
        parameters.remove(index);
    }
    parameters.push((name, rendered));
}

fn put_exact_media_parameter<'a>(
    parameters: &mut Vec<(&'a str, String)>,
    name: &'a str,
    rendered: String,
) {
    if let Some((_, value)) = parameters.iter_mut().find(|(key, _)| *key == name) {
        *value = rendered;
    } else {
        parameters.push((name, rendered));
    }
}

async fn semantic_get_handler() -> axum::response::Response {
    axum::http::Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .body(axum::body::Body::from(
            "GET is only supported for -assist- previews and .md/.html file previews.",
        ))
        .unwrap()
}

async fn ignored_post_handler() -> StatusCode {
    StatusCode::OK
}

async fn usage_key_post_handler() -> axum::response::Response {
    errors::invalid_target("Target '.well-known' is not in the whitelist and is not an assist target.")
}

async fn version_handler() -> impl IntoResponse {
    let build_version = option_env!("TEAQL_BUILD_VERSION").unwrap_or("197001010101");
    let commits = option_env!("TEAQL_RECENT_COMMITS").unwrap_or("Git info not available");
    let content = format!("# TeaQL Component Versions\n\n| Component | Version |\n| --- | --- |\n| **Generator Server** | `{build_version}` |\n| **teaql-java** | `1.553-RELEASE` |\n| **teaql-rs** | `5.0.5` |\n| **teaql-golang** | `v0.2.9` |\n| **teaql-python** | `0.2.9` |\n| **teaql-dotnet** | `0.2.9` |\n| **teaql-ts** | `0.2.10` |\n\n| **teaql-swift** | `0.2.9` |\n\n### Recent Server Commits\n\n```text\n{commits}\n```\n");
    (
        [(header::CONTENT_TYPE, "text/markdown;charset=UTF-8")],
        content,
    )
}

async fn services_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/markdown;charset=UTF-8")],
        include_str!("../../teaql-forge-codegen/templates/services.md"),
    )
}

async fn index_handler() -> impl IntoResponse {
    match demo_input() {
        Ok(input) => render_preview_internal("index".into(), &input, false).await,
        Err(error) => errors::generation_error(
            &error.message,
            error.exception_class == "java.lang.IllegalArgumentException",
        ),
    }
}

fn demo_input() -> Result<ModelInput, teaql_forge_codegen::demo_model::ReadError> {
    let (bytes, filename) = teaql_forge_codegen::demo_model::read()?;
    // GET appends the configured resource directly, even when its name or
    // signature looks like an uploaded ZIP. Only POST uses ModelInputLoader.
    Ok(ModelInput {
        files: std::collections::BTreeMap::from([(
            teaql_forge_codegen::input::normalize_path(&filename), bytes,
        )]),
        entrance_file: Some(filename),
        ..ModelInput::default()
    })
}

fn normalize_preview_path(path: &str) -> String {
    let path = path.strip_prefix("teaql/").unwrap_or(path);
    if path.is_empty() || path == "teaql" {
        "index".into()
    } else {
        path.into()
    }
}

fn ignored_preview_path(path: &str) -> bool {
    matches!(path, "generate" | "evaluate" | "evaluation" | "services" | "version" | "version/")
}

fn root_document_path(path: &str) -> Option<String> {
    (!path.contains('/') && templates::document_preview_template(path).is_some())
        .then(|| format!("doc/{}", path.to_lowercase()))
}

async fn preview_get_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Only the empty clean path is the source homepage. An explicit `index`
    // path has no supported GET preview suffix and must follow the 405 branch.
    let explicit_index = path == "index" || path == "teaql/index";
    let path = normalize_preview_path(&path);
    if ignored_preview_path(&path) {
        return StatusCode::OK.into_response();
    }
    if !explicit_index
        && (path == "index"
            || path.to_lowercase().contains("-assist-")
            || path.to_lowercase().ends_with(".md")
            || path.to_lowercase().ends_with(".html"))
    {
        let target = if path == "index" {
            "".into()
        } else if let Some(document_path) = root_document_path(&path) {
            document_path
        } else {
            path.clone()
        };
        telemetry::operation(4, &target);
    }
    if path.contains("-assist-") {
        if let Err(message) = errors::validate_target(&path) {
            return errors::invalid_target(&message);
        }
    }
    if explicit_index
        || (path != "index"
            && !path.to_lowercase().contains("-assist-")
            && !(path.to_lowercase().ends_with(".md") || path.to_lowercase().ends_with(".html")))
    {
        return axum::http::Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(axum::body::Body::from(
                "GET is only supported for -assist- previews and .md/.html file previews.",
            ))
            .unwrap();
    }
    match demo_input() {
        Ok(input) => {
            let document_path = root_document_path(&path).unwrap_or(path);
            render_preview_internal(document_path, &input, false).await
        }
        Err(error) => errors::generation_error(
            &error.message,
            error.exception_class == "java.lang.IllegalArgumentException",
        ),
    }
}

async fn preview_post_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    use axum::extract::FromRequest;
    let path = normalize_preview_path(&path);
    if ignored_preview_path(&path) {
        return StatusCode::OK.into_response();
    }
    if let Err(message) = errors::validate_target(&path) {
        return errors::invalid_target(&message);
    }
    telemetry::operation(3, &path);
    let multipart_request = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("multipart/form-data"));
    if !multipart_request {
        let bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
        };
        telemetry::upload(&bytes, "model.xml");
        let input = match ModelInput::load(&bytes, "model.xml") {
            Ok(input) => input,
            Err(error) => return errors::generation_error(&error, false),
        };
        return if path.contains("-assist-") {
            render_preview_internal(path, &input, true).await
        } else {
            generate_input(&input, &path).await
        };
    }
    let mut multipart = match Multipart::from_request(request, &()).await {
        Ok(multipart) => multipart,
        Err(error) => return error.into_response(),
    };
    let mut file_content = None;
    let mut xml_path = "model.xml".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(file_name) = field.file_name() {
                xml_path = file_name.to_string();
            }
            let data = field.bytes().await.unwrap();
            file_content = Some(data.to_vec());
        }
    }

    let xml = match file_content {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, "Missing file part").into_response(),
    };

    telemetry::upload(&xml, &xml_path);
    let input = match ModelInput::load(&xml, &xml_path) {
        Ok(input) => input,
        Err(error) => return errors::generation_error(&error, false),
    };
    if path.contains("-assist-") {
        render_preview_internal(path, &input, true).await
    } else {
        generate_input(&input, &path).await
    }
}

async fn render_preview_internal(
    path: String,
    input: &ModelInput,
    validate: bool,
) -> axum::response::Response {
    if validate && input.entrance_file.is_none() {
        return errors::generation_error("missing entrance file: main.xml", false);
    }
    let canonical = path.strip_prefix("doc/").unwrap_or(&path);
    let lower_path = canonical.to_lowercase();
    let doc_template = (path.starts_with("doc/") || !path.contains('/'))
        .then(|| canonical.rsplit('/').next().unwrap())
        .and_then(templates::document_preview_template);
    let template_name;
    let mut target_entity = None;

    if !path.contains("-assist-")
        && path != "index"
        && (lower_path.contains("-assist-")
            || lower_path.ends_with(".md")
            || lower_path.ends_with(".html"))
    {
        let full_path = if !path.contains('/') && doc_template.is_some() {
            format!("doc/{canonical}")
        } else {
            path.clone()
        };
        if let Err(message) = errors::validate_preview_path(&full_path) {
            return errors::invalid_target(&message);
        }
        if templates::read(&full_path).is_none()
            && teaql_forge_codegen::preview::has_builtin(&full_path)
        {
            let domain = match input.parse_preview_domain() {
                Ok(domain) => build_render_context(&domain),
                Err(error) => return errors::model_parse_error(input, &error),
            };
            return match teaql_forge_codegen::preview::render_builtin(&domain, &full_path).unwrap()
            {
                Ok(body) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/markdown;charset=utf-8")],
                    body,
                )
                    .into_response(),
                Err(error) => errors::generation_error(&error, false),
            };
        }
        if let Some(source) = templates::read(&full_path) {
            let content_type = if path.ends_with(".html") {
                "text/html;charset=utf-8"
            } else {
                "text/markdown;charset=utf-8"
            };
            if source.is_empty() {
                let target = full_path.split('/').next().unwrap_or("");
                let filename = full_path.rsplit('/').next().unwrap_or("");
                return (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, content_type)],
                    format!("## ❌ Error\n\nTemplate `{filename}.stg` not found in `{target}`.\n"),
                )
                    .into_response();
            }
            let domain = match input.parse_preview_domain() {
                Ok(domain) => build_render_context(&domain),
                Err(error) => return errors::model_parse_error(input, &error),
            };
            return match teaql_forge_codegen::engine::render_named_external_preview(&domain, &full_path, &source) {
                Ok(body) => {
                    (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], body).into_response()
                }
                Err(error) if error == "missing index template" => (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, content_type)],
                    "## ❌ Error\n\nTemplate must contain an 'index' definition.",
                )
                    .into_response(),
                Err(error) => errors::generation_error(&error, false),
            };
        }
        if doc_template.is_none() {
            let target = full_path.split('/').next().unwrap_or("");
            let filename = full_path.rsplit('/').next().unwrap_or("");
            let content_type = if path.ends_with(".html") {
                "text/html;charset=utf-8"
            } else {
                "text/markdown;charset=utf-8"
            };
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, content_type)],
                format!("## ❌ Error\n\nTemplate `{filename}.stg` not found in `{target}`.\n"),
            )
                .into_response();
        }
    }

    if path == "index" {
        template_name = "index".into();
    } else if let Some(document_template) = doc_template {
        template_name = format!("doc/{document_template}");
    } else if lower_path == "model-view.md" || lower_path == "doc/model-view.md" {
        template_name = "doc/model-view.MD".into();
    } else if lower_path.contains("-assist-") {
        let parts: Vec<&str> = path.split('/').collect();
        template_name = parts[0].to_string();
        if parts.len() > 1 {
            target_entity = Some(parts[1].to_string());
        }
    } else {
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            "GET is only supported for -assist- previews and .md/.html file previews.",
        )
            .into_response();
    }

    if let Some(message) = input.fatal_parse_failure() {
        return errors::generation_error(&message, false);
    }
    let outcome = if validate {
        match input.try_evaluate(&template_name) {
            Ok(outcome) => Some(outcome),
            Err(error) => return errors::generation_error(&error.servlet_message(), false),
        }
    } else { None };
    if let Some(evaluation) = &outcome {
        if evaluation.response.blocks_generation() {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "text/markdown;charset=utf-8")],
                render_markdown_report(&evaluation.response),
            )
                .into_response();
        }
    }
    let parsed = outcome
        .and_then(|evaluation| evaluation.domain.map(Ok))
        .unwrap_or_else(|| input.parse_preview_domain());
    let domain = match parsed {
        Ok(d) => d,
        Err(e) => return errors::model_parse_error(input, &e),
    };
    let render_domain = build_render_context(&domain);

    if teaql_forge_codegen::engine::assist_query_null_entity_path(&path) {
        return errors::generation_error(
            teaql_forge_codegen::engine::ASSIST_QUERY_NULL_ENTITY_MESSAGE,
            false,
        );
    }

    match teaql_forge_codegen::engine::render_preview(&render_domain, &template_name, target_entity)
    {
        Ok(content) => {
            let content_type = if path == "index" {
                "text/html;charset=utf-8"
            } else if path.ends_with(".html") {
                "text/html;charset=utf-8"
            } else {
                "text/markdown;charset=utf-8"
            };
            let status = if content.starts_with("## ❌ Error\n\nUnknown Tool API module `") {
                StatusCode::NOT_FOUND
            } else if content.starts_with("## Unsupported Query Assist location")
                || content.starts_with("## ❌ Error\n\nObject `")
            {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::OK
            };
            (status, [(header::CONTENT_TYPE, content_type)], content).into_response()
        }
        Err(e) => errors::generation_error(&e, false),
    }
}

async fn generate_handler(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_content = None;
    let mut scope = "rust-lib-core".to_string();
    let mut telemetry_scope = String::new();
    let mut xml_path = "model.xml".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(file_name) = field.file_name() {
                xml_path = file_name.to_string();
            }
            let data = field.bytes().await.unwrap();
            file_content = Some(data.to_vec());
        } else if name == "scope" {
            let data = field.bytes().await.unwrap();
            scope = String::from_utf8_lossy(&data).to_string();
            telemetry_scope = scope.clone();
        }
    }

    telemetry::operation(3, &telemetry_scope);
    let xml = match file_content {
        Some(c) => c,
        None => return errors::generation_error("missing entrance file: main.xml", false),
    };

    telemetry::upload(&xml, &xml_path);
    let input = match ModelInput::load(&xml, &xml_path) {
        Ok(input) => input,
        Err(error) => return errors::generation_error(&error, false),
    };
    if scope.contains("-assist-") {
        render_preview_internal(scope, &input, true).await
    } else {
        generate_input(&input, &scope).await
    }
}

async fn generate_input(input: &ModelInput, scope: &str) -> axum::response::Response {
    if input.entrance_file.is_none() {
        return errors::generation_error("missing entrance file: main.xml", false);
    }
    if let Some(message) = input.fatal_parse_failure() {
        return errors::generation_error(&message, false);
    }
    let evaluation = match input.try_evaluate(scope) {
        Ok(outcome) => outcome,
        Err(error) => return errors::generation_error(&error.servlet_message(), false),
    };
    if evaluation.response.blocks_generation() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "text/markdown;charset=utf-8")],
            render_markdown_report(&evaluation.response),
        )
            .into_response();
    }
    let domain = match evaluation
        .domain
        .ok_or("Evaluation completed without a parsed domain")
    {
        Ok(d) => d,
        Err(e) => return errors::generation_error(e, false),
    };

    let render_domain = build_render_context(&domain);
    let mut files = match generate_for_scope(&render_domain, scope) {
        Ok(files) => files,
        Err(teaql_forge_codegen::engine::GenerationError::UnsupportedScope(_)) => {
            return errors::unsupported_scope(scope);
        }
        Err(teaql_forge_codegen::engine::GenerationError::InvalidJavaModel(message))
        | Err(teaql_forge_codegen::engine::GenerationError::InvalidRustModel(message)) => {
            return errors::generation_error(&message, false);
        }
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("CodeGen error: {error}"),
            )
                .into_response();
        }
    };

    if let Ok(Some(document)) = input.semantic_document() {
        files.push(teaql_forge_codegen::engine::GeneratedFile {
            path: semantic::OUTPUT_PATH.into(),
            content: semantic::write_string(&document),
        });
    }
    let mut zip_buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for generated_file in files {
            // Need to add directories if missing, but zip crate might handle it or we can just add files
            // For safety, we can just add the file and zip viewers usually figure it out,
            // but let's add files properly.
            if let Err(e) = zip.start_file(&generated_file.path, options) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Zip error: {}", e),
                )
                    .into_response();
            }
            if let Err(e) = zip.write_all(generated_file.content.as_bytes()) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Zip write error: {}", e),
                )
                    .into_response();
            }
        }

        if let Err(e) = zip.finish() {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Zip finish error: {}", e),
            )
                .into_response();
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=\"domain.zip\"".parse().unwrap(),
    );

    (headers, zip_buffer).into_response()
}
