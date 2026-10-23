# TeaQL Tool API: HTTP Client

Use the versioned request-scoped `teaql-tool-context` facade. The current Rust implementation
supports an asynchronous GET returning response text. It does not expose POST, headers, JSON
bodies, or a `purpose(...)` method. Do not invent those APIs.

## Required Dependencies

To use the HTTP client and other network features, you must add `teaql-tool` to your project with the `http` feature enabled. Run the following command in your terminal:

```bash
cargo add teaql-tool-context --features http
```

Or manually add it to your `Cargo.toml`:

```toml
teaql-tool-context = { version = "2.0.0", features = ["http"] }
```

## GET Request

```rust
use teaql_tool_context::prelude::ContextHttpExt;

let body: String = ctx.http()
    .get("https://example.com")
    .comment("fetch the external status document")
    .await?;
```

## Key Methods
- `ContextHttpExt::http(&UserContext) -> ContextHttpFacade`
- `get(url) -> PendingHttpGet`
- `comment(intent) -> ExecutableHttpGet`
- awaiting the executable request returns `Result<String, String>`

The native response is the response body `String`; preserve the error on failure.
