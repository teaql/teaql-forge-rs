//! JDK Graph client's HTTP/2 preference, headers and HTTPS proxy tunnel.
use super::{Request, Response};
use http_body_util::{BodyExt, Full};
use hyper::{Request as HttpRequest, body::Bytes};
use hyper_util::{
    client::legacy::{
        Client,
        connect::{Connect, HttpConnector, proxy::Tunnel},
    },
    rt::TokioExecutor,
};
use std::collections::BTreeMap;

type Body = Full<Bytes>;
pub(super) enum Failure {
    Io,
    Argument(String),
    Initialization,
}

async fn exchange<C>(client: Client<C, Body>, request: HttpRequest<Body>) -> Result<Response, ()>
where
    C: Connect + Clone + Send + Sync + 'static,
{
    #[cfg(feature = "parity-probes")]
    let request_headers = request
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.as_str().to_string(),
                vec![value.to_str().unwrap().to_string()],
            )
        })
        .collect::<BTreeMap<_, _>>();
    let response = client.request(request).await.map_err(|_| ())?;
    #[cfg(feature = "parity-probes")]
    let (status, version) = (
        response.status().as_u16(),
        format!("{:?}", response.version()),
    );
    let mut headers = BTreeMap::new();
    for name in response.headers().keys() {
        headers.insert(
            name.as_str().to_string(),
            response
                .headers()
                .get_all(name)
                .iter()
                .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
                .collect(),
        );
    }
    let bytes = response
        .into_body()
        .collect()
        .await
        .map_err(|_| ())?
        .to_bytes();
    Ok(Response {
        #[cfg(feature = "parity-probes")]
        observation: Some(serde_json::json!({"request_headers":request_headers,
            "response_status":status,"response_version":version,"response_body_hex":super::hex(&bytes),
            "request_header_capture":"header map passed to Hyper; version observed on actual response"})),
        headers,
        body_hex: super::hex(&bytes),
        failure: String::new(),
    })
}

pub(super) fn transmit(request: Request, body: String) -> Result<Response, Failure> {
    let tls = super::tls::config().map_err(|_| Failure::Initialization)?;
    let proxy = super::proxy::select("mq.teaql.io", crate::process_properties::property)
        .map_err(Failure::Argument)?;
    // The source makes a new client for every getter. Own its runtime on a
    // separate thread so both synchronous CLI and Axum callers are supported.
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ())?;
        runtime.block_on(async move {
            let https = hyper_rustls::HttpsConnectorBuilder::new()
                .with_tls_config(tls)
                .https_only()
                .enable_http1()
                .enable_http2();
            let mut http = HttpConnector::new();
            http.enforce_http(false);
            let mut builder = HttpRequest::builder()
                .method(request.method)
                .uri(request.uri)
                // JDK's HttpRequestImpl reads the pinned VM's java.version.
                .header("User-Agent", "Java-http-client/25.0.2")
                .header("Content-Length", body.len());
            for (name, values) in request.headers {
                for value in values {
                    builder = builder.header(name, value);
                }
            }
            let outgoing = builder.body(Full::new(Bytes::from(body))).map_err(|_| ())?;
            let operation = async {
                if let Some(proxy) = proxy {
                    let tunnel = Tunnel::new(proxy.uri()?, http);
                    let client =
                        Client::builder(TokioExecutor::new()).build(https.wrap_connector(tunnel));
                    exchange(client, outgoing).await
                } else {
                    let client =
                        Client::builder(TokioExecutor::new()).build(https.wrap_connector(http));
                    exchange(client, outgoing).await
                }
            };
            tokio::time::timeout(
                std::time::Duration::from_millis(request.timeout_ms),
                operation,
            )
            .await
            .map_err(|_| ())?
        })
    })
    .join()
    .map_err(|_| Failure::Io)?
    .map_err(|_| Failure::Io)
}
