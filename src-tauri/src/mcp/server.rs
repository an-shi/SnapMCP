use crate::diag;
use crate::mcp::{protocol, tools};
use crate::paths;
use crate::state::{AppState, McpStatus, RuntimeInfo};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::net::TcpListener;
use std::sync::Arc;
use tauri::AppHandle;

const MAX_BODY_BYTES: usize = 24 * 1024 * 1024;

struct HttpCtx {
    app: AppHandle,
    state: Arc<AppState>,
    port: u16,
    token: String,
}

pub fn start(app: AppHandle, state: Arc<AppState>) -> Result<u16, String> {
    let settings = state.settings_snapshot();
    if !settings.mcp_enabled {
        if let Ok(mut status) = state.mcp.lock() {
            *status = McpStatus {
                running: false,
                port: 0,
                endpoint: String::new(),
                token_required: false,
                error: Some("MCP is disabled in settings.json".into()),
            };
        }
        let _ = std::fs::remove_file(paths::runtime_file());
        return Ok(0);
    }

    let preferred = settings.mcp_port.max(1024);
    let mut listener = None;
    let mut last_error = String::new();
    for offset in 0..20u16 {
        let port = preferred.saturating_add(offset);
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(bound) => {
                listener = Some((bound, port));
                break;
            }
            Err(err) => last_error = format!("port {port}: {err}"),
        }
    }
    let (listener, port) = listener.ok_or_else(|| {
        format!(
            "Could not bind the MCP HTTP server on 127.0.0.1:{preferred}-{}: {last_error}",
            preferred.saturating_add(19)
        )
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Cannot configure MCP socket: {e}"))?;
    let endpoint = format!("http://127.0.0.1:{port}/mcp");
    let token = settings.auth_token.clone();

    if let Ok(mut status) = state.mcp.lock() {
        *status = McpStatus {
            running: true,
            port,
            endpoint: endpoint.clone(),
            token_required: !token.is_empty(),
            error: None,
        };
    }
    write_runtime(port, &endpoint, &token)?;
    diag::info(format!("MCP HTTP server listening on {endpoint}"));

    let ctx = Arc::new(HttpCtx {
        app,
        state: Arc::clone(&state),
        port,
        token,
    });
    let router = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/mcp", post(mcp_post).get(mcp_get).delete(mcp_delete))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not found") })
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(ctx);
    let status_state = Arc::clone(&state);
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(listener) {
            Ok(listener) => listener,
            Err(err) => {
                diag::error(format!("Cannot start MCP listener: {err}"));
                if let Ok(mut status) = status_state.mcp.lock() {
                    status.running = false;
                    status.error = Some(format!("MCP listener failed: {err}"));
                }
                return;
            }
        };
        if let Err(err) = axum::serve(listener, router).await {
            diag::error(format!("MCP server stopped: {err}"));
            if let Ok(mut status) = status_state.mcp.lock() {
                status.running = false;
                status.error = Some(format!("MCP server stopped: {err}"));
            }
        }
    });
    Ok(port)
}

fn write_runtime(port: u16, endpoint: &str, token: &str) -> Result<(), String> {
    let info = RuntimeInfo {
        pid: std::process::id(),
        port,
        endpoint: endpoint.to_string(),
        token: token.to_string(),
        version: protocol::version().to_string(),
    };
    let bytes = serde_json::to_vec_pretty(&info).map_err(|e| e.to_string())?;
    paths::write_atomic(&paths::runtime_file(), &bytes)
        .map_err(|e| format!("Could not write runtime.json: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(paths::runtime_file()) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(paths::runtime_file(), perms);
        }
    }
    Ok(())
}

pub fn shutdown() {
    let _ = std::fs::remove_file(paths::runtime_file());
}

fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let origin = origin.trim().to_ascii_lowercase();
    let allowed = [
        "http://localhost",
        "https://localhost",
        "http://127.0.0.1",
        "https://127.0.0.1",
        "http://[::1]",
        "https://[::1]",
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
        "http://asset.localhost",
    ];
    allowed.iter().any(|base| {
        origin == *base
            || origin.starts_with(&format!("{base}:"))
            || origin.starts_with(&format!("{base}/"))
    })
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    if token.is_empty() {
        return true;
    }
    if let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(bearer) = value.strip_prefix("Bearer ") {
            if bearer == token {
                return true;
            }
        }
    }
    headers
        .get("x-snapmcp-token")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == token)
        .unwrap_or(false)
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>, data: Option<Value>) -> Value {
    let mut error = json!({ "code": code, "message": message.into() });
    if let Some(data) = data {
        error["data"] = data;
    }
    json!({ "jsonrpc": "2.0", "id": id, "error": error })
}

fn response(status: StatusCode, value: Value) -> Response {
    let mut res = (status, Json(value)).into_response();
    let headers = res.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    res
}

async fn health(State(ctx): State<Arc<HttpCtx>>) -> Response {
    let status = ctx.state.mcp.lock().map(|s| s.clone()).unwrap_or_default();
    let busy = ctx.state.busy.load(std::sync::atomic::Ordering::SeqCst);
    response(
        StatusCode::OK,
        json!({
            "ok": true,
            "name": protocol::SERVER_NAME,
            "version": protocol::version(),
            "mcp": status,
            "captureBusy": busy,
            "port": ctx.port,
        }),
    )
}

async fn root(State(ctx): State<Arc<HttpCtx>>) -> Html<String> {
    let endpoint = format!("http://127.0.0.1:{}/mcp", ctx.port);
    let tools = protocol::tools();
    let tool_names: Vec<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    Html(format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>SnapMCP MCP server</title>\
         <style>body{{font-family:system-ui,sans-serif;max-width:720px;margin:48px auto;padding:0 20px;\
         background:#0b0e14;color:#e8eefc}}code{{background:#151b26;padding:2px 6px;border-radius:6px}}\
         a{{color:#22d3ee}}</style></head><body><h1>SnapMCP</h1>\
         <p>A local screen capture and annotation MCP server.</p>\
         <p>Streamable HTTP endpoint: <code>{endpoint}</code></p>\
         <p>Tools: <code>{}</code></p>\
         <p>Health: <a href=\"/health\">/health</a></p>\
         <p>Bind address: <code>127.0.0.1:{}</code> (loopback only)</p></body></html>",
        tool_names.join("</code>, <code>"),
        ctx.port
    ))
}

async fn mcp_get() -> Response {
    response(
        StatusCode::METHOD_NOT_ALLOWED,
        json!({
            "jsonrpc": "2.0",
            "error": { "code": -32000, "message": "This server uses JSON POST responses; the legacy SSE GET stream is not supported." }
        }),
    )
}

async fn mcp_delete() -> Response {
    (StatusCode::OK, "session terminated").into_response()
}

fn body_protocol_version(body: &Value) -> Option<&str> {
    body.get("params")
        .and_then(|p| p.get("_meta"))
        .and_then(|m| m.get("io.modelcontextprotocol/protocolVersion"))
        .and_then(Value::as_str)
}

fn tool_exists(name: &str) -> bool {
    protocol::tools()
        .iter()
        .any(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
}

async fn mcp_post(State(ctx): State<Arc<HttpCtx>>, headers: HeaderMap, body: Bytes) -> Response {
    if !origin_allowed(&headers) {
        return response(
            StatusCode::FORBIDDEN,
            rpc_error(Value::Null, -32600, "Origin is not allowed", None),
        );
    }
    if !authorized(&headers, &ctx.token) {
        let mut res = response(
            StatusCode::UNAUTHORIZED,
            rpc_error(
                Value::Null,
                -32600,
                "Missing or invalid SnapMCP token",
                None,
            ),
        );
        res.headers_mut()
            .insert(header::WWW_AUTHENTICATE, "Bearer".parse().unwrap());
        return res;
    }

    let value: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(err) => {
            return response(
                StatusCode::BAD_REQUEST,
                rpc_error(
                    Value::Null,
                    -32700,
                    "Parse error",
                    Some(json!(err.to_string())),
                ),
            );
        }
    };
    if value.is_array() {
        return response(
            StatusCode::BAD_REQUEST,
            rpc_error(
                Value::Null,
                -32600,
                "JSON-RPC batches are not supported; send one request at a time",
                None,
            ),
        );
    }
    let Some(body_obj) = value.as_object() else {
        return response(
            StatusCode::BAD_REQUEST,
            rpc_error(Value::Null, -32600, "Invalid JSON-RPC request", None),
        );
    };

    let id_value = body_obj.get("id");
    if let Some(Value::Null) = id_value {
        return response(
            StatusCode::BAD_REQUEST,
            rpc_error(Value::Null, -32600, "Request id must not be null", None),
        );
    }
    let is_notification = id_value.is_none();
    let Some(method) = body_obj.get("method").and_then(Value::as_str) else {
        return response(
            StatusCode::BAD_REQUEST,
            rpc_error(
                id_value.cloned().unwrap_or(Value::Null),
                -32600,
                "Missing method",
                None,
            ),
        );
    };
    let method = method.to_string();
    let params = body_obj.get("params").cloned().unwrap_or_else(|| json!({}));
    let id = id_value.cloned().unwrap_or(Value::Null);

    if is_notification {
        // The protocol is stateless; no client notifications need action.
        return StatusCode::ACCEPTED.into_response();
    }

    let modern = body_protocol_version(&value).is_some() || method == "server/discover";
    let header_version = headers
        .get("mcp-protocol-version")
        .and_then(|v| v.to_str().ok());
    if modern {
        if let (Some(from_body), Some(from_header)) =
            (body_protocol_version(&value), header_version)
        {
            if from_body != from_header {
                return response(
                    StatusCode::BAD_REQUEST,
                    rpc_error(
                        id,
                        -32020,
                        "MCP-Protocol-Version header does not match request _meta",
                        Some(json!({ "body": from_body, "header": from_header })),
                    ),
                );
            }
        }
        if let Some(version) = body_protocol_version(&value) {
            if version != protocol::PROTOCOL_MODERN && method != "server/discover" {
                return response(
                    StatusCode::BAD_REQUEST,
                    rpc_error(
                        id,
                        -32022,
                        "Unsupported protocol version",
                        Some(json!({
                            "supported": protocol::SUPPORTED_VERSIONS,
                            "requested": version
                        })),
                    ),
                );
            }
        }
        if method != "server/discover" {
            if let Some(from_header) = headers.get("mcp-method").and_then(|v| v.to_str().ok()) {
                if from_header != method {
                    return response(
                        StatusCode::BAD_REQUEST,
                        rpc_error(
                            id,
                            -32020,
                            "Mcp-Method header does not match the request method",
                            Some(json!({ "method": method, "header": from_header })),
                        ),
                    );
                }
            }
            if method == "tools/call" {
                if let Some(name) = params.get("name").and_then(Value::as_str) {
                    if let Some(from_header) = headers.get("mcp-name").and_then(|v| v.to_str().ok())
                    {
                        if from_header != name {
                            return response(
                                StatusCode::BAD_REQUEST,
                                rpc_error(
                                    id,
                                    -32020,
                                    "Mcp-Name header does not match params.name",
                                    Some(json!({ "name": name, "header": from_header })),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    match method.as_str() {
        "server/discover" => {
            let mut result = json!({
                "supportedVersions": protocol::SUPPORTED_VERSIONS,
                "capabilities": { "tools": { "listChanged": false } },
                "instructions": protocol::instructions(),
                "ttlMs": 60000,
                "cacheScope": "private"
            });
            if let Some(obj) = result.as_object_mut() {
                obj.insert(
                    "_meta".into(),
                    json!({ "io.modelcontextprotocol/serverInfo": protocol::server_info() }),
                );
            }
            response(
                StatusCode::OK,
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": protocol::add_modern_result(result)
                }),
            )
        }
        "initialize" if !modern => {
            let requested = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-11-25");
            let negotiated = protocol::legacy_negotiate(requested);
            let result = json!({
                "protocolVersion": negotiated,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": protocol::server_info(),
                "instructions": protocol::instructions()
            });
            response(
                StatusCode::OK,
                json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            )
        }
        "ping" => {
            let result = if modern {
                protocol::add_modern_result(json!({}))
            } else {
                json!({})
            };
            response(
                StatusCode::OK,
                json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            )
        }
        "logging/setLevel" if !modern => response(
            StatusCode::OK,
            json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        ),
        "tools/list" => {
            let mut result = json!({ "tools": protocol::tools() });
            if modern {
                result["ttlMs"] = json!(60000);
                result["cacheScope"] = json!("private");
                result = protocol::add_modern_result(result);
            }
            response(
                StatusCode::OK,
                json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            )
        }
        "tools/call" => {
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return response(
                    StatusCode::OK,
                    rpc_error(id, -32602, "tools/call requires params.name", None),
                );
            };
            if !tool_exists(name) {
                return response(
                    StatusCode::OK,
                    rpc_error(id, -32602, format!("Unknown tool: {name}"), None),
                );
            }
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let output = tools::execute(&ctx.app, Arc::clone(&ctx.state), name, arguments).await;
            let result = match output {
                Ok(output) => {
                    let mut result = json!({
                        "content": output.content,
                        "isError": output.is_error
                    });
                    if modern {
                        result = protocol::add_modern_result(result);
                    }
                    result
                }
                Err(message) => {
                    let mut result = json!({
                        "content": [{ "type": "text", "text": message }],
                        "isError": true
                    });
                    if modern {
                        result = protocol::add_modern_result(result);
                    }
                    result
                }
            };
            response(
                StatusCode::OK,
                json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            )
        }
        _ => {
            if modern {
                response(
                    StatusCode::NOT_FOUND,
                    rpc_error(
                        id,
                        -32601,
                        "Method not found",
                        Some(json!({ "method": method })),
                    ),
                )
            } else {
                response(
                    StatusCode::OK,
                    rpc_error(
                        id,
                        -32601,
                        "Method not found",
                        Some(json!({ "method": method })),
                    ),
                )
            }
        }
    }
}
