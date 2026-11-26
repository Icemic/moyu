//! Async domain services and shared I/O helpers for `moyu-devtool`.

pub mod android;
pub mod debug;
pub mod engine;
pub mod init;
pub mod mcp;
pub mod node;
pub mod pack;
pub mod project;
pub mod run;
pub mod server;
pub mod video_decoder;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use ring::digest::{Context as Sha256Context, SHA256};

use crate::model::tasks::{TaskEvent, TaskId};

pub type EventSink<T> = async_channel::Sender<T>;

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = ring::digest::digest(&SHA256, data);
    hex_lower(digest.as_ref())
}

pub fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Perform an HTTP GET request with cache-busting headers and return raw bytes.
pub async fn http_get_bytes(url: &str) -> Result<Vec<u8>> {
    let url = url.to_string();
    tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let resp = ureq::get(&url)
            .header("Cache-Control", "no-cache, no-store, max-age=0")
            .header("Pragma", "no-cache")
            .call()
            .with_context(|| format!("HTTP GET failed for {url}"))?;
        let mut reader = resp.into_body().into_reader();
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(buf)
    })
    .await?
}

/// Stream a download to `dest_file` while computing SHA-256 and emitting progress events.
pub async fn download_to_file_verified(
    url: &str,
    dest_file: &Path,
    expected_sha256: &str,
    task_id: Option<TaskId>,
    tx: Option<&EventSink<TaskEvent>>,
) -> Result<()> {
    let url = url.to_string();
    let dest = dest_file.to_path_buf();
    let expected = expected_sha256.to_ascii_lowercase();
    let tx = tx.cloned();

    tokio::task::spawn_blocking(move || -> Result<()> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let resp = ureq::get(&url)
            .header("Cache-Control", "no-cache, no-store, max-age=0")
            .header("Pragma", "no-cache")
            .call()
            .with_context(|| format!("Download failed for {url}"))?;

        let total_bytes = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok());

        let mut reader = resp.into_body().into_reader();
        let mut file = File::create(&dest)?;
        let mut hasher = Sha256Context::new(&SHA256);
        let mut buf = [0u8; 64 * 1024];
        let mut downloaded = 0u64;

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            hasher.update(&buf[..n]);
            downloaded += n as u64;
            if let (Some(id), Some(sender)) = (task_id, tx.as_ref()) {
                let _ = sender.try_send(TaskEvent::Progress {
                    id,
                    downloaded_bytes: downloaded,
                    total_bytes,
                });
            }
        }
        file.flush()?;

        let actual = hex_lower(hasher.finish().as_ref());
        if actual != expected {
            bail!("SHA-256 mismatch!\n  Expected: {expected}\n  Actual:   {actual}");
        }
        Ok(())
    })
    .await?
}

/// Recursively copy a directory tree.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
