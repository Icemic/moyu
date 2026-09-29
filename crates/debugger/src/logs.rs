//! In-memory log buffer.
//!
//! The recorder wrapping the platform logger hands every record it accepts to
//! [`record`], and recording only starts once [`enable`] is called, so an engine that
//! is not being debugged pays nothing for this buffer beyond one atomic load per record.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::{Level, Record};
use moyu_pal::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;

/// Entries kept before the oldest ones are dropped.
const CAPACITY: usize = 2000;

/// Severity of a recorded entry, serialized as a lowercase name.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn of(level: Level) -> Self {
        match level {
            Level::Error => Self::Error,
            Level::Warn => Self::Warn,
            Level::Info => Self::Info,
            Level::Debug => Self::Debug,
            Level::Trace => Self::Trace,
        }
    }

    /// Parse the lowercase name used on the wire, as in `engine:logs` requests.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }
}

/// One recorded log record.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogEntry {
    pub seq: u64,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub timestamp_ms: u64,
}

/// Entries returned by [`snapshot`], together with the cursor for the next call.
pub(crate) struct Snapshot {
    pub entries: Vec<LogEntry>,
    pub next_seq: u64,
    pub dropped: u64,
}

/// Size and loss counters of the buffer.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogStats {
    pub buffered: usize,
    pub dropped: u64,
}

/// Called for every entry recorded while a listener is registered.
pub(crate) type Listener = Arc<dyn Fn(LogEntry) + Send + Sync>;

struct State {
    entries: VecDeque<LogEntry>,
    next_seq: u64,
    dropped: u64,
}

static STATE: Mutex<State> = Mutex::new(State {
    entries: VecDeque::new(),
    next_seq: 0,
    dropped: 0,
});
static LISTENER: Mutex<Option<Listener>> = Mutex::new(None);
/// Whether the bridge keeps records. Off until it starts, so that an engine nobody is
/// debugging never buffers.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Start keeping records. Called by the bridge once a client may connect.
pub(crate) fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Register the listener that receives entries as they are recorded. Pass `None`
/// when the client stops caring about them.
pub(crate) fn set_listener(listener: Option<Listener>) {
    *LISTENER.lock().unwrap() = listener;
}

/// Return recorded entries newer than `since_seq`, optionally limited by severity
/// and count. `next_seq` is the cursor to pass as `since_seq` on the next call: when
/// `limit` cut the result short it points at the last returned entry, so the rest can
/// be fetched next; otherwise it points at the newest recorded entry.
pub(crate) fn snapshot(
    since_seq: Option<u64>,
    level: Option<LogLevel>,
    limit: Option<usize>,
) -> Snapshot {
    let state = STATE.lock().unwrap();

    let entries: Vec<LogEntry> = state
        .entries
        .iter()
        .filter(|entry| since_seq.is_none_or(|since| entry.seq > since))
        .filter(|entry| level.is_none_or(|level| entry.level <= level))
        .take(limit.unwrap_or(usize::MAX))
        .cloned()
        .collect();

    let next_seq = match (limit, entries.last()) {
        (Some(limit), Some(last)) if entries.len() == limit => last.seq,
        _ => state.next_seq,
    };

    Snapshot {
        entries,
        next_seq,
        dropped: state.dropped,
    }
}

/// Size and loss counters of the buffer.
pub(crate) fn stats() -> LogStats {
    let state = STATE.lock().unwrap();

    LogStats {
        buffered: state.entries.len(),
        dropped: state.dropped,
    }
}

/// Record one log record. Called by the recorder installed through
/// [`super::create_logger`]; does nothing until [`enable`].
pub(crate) fn record(record: &Record) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let (entry, listener) = {
        let mut state = STATE.lock().unwrap();

        state.next_seq += 1;

        let entry = LogEntry {
            seq: state.next_seq,
            level: LogLevel::of(record.level()),
            target: record.target().to_string(),
            message: record.args().to_string(),
            timestamp_ms: now_ms(),
        };

        if state.entries.len() == CAPACITY {
            state.entries.pop_front();
            state.dropped += 1;
        }

        state.entries.push_back(entry.clone());

        let listener = LISTENER.lock().unwrap().clone();
        (entry, listener)
    };

    if let Some(listener) = listener {
        listener(entry);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}
