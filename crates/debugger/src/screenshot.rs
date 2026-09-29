//! Taking a screenshot of the stage.
//!
//! Reuses the engine's existing snapshot path: the render pass copies the surface into
//! a buffer that is read back once mapped, so native and web produce the same result
//! and no separate canvas reading is needed.

use std::time::Duration;

use moyu_core::base::{Snapshot, SnapshotFormat};
use moyu_core::core::try_get_core;
use moyu_pal::time::{Instant, sleep};

use super::DebugSession;

/// How long to wait for the engine to render the frame carrying the snapshot.
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
/// Gap between checks for the readback result.
const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// A captured frame, encoded as an image file.
pub(super) struct Capture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Capture the next rendered frame.
///
/// The engine only copies the surface while rendering, so this asks for a snapshot and
/// waits for the frame that carries it; a paused engine therefore reports a timeout.
pub(super) async fn capture(
    _session: &DebugSession,
    max_width: Option<u32>,
    max_height: Option<u32>,
    keep_aspect_ratio: bool,
) -> Result<Capture, String> {
    let core = try_get_core().ok_or_else(|| "Engine core is not ready yet".to_string())?;
    let graphics = core
        .graphics()
        .ok_or_else(|| "Graphics are not ready yet".to_string())?;

    if !graphics.request_snapshot() {
        return Err("The rendering surface does not support snapshots".to_string());
    }

    let deadline = Instant::now() + SNAPSHOT_TIMEOUT;

    let snapshot = loop {
        if let Some((data, width, height, stride, format)) = graphics.try_get_snapshot() {
            let format = SnapshotFormat::try_from(format)
                .map_err(|format| format!("Unsupported snapshot format: {format:?}"))?;

            break Snapshot {
                width,
                height,
                data,
                stride,
                format,
            };
        }

        if Instant::now() >= deadline {
            return Err(
                "The engine did not render a frame to capture; it may be paused".to_string(),
            );
        }

        sleep(SNAPSHOT_POLL_INTERVAL).await;
    };

    let snapshot = resize(snapshot, max_width, max_height, keep_aspect_ratio)?;
    let data = snapshot
        .save_to_buffer()
        .map_err(|err| format!("Failed to encode the snapshot: {err}"))?;

    Ok(Capture {
        width: snapshot.width,
        height: snapshot.height,
        data,
    })
}

/// Scale the capture down to fit the requested bounds. The snapshot is left untouched
/// when it already fits, or when no bounds were given.
fn resize(
    mut snapshot: Snapshot,
    max_width: Option<u32>,
    max_height: Option<u32>,
    keep_aspect_ratio: bool,
) -> Result<Snapshot, String> {
    let (Some(max_width), Some(max_height)) = (max_width, max_height) else {
        return Ok(snapshot);
    };

    if snapshot.width <= max_width && snapshot.height <= max_height {
        return Ok(snapshot);
    }

    snapshot
        .resize(max_width, max_height, keep_aspect_ratio)
        .map_err(|err| format!("Failed to resize the snapshot: {err}"))?;

    Ok(snapshot)
}
