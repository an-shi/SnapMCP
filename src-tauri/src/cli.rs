//! Headless stdio bridge.
//!
//! GUI apps cannot hand their stdio to an already-running instance, so the
//! SnapMCP executable doubles as a tiny stateless proxy: `--mcp-stdio` reads
//! JSON-RPC lines from stdin, forwards them to the local HTTP MCP endpoint,
//! and writes the replies to stdout. It starts the GUI quietly when needed.

use crate::state::RuntimeInfo;
use crate::{diag, paths};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_PORT: u16 = 37621;

pub fn try_run_stdio_bridge() -> bool {
    let args: Vec<String> = std::env::args().collect();
    let enabled = args.iter().any(|a| a == "--mcp-stdio")
        || std::env::var("SNAPMCP_STDIO").ok().as_deref() == Some("1");
    if !enabled {
        return false;
    }
    diag::init();
    run_stdio_bridge();
    true
}

fn read_runtime() -> Option<RuntimeInfo> {
    let bytes = std::fs::read(paths::runtime_file()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn health_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(600))
        .timeout_read(Duration::from_millis(1200))
        .timeout_write(Duration::from_millis(600))
        .build()
}

fn health_ok(agent: &ureq::Agent, port: u16) -> bool {
    agent
        .get(&format!("http://127.0.0.1:{port}/health"))
        .call()
        .map(|r| r.status() == 200)
        .unwrap_or(false)
}

fn resolve_runtime() -> Option<RuntimeInfo> {
    let agent = health_agent();
    let stored = read_runtime();
    if let Some(info) = &stored {
        if info.port != 0 && health_ok(&agent, info.port) {
            return Some(info.clone());
        }
    }
    for port in DEFAULT_PORT..DEFAULT_PORT.saturating_add(20) {
        if health_ok(&agent, port) {
            let mut info = stored.clone().unwrap_or_else(|| RuntimeInfo {
                pid: 0,
                port,
                endpoint: format!("http://127.0.0.1:{port}/mcp"),
                token: String::new(),
                version: String::new(),
            });
            if info.port != port {
                info.port = port;
                info.endpoint = format!("http://127.0.0.1:{port}/mcp");
            }
            return Some(info);
        }
    }
    None
}

fn spawn_app() -> Result<(), String> {
    if std::env::var("SNAPMCP_NO_SPAWN").ok().as_deref() == Some("1") {
        return Err("SNAPMCP_NO_SPAWN is set".into());
    }
    let exe = std::env::current_exe().map_err(|e| format!("Cannot locate SnapMCP.exe: {e}"))?;
    let mut command = Command::new(exe);
    command
        .arg("--hidden")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start SnapMCP: {e}"))
}

fn ensure_runtime() -> Result<RuntimeInfo, String> {
    if let Some(info) = resolve_runtime() {
        return Ok(info);
    }
    spawn_app()?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(250));
        if let Some(info) = resolve_runtime() {
            return Ok(info);
        }
    }
    Err("SnapMCP started but its MCP endpoint did not become ready within 10 seconds".into())
}

fn request_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(360))
        .timeout_write(Duration::from_secs(30))
        .build()
}

fn post_json(agent: &ureq::Agent, info: &RuntimeInfo, raw: &str) -> Result<(u16, String), String> {
    let mut request = agent
        .post(&info.endpoint)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json, text/event-stream");
    if !info.token.is_empty() {
        request = request.set("Authorization", &format!("Bearer {}", info.token));
    }
    match request.send_string(raw) {
        Ok(response) => {
            let status = response.status();
            let body = response.into_string().unwrap_or_default();
            Ok((status, body))
        }
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Ok((status, body))
        }
        Err(err) => Err(format!("Cannot reach the SnapMCP MCP server: {err}")),
    }
}

fn compact_body(body: &str, content_type: &str) -> Option<String> {
    if body.trim().is_empty() {
        return None;
    }
    if content_type.contains("text/event-stream") {
        let mut last = None;
        for line in body.lines() {
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data.starts_with('{') {
                    last = Some(data.to_string());
                }
            }
        }
        return last;
    }
    match serde_json::from_str::<Value>(body) {
        Ok(value) => serde_json::to_string(&value).ok(),
        Err(_) => None,
    }
}

fn error_for(id: Value, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32603, "message": message }
    })
    .to_string()
}

fn run_stdio_bridge() {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    let agent = request_agent();
    let mut runtime = resolve_runtime();
    if runtime.is_none() {
        eprintln!("[snapmcp] Starting the desktop app...");
        match ensure_runtime() {
            Ok(info) => {
                eprintln!("[snapmcp] Connected to MCP endpoint on port {}", info.port);
                runtime = Some(info);
            }
            Err(err) => {
                eprintln!("[snapmcp] {err}");
            }
        }
    } else if let Some(info) = &runtime {
        eprintln!("[snapmcp] Connected to MCP endpoint on port {}", info.port);
    }

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(err) => {
                eprintln!("[snapmcp] stdin error: {err}");
                break;
            }
        }
        let raw = line.trim();
        if raw.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(raw) {
            Ok(value) => value,
            Err(err) => {
                let _ = writeln!(
                    writer,
                    "{}",
                    error_for(Value::Null, &format!("Invalid JSON-RPC input: {err}"))
                );
                let _ = writer.flush();
                continue;
            }
        };
        let id = parsed.get("id").cloned();
        let is_notification = id.is_none();
        let request_id = id.unwrap_or(Value::Null);

        let Some(info) = runtime.clone().or_else(resolve_runtime) else {
            if !is_notification {
                let _ = writeln!(
                    writer,
                    "{}",
                    error_for(
                        request_id,
                        "SnapMCP is not running. Start SnapMCP.exe and try again."
                    )
                );
                let _ = writer.flush();
            }
            continue;
        };
        runtime = Some(info.clone());

        let result = post_json(&agent, &info, raw).or_else(|first_error| {
            // The app may have restarted on another port.
            if let Some(fresh) = resolve_runtime() {
                runtime = Some(fresh.clone());
                post_json(&agent, &fresh, raw)
            } else {
                Err(first_error)
            }
        });

        match result {
            Ok((_status, body)) => {
                if is_notification {
                    continue;
                }
                let compact = compact_body(&body, "application/json");
                match compact {
                    Some(body) => {
                        let _ = writeln!(writer, "{body}");
                    }
                    None => {
                        let _ = writeln!(
                            writer,
                            "{}",
                            error_for(request_id, "SnapMCP returned an empty response")
                        );
                    }
                }
            }
            Err(err) => {
                if !is_notification {
                    let _ = writeln!(writer, "{}", error_for(request_id, &err));
                }
            }
        }
        let _ = writer.flush();
    }
}
