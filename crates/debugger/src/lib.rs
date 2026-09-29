//! Runtime debug bridge.
//!
//! When the host injects a debug endpoint at startup, the engine opens an outbound
//! WebSocket connection to it and answers `engine:*` requests over that connection.
//! The bridge is its own crate rather than a Core plugin: it is driven by network
//! events instead of frames, and nothing at all is started when no endpoint is given.
//!
//! The engine only calls [`create_logger`], [`start`] and [`mark_ready`]; everything
//! else in here is an implementation detail. State that only the bridge cares about is
//! kept in this crate, so the rest of the engine at most offers a hook to register into.
//!
//! See `rfcs/2026-09-25-runtime-debug-bridge.md`.

mod logger;
mod logs;
mod nodes;
mod props;
mod protocol;
mod screenshot;
mod state;

pub use logger::create_logger;

#[cfg(native)]
mod native;
#[cfg(web)]
mod web;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use base64::prelude::BASE64_STANDARD as BASE64;
use moyu_pal::config::get_engine_config;
use moyu_pal::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::logs::{LogEntry, LogLevel};
use protocol::{
    EvalDone, EvalOutcome, EvalRequest, Hello, LogPush, LogsDone, LogsRequest, PropsDone,
    PropsRequest, ReadyPush, RequestError, ScreenshotDone, ScreenshotRequest, StateDone, TreeDone,
    TreeRequest,
};
use serde::Serialize;

#[cfg(native)]
use native::eval as platform_eval;
#[cfg(web)]
use web::eval as platform_eval;

/// Budget for one evaluation when the client does not ask for a specific one.
const DEFAULT_EVAL_TIMEOUT: Duration = Duration::from_secs(5);
/// Longest JSON text one evaluation result may produce.
const EVAL_RESULT_LIMIT: usize = 256 * 1024;
/// Id of the root node, which the tree starts from.
const ROOT_NODE_ID: u32 = 0;
/// Levels of children one tree request includes unless it asks for another number.
const DEFAULT_TREE_DEPTH: u32 = 1;

/// Everything the bridge needs to talk to its host.
#[derive(Clone)]
pub(crate) struct DebugSession {
    /// Endpoint the engine connects to, including `sessionId` and `role`.
    pub url: String,
    /// Session this connection belongs to, also announced in `engine:hello`.
    pub session_id: String,
    /// When the bridge started, reported as engine uptime.
    pub started_at: Instant,
}

/// Why an evaluation did not produce a result.
///
/// A page cannot interrupt a running snippet, so only native reports every case here.
#[cfg_attr(web, allow(dead_code))]
pub(crate) enum EvalFailure {
    /// The JavaScript context cannot run the snippet at all.
    Unavailable(String),
    /// The runtime did not run the snippet in time.
    Timeout(Duration),
}

/// How the bridge reaches its host connection.
///
/// Responses must arrive, so they take a path that never drops a message. Pushes only
/// carry a live stream for the client to follow, so they may be dropped when the client
/// cannot keep up: the log buffer and its cursor let the client fetch what it missed.
pub(crate) struct Outgoing {
    send: Box<dyn Fn(String) + Send + Sync>,
    push: Box<dyn Fn(String) + Send + Sync>,
}

impl Outgoing {
    pub(crate) fn new(
        send: impl Fn(String) + Send + Sync + 'static,
        push: impl Fn(String) + Send + Sync + 'static,
    ) -> Self {
        Self {
            send: Box::new(send),
            push: Box::new(push),
        }
    }
}

static OUTGOING: Mutex<Option<Arc<Outgoing>>> = Mutex::new(None);
/// The active session, needed to address messages that the engine sends on its own.
static SESSION: Mutex<Option<DebugSession>> = Mutex::new(None);
static READY: AtomicBool = AtomicBool::new(false);

/// Start the debug bridge when the host injected an endpoint. Called once the engine
/// core exists, so that requests can be answered as soon as the connection is up.
pub fn start() {
    let Some(url) = resolve_url() else {
        return;
    };

    // Record from the start, so that a client sees what happened during startup too.
    logs::enable();

    // Property objects are only observable at the JS boundary, and nodes created by the
    // project script are exactly the ones a client asks about, so recording starts here.
    props::start();

    let session = DebugSession {
        session_id: resolve_session_id(&url),
        url,
        started_at: Instant::now(),
    };

    *SESSION.lock().unwrap() = Some(session.clone());

    #[cfg(native)]
    native::start(session);

    #[cfg(web)]
    web::start(session);
}

/// Report that the engine has finished starting up. Called once, after the project
/// script has run and the first frame has been asked for.
pub fn mark_ready() {
    if READY.swap(true, Ordering::Relaxed) {
        return;
    }

    let session = SESSION.lock().unwrap().clone();

    if let Some(session) = session {
        emit(serialize(&ReadyPush {
            kind: "engine:ready",
            session_id: &session.session_id,
        }));
    }
}

pub(crate) fn is_ready() -> bool {
    READY.load(Ordering::Relaxed)
}

/// Drop the connection-bound state; called by the transports once a connection ends.
pub(crate) fn on_disconnect() {
    set_outgoing(None);
    logs::set_listener(None);
}

/// Register how outgoing messages reach the host, or clear it.
pub(crate) fn set_outgoing(outgoing: Option<Outgoing>) {
    *OUTGOING.lock().unwrap() = outgoing.map(Arc::new);
}

/// Message announcing the bridge to the host.
pub(crate) fn hello(session: &DebugSession) -> String {
    serialize(&Hello {
        kind: "engine:hello",
        session_id: &session.session_id,
        platform: state::platform_name(),
        engine_version: env!("CARGO_PKG_VERSION"),
        entry: get_engine_config().entry.clone().unwrap_or_default(),
        capabilities: vec!["state", "eval", "logs", "tree", "props", "screenshot"],
        ready: is_ready(),
    })
}

/// Answer one request. Returns `None` for payloads that are not requests at all.
pub(crate) async fn handle_request(text: &str, session: &DebugSession) -> Option<String> {
    let request: serde_json::Value = serde_json::from_str(text).ok()?;
    let kind = request.get("type")?.as_str()?.to_string();
    let request_id = request.get("requestId").and_then(serde_json::Value::as_u64);

    match kind.as_str() {
        "engine:state" => Some(state_answer(&kind, request_id, session)),
        "engine:eval" => Some(eval_answer(&kind, request_id, session, request).await),
        "engine:logs" => Some(logs_answer(&kind, request_id, session, request)),
        "engine:tree" => Some(tree_answer(&kind, request_id, session, request)),
        "engine:props" => Some(props_answer(&kind, request_id, session, request)),
        "engine:screenshot" => {
            Some(screenshot_answer(&kind, request_id, session, request).await)
        }
        other => Some(error(
            other,
            request_id,
            session,
            "not_supported",
            format!("Unsupported request type: {other}"),
        )),
    }
}

fn state_answer(kind: &str, request_id: Option<u64>, session: &DebugSession) -> String {
    let Some(request_id) = request_id else {
        return invalid_request(kind, session);
    };

    match state::EngineStateSnapshot::capture(session) {
        Some(snapshot) => serialize(&StateDone {
            kind: "engine:state:done",
            session_id: &session.session_id,
            request_id,
            state: snapshot,
        }),
        None => error(
            kind,
            Some(request_id),
            session,
            "internal",
            "Engine core is not ready yet".to_string(),
        ),
    }
}

/// Capture the stage as an image.
async fn screenshot_answer(
    kind: &str,
    request_id: Option<u64>,
    session: &DebugSession,
    request: serde_json::Value,
) -> String {
    let Some(request_id) = request_id else {
        return invalid_request(kind, session);
    };

    let request = serde_json::from_value::<ScreenshotRequest>(request).unwrap_or_default();

    let capture = match screenshot::capture(
        session,
        request.max_width,
        request.max_height,
        request.keep_aspect_ratio.unwrap_or(true),
    )
    .await
    {
        Ok(capture) => capture,
        Err(reason) => return error(kind, Some(request_id), session, "internal", reason),
    };

    serialize(&ScreenshotDone {
        kind: "engine:screenshot:done",
        session_id: &session.session_id,
        request_id,
        format: "webp",
        width: capture.width,
        height: capture.height,
        data: BASE64.encode(&capture.data),
    })
}

/// Read one level of the node tree, or the root node when no id is given.
fn tree_answer(
    kind: &str,
    request_id: Option<u64>,
    session: &DebugSession,
    request: serde_json::Value,
) -> String {
    let Some(request_id) = request_id else {
        return invalid_request(kind, session);
    };

    let request = serde_json::from_value::<TreeRequest>(request).unwrap_or_default();
    let node_id = request.node_id.unwrap_or(ROOT_NODE_ID);

    let Some(node) = nodes::find(node_id) else {
        return missing_node(kind, request_id, session, node_id);
    };

    props::prune(|node_id| nodes::find(node_id).is_some());

    serialize(&TreeDone {
        kind: "engine:tree:done",
        session_id: &session.session_id,
        request_id,
        node: nodes::tree(&**node.read(), request.depth.unwrap_or(DEFAULT_TREE_DEPTH)),
    })
}

/// Read one node's properties and derived state.
fn props_answer(
    kind: &str,
    request_id: Option<u64>,
    session: &DebugSession,
    request: serde_json::Value,
) -> String {
    let Some(request_id) = request_id else {
        return invalid_request(kind, session);
    };

    let Ok(request) = serde_json::from_value::<PropsRequest>(request) else {
        return error(
            kind,
            Some(request_id),
            session,
            "invalid_request",
            "Request needs a nodeId".to_string(),
        );
    };

    let Some(node) = nodes::find(request.node_id) else {
        return missing_node(kind, request_id, session, request.node_id);
    };

    props::prune(|node_id| nodes::find(node_id).is_some());

    serialize(&PropsDone {
        kind: "engine:props:done",
        session_id: &session.session_id,
        request_id,
        node: nodes::details(&**node.read(), props::get(request.node_id)),
    })
}

fn missing_node(
    kind: &str,
    request_id: u64,
    session: &DebugSession,
    node_id: u32,
) -> String {
    error(
        kind,
        Some(request_id),
        session,
        "not_found",
        format!("No node with id {node_id}"),
    )
}

/// Evaluate a snippet in the runtime's JavaScript context.
async fn eval_answer(
    kind: &str,
    request_id: Option<u64>,
    session: &DebugSession,
    request: serde_json::Value,
) -> String {
    let Some(request_id) = request_id else {
        return invalid_request(kind, session);
    };

    let Ok(request) = serde_json::from_value::<EvalRequest>(request) else {
        return error(
            kind,
            Some(request_id),
            session,
            "invalid_request",
            "Request needs a code string".to_string(),
        );
    };

    let timeout = request
        .timeout_ms
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_EVAL_TIMEOUT);

    let outcome = match platform_eval(&request.code, timeout).await {
        Ok(outcome) => outcome,
        Err(EvalFailure::Timeout(timeout)) => {
            return error(
                kind,
                Some(request_id),
                session,
                "timeout",
                format!("Evaluation did not finish within {}ms", timeout.as_millis()),
            );
        }
        Err(EvalFailure::Unavailable(reason)) => {
            return error(kind, Some(request_id), session, "internal", reason);
        }
    };

    let (repr, value, truncated) = match outcome {
        EvalOutcome::Value { repr, json } => {
            let truncated = json
                .as_ref()
                .is_some_and(|text| text.len() > EVAL_RESULT_LIMIT);

            let value = match json {
                Some(text) if !truncated => serde_json::from_str(&text).ok(),
                _ => None,
            };

            (repr, value, truncated)
        }
        // A throwing snippet is a normal result of the evaluation, not a bridge
        // failure, so it is reported without one of the protocol's codes.
        EvalOutcome::Thrown { message, stack } => {
            return serialize(&RequestError {
                kind: &error_kind(kind),
                session_id: &session.session_id,
                request_id: Some(request_id),
                code: None,
                message,
                stack,
            });
        }
    };

    serialize(&EvalDone {
        kind: "engine:eval:done",
        session_id: &session.session_id,
        request_id,
        repr,
        value,
        truncated,
    })
}

/// Read the engine's log buffer, optionally subscribing to new entries.
fn logs_answer(
    kind: &str,
    request_id: Option<u64>,
    session: &DebugSession,
    request: serde_json::Value,
) -> String {
    let Some(request_id) = request_id else {
        return invalid_request(kind, session);
    };

    let Ok(request) = serde_json::from_value::<LogsRequest>(request) else {
        return error(
            kind,
            Some(request_id),
            session,
            "invalid_request",
            "Invalid logs request".to_string(),
        );
    };

    let level = match request.level.as_deref() {
        Some(name) => match LogLevel::parse(name) {
            Some(level) => Some(level),
            None => {
                return error(
                    kind,
                    Some(request_id),
                    session,
                    "invalid_request",
                    format!("Unknown log level: {name}"),
                );
            }
        },
        None => None,
    };

    match request.subscribe {
        Some(true) => subscribe_logs(&session.session_id, level),
        Some(false) => logs::set_listener(None),
        None => {}
    }

    let snapshot = logs::snapshot(request.since_seq, level, request.limit);

    serialize(&LogsDone {
        kind: "engine:logs:done",
        session_id: &session.session_id,
        request_id,
        entries: snapshot.entries,
        next_seq: snapshot.next_seq,
        dropped: snapshot.dropped,
    })
}

/// Forward every recorded entry to the host while the client is subscribed, applying
/// the same severity filter as the snapshot.
fn subscribe_logs(session_id: &str, level: Option<LogLevel>) {
    let session_id = session_id.to_string();

    logs::set_listener(Some(Arc::new(move |entry: LogEntry| {
        if level.is_some_and(|level| entry.level > level) {
            return;
        }

        push(serialize(&LogPush {
            kind: "engine:log",
            session_id: &session_id,
            entry,
        }));
    })));
}

/// Send one already serialized message that must reach the host.
pub(crate) fn emit(text: String) {
    let outgoing = OUTGOING.lock().unwrap().clone();

    if let Some(outgoing) = outgoing {
        (outgoing.send)(text);
    }
}

/// Send one already serialized message that the host may miss without harm.
pub(crate) fn push(text: String) {
    let outgoing = OUTGOING.lock().unwrap().clone();

    if let Some(outgoing) = outgoing {
        (outgoing.push)(text);
    }
}

fn invalid_request(kind: &str, session: &DebugSession) -> String {
    error(
        kind,
        None,
        session,
        "invalid_request",
        "Request has no requestId".to_string(),
    )
}

fn error(
    kind: &str,
    request_id: Option<u64>,
    session: &DebugSession,
    code: &'static str,
    message: String,
) -> String {
    serialize(&RequestError {
        kind: &error_kind(kind),
        session_id: &session.session_id,
        request_id,
        code: Some(code),
        message,
        stack: None,
    })
}

fn error_kind(kind: &str) -> String {
    if kind.starts_with("engine:") {
        format!("{kind}:error")
    } else {
        "engine:error".to_string()
    }
}

/// The protocol structs are plain data, so serialization only fails on values JSON
/// cannot express; an empty message is ignored by the host instead of breaking it.
fn serialize<T: Serialize>(message: &T) -> String {
    serde_json::to_string(message).unwrap_or_default()
}

/// The endpoint is provided by the host through startup parameters. On native the
/// `MOYU_ENGINE_DEBUG_WS` environment variable allows attaching to an engine that was
/// started by hand, such as a packaged build; on web the page query carries it.
fn resolve_url() -> Option<String> {
    #[cfg(native)]
    {
        url_from_params().or_else(|| {
            std::env::var("MOYU_ENGINE_DEBUG_WS")
                .ok()
                .filter(|url| !url.is_empty())
        })
    }

    #[cfg(web)]
    {
        let search = web_sys::window()?.location().search().ok()?;
        let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
        params.get("engineDebugWsUrl").filter(|url| !url.is_empty())
    }
}

#[cfg(native)]
fn url_from_params() -> Option<String> {
    let params: serde_json::Value = serde_json::from_str(&get_engine_config().params).ok()?;

    params
        .get("engineDebugWsUrl")
        .and_then(|url| url.as_str())
        .filter(|url| !url.is_empty())
        .map(str::to_string)
}

/// The host usually names the session in the endpoint; when it does not, the engine
/// generates one so that clients can still tell concurrent sessions apart.
fn resolve_session_id(url: &str) -> String {
    session_id_from_url(url).unwrap_or_else(generate_session_id)
}

/// Session ids are opaque URL-safe tokens, so the raw query value is used as is.
fn session_id_from_url(url: &str) -> Option<String> {
    let query = url.split_once('?')?.1;

    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;

        (key == "sessionId" && !value.is_empty()).then(|| value.to_string())
    })
}

fn generate_session_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();

    format!("engine-{millis:x}")
}
