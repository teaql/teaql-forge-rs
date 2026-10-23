//! Wire compatible with the pinned source's UsageSender and TelemetryFilter.
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use ed25519_dalek::{
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    Signer, SigningKey, VerifyingKey,
};
use futures_util::FutureExt;
use prost::Message;
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{
    net::{SocketAddr, UdpSocket},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, PartialEq, Message)]
pub struct Payload {
    #[prost(bytes = "vec", tag = "1")]
    pub api_key_hash: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub client_ip: Vec<u8>,
    #[prost(int32, tag = "3")]
    pub operation: i32,
    #[prost(string, tag = "4")]
    pub target: String,
    #[prost(string, tag = "5")]
    pub client_version: String,
    #[prost(int32, tag = "6")]
    pub outcome: i32,
    #[prost(uint32, tag = "7")]
    pub duration_ms: u32,
    #[prost(message, optional, tag = "8")]
    pub model: Option<ModelSummary>,
    #[prost(message, optional, tag = "9")]
    pub result: Option<ResultSummary>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ModelSummary {
    #[prost(string, optional, tag = "1")]
    pub model_name: Option<String>,
    #[prost(uint32, optional, tag = "2")]
    pub source_file_count: Option<u32>,
    #[prost(uint64, optional, tag = "3")]
    pub source_bytes: Option<u64>,
    #[prost(uint32, optional, tag = "4")]
    pub object_count: Option<u32>,
    #[prost(uint32, optional, tag = "5")]
    pub field_count: Option<u32>,
    #[prost(uint32, optional, tag = "6")]
    pub relation_count: Option<u32>,
    #[prost(uint32, optional, tag = "7")]
    pub root_object_count: Option<u32>,
    #[prost(uint32, optional, tag = "8")]
    pub constant_object_count: Option<u32>,
    #[prost(uint32, optional, tag = "9")]
    pub view_object_count: Option<u32>,
    #[prost(uint32, optional, tag = "10")]
    pub custom_type_count: Option<u32>,
    #[prost(uint32, optional, tag = "11")]
    pub data_service_count: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ResultSummary {
    #[prost(uint32, optional, tag = "1")]
    pub generated_file_count: Option<u32>,
    #[prost(uint64, optional, tag = "2")]
    pub generated_bytes: Option<u64>,
    #[prost(uint32, optional, tag = "3")]
    pub evaluation_error_count: Option<u32>,
    #[prost(uint32, optional, tag = "4")]
    pub evaluation_warning_count: Option<u32>,
    #[prost(uint32, optional, tag = "5")]
    pub evaluation_suggestion_count: Option<u32>,
    #[prost(uint32, optional, tag = "6")]
    pub privacy_finding_count: Option<u32>,
    #[prost(bool, optional, tag = "7")]
    pub entity_scoped_assist: Option<bool>,
}

impl Payload {
    pub fn empty() -> Self {
        Self {
            model: Some(ModelSummary::default()),
            result: Some(ResultSummary::default()),
            ..Self::default()
        }
    }
}

pub struct UsageKey {
    signing: SigningKey,
    public: VerifyingKey,
}

fn pem(kind: &str, der: &[u8]) -> String {
    let encoded = STANDARD.encode(der);
    let mut result = format!("-----BEGIN {kind}-----\n");
    // Java's replaceAll adds a newline after each full 64 characters, followed
    // by an unconditional newline before END (even for exact multiples).
    for chunk in encoded.as_bytes().chunks(64) {
        result.push_str(std::str::from_utf8(chunk).unwrap());
        if chunk.len() == 64 {
            result.push('\n');
        }
    }
    result.push_str(&format!("\n-----END {kind}-----\n"));
    result
}

impl UsageKey {
    fn load_or_generate(home: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = home.join(".teaql/usage");
        std::fs::create_dir_all(&directory)?;
        let private = directory.join("ed25519-private.pem");
        let public = directory.join("ed25519-public.pem");
        if private.exists() && public.exists() {
            return Ok(Self {
                signing: SigningKey::from_pkcs8_pem(&std::fs::read_to_string(private)?)?,
                public: VerifyingKey::from_public_key_pem(&std::fs::read_to_string(public)?)?,
            });
        }
        let signing = SigningKey::generate(&mut OsRng);
        let key = Self {
            public: signing.verifying_key(),
            signing,
        };
        // Match Java's PKCS#8 v1 and SPKI encodings, including PEM wrapping.
        let mut der = vec![
            0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22,
            0x04, 0x20,
        ];
        der.extend_from_slice(&key.signing.to_bytes());
        std::fs::write(private, pem("PRIVATE KEY", &der))?;
        std::fs::write(public, pem("PUBLIC KEY", &key.public_der()))?;
        Ok(key)
    }

    fn public_der(&self) -> Vec<u8> {
        let mut der = vec![
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ];
        der.extend_from_slice(self.public.as_bytes());
        der
    }

    fn key_id(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(b"teaql-usage-signing-key-v1\0");
        hash.update(self.public.as_bytes());
        format!("tuk_{}", URL_SAFE_NO_PAD.encode(hash.finalize()))
    }

    fn frame(
        &self,
        payload: &Payload,
        process_id: u64,
        timestamp: u64,
        uuid: [u8; 16],
    ) -> Option<Vec<u8>> {
        let payload = payload.encode_to_vec();
        if 72 + payload.len() + 64 > 1200 {
            return None;
        }
        let mut frame = b"TU\x01\x00\x00\x48".to_vec();
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        frame.extend_from_slice(self.public.as_bytes());
        frame.extend_from_slice(&process_id.to_be_bytes());
        frame.extend_from_slice(&timestamp.to_be_bytes());
        frame.extend_from_slice(&uuid);
        frame.extend_from_slice(&payload);
        let mut signed = b"teaql-usage-signature-v1\0".to_vec();
        signed.extend_from_slice(&frame);
        frame.extend_from_slice(&self.signing.sign(&signed).to_bytes());
        Some(frame)
    }
}

#[derive(Clone, Default)]
pub struct UsageSender {
    key: Option<Arc<UsageKey>>,
    sender: Option<Arc<SenderQueue>>,
    process_id: u64,
}

struct SenderQueue {
    sender: mpsc::SyncSender<Payload>,
    running: Arc<AtomicBool>,
}

impl Drop for SenderQueue {
    fn drop(&mut self) {
        // Closing the last service handle stops the worker without draining
        // pending usage, matching the source's shutdown/interrupt behavior.
        self.running.store(false, Ordering::Release);
    }
}

impl UsageSender {
    pub fn initialize_configured() -> Self {
        Self::initialize(std::path::Path::new(&teaql_forge_codegen::process_properties::user_home()))
    }

    pub fn initialize(home: &Path) -> Self {
        Self::connect(home, "127.0.0.1:9999".parse().unwrap()).unwrap_or_else(|error| {
            eprintln!("Failed to initialize UsageSender, telemetry disabled: {error}");
            Self::default()
        })
    }

    fn connect(home: &Path, collector: SocketAddr) -> Result<Self, Box<dyn std::error::Error>> {
        let key = Arc::new(UsageKey::load_or_generate(home)?);
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let (sender, receiver) = mpsc::sync_channel::<Payload>(1000);
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = running.clone();
        let process_id = u64::from(std::process::id());
        let signing = key.clone();
        std::thread::Builder::new()
            .name("UsageSender-Thread".into())
            .spawn(move || {
                while let Ok(payload) = receiver.recv() {
                    if !worker_running.load(Ordering::Acquire) {
                        break;
                    }
                    let mut uuid = [0; 16];
                    OsRng.fill_bytes(&mut uuid);
                    uuid[6] = (uuid[6] & 0x0f) | 0x40;
                    uuid[8] = (uuid[8] & 0x3f) | 0x80;
                    if let Some(bytes) = signing.frame(&payload, process_id, now_ms(), uuid) {
                        let _ = socket.send_to(&bytes, collector);
                    }
                }
            })?;
        Ok(Self {
            key: Some(key),
            sender: Some(Arc::new(SenderQueue { sender, running })),
            process_id,
        })
    }

    fn offer(&self, payload: Payload) {
        if let Some(sender) = &self.sender {
            let _ = sender.sender.try_send(payload);
        }
    }
}

pub async fn key_handler(State(sender): State<UsageSender>) -> Response {
    let (status, text) = if let Some(key) = sender.key.as_ref().filter(|_| sender.sender.is_some())
    {
        (StatusCode::OK, format!("# TeaQL Usage Signing Key\n\n- Algorithm: `Ed25519`\n- Key ID: `{}`\n- Public Key: `{}`\n- Public Key Format: `X.509 SubjectPublicKeyInfo / Base64URL`\n- Current Process ID: `{}`\n\nRegister this Key ID and public key in the authenticated TeaQL key portal.\n\nThe private key remains on this service and is never returned by this endpoint.\n", key.key_id(), URL_SAFE_NO_PAD.encode(key.public_der()), sender.process_id))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "# TeaQL Usage Signing Key Unavailable\n\nUDP usage telemetry is disabled because the local signing key is unavailable.\nThe TeaQL generation and evaluation functions are not affected.\n".into())
    };
    (
        status,
        [
            (header::CONTENT_TYPE, "text/markdown;charset=UTF-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        text,
    )
        .into_response()
}

tokio::task_local! { static REQUEST_USAGE: Arc<Mutex<Payload>>; }

pub fn operation(operation: i32, target: &str) {
    let _ = REQUEST_USAGE.try_with(|payload| {
        let mut payload = payload.lock().unwrap();
        payload.operation = operation;
        payload.target = target.into();
    });
}

pub fn upload(bytes: &[u8], filename: &str) {
    let _ = REQUEST_USAGE.try_with(|payload| {
        let mut payload = payload.lock().unwrap();
        let model = payload.model.as_mut().unwrap();
        model.source_bytes = Some(bytes.len() as u64);
        model.source_file_count = teaql_forge_codegen::input::ModelInput::load(bytes, filename)
            .ok()
            .and_then(|input| input.source_file_count);
    });
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub async fn filter(State(sender): State<UsageSender>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if path == "/"
        || [
            "/version",
            "/services",
            "/.well-known/",
            "/health",
            "/static",
        ]
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return next.run(request).await;
    }
    let mut usage = Payload::empty();
    if let Some(ConnectInfo(address)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        usage.client_ip = match address.ip() {
            std::net::IpAddr::V4(ip) => ip.octets().to_vec(),
            std::net::IpAddr::V6(ip) => ip.octets().to_vec(),
        };
    }
    let usage = Arc::new(Mutex::new(usage));
    let start = now_ms();
    let result = REQUEST_USAGE
        .scope(
            usage.clone(),
            std::panic::AssertUnwindSafe(next.run(request)).catch_unwind(),
        )
        .await;
    let mut usage = usage.lock().unwrap().clone();
    usage.duration_ms = now_ms().wrapping_sub(start) as u32;
    usage.outcome = match &result {
        Err(_) => 4,
        Ok(response) if response.status().is_success() => 1,
        Ok(response) if response.status().is_client_error() => 3,
        _ => 4,
    };
    sender.offer(usage);
    match result {
        Ok(response) => response,
        Err(error) => std::panic::resume_unwind(error),
    }
}
