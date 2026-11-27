//! Runtime debug bridge WebSocket host and engine session launcher.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, broadcast, mpsc, oneshot, watch};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use crate::config::{DEFAULT_WEB_PORT, DEV_SERVER_PORT, meta_file, platform_dir};
use crate::model::debug::{DebugEvent, LogEntry, SessionMode};
use crate::service::engine::{detect_platform, load_meta, native_executable_name};
use crate::service::project::require_project_root;
use crate::service::server::{
    StaticFileServer, require_web_engine_assets, start_static_file_server,
};

pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 60_000;
pub const DEFAULT_READY_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone)]
pub struct SessionOptions {
    pub project_root: Option<PathBuf>,
    pub web: bool,
    pub port: Option<u16>,
    pub entry: Option<String>,
    pub headless: bool,
    pub attach_port: Option<u16>,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            project_root: None,
            web: false,
            port: None,
            entry: None,
            headless: true,
            attach_port: None,
        }
    }
}

pub struct DebugHost {
    pub port: u16,
    session_id: String,
    connected_rx: watch::Receiver<bool>,
    ready_rx: watch::Receiver<bool>,
    next_request_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
    outbound_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    message_tx: broadcast::Sender<Value>,
    shutdown_tx: watch::Sender<bool>,
    server_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl DebugHost {
    pub async fn wait_for_engine(&self, timeout_ms: u64) -> Result<()> {
        let mut rx = self.connected_rx.clone();
        if *rx.borrow() {
            return Ok(());
        }
        let res = tokio::time::timeout(Duration::from_millis(timeout_ms), async move {
            while !*rx.borrow() {
                if rx.changed().await.is_err() {
                    bail!("Debug host stopped while waiting for engine.");
                }
            }
            Ok::<(), anyhow::Error>(())
        })
        .await;

        match res {
            Ok(inner) => inner,
            Err(_) => bail!(
                "No engine connected within {timeout_ms}ms.\nCheck that the engine is running, and that its version supports the debug bridge."
            ),
        }
    }

    pub async fn wait_for_ready(&self, timeout_ms: u64) -> Result<()> {
        let mut rx = self.ready_rx.clone();
        if *rx.borrow() {
            return Ok(());
        }
        let res = tokio::time::timeout(Duration::from_millis(timeout_ms), async move {
            while !*rx.borrow() {
                if rx.changed().await.is_err() {
                    bail!("Debug host stopped while waiting for engine readiness.");
                }
            }
            Ok::<(), anyhow::Error>(())
        })
        .await;

        match res {
            Ok(inner) => inner,
            Err(_) => bail!("The engine did not finish starting up within {timeout_ms}ms."),
        }
    }

    pub fn subscribe_messages(&self) -> broadcast::Receiver<Value> {
        self.message_tx.subscribe()
    }

    pub async fn request(&self, msg_type: &str, payload: Option<Value>) -> Result<Value> {
        let tx = self
            .outbound_tx
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No engine is connected."))?;

        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (resp_tx, resp_rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id, resp_tx);

        let mut obj = match payload {
            Some(Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        };
        obj.insert("type".to_string(), Value::String(msg_type.to_string()));
        obj.insert("requestId".to_string(), Value::from(request_id));
        obj.insert(
            "sessionId".to_string(),
            Value::String(self.session_id.clone()),
        );

        let wire = serde_json::to_string(&Value::Object(obj))?;
        if tx.send(wire).is_err() {
            self.pending.lock().await.remove(&request_id);
            bail!("No engine is connected.");
        }

        resp_rx
            .await
            .map_err(|_| anyhow::anyhow!("Engine disconnected before answering."))?
    }

    pub fn close(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub async fn close_and_wait(&self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(handle) = self.server_task.lock().await.take() {
            let _ = handle.await;
        }
    }
}

pub struct DebugSession {
    pub host: Arc<DebugHost>,
    pub mode: SessionMode,
    pub headless: bool,
    pub url: Option<String>,
    pub attach_endpoint: Option<String>,
    child: Option<Child>,
    static_server: Option<StaticFileServer>,
}

impl DebugSession {
    pub async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        if let Some(server) = self.static_server.take() {
            server.close();
        }
        self.host.close_and_wait().await;
    }
}

impl Drop for DebugSession {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
        if let Some(server) = self.static_server.take() {
            server.close();
        }
        self.host.close();
    }
}

pub async fn start_debug_session(
    options: SessionOptions,
    event_tx: Option<async_channel::Sender<DebugEvent>>,
) -> Result<DebugSession> {
    if let Some(attach_port) = options.attach_port {
        let session_id = uuid::Uuid::new_v4().to_string();
        let host = start_host(session_id.clone(), attach_port, event_tx.clone()).await?;
        let endpoint = format!(
            "ws://127.0.0.1:{}/debug/ws?sessionId={session_id}&role=engine",
            host.port
        );
        eprintln!(
            "info: Waiting for an engine on port {}. Start it with:",
            host.port
        );
        eprintln!("info:   MOYU_ENGINE_DEBUG_WS={endpoint}");

        if let Some(tx) = &event_tx {
            let _ = tx.try_send(DebugEvent::SessionStarted {
                mode: SessionMode::Attach,
                headless: false,
                port: host.port,
                web_url: None,
                attach_endpoint: Some(endpoint.clone()),
            });
        }

        return Ok(DebugSession {
            host,
            mode: SessionMode::Attach,
            headless: false,
            url: None,
            attach_endpoint: Some(endpoint),
            child: None,
            static_server: None,
        });
    }

    let project_root = match &options.project_root {
        Some(p) => p.clone(),
        None => require_project_root(None)?,
    };

    let meta = load_meta(&meta_file(&project_root));
    let active = meta.and_then(|m| m.active).ok_or_else(|| {
        anyhow::anyhow!("No active engine version. Run \"moyu-devtool download\" first.")
    })?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let host = start_host(session_id.clone(), 0, event_tx.clone()).await?;
    let endpoint = format!(
        "ws://127.0.0.1:{}/debug/ws?sessionId={session_id}&role=engine",
        host.port
    );
    let headless = !options.web && options.headless;

    eprintln!("info: Debug endpoint listening on port {}", host.port);

    if options.web {
        let web_path = require_web_engine_assets(&project_root, &active.version)?;
        let port = options.port.unwrap_or(DEFAULT_WEB_PORT);
        let server = match start_static_file_server(&project_root, &web_path, port).await {
            Ok(s) => s,
            Err(e) => {
                host.close();
                return Err(e);
            }
        };

        let entry_url = format!("{}/index.json", server.url);
        let query = url_encode_query(&[
            ("entry", &entry_url),
            ("engineDebugWsUrl", &endpoint),
            ("engineDebugSessionId", &session_id),
        ]);
        let page_url = format!("{}/?{query}", server.url);

        eprintln!("info: Web engine server running at {}", server.url);
        eprintln!("info: Open this page to start the engine: {page_url}");

        if let Some(tx) = &event_tx {
            let _ = tx.try_send(DebugEvent::SessionStarted {
                mode: SessionMode::Web,
                headless: false,
                port: host.port,
                web_url: Some(page_url.clone()),
                attach_endpoint: None,
            });
        }

        return Ok(DebugSession {
            host,
            mode: SessionMode::Web,
            headless: false,
            url: Some(page_url),
            attach_endpoint: None,
            child: None,
            static_server: Some(server),
        });
    }

    let platform = detect_platform()?;
    let exe_name = native_executable_name(platform)
        .ok_or_else(|| anyhow::anyhow!("Unsupported native platform: {platform}"))?;
    let engine_path = platform_dir(&project_root, &active.version, platform)?.join(exe_name);
    if !engine_path.is_file() {
        host.close();
        bail!(
            "Engine binary not found at {}\nThe engine files may be corrupted. Run \"moyu-devtool download\" to re-download.",
            engine_path.display()
        );
    }

    let entry = options
        .entry
        .unwrap_or_else(|| format!("http://localhost:{DEV_SERVER_PORT}/index.json"));
    let params = serde_json::json!({
        "engineDebugWsUrl": endpoint,
        "engineDebugSessionId": session_id,
    })
    .to_string();

    eprintln!("info: Starting native engine: {}", engine_path.display());

    let mut cmd = Command::new(&engine_path);
    cmd.args(["--entry", &entry, "--params", &params])
        .current_dir(&project_root)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if headless {
        cmd.arg("--headless");
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            host.close();
            return Err(e.into());
        }
    };

    // Forward engine stdout to our stderr so MCP stdout stays protocol-pure.
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            let mut stderr = tokio::io::stderr();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = stderr.write_all(line.as_bytes()).await;
                let _ = stderr.write_all(b"\n").await;
            }
        });
    }

    if let Some(tx) = &event_tx {
        let _ = tx.try_send(DebugEvent::SessionStarted {
            mode: SessionMode::Native,
            headless,
            port: host.port,
            web_url: None,
            attach_endpoint: None,
        });
    }

    Ok(DebugSession {
        host,
        mode: SessionMode::Native,
        headless,
        url: None,
        attach_endpoint: None,
        child: Some(child),
        static_server: None,
    })
}

async fn start_host(
    session_id: String,
    requested_port: u16,
    event_tx: Option<async_channel::Sender<DebugEvent>>,
) -> Result<Arc<DebugHost>> {
    let listener = TcpListener::bind(("127.0.0.1", requested_port)).await?;
    let port = listener.local_addr()?.port();

    let (connected_tx, connected_rx) = watch::channel(false);
    let (ready_tx, ready_rx) = watch::channel(false);
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let (message_tx, _) = broadcast::channel(512);

    let host = Arc::new(DebugHost {
        port,
        session_id,
        connected_rx,
        ready_rx,
        next_request_id: AtomicU64::new(1),
        pending: Arc::new(Mutex::new(HashMap::new())),
        outbound_tx: Mutex::new(None),
        message_tx,
        shutdown_tx,
        server_task: Mutex::new(None),
    });

    let host_clone = Arc::clone(&host);
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                accept_res = listener.accept() => {
                    let Ok((stream, _)) = accept_res else {
                        break;
                    };
                    let ws_stream = tokio::select! {
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                break;
                            }
                            continue;
                        }
                        res = accept_async(stream) => match res {
                            Ok(s) => s,
                            Err(_) => continue,
                        },
                    };
                    let (mut ws_write, mut ws_read) = ws_stream.split();
                    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
                    *host_clone.outbound_tx.lock().await = Some(out_tx);

                    let write_task = tokio::spawn(async move {
                        while let Some(text) = out_rx.recv().await {
                            if ws_write.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                    });

                    let mut should_stop = false;
                    loop {
                        let next_msg = tokio::select! {
                            _ = shutdown_rx.changed() => {
                                if *shutdown_rx.borrow() {
                                    should_stop = true;
                                    break;
                                }
                                continue;
                            }
                            msg = ws_read.next() => msg,
                        };
                        let Some(Ok(msg)) = next_msg else {
                            break;
                        };
                        let text = match msg {
                            Message::Text(t) => t.to_string(),
                            Message::Close(_) => break,
                            _ => continue,
                        };
                        let Ok(val) = serde_json::from_str::<Value>(&text) else {
                            eprintln!("warn: Ignoring malformed message: {text}");
                            continue;
                        };
                        let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if msg_type == "engine:hello" {
                            let platform = val.get("platform").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let ver = val.get("engineVersion").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let entry = val.get("entry").and_then(|v| v.as_str()).unwrap_or("");
                            eprintln!("info: Engine connected: {platform} {ver} (entry {entry})");
                            let _ = connected_tx.send(true);
                            if let Some(tx) = &event_tx {
                                let _ = tx.try_send(DebugEvent::EngineConnected);
                            }
                            if val.get("ready").and_then(|v| v.as_bool()) == Some(true) {
                                let _ = ready_tx.send(true);
                                if let Some(tx) = &event_tx {
                                    let _ = tx.try_send(DebugEvent::EngineReady);
                                }
                            }
                            continue;
                        }
                        if msg_type == "engine:ready" {
                            let _ = ready_tx.send(true);
                            if let Some(tx) = &event_tx {
                                let _ = tx.try_send(DebugEvent::EngineReady);
                            }
                            continue;
                        }

                        if msg_type == "engine:log"
                            && let Some(tx) = &event_tx
                            && let Some(entry_val) = val.get("entry")
                            && let Ok(entry) = serde_json::from_value::<LogEntry>(entry_val.clone())
                        {
                            let _ = tx.try_send(DebugEvent::LogReceived(entry));
                        }

                        let _ = host_clone.message_tx.send(val.clone());

                        if let Some(req_id) = val.get("requestId").and_then(|v| v.as_u64())
                            && let Some(waiter) = host_clone.pending.lock().await.remove(&req_id)
                        {
                            if msg_type.ends_with(":error") {
                                let message = val
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error");
                                let stack = val
                                    .get("stack")
                                    .and_then(|v| v.as_str())
                                    .map(|s| format!("\n{s}"))
                                    .unwrap_or_default();
                                let _ = waiter.send(Err(anyhow::anyhow!("{message}{stack}")));
                            } else {
                                let _ = waiter.send(Ok(val));
                            }
                        }
                    }

                    write_task.abort();
                    *host_clone.outbound_tx.lock().await = None;
                    let mut pending = host_clone.pending.lock().await;
                    for (_, waiter) in pending.drain() {
                        let _ = waiter.send(Err(anyhow::anyhow!("Engine disconnected before answering.")));
                    }
                    if should_stop {
                        break;
                    }
                }
            }
        }
    });

    *host.server_task.lock().await = Some(handle);
    Ok(host)
}

fn url_encode_query(pairs: &[(&str, &str)]) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                utf8_percent_encode(k, NON_ALPHANUMERIC),
                utf8_percent_encode(v, NON_ALPHANUMERIC)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}
