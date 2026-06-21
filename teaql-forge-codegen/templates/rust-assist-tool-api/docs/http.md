# TeaQL Tool API: HTTP Client

Use `ctx.http()` to make outgoing HTTP requests. This client is fully integrated with the TeaQL runtime and automatically logs request/response details if tracing is enabled.

> [!WARNING]
> Always provide a `.purpose()` describing the business reason for the request.

## Required Dependencies

To use the HTTP client and other network features, you must add `teaql-tool` to your project with the `http` feature enabled. Run the following command in your terminal:

```bash
cargo add teaql-tool --features std,http
```

Or manually add it to your `Cargo.toml`:

```toml
teaql-tool = { version = "1.0", features = ["std", "http"] }
```

## GET Request

```rust
let response = ctx.http()
    .purpose("fetch user profile from external identity provider")
    .get("https://api.example.com/users/123")
    .header("Authorization", "Bearer token")
    .execute()
    .await?;

if response.status().is_success() {
    let json: serde_json::Value = response.json().await?;
}
```

## POST Request with JSON

```rust
let payload = serde_json::json!({
    "event": "user_signed_up",
    "userId": 123
});

let response = ctx.http()
    .purpose("notify external webhook of user signup")
    .post("https://webhook.example.com/notify")
    .json(&payload)
    .execute()
    .await?;
```

## Key Methods
- `.purpose(reason)`: **(Required)** Describe why this request is being made.
- `.get(url)` / `.post(url)` / `.put(url)` / `.delete(url)`: Start a request.
- `.header(key, value)`: Add an HTTP header.
- `.json(&payload)`: Set the body as JSON.
- `.form(&payload)`: Set the body as URL-encoded form.
- `.timeout(std::time::Duration)`: Set a custom timeout.
- `.execute().await?`: Execute the request.
