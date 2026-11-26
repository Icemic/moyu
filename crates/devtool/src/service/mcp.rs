//! Model Context Protocol (MCP) JSON-RPC 2.0 server over stdio.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc};

use crate::config::DEFAULT_ATTACH_PORT;
use crate::model::debug::NodeSummary;
use crate::service::debug::{
    DEFAULT_CONNECT_TIMEOUT_MS, DebugSession, SessionOptions, start_debug_session,
};
use crate::service::project::find_project_root;

const SUPPORTED_VERSIONS: &[&str] = &["2026-07-28", "2025-06-18", "2025-03-26", "2024-11-05"];
const START_READY_TIMEOUT_MS: u64 = 60_000;
const READY_TIMEOUT_MS: u64 = 30_000;

struct McpState {
    session: Mutex<Option<DebugSession>>,
}

impl McpState {
    async fn stop_session(&self) -> bool {
        let mut guard = self.session.lock().await;
        if let Some(mut s) = guard.take() {
            s.stop().await;
            true
        } else {
            false
        }
    }

    async fn request(&self, msg_type: &str, payload: Option<Value>) -> anyhow::Result<Value> {
        let host = {
            let guard = self.session.lock().await;
            guard
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No session is running. Call debug_start first."))?
                .host
                .clone()
        };
        host.wait_for_ready(READY_TIMEOUT_MS).await?;
        host.request(msg_type, payload).await
    }
}

pub async fn serve_mcp() -> anyhow::Result<()> {
    eprintln!("info: Moyu MCP server ready on stdio.");

    let state = Arc::new(McpState {
        session: Mutex::new(None),
    });

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    let pending = Arc::new(AtomicUsize::new(0));

    let writer_handle = tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(line) = out_rx.recv().await {
            let _ = stdout.write_all(line.as_bytes()).await;
            let _ = stdout.write_all(b"\n").await;
            let _ = stdout.flush().await;
        }
    });

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        tokio::select! {
            line_res = lines.next_line() => {
                match line_res {
                    Ok(Some(line)) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
                            eprintln!("warn: Ignoring unreadable message: {trimmed}");
                            continue;
                        };
                        let state = Arc::clone(&state);
                        let out_tx = out_tx.clone();
                        let pending = Arc::clone(&pending);
                        pending.fetch_add(1, Ordering::SeqCst);
                        tokio::spawn(async move {
                            handle_request(req, &state, &out_tx).await;
                            pending.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    while pending.load(Ordering::SeqCst) > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    state.stop_session().await;
    drop(out_tx);
    let _ = writer_handle.await;
    Ok(())
}

async fn handle_request(
    req: Value,
    state: &Arc<McpState>,
    out_tx: &mpsc::UnboundedSender<String>,
) {
    let id = req.get("id").cloned().filter(|v| !v.is_null());
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

    let send_result = |result: Value| {
        if let Some(ref req_id) = id {
            let msg = json!({ "jsonrpc": "2.0", "id": req_id, "result": result });
            let _ = out_tx.send(msg.to_string());
        }
    };
    let send_error = |code: i64, message: &str| {
        if let Some(ref req_id) = id {
            let msg = json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": { "code": code, "message": message }
            });
            let _ = out_tx.send(msg.to_string());
        }
    };

    let info = json!({ "name": "moyu", "version": env!("CARGO_PKG_VERSION") });

    match method {
        "initialize" => {
            let proto = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or(SUPPORTED_VERSIONS[0]);
            send_result(json!({
                "protocolVersion": proto,
                "capabilities": { "tools": {} },
                "serverInfo": info,
            }));
        }
        "server/discover" => {
            send_result(json!({
                "resultType": "complete",
                "supportedVersions": SUPPORTED_VERSIONS,
                "capabilities": { "tools": {} },
                "_meta": { "io.modelcontextprotocol/serverInfo": info },
            }));
        }
        "tools/list" => {
            send_result(json!({ "tools": tool_definitions() }));
        }
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !is_known_tool(name) {
                send_error(-32602, &format!("Unknown tool: {name}"));
                return;
            }
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            match call_tool(name, args, state).await {
                Ok(res) => send_result(res),
                Err(err) => send_result(json!({
                    "content": [{ "type": "text", "text": err.to_string() }],
                    "isError": true
                })),
            }
        }
        "ping" => send_result(json!({})),
        "notifications/initialized" | "notifications/cancelled" => {}
        _ => {
            if id.is_some() {
                send_error(-32601, &format!("Method not found: {method}"));
            }
        }
    }
}

fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "debug_start"
            | "debug_stop"
            | "debug_state"
            | "debug_eval"
            | "debug_logs"
            | "debug_tree"
            | "debug_props"
            | "debug_screenshot"
            | "debug_mouse"
            | "debug_touch"
            | "debug_key"
    )
}

async fn call_tool(name: &str, args: Value, state: &Arc<McpState>) -> anyhow::Result<Value> {
    match name {
        "debug_start" => {
            state.stop_session().await;

            if args.get("attach").and_then(|v| v.as_bool()) == Some(true) {
                let listen = args
                    .get("listen")
                    .and_then(|v| v.as_u64())
                    .map(|p| p as u16)
                    .unwrap_or(DEFAULT_ATTACH_PORT);
                let session = start_debug_session(
                    SessionOptions {
                        attach_port: Some(listen),
                        ..Default::default()
                    },
                    None,
                )
                .await?;
                let port = session.host.port;
                let endpoint = session.attach_endpoint.clone().unwrap_or_default();
                *state.session.lock().await = Some(session);

                return Ok(text_result(&format!(
                    "Listening for an engine on port {port} (attach).\nStart the engine yourself and point it at this endpoint:\n  MOYU_ENGINE_DEBUG_WS={endpoint}\nThen call any other debug tool to work with it."
                )));
            }

            let project_root = match args.get("projectRoot").and_then(|v| v.as_str()) {
                Some(p) => PathBuf::from(p),
                None => find_project_root(None).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Could not find a Moyu project (no index.json in this directory or above it). Pass projectRoot to point at one."
                    )
                })?,
            };
            let web = args.get("web").and_then(|v| v.as_bool()) == Some(true);
            let port = args.get("port").and_then(|v| v.as_u64()).map(|p| p as u16);
            let entry = args
                .get("entry")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let headless = args.get("headless").and_then(|v| v.as_bool()) != Some(false);

            let session = start_debug_session(
                SessionOptions {
                    project_root: Some(project_root),
                    web,
                    port,
                    entry,
                    headless,
                    attach_port: None,
                },
                None,
            )
            .await?;

            session
                .host
                .wait_for_engine(DEFAULT_CONNECT_TIMEOUT_MS)
                .await?;
            let ready = session
                .host
                .wait_for_ready(START_READY_TIMEOUT_MS)
                .await
                .is_ok();

            let mode_str = match session.mode {
                crate::model::debug::SessionMode::Native => "native",
                crate::model::debug::SessionMode::Web => "web",
                crate::model::debug::SessionMode::Attach => "attach",
            };
            let mut lines = vec![format!(
                "Engine started ({mode_str}{}).",
                if session.headless { ", headless" } else { "" }
            )];
            if let Some(url) = &session.url {
                lines.push(format!("Open this page to run it: {url}"));
            }
            if !ready {
                lines.push(
                    "The engine has not finished starting up yet; engine logs may explain why."
                        .to_string(),
                );
            }

            *state.session.lock().await = Some(session);
            Ok(text_result(&lines.join("\n")))
        }
        "debug_stop" => {
            let had = state.stop_session().await;
            Ok(text_result(if had {
                "Session stopped."
            } else {
                "No session was running."
            }))
        }
        "debug_state" => {
            let res = state.request("engine:state", None).await?;
            Ok(text_result(&serde_json::to_string_pretty(&res)?))
        }
        "debug_eval" => {
            let code = args
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("code is required"))?;
            let timeout_ms = args.get("timeoutMs").and_then(|v| v.as_u64());
            let mut payload = serde_json::Map::new();
            payload.insert("code".to_string(), json!(code));
            if let Some(ms) = timeout_ms {
                payload.insert("timeoutMs".to_string(), json!(ms));
            }
            let res = state
                .request("engine:eval", Some(Value::Object(payload)))
                .await?;
            let value_str = match res.get("value") {
                Some(v) if !v.is_null() => serde_json::to_string_pretty(v)?,
                _ => res
                    .get("repr")
                    .and_then(|v| v.as_str())
                    .unwrap_or("undefined")
                    .to_string(),
            };
            let note = if res.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
                "\n(result exceeded the size limit and is only shown in short form)"
            } else {
                ""
            };
            Ok(text_result(&format!("{value_str}{note}")))
        }
        "debug_logs" => {
            let mut payload = serde_json::Map::new();
            if let Some(lvl) = args.get("level").and_then(|v| v.as_str()) {
                payload.insert("level".to_string(), json!(lvl));
            }
            if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
                payload.insert("limit".to_string(), json!(limit));
            }
            if let Some(since) = args.get("sinceSeq").and_then(|v| v.as_u64()) {
                payload.insert("sinceSeq".to_string(), json!(since));
            }
            let res = state
                .request("engine:logs", Some(Value::Object(payload)))
                .await?;
            let mut lines = Vec::new();
            if let Some(entries) = res.get("entries").and_then(|v| v.as_array()) {
                for entry in entries {
                    let ts = entry.get("timestampMs").and_then(|v| v.as_u64()).unwrap_or(0);
                    let lvl = entry
                        .get("level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("INFO")
                        .to_ascii_uppercase();
                    let target = entry.get("target").and_then(|v| v.as_str()).unwrap_or("");
                    let msg = entry.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    lines.push(format!("{} {lvl} {target}: {msg}", format_hms_ms(ts)));
                }
            }
            let next_seq = res.get("nextSeq").and_then(|v| v.as_u64()).unwrap_or(0);
            let dropped = res.get("dropped").and_then(|v| v.as_u64()).unwrap_or(0);
            let dropped_note = if dropped > 0 {
                format!(" (dropped {dropped})")
            } else {
                String::new()
            };
            lines.push(format!("nextSeq: {next_seq}{dropped_note}"));
            Ok(text_result(&lines.join("\n")))
        }
        "debug_tree" => {
            let mut payload = serde_json::Map::new();
            if let Some(id) = args.get("nodeId").and_then(|v| v.as_u64()) {
                payload.insert("nodeId".to_string(), json!(id));
            }
            if let Some(depth) = args.get("depth").and_then(|v| v.as_u64()) {
                payload.insert("depth".to_string(), json!(depth));
            }
            let res = state
                .request("engine:tree", Some(Value::Object(payload)))
                .await?;
            let node: NodeSummary = serde_json::from_value(
                res.get("node")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing node in engine:tree response"))?,
            )?;
            let mut lines = Vec::new();
            collect_node_lines(&node, 0, &mut lines);
            Ok(text_result(&lines.join("\n")))
        }
        "debug_props" => {
            let node_id = args
                .get("nodeId")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("nodeId is required"))?;
            let res = state
                .request("engine:props", Some(json!({ "nodeId": node_id })))
                .await?;
            Ok(text_result(&serde_json::to_string_pretty(&res)?))
        }
        "debug_screenshot" => {
            let mut payload = serde_json::Map::new();
            if let Some(w) = args.get("maxWidth").and_then(|v| v.as_u64()) {
                payload.insert("maxWidth".to_string(), json!(w));
            }
            if let Some(h) = args.get("maxHeight").and_then(|v| v.as_u64()) {
                payload.insert("maxHeight".to_string(), json!(h));
            }
            let res = state
                .request("engine:screenshot", Some(Value::Object(payload)))
                .await?;
            let width = res.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
            let height = res.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
            let format = res.get("format").and_then(|v| v.as_str()).unwrap_or("webp");
            let data = res.get("data").and_then(|v| v.as_str()).unwrap_or("");
            Ok(json!({
                "content": [
                    { "type": "image", "data": data, "mimeType": format!("image/{format}") },
                    { "type": "text", "text": format!("{width}x{height} {format}") }
                ]
            }))
        }
        "debug_mouse" => {
            let res = state.request("engine:mouse", Some(args)).await?;
            Ok(text_result(&serde_json::to_string_pretty(&res)?))
        }
        "debug_touch" => {
            let res = state.request("engine:touch", Some(args)).await?;
            Ok(text_result(&serde_json::to_string_pretty(&res)?))
        }
        "debug_key" => {
            let res = state.request("engine:key", Some(args)).await?;
            Ok(text_result(&serde_json::to_string_pretty(&res)?))
        }
        _ => anyhow::bail!("Unknown tool: {name}"),
    }
}

pub fn format_hms_ms(ts_ms: u64) -> String {
    let dt = chrono::DateTime::from_timestamp_millis(ts_ms as i64).unwrap_or_default();
    dt.format("%H:%M:%S%.3f").to_string()
}

pub fn collect_node_lines(node: &NodeSummary, indent: usize, out: &mut Vec<String>) {
    let label = if node.label.is_empty() {
        String::new()
    } else {
        format!(" {}", serde_json::to_string(&node.label).unwrap_or_default())
    };
    let hidden = if node.visible { "" } else { " (hidden)" };
    out.push(format!(
        "{}#{} {}{}{}",
        "  ".repeat(indent),
        node.id,
        node.node_type,
        label,
        hidden
    ));
    for child in &node.children {
        collect_node_lines(child, indent + 1, out);
    }
}

fn text_result(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "debug_start",
            "title": "Start a debug session",
            "description": "Start the project engine with the debug bridge and wait until it is running. Call this before any other moyu tool. Starting a session stops the previous one.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "web": { "type": "boolean", "default": false },
                    "port": { "type": "number" },
                    "entry": { "type": "string" },
                    "headless": { "type": "boolean", "default": true },
                    "projectRoot": { "type": "string" },
                    "attach": { "type": "boolean", "default": false },
                    "listen": { "type": "number", "default": DEFAULT_ATTACH_PORT }
                }
            }
        },
        {
            "name": "debug_stop",
            "title": "Stop the debug session",
            "description": "Stop the debug session started by debug_start.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "debug_state",
            "title": "Read engine state",
            "description": "Read the engine entry file, platform, version, surface size, node count, uptime, and whether it has finished starting up.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "debug_eval",
            "title": "Evaluate JavaScript",
            "description": "Run JavaScript in the engine and return the result.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "timeoutMs": { "type": "number", "default": 5000 }
                },
                "required": ["code"]
            }
        },
        {
            "name": "debug_logs",
            "title": "Read engine logs",
            "description": "Read buffered engine logs, oldest first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "level": { "type": "string", "enum": ["error", "warn", "info", "debug", "trace"] },
                    "limit": { "type": "number" },
                    "sinceSeq": { "type": "number" }
                }
            }
        },
        {
            "name": "debug_tree",
            "title": "Read the node tree",
            "description": "List nodes and their children, starting from the root unless a nodeId is given.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "nodeId": { "type": "number" },
                    "depth": { "type": "number", "default": 4 }
                }
            }
        },
        {
            "name": "debug_props",
            "title": "Read node properties",
            "description": "Read what JavaScript asked of one node, next to the values the engine derived from it.",
            "inputSchema": {
                "type": "object",
                "properties": { "nodeId": { "type": "number" } },
                "required": ["nodeId"]
            }
        },
        {
            "name": "debug_screenshot",
            "title": "Capture the stage",
            "description": "Capture the next rendered frame as an image.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "maxWidth": { "type": "number" },
                    "maxHeight": { "type": "number" }
                }
            }
        },
        {
            "name": "debug_mouse",
            "title": "Send a mouse action",
            "description": "Simulate a mouse action (move, down, up, click or wheel) at a stage position or at a node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["move", "down", "up", "click", "wheel"] },
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "nodeId": { "type": "number" },
                    "button": { "type": "string", "enum": ["left", "right", "middle", "back", "forward"] },
                    "deltaX": { "type": "number" },
                    "deltaY": { "type": "number" },
                    "mode": { "type": "string", "enum": ["line", "pixel"] }
                },
                "required": ["action"]
            }
        },
        {
            "name": "debug_touch",
            "title": "Send a touch phase",
            "description": "Simulate one touch phase (start, move, end or cancel).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["start", "move", "end", "cancel"] },
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "nodeId": { "type": "number" },
                    "identifier": { "type": "number" }
                },
                "required": ["action"]
            }
        },
        {
            "name": "debug_key",
            "title": "Send a keyboard event",
            "description": "Simulate a keyboard event (down, up or press).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["down", "up", "press"] },
                    "key": { "type": "string" },
                    "code": { "type": "string" },
                    "repeat": { "type": "boolean" },
                    "ctrlKey": { "type": "boolean" },
                    "shiftKey": { "type": "boolean" },
                    "altKey": { "type": "boolean" },
                    "metaKey": { "type": "boolean" }
                },
                "required": ["action", "key"]
            }
        }
    ])
}
