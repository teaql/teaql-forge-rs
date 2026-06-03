# TeaQL Forge Server

TeaQL Forge Server is a local-first code generation service for TeaQL, written entirely in Rust.

It turns TeaQL `.xml` domain models into Rust libraries and fully configured workspace scaffolds, while keeping the same service API used by TeaQL clients, plugins, and agent workflows.

The generated workspace is designed around context-bound execution and audit-ready runtime patterns, making it suitable for AI-assisted business software development.
## Features
- **In-Memory Generation:** Renders Rust files directly in memory and packages them as a streamable `.zip` file for speed.
- **Multiple Scopes:** Supports generating simple library crates (`rust-lib`) and full application workspaces (`rust-workspace`).
- **Axum Web Server:** Highly concurrent and memory-safe web server backend listening on configurable ports.
- **Lightweight Docker Image:** Built as a statically-linked binary from `scratch` for ARM64 using musl, resulting in a tiny (~3MB) footprint.

## Quick Start via Docker

The easiest way to run the local server is via Docker:

```bash
docker run -d --name teaql-forge-server -p 8080:8080 teaql/teaql-forge-rs:latest
```

Once running, you can quickly test if the server is up:

```bash
curl http://127.0.0.1:8080/version
```

The server exposes the following endpoints:
- `GET /version` - Returns the current server version.
- `POST /generate` - Accepts `multipart/form-data` with an `xml` file and a `scope` string.

## Usage with `cargo-teaql`

You can seamlessly integrate the local server with your existing `cargo-teaql` CLI workflow by pointing it to your local endpoint:

```bash
cargo-teaql gen-workspace --endpoint-prefix http://127.0.0.1:8080/ models/model.xml
```

This command will send your model to the Docker container, receive the generated zip file in-memory, and unpack the domain and workspace directly into your project directory.

## Configuration

When running the binary manually (outside of Docker), you can pass the following command-line arguments:

- `--host`: The interface to bind to (e.g., `0.0.0.0` to expose to the network, `127.0.0.1` for local-only).
- `-p, --port`: The port to listen on (default `8080`).

**Example:**
```bash
cargo run --bin teaql-forge-server -- --host 0.0.0.0 --port 8080
```
> **Warning:** Binding to `0.0.0.0` will expose the TeaQL local server to the network. Use Enterprise Mode or configure TLS/auth for production environments.

## License
Apache-2.0 License
