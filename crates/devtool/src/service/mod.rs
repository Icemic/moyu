//! Async domain services, cancellation token, diagnostic reporter, and shared I/O helpers.

pub mod android;
pub mod debug;
pub mod dev_server;
pub mod editor;
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
use tokio::sync::watch;

use crate::model::tasks::{LogLevel, TaskEvent, TaskId, TaskKind, TaskStatus};

pub type EventSink<T> = async_channel::Sender<T>;

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

/// Lightweight, executor-agnostic cooperative cancellation token.
#[derive(Clone)]
pub struct CancelToken {
    tx: watch::Sender<bool>,
    rx: watch::Receiver<bool>,
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancelToken {
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self { tx, rx }
    }

    #[allow(dead_code)]
    pub fn cancel(&self) {
        let _ = self.tx.send(true);
    }

    pub fn is_cancelled(&self) -> bool {
        *self.rx.borrow()
    }

    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            bail!("Operation cancelled.");
        }
        Ok(())
    }

    pub async fn cancelled(&self) {
        let mut rx = self.rx.clone();
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                break;
            }
        }
    }
}

/// Structured reporter passed into async services to emit task progress, logs, and handle cancellation.
#[derive(Clone)]
pub struct ServiceReporter {
    pub task_id: TaskId,
    pub tx: Option<EventSink<TaskEvent>>,
    pub cancel: CancelToken,
}

impl Default for ServiceReporter {
    fn default() -> Self {
        Self::noop()
    }
}

impl ServiceReporter {
    pub fn noop() -> Self {
        Self {
            task_id: next_task_id(),
            tx: None,
            cancel: CancelToken::new(),
        }
    }

    pub fn with_channel(task_id: TaskId, tx: EventSink<TaskEvent>, cancel: CancelToken) -> Self {
        Self {
            task_id,
            tx: Some(tx),
            cancel,
        }
    }

    pub fn child(&self) -> Self {
        Self {
            task_id: next_task_id(),
            tx: self.tx.clone(),
            cancel: self.cancel.clone(),
        }
    }

    pub fn started(&self, kind: TaskKind, title: impl Into<String>) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(TaskEvent::Started {
                id: self.task_id,
                kind,
                title: title.into(),
            });
        }
    }

    pub fn stage(&self, stage: impl Into<String>) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(TaskEvent::Stage {
                id: self.task_id,
                stage: stage.into(),
            });
        }
    }

    pub fn progress(&self, downloaded_bytes: u64, total_bytes: Option<u64>) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(TaskEvent::Progress {
                id: self.task_id,
                downloaded_bytes,
                total_bytes,
            });
        }
    }

    pub fn log(&self, level: LogLevel, line: impl Into<String>) {
        let line = line.into();
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(TaskEvent::LogLine {
                id: self.task_id,
                level,
                line,
            });
        }
    }

    pub fn info(&self, line: impl Into<String>) {
        self.log(LogLevel::Info, line);
    }

    pub fn success(&self, line: impl Into<String>) {
        self.log(LogLevel::Success, line);
    }

    pub fn warn(&self, line: impl Into<String>) {
        self.log(LogLevel::Warn, line);
    }

    pub fn finished(&self, status: TaskStatus) {
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(TaskEvent::Finished {
                id: self.task_id,
                status,
            });
        }
    }
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

#[allow(dead_code)]
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

/// Stream a download to `dest_file`, emitting progress and respecting cancellation (no checksum check).
pub async fn download_to_file(
    url: &str,
    dest_file: &Path,
    reporter: &ServiceReporter,
) -> Result<()> {
    download_to_file_impl(url, dest_file, None, reporter).await
}

/// Stream a download to `dest_file` while computing SHA-256, emitting progress, and respecting cancellation.
pub async fn download_to_file_verified(
    url: &str,
    dest_file: &Path,
    expected_sha256: &str,
    reporter: &ServiceReporter,
) -> Result<()> {
    download_to_file_impl(
        url,
        dest_file,
        Some(expected_sha256.to_ascii_lowercase()),
        reporter,
    )
    .await
}

async fn download_to_file_impl(
    url: &str,
    dest_file: &Path,
    expected_sha256: Option<String>,
    reporter: &ServiceReporter,
) -> Result<()> {
    reporter.cancel.check()?;
    let url = url.to_string();
    let dest = dest_file.to_path_buf();
    let reporter = reporter.clone();

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
        let mut hasher = expected_sha256
            .as_ref()
            .map(|_| Sha256Context::new(&SHA256));
        let mut buf = [0u8; 64 * 1024];
        let mut downloaded = 0u64;

        loop {
            if reporter.cancel.is_cancelled() {
                drop(file);
                let _ = fs::remove_file(&dest);
                bail!("Download cancelled.");
            }
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            if let Some(h) = &mut hasher {
                h.update(&buf[..n]);
            }
            downloaded += n as u64;
            reporter.progress(downloaded, total_bytes);
        }
        file.flush()?;

        if let (Some(expected), Some(h)) = (expected_sha256, hasher) {
            let actual = hex_lower(h.finish().as_ref());
            if actual != expected {
                let _ = fs::remove_file(&dest);
                bail!("SHA-256 mismatch!\n  Expected: {expected}\n  Actual:   {actual}");
            }
        }
        Ok(())
    })
    .await?
}

/// Recursively copy a directory tree, following directory symlinks.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if fs::metadata(&path)?.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Configure a `std::process::Command` so it does not flash a console window on Windows.
pub fn configure_no_window_std(cmd: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Configure a `tokio::process::Command` so it does not flash a console window on Windows.
pub fn configure_no_window_tokio(
    cmd: &mut tokio::process::Command,
) -> &mut tokio::process::Command {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Kill an entire child process tree synchronously on Windows (`taskkill /PID <pid> /T /F`)
/// so nested shells (`cmd.exe` -> `node`/`java`/`gradlew`) do not leak orphan processes.
pub fn kill_process_tree_sync(pid: u32) {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        configure_no_window_std(&mut cmd);
        let _ = cmd.status();
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
    }
}
