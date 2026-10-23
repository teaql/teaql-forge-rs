//! External subsets are loaded by the source SAX parser, independently of DOM.
use super::Failure;
use std::{error::Error, path::PathBuf};

fn io_failure(message: String) -> Failure {
    Failure {
        message,
        line: None,
        offset: None,
        factory: None,
    }
}
fn quoted(value: &str) -> Option<(&str, &str)> {
    let value = value.trim_start();
    let quote = value.chars().next().filter(|ch| matches!(ch, '\'' | '"'))?;
    let end = value[1..].find(quote)? + 1;
    Some((&value[1..end], &value[end + 1..]))
}

fn location_url(location: &str, base: Option<&reqwest::Url>) -> Result<reqwest::Url, Failure> {
    if let Some(base) = base {
        return base
            .join(location)
            .map_err(|_| io_failure(format!("MalformedURLException: {location}")));
    }
    if location.contains(':') {
        reqwest::Url::parse(location)
            .map_err(|_| io_failure(format!("MalformedURLException: {location}")))
    } else {
        let root = crate::process_properties::property("user.dir")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        reqwest::Url::from_file_path(root.join(location))
            .map_err(|_| io_failure(format!("MalformedURLException: {location}")))
    }
}

fn load(url: &reqwest::Url) -> Result<Vec<u8>, Failure> {
    if url.scheme() == "file" {
        let path = url.to_file_path().map_err(|_| {
            io_failure(format!("IllegalArgumentException: URI is not hierarchical"))
        })?;
        return std::fs::read(&path).map_err(|error| {
            io_failure(format!(
                "FileNotFoundException: {} ({})",
                path.display(),
                crate::demo_model::native_error(&error)
            ))
        });
    }
    let original = url.to_string();
    let url = url.clone();
    // reqwest's blocking client owns a Tokio runtime. Keep its lifetime on a
    // separate OS thread when the generator is called by an async HTTP handler.
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .http1_only()
            .no_proxy()
            .timeout(None)
            .redirect(reqwest::redirect::Policy::limited(20))
            .build()
            .map_err(|error| io_failure(format!("IOException: {error}")))?;
        let response = client.get(url.clone()).send().map_err(|error| {
            let mut cause: Option<&(dyn Error + 'static)> = Some(&error);
            while let Some(value) = cause {
                if let Some(io) = value.downcast_ref::<std::io::Error>() {
                    let kind = if io.kind() == std::io::ErrorKind::ConnectionRefused {
                        "ConnectException"
                    } else {
                        "SocketException"
                    };
                    return io_failure(format!("{kind}: {}", crate::demo_model::native_error(io)));
                }
                cause = value.source();
            }
            if error.to_string().contains("dns") {
                io_failure(format!(
                    "UnknownHostException: {}",
                    url.host_str().unwrap_or("")
                ))
            } else {
                io_failure(format!("IOException: {error}"))
            }
        })?;
        let status = response.status().as_u16();
        if status >= 400 {
            return Err(io_failure(if matches!(status, 404 | 410) {
                format!("FileNotFoundException: {original}")
            } else {
                format!(
                    "IOException: Server returned HTTP response code: {status} for URL: {original}"
                )
            }));
        }
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| io_failure(format!("IOException: {error}")))
    })
    .join()
    .map_err(|_| io_failure("IOException: external subset reader failed".into()))?
}

/// Each external entity keeps the URL at its own declaration, rather than
/// resolving nested references against the uploaded XML filename.
/// Keep the decoded source and text-declaration cursor for provider diagnostics.
pub(super) fn read_source(
    location: &str,
    base: Option<&reqwest::Url>,
    woodstox: bool,
) -> Result<(String, reqwest::Url, String, usize), Failure> {
    let url = location_url(location, base)?;
    let bytes = load(&url)?;
    let source = if woodstox { super::woodstox::decode(&bytes)? } else { super::decode(&bytes)? };
    let start = if source.starts_with("<?xml") { source.find("?>").map(|end| end + 2).unwrap_or(0) } else { 0 };
    Ok((source[start..].into(), url, source, start))
}

pub(super) struct Expanded {
    pub xml: String,
    pub nodes: Vec<(String, usize)>,
}

pub(super) fn expand(xml: &str) -> Result<String, Failure> {
    Ok(expand_document(xml)?.xml)
}

pub(super) fn expand_document(xml: &str) -> Result<Expanded, Failure> {
    let Some(start) = xml
        .match_indices("<!DOCTYPE")
        .find(|(index, _)| {
            !matches!(
                super::context(xml, *index).area,
                "comment" | "cdata" | "instruction" | "attribute"
            )
        })
        .map(|(index, _)| index)
    else {
        return Ok(Expanded { xml: xml.into(), nodes: Vec::new() });
    };
    let Some((end, internal_range)) = super::dtd::doctype_span(xml, start) else {
        return Ok(Expanded { xml: xml.into(), nodes: Vec::new() });
    };
    let header = &xml[start + 9..end - 1];
    let mut parts = header
        .trim_start()
        .splitn(2, |ch: char| ch.is_ascii_whitespace());
    let name = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim_start();
    let location = if let Some(rest) = rest.strip_prefix("SYSTEM") {
        quoted(rest).map(|(location, _)| location)
    } else if let Some(rest) = rest.strip_prefix("PUBLIC") {
        quoted(rest)
            .and_then(|(_, rest)| quoted(rest))
            .map(|(location, _)| location)
    } else {
        None
    };
    let internal_start = internal_range.as_ref().map(|range| range.start).unwrap_or(0);
    let internal = internal_range.map(|range| &xml[range]).unwrap_or("");
    let extra = xml[start..end]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .saturating_sub(internal.bytes().filter(|byte| *byte == b'\n').count());
    let (subset, body, nodes) = super::dtd::subsets(internal, location, &xml[end..], xml, internal_start, end)?;
    Ok(Expanded { xml: format!(
        "{}<!DOCTYPE {name} [{subset}]>{}{}",
        &xml[..start],
        "\n".repeat(extra),
        body
    ), nodes })
}
