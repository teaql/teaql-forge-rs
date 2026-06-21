use axum::{
    extract::Multipart,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use std::io::Write;
use teaql_forge_codegen::context::build_render_context;
use teaql_forge_codegen::engine::generate_virtual_crate;
use teaql_forge_model::parser::parse_model;
use zip::write::SimpleFileOptions;

mod eval;
mod rules;

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
    let args = Args::parse();

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/version", get(version_handler))
        .route("/generate", post(generate_handler))
        .route("/evaluate", post(eval::evaluate_handler))
        .route("/*path", get(preview_get_handler).post(preview_post_handler));

    if args.host == "0.0.0.0" {
        println!("Warning: You are exposing TeaQL Local Server to the network.");
        println!("Use Enterprise Mode or configure TLS/auth for production.");
    }

    let addr = format!("{}:{}", args.host, args.port);
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn version_handler() -> impl IntoResponse {
    let mut map = serde_json::Map::new();
    map.insert(
        "version".to_string(),
        serde_json::Value::String("1.0.0".to_string()),
    );
    map.insert(
        "name".to_string(),
        serde_json::Value::String("teaql-code-generator".to_string()),
    );
    Json(map)
}

async fn index_handler() -> impl IntoResponse {
    let xml = include_str!("demo.xml");
    let domain = match parse_model(xml, "demo.xml") {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Parse error: {}", e),
            )
                .into_response()
        }
    };
    let render_domain = build_render_context(&domain);
    let first_entity = render_domain
        .entities
        .first()
        .map(|e| e.rust_module.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let domain_name = &render_domain.name;

    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>TeaQL Code Gen Server</title>
<meta name="description" content="TeaQL Code Generation Server — generate Rust data layers, preview models, and explore AI-assisted code prompts.">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<style>
  :root {{
    --bg-main: #0f172a;
    --bg-card: rgba(30, 41, 59, 0.7);
    --border: rgba(255,255,255,0.1);
    --text-main: #f8fafc;
    --text-muted: #94a3b8;
    --primary: #3b82f6;
  }}
  *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
    background: var(--bg-main);
    color: var(--text-main);
    line-height: 1.6;
    min-height: 100vh;
    background-image:
      radial-gradient(ellipse 80% 50% at 50% -20%, rgba(59,130,246,0.15), transparent),
      radial-gradient(ellipse 60% 40% at 80% 100%, rgba(139,92,246,0.1), transparent);
  }}
  .container {{ max-width: 1100px; margin: 0 auto; padding: 2.5rem 1.5rem; }}
  h1 {{
    font-size: 2.2rem; font-weight: 700; text-align: center; margin-bottom: 0.3rem;
    background: linear-gradient(135deg, #3b82f6, #8b5cf6);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
  }}
  .subtitle {{ text-align: center; color: var(--text-muted); margin-bottom: 2rem; font-size: 1.05rem; }}
  .card {{
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 16px;
    padding: 1.5rem 1.8rem;
    margin-bottom: 1.2rem;
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    transition: border-color 0.2s;
  }}
  .card:hover {{ border-color: rgba(255,255,255,0.18); }}
  .card h2 {{
    font-size: 1.1rem; font-weight: 600; margin-bottom: 0.8rem;
    display: flex; align-items: center; gap: 0.5rem;
  }}
  .setup-block {{ margin-bottom: 0; }}
  .code-container {{
    position: relative;
    background: rgba(15, 23, 42, 0.8);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.2rem;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.88rem;
    overflow-x: auto;
  }}
  .copy-btn {{
    position: absolute; top: 0.6rem; right: 0.6rem;
    background: var(--primary); color: #fff; border: none;
    padding: 0.35rem 0.85rem; border-radius: 6px;
    font-size: 0.78rem; font-weight: 500; cursor: pointer;
    font-family: 'Inter', sans-serif;
    transition: background 0.2s, transform 0.1s;
  }}
  .copy-btn:hover {{ background: #2563eb; }}
  .copy-btn:active {{ transform: scale(0.96); }}
  .row {{ display: flex; gap: 1.2rem; }}
  @media (max-width: 768px) {{ .row {{ flex-direction: column; }} }}
  .col {{ flex: 1; min-width: 0; }}
  .preview-links {{ display: flex; gap: 0.6rem; flex-wrap: wrap; margin-top: 0.5rem; }}
  .link-chip {{
    display: inline-flex; align-items: center; gap: 0.4rem;
    background: rgba(59,130,246,0.12); color: #60a5fa;
    border: 1px solid rgba(59,130,246,0.25);
    padding: 0.45rem 1rem; border-radius: 999px;
    text-decoration: none; font-size: 0.85rem; font-weight: 500;
    transition: background 0.2s, border-color 0.2s;
  }}
  .link-chip:hover {{ background: rgba(59,130,246,0.22); border-color: rgba(59,130,246,0.45); }}
  .target-list {{ display: flex; flex-wrap: wrap; gap: 0.45rem; margin-top: 0.5rem; }}
  .target-tag {{
    font-family: 'JetBrains Mono', monospace; font-size: 0.8rem;
    background: rgba(148, 163, 184, 0.1); color: #cbd5e1;
    border: 1px solid var(--border);
    padding: 0.3rem 0.7rem; border-radius: 6px;
  }}
  .prompt-table {{
    width: 100%; border-collapse: separate; border-spacing: 0;
    font-size: 0.9rem;
  }}
  .prompt-table th {{
    text-align: left; padding: 0.65rem 0.8rem;
    color: var(--text-muted); font-weight: 500; font-size: 0.8rem;
    text-transform: uppercase; letter-spacing: 0.05em;
    border-bottom: 1px solid var(--border);
  }}
  .prompt-table td {{
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid rgba(255,255,255,0.05);
    vertical-align: middle;
  }}
  .prompt-table tr:last-child td {{ border-bottom: none; }}
  .prompt-table tr:hover td {{ background: rgba(255,255,255,0.03); }}
  .assist-btn {{
    display: inline-block; padding: 0.3rem 0.8rem;
    background: rgba(59,130,246,0.12); color: #60a5fa;
    border: 1px solid rgba(59,130,246,0.25);
    border-radius: 6px; text-decoration: none; font-size: 0.82rem;
    font-family: 'JetBrains Mono', monospace;
    cursor: pointer; transition: background 0.2s;
  }}
  .assist-btn:hover {{ background: rgba(59,130,246,0.25); }}
  .rust-btn {{
    background: rgba(239,68,68,0.15); color: #fca5a5;
    border-color: rgba(239,68,68,0.3);
  }}
  .rust-btn:hover {{ background: rgba(239,68,68,0.28); }}
  .modal-overlay {{
    display: none; position: fixed; inset: 0;
    background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);
    z-index: 1000; justify-content: center; align-items: center;
  }}
  .modal-overlay.active {{ display: flex; }}
  .modal-container {{
    background: #1e293b; border: 1px solid var(--border);
    border-radius: 16px; width: 90%; max-width: 900px;
    max-height: 85vh; display: flex; flex-direction: column;
    box-shadow: 0 25px 60px rgba(0,0,0,0.5);
  }}
  .modal-header {{
    display: flex; justify-content: space-between; align-items: center;
    padding: 1rem 1.5rem; border-bottom: 1px solid var(--border);
    font-weight: 600; font-size: 0.95rem;
  }}
  .modal-close {{
    background: rgba(239,68,68,0.15); color: #fca5a5;
    border: 1px solid rgba(239,68,68,0.3);
    width: 32px; height: 32px; border-radius: 8px;
    font-size: 1.1rem; cursor: pointer; display: flex;
    align-items: center; justify-content: center;
    transition: background 0.2s;
  }}
  .modal-close:hover {{ background: rgba(239,68,68,0.3); }}
  .modal-body {{
    padding: 1.5rem; overflow-y: auto; flex: 1;
  }}
  .modal-body pre {{
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.82rem; line-height: 1.65;
    white-space: pre-wrap; word-break: break-word;
    color: #e2e8f0;
  }}
  .modal-loading {{
    display: flex; align-items: center; justify-content: center;
    padding: 3rem; color: var(--text-muted);
  }}
  @keyframes pulse {{ 0%,100% {{ opacity: 1; }} 50% {{ opacity: 0.5; }} }}
  .modal-loading {{ animation: pulse 1.5s ease-in-out infinite; }}
  .footer {{
    text-align: center; color: var(--text-muted); font-size: 0.82rem;
    margin-top: 2rem; padding-top: 1.5rem;
    border-top: 1px solid var(--border);
  }}
  .footer a {{ color: #60a5fa; text-decoration: none; }}
  .footer a:hover {{ text-decoration: underline; }}
</style>
</head>
<body>
<div class="container">
  <h1>⚡ TeaQL Code Gen Server</h1>
  <p class="subtitle">Domain: <strong>{domain_name}</strong> — local development server for code generation &amp; AI-assisted prompts</p>

  <div class="card setup-block">
    <h2>🔧 Setup</h2>
    <p style="color:var(--text-muted);font-size:0.9rem;margin-bottom:0.6rem;">Point your CLI at this server:</p>
    <div class="code-container">
      <code id="endpoint-code">export TEAQL_ENDPOINT_PREFIX=http://localhost:8081/</code>
      <button class="copy-btn" onclick="copyEndpoint()">Copy</button>
    </div>
  </div>

  <div class="row">
    <div class="col">
      <div class="card">
        <h2>👁️ Live Previews</h2>
        <p style="color:var(--text-muted);font-size:0.88rem;">Interactive visualisations of the loaded model</p>
        <div class="preview-links">
          <a class="link-chip" href="/model-view.html">📊 Model View</a>
          <a class="link-chip" href="/data-design-react.html">🎨 Data Design</a>
        </div>
      </div>
    </div>
    <div class="col">
      <div class="card">
        <h2>📦 Code Generation API</h2>
        <div class="code-container" style="margin-bottom:0.6rem;">
          <code>cargo teaql --input model.xml [target]</code>
        </div>
        <div class="target-list">
          <span class="target-tag">rust-lib-core</span>
          <span class="target-tag">rust-app-console</span>
        </div>
      </div>
    </div>
  </div>

  <div class="card">
    <h2>🚀 Try It Live</h2>
    <p style="color:var(--text-muted);font-size:0.88rem;margin-bottom:0.8rem;">Click any endpoint to preview the AI-assist output for <code style="color:#60a5fa;">{first_entity}</code></p>
    <table class="prompt-table">
      <thead><tr><th>Feature</th><th>Endpoint</th><th></th></tr></thead>
      <tbody>
        <tr>
          <td>🔹 Create</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-create/{first_entity}</code></td>
          <td><a class="assist-btn" data-url="/rust-assist-create/{first_entity}" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🔹 Update</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-update/{first_entity}</code></td>
          <td><a class="assist-btn" data-url="/rust-assist-update/{first_entity}" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🔹 Query</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-query/{first_entity}</code></td>
          <td><a class="assist-btn" data-url="/rust-assist-query/{first_entity}" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🔹 List Page</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-list-page/{first_entity}</code></td>
          <td><a class="assist-btn" data-url="/rust-assist-list-page/{first_entity}" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🔹 Expression</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-expression/{first_entity}</code></td>
          <td><a class="assist-btn" data-url="/rust-assist-expression/{first_entity}" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🔹 Delete</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-delete/{first_entity}</code></td>
          <td><a class="assist-btn" data-url="/rust-assist-delete/{first_entity}" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🛠️ Debug Guide</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-debug</code></td>
          <td><a class="assist-btn rust-btn" data-url="/rust-assist-debug" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>🛠️ Tool API: Http</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-tool-api/http</code></td>
          <td><a class="assist-btn rust-btn" data-url="/rust-assist-tool-api/http" onclick="openAssist(event)">Preview</a></td>
        </tr>
        <tr>
          <td>⚙️ Runtime Custom</td>
          <td><code style="color:#cbd5e1;font-size:0.83rem;">rust-assist-runtime-custom</code></td>
          <td><a class="assist-btn rust-btn" data-url="/rust-assist-runtime-custom" onclick="openAssist(event)">Preview</a></td>
        </tr>
      </tbody>
    </table>
  </div>

  <div class="footer">
    TeaQL Forge Server &middot; <a href="/version">API Version</a>
  </div>
</div>

<div class="modal-overlay" id="modal">
  <div class="modal-container">
    <div class="modal-header">
      <span id="modal-title">Preview</span>
      <button class="modal-close" onclick="closeModal()">&times;</button>
    </div>
    <div class="modal-body" id="modal-body">
      <div class="modal-loading">Loading…</div>
    </div>
  </div>
</div>

<script>
(function() {{
  var loc = window.location;
  var base = loc.protocol + '//' + loc.host + '/';
  var el = document.getElementById('endpoint-code');
  if (el) el.textContent = 'export TEAQL_ENDPOINT_PREFIX=' + base;
}})();

function copyEndpoint() {{
  var el = document.getElementById('endpoint-code');
  var text = el ? el.textContent : '';
  navigator.clipboard.writeText(text).then(function() {{
    var btn = document.querySelector('.copy-btn');
    btn.textContent = 'Copied!';
    btn.style.background = '#16a34a';
    setTimeout(function() {{ btn.textContent = 'Copy'; btn.style.background = ''; }}, 1500);
  }});
}}

function openAssist(e) {{
  e.preventDefault();
  var url = e.currentTarget.getAttribute('data-url');
  var modal = document.getElementById('modal');
  var body = document.getElementById('modal-body');
  var title = document.getElementById('modal-title');
  title.textContent = url;
  body.innerHTML = '<div class="modal-loading">Loading…</div>';
  modal.classList.add('active');
  fetch(url).then(function(r) {{ return r.text(); }}).then(function(text) {{
    body.innerHTML = '<pre>' + text.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</pre>';
  }}).catch(function(err) {{
    body.innerHTML = '<pre style="color:#fca5a5;">Error: ' + err.message + '</pre>';
  }});
}}

function closeModal() {{
  document.getElementById('modal').classList.remove('active');
}}

document.getElementById('modal').addEventListener('click', function(e) {{
  if (e.target === this) closeModal();
}});

document.addEventListener('keydown', function(e) {{
  if (e.key === 'Escape') closeModal();
}});
</script>
</body>
</html>"##,
        first_entity = first_entity,
        domain_name = domain_name,
    );

    ([(header::CONTENT_TYPE, "text/html;charset=UTF-8")], html).into_response()
}

async fn preview_get_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    render_preview_internal(path, include_str!("demo.xml"), "demo.xml").await
}

async fn preview_post_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_content = None;
    let mut xml_path = "model.xml".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(file_name) = field.file_name() {
                xml_path = file_name.to_string();
            }
            let data = field.bytes().await.unwrap();
            file_content = Some(String::from_utf8_lossy(&data).to_string());
        }
    }

    let xml = match file_content {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, "Missing file part").into_response(),
    };

    render_preview_internal(path, &xml, &xml_path).await
}

async fn render_preview_internal(path: String, xml: &str, xml_path: &str) -> axum::response::Response {
    let lower_path = path.to_lowercase();
    let template_name;
    let mut target_entity = None;

    if path == "model-view.html" {
        template_name = "doc/model-view".to_string();
    } else if lower_path == "data-design.md"
        || lower_path == "data-design.html"
        || lower_path == "data-design-react.html"
    {
        template_name = format!("doc/{}", path.split('.').next().unwrap());
    } else if lower_path.contains("-assist-") {
        let parts: Vec<&str> = path.split('/').collect();
        template_name = parts[0].to_string();
        if parts.len() > 1 {
            target_entity = Some(parts[1].to_string());
        }
    } else {
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            "GET/POST is only supported for -assist- previews and doc previews",
        )
            .into_response();
    }

    let domain = match parse_model(xml, xml_path) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Parse error: {}", e),
            )
                .into_response()
        }
    };
    let render_domain = build_render_context(&domain);

    match teaql_forge_codegen::engine::render_preview(&render_domain, &template_name, target_entity)
    {
        Ok(content) => {
            let content_type = if path.ends_with(".html") {
                "text/html;charset=UTF-8"
            } else {
                "text/markdown;charset=UTF-8"
            };
            ([(header::CONTENT_TYPE, content_type)], content).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Render error: {}", e),
        )
            .into_response(),
    }
}

async fn generate_handler(mut multipart: Multipart) -> impl IntoResponse {
    let mut file_content = None;
    let mut scope = "rust-lib".to_string();
    let mut xml_path = "model.xml".to_string();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(file_name) = field.file_name() {
                xml_path = file_name.to_string();
            }
            let data = field.bytes().await.unwrap();
            file_content = Some(String::from_utf8_lossy(&data).to_string());
        } else if name == "scope" {
            let data = field.bytes().await.unwrap();
            scope = String::from_utf8_lossy(&data).to_string();
        }
    }

    let xml = match file_content {
        Some(c) => c,
        None => return (StatusCode::BAD_REQUEST, "Missing file part").into_response(),
    };

    if scope != "rust-lib" && scope != "rust_lib" && scope != "rust-lib-core"
        && scope != "rust-workspace" && scope != "rust-app-console"
    {
        println!("Warning: unsupported scope {}, assuming rust-lib-core", scope);
    }

    let domain = match parse_model(&xml, &xml_path) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Parse error: {}", e),
            )
                .into_response()
        }
    };

    let render_domain = build_render_context(&domain);
    let files = if scope == "rust-workspace" || scope == "rust-app-console" {
        match teaql_forge_codegen::engine::generate_virtual_workspace(&render_domain) {
            Ok(f) => f,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("CodeGen error: {}", e),
                )
                    .into_response()
            }
        }
    } else {
        match generate_virtual_crate(&render_domain) {
            Ok(f) => f,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("CodeGen error: {}", e),
                )
                    .into_response()
            }
        }
    };

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
