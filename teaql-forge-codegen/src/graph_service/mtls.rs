//! Run the production TLS configuration against the shared OpenSSL memory peer.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::Arc,
};

struct Peer(Child);
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn run(peer_script: &Path, fixture: &Path, output: &Path) -> Value {
    let Ok(mut config) = super::tls::config() else {
        return json!({"error":super::error_json(super::ClientError::Initialization)});
    };
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
        let mut peer = Peer(
            Command::new("python3")
                .arg(peer_script)
                .arg(fixture)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::from(fs::File::create(
                    output.with_extension("peer.stderr"),
                )?))
                .spawn()?,
        );
        let mut writer = peer.0.stdin.take().ok_or("peer input absent")?;
        let mut reader = BufReader::new(peer.0.stdout.take().ok_or("peer output absent")?);
        let mut trace = fs::File::create(output.with_extension("trace.jsonl"))?;
        let mut client =
            rustls::ClientConnection::new(Arc::new(config), "mq.teaql.io".try_into()?)?;
        let payload = "TLS client authentication / 词云\n".as_bytes();
        let (mut sent, mut complete, mut echoed) = (false, false, Vec::new());
        for _ in 0..100 {
            if !sent && complete && !client.is_handshaking() {
                client.writer().write_all(payload)?;
                sent = true;
            }
            let mut bytes = Vec::new();
            while client.wants_write() {
                client.write_tls(&mut bytes)?;
            }
            let request = json!({"incoming_hex":super::hex(&bytes)}).to_string();
            writeln!(writer, "{request}")?;
            writer.flush()?;
            let mut response = String::new();
            if reader.read_line(&mut response)? == 0 {
                return Err("TLS peer closed".into());
            }
            writeln!(trace, "{request}")?;
            write!(trace, "{response}")?;
            let response: Value = serde_json::from_str(&response)?;
            if response["failure"].is_string() {
                return Err(response["failure"].to_string().into());
            }
            complete = response["handshake_complete"] == true;
            let bytes = super::unhex(
                response["outgoing_hex"]
                    .as_str()
                    .ok_or("peer bytes absent")?,
            );
            if !bytes.is_empty() {
                client.read_tls(&mut std::io::Cursor::new(bytes))?;
                client.process_new_packets()?;
            }
            let mut block = [0u8; 65536];
            loop {
                match client.reader().read(&mut block) {
                    Ok(0) => break,
                    Ok(length) => echoed.extend_from_slice(&block[..length]),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(error) => return Err(error.into()),
                }
            }
            if sent && echoed.len() == payload.len() {
                let mut result = serde_json::Map::new();
                result.insert("handshake_ok".into(), json!(true));
                for key in [
                    "protocol",
                    "alpn",
                    "cipher",
                    "client_leaf_der_hex",
                    "application_echo_hex",
                ] {
                    result.insert(key.into(), response[key].clone());
                }
                result.insert("client_echo_hex".into(), json!(super::hex(&echoed)));
                return Ok(Value::Object(result));
            }
        }
        Err("TLS memory pump did not finish".into())
    })();
    match result {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::write(output.with_extension("failure.txt"), error.to_string());
            json!({"handshake_ok":false})
        }
    }
}
