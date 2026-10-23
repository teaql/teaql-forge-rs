# TeaQL Model-Aware Services

Evaluate one KSML semantic model, generate language-native libraries and runnable workspaces, then discover the generated API progressively through Assist.

## Start here

```bash
# 1. Evaluate the model before generation
cargo teaql evaluate --input models/

# 2. Generate one library or runnable workspace
cargo teaql rust-lib-core --input models/ --output build/
cargo teaql rust-app-console --input models/ --output build/

# 3. Discover an entity API, then request field-level help only when needed
cargo teaql rust-assist-query/order --input models/
cargo teaql rust-assist-query/order.total_amount --input models/
```

## Generation targets

| Language | CLI key | Domain library | Runnable workspaces |
| --- | --- | --- | --- |
| Java | `java` | `java-lib-core` | `java-app-console`, `java-web-micronaut`, `java-web-quarkus`, `java-web-spring-boot` |
| Rust | `rust` | `rust-lib-core` | `rust-app-console`, `rust-web-axum`, `rust-web-topcoat` |
| TypeScript | `typescript` | `typescript-lib-core` | `typescript-app-console`, `typescript-app-expo`, `typescript-web-hono` |
| Go | `golang` | `golang-lib-core` | `golang-app-console`, `golang-web-gin` |
| Swift | `swift` | `swift-lib-core` | `swift-app-console` |
| .NET | `dotnet` | `dotnet-lib-core` | `dotnet-app-console`, `dotnet-web-aspnet` |
| Python | `python` | `python-lib-core` | `python-app-console`, `python-web-fastapi` |

## Progressive model-aware Assist

Use the exact CLI key from the table; display names are not command tokens.

```text
cargo teaql <language-key>-assist-<action>/<entity> --input <model>
cargo teaql <language-key>-assist-query/<entity>.<field> --input <model>
```

Available actions: `query`, `list-page`, `create`, `update`, `delete`, `expression`, `ensure-schema`, `debug`, `i18n`, `runtime-custom`, and `tool-api`.

Start with entity-level Assist. Request field-level query help only for the field being used; this keeps AI context small and avoids guessed generated APIs.

## Stable semantic identity (V1)

A model package may opt in with `.teaql/semantic-ids.json`. Evaluate, generate and Assist are read-only consumers. Identity allocation and lifecycle changes are explicit:

| Endpoint | Operation |
| --- | --- |
| `POST /semantic-identity/initialize` | Allocate IDs for a multipart KSML package |
| `POST /semantic-identity/reconcile` | Allocate IDs for newly added model elements |
| `POST /semantic-identity/rename` | Preserve an ID while changing its current path |
| `POST /semantic-identity/delete` | Reserve an ID as a tombstone |
| `POST /semantic-identity/restore` | Explicitly reactivate a tombstone |

Rename, delete and restore require `expectedRevision`; revision conflicts fail closed. Generated archives retain the validated sidecar as `teaql/semantic-ids.json`.

## Model and support targets

| Target | Purpose |
| --- | --- |
| `evaluate` | Validate the KSML model and return actionable findings before generation |
| `agent` | Generate model-aware agent instructions |
| `doc` | Generate data-design documentation |
| `evaluation` | Generate evaluation metrics or scripts |
| `mergemodel` | Merge model artifacts |

## Service metadata

- API version: `1.1.0`
- Deployed component versions: `cargo teaql version`
- This catalog: `cargo teaql services`
