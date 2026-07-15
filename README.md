# TeaQL Forge Rust Server

[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/13621/badge)](https://www.bestpractices.dev/projects/13621)
[🇺🇸 English](README.md) | [🇨🇳 中文](README_zh.md)

TeaQL Forge Server is a local-first code generation service for TeaQL, written entirely in Rust.

It turns TeaQL `.xml` domain models into Rust libraries and fully configured workspace scaffolds. It provides a highly concurrent `axum`-based backend to process code generation, validate models, and serve AI-assist prompts for modern agentic workflows (e.g., Cursor, Claude, etc.).

## Features
- **In-Memory Generation:** Renders Rust files directly in memory and packages them as a streamable `.zip` file for lightning-fast execution.
- **AI Assist API:** Provides dynamic markdown templates (`rust-assist-*`) containing exact, model-aware code snippets to guide AI coding assistants in writing TeaQL standard code (CRUD, Queries, UI list pages, etc.).
- **Live Previews & Dashboard:** Serves an interactive HTML dashboard, a React Flow Entity Graph, and a Data Dictionary directly from the loaded model.
- **Multiple Targets:** Generates pure domain libraries (`rust-lib-core`) or fully configured CLI/Server workspaces (`rust-app-console`).
- **Tiny Multi-Arch Docker:** Built statically from `scratch` using `cargo-zigbuild` and `musl`. The final image is extremely lightweight (~5MB) and native to both `linux/amd64` and `linux/arm64`.

## Quick Start via Docker

The easiest way to run the local server is via Docker. This image runs smoothly on Linux, Windows (WSL), and Apple Silicon (M1/M2/M3) Macs.

```bash
docker run -d --name teaql-forge-server -p 8080:8080 teaql/teaql-forge-rs:latest
```

Once running, open your browser to view the interactive dashboard:
👉 **[http://localhost:8080/](http://localhost:8080/)**

### Integrate with AI Assistants
Point your CLI tool or AI IDE to the local server to unlock real-time, model-aware code assistance:
```bash
export TEAQL_ENDPOINT_PREFIX=http://localhost:8080/
```

## Usage with `cargo-teaql`

You can seamlessly integrate the local server with your existing `cargo-teaql` CLI workflow by pointing it to your local endpoint:

```bash
# Generate a full Workspace
cargo-teaql gen-workspace --endpoint-prefix http://127.0.0.1:8080/ models/model.xml

# Generate via the standard generate command
cargo teaql --input model.xml rust-app-console
```

## Available API Endpoints

- `GET /` - Interactive HTML Dashboard with Live Previews.
- `GET /version` - Returns the current server version.
- `GET /model-view.html` - Interactive Entity Relationship Graph (React Flow).
- `GET /data-design-react.html` - Data Dictionary UI.
- `GET /rust-assist-*/[entity]` - Generates AI context prompts for specific operations (e.g., `rust-assist-query/platform`).
- `POST /generate` - Accepts `multipart/form-data` with an `xml` file and a `scope` (`rust-lib-core` or `rust-app-console`).
- `POST /evaluate` - Evaluates KSML scripts and expressions.

## Building and Releasing

The server uses `cargo-zigbuild` for true multi-arch static compilation.

### Local Development
```bash
cargo run --bin teaql-forge-server -- --host 127.0.0.1 --port 8080
```

### Building Docker Images (Publishing)
To publish a new version of the Docker image to Docker Hub, use the automated script. This requires `cargo-zigbuild` installed locally (`cargo install cargo-zigbuild`).

```bash
# Publish as "latest"
./publish_docker.sh

# Publish a specific version tag
./publish_docker.sh v0.9.0
```

The script automatically:
1. Cross-compiles statically for `x86_64` and `aarch64`.
2. Packages both binaries into scratch containers (`-amd64` and `-arm64`).
3. Creates and pushes a unified Docker Manifest.

## Configuration

When running the binary manually, you can pass arguments:
- `--host`: The interface to bind to (e.g., `0.0.0.0` to expose to the network, `127.0.0.1` for local-only).
- `-p, --port`: The port to listen on (default `8081`).

> **Warning:** Binding to `0.0.0.0` exposes the TeaQL local server to the network. Use Enterprise Mode or configure TLS/auth for production environments.

## License
Apache-2.0 License
