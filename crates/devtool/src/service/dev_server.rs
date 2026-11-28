//! Project dev server process manager (supports rspack, Vite, custom ports, and process-tree cleanup).

use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};

use crate::config::DEV_SERVER_PORT;
use crate::model::tasks::LogLevel;
use crate::service::node::{
    build_augmented_path, detect_package_manager, ensure_package_manager, require_node,
};
use crate::service::{ServiceReporter, configure_no_window_tokio, kill_process_tree_sync};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DevServerStatus {
    Stopped,
    Starting,
    Running { port: u16 },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DevServerEvent {
    Log { level: LogLevel, line: String },
    Ready { port: u16 },
    Exited { code: Option<i32> },
}

#[allow(dead_code)]
pub struct DevServerHandle {
    pid: Option<u32>,
    kill_tx: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
    port_rx: watch::Receiver<Option<u16>>,
}

#[allow(dead_code)]
impl DevServerHandle {
    pub fn current_port(&self) -> Option<u16> {
        *self.port_rx.borrow()
    }

    pub async fn wait_ready(&self, timeout_ms: u64) -> Result<u16> {
        let mut rx = self.port_rx.clone();
        if let Some(port) = *rx.borrow() {
            return Ok(port);
        }
        let res = tokio::time::timeout(Duration::from_millis(timeout_ms), async move {
            loop {
                if let Some(port) = *rx.borrow() {
                    return Ok(port);
                }
                if rx.changed().await.is_err() {
                    bail!("Dev server exited before becoming ready. Check the task output for details.");
                }
            }
        })
        .await;

        match res {
            Ok(inner) => inner,
            Err(_) => bail!(
                "Dev server did not become ready within {}s. Check the task output for details.",
                timeout_ms / 1000
            ),
        }
    }

    pub async fn stop(&mut self) {
        if let Some(tx) = self.kill_tx.take() {
            let _ = tx.send(());
        }
        if let Some(pid) = self.pid.take() {
            let _ = tokio::task::spawn_blocking(move || kill_process_tree_sync(pid)).await;
        }
        if let Some(handle) = self.task.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for DevServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.kill_tx.take() {
            let _ = tx.send(());
        }
        if let Some(pid) = self.pid.take() {
            kill_process_tree_sync(pid);
        }
    }
}

/// Check whether a local TCP server is currently accepting connections on `127.0.0.1:<port>` or `[::1]:<port>`.
#[allow(dead_code)]
pub async fn is_port_listening(port: u16) -> bool {
    if tokio::time::timeout(
        Duration::from_millis(200),
        TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .is_ok_and(|res| res.is_ok())
    {
        return true;
    }
    tokio::time::timeout(
        Duration::from_millis(200),
        TcpStream::connect(("::1", port)),
    )
    .await
    .is_ok_and(|res| res.is_ok())
}

/// Check that `port` is genuinely free on IPv4 (`0.0.0.0` / `127.0.0.1`) and IPv6 (`[::]` / `[::1]`).
fn is_port_free(port: u16) -> bool {
    let v4_loopback = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    if std::net::TcpStream::connect_timeout(&v4_loopback, Duration::from_millis(80)).is_ok() {
        return false;
    }
    let v6_loopback = std::net::SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port));
    if std::net::TcpStream::connect_timeout(&v6_loopback, Duration::from_millis(80)).is_ok() {
        return false;
    }
    if std::net::TcpListener::bind(("0.0.0.0", port)).is_err() {
        return false;
    }
    if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
        return false;
    }
    match std::net::TcpListener::bind(("::", port)) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => false,
        Err(_) => true,
    }
}

fn pick_available_port(preferred: u16) -> u16 {
    for offset in 0..100u16 {
        let port = preferred.saturating_add(offset);
        if is_port_free(port) {
            return port;
        }
    }
    // Fall back to an OS-assigned ephemeral port.
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(preferred)
}

fn has_dev_script(project_root: &Path) -> bool {
    let pkg_path = project_root.join("package.json");
    let Ok(text) = fs::read_to_string(pkg_path) else {
        return false;
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    val.get("scripts")
        .and_then(|s| s.get("dev"))
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

/// Start the project's `dev` script (`<pm> run dev`), injecting `MOYU_DEV_PORT` and `PORT`,
/// streaming output logs, and detecting the actual listening port (works for rspack, Vite, etc.).
#[allow(dead_code)]
pub async fn start_dev_server(
    project_root: &Path,
    event_tx: async_channel::Sender<DevServerEvent>,
) -> Result<DevServerHandle> {
    if !has_dev_script(project_root) {
        bail!(
            "No \"dev\" script found in {}/package.json.",
            project_root.display()
        );
    }

    let root = project_root.to_path_buf();
    let node_info = tokio::task::spawn_blocking(require_node).await??;
    let path_override = if node_info.is_managed {
        node_info.executable.parent().and_then(build_augmented_path)
    } else {
        None
    };

    let detected_pm = detect_package_manager(&root);
    let node_for_pm = node_info.clone();
    let path_for_pm = path_override.clone();
    let pm = tokio::task::spawn_blocking(move || {
        ensure_package_manager(
            detected_pm,
            &node_for_pm,
            path_for_pm.as_deref(),
            &ServiceReporter::noop(),
        )
    })
    .await?;

    let suggested_port = tokio::task::spawn_blocking(|| pick_available_port(DEV_SERVER_PORT))
        .await
        .unwrap_or(DEV_SERVER_PORT);
    let _ = event_tx.try_send(DevServerEvent::Log {
        level: LogLevel::Info,
        line: format!("[dev] Allocated dev server port: {suggested_port}"),
    });

    let mut cmd = if cfg!(windows) {
        let mut c = tokio::process::Command::new("cmd.exe");
        c.args(["/c", &format!("{}.cmd", pm.binary_name()), "run", "dev"]);
        c
    } else {
        let mut c = tokio::process::Command::new(pm.binary_name());
        c.args(["run", "dev"]);
        c
    };

    if let Some(joined) = path_override {
        cmd.env("PATH", joined);
    }
    cmd.env("MOYU_DEV_PORT", suggested_port.to_string())
        .env("PORT", suggested_port.to_string());
    configure_no_window_tokio(&mut cmd);

    let mut child = cmd
        .current_dir(&root)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to launch {} run dev", pm.binary_name()))?;

    let pid = child.id();
    let (port_tx, port_rx) = watch::channel(None);
    let (candidate_tx, mut candidate_rx) = mpsc::unbounded_channel::<u16>();

    if let Some(stdout) = child.stdout.take() {
        let ev_tx = event_tx.clone();
        let cand_tx = candidate_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let clean = strip_ansi(&line);
                if let Some(p) = extract_local_port(&clean) {
                    let _ = cand_tx.send(p);
                }
                let _ = ev_tx.try_send(DevServerEvent::Log {
                    level: LogLevel::Info,
                    line: format!("[dev] {clean}"),
                });
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let ev_tx = event_tx.clone();
        let cand_tx = candidate_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let clean = strip_ansi(&line);
                if let Some(p) = extract_local_port(&clean) {
                    let _ = cand_tx.send(p);
                }
                let _ = ev_tx.try_send(DevServerEvent::Log {
                    level: LogLevel::Warn,
                    line: format!("[dev] {clean}"),
                });
            }
        });
    }

    // Probe candidate ports discovered from stdout/stderr (with fallback polling on `suggested_port`).
    let ev_tx_probe = event_tx.clone();
    let port_tx_probe = port_tx.clone();
    let probe_task = tokio::spawn(async move {
        let mut candidates: Vec<u16> = vec![suggested_port];
        let mut interval = tokio::time::interval(Duration::from_millis(300));
        loop {
            tokio::select! {
                Some(p) = candidate_rx.recv() => {
                    if !candidates.contains(&p) {
                        candidates.insert(0, p);
                    }
                    if is_port_listening(p).await {
                        let _ = port_tx_probe.send(Some(p));
                        let _ = ev_tx_probe.try_send(DevServerEvent::Ready { port: p });
                        break;
                    }
                }
                _ = interval.tick() => {
                    let mut ready_port = None;
                    for &p in &candidates {
                        if is_port_listening(p).await {
                            ready_port = Some(p);
                            break;
                        }
                    }
                    if let Some(p) = ready_port {
                        let _ = port_tx_probe.send(Some(p));
                        let _ = ev_tx_probe.try_send(DevServerEvent::Ready { port: p });
                        break;
                    }
                }
            }
        }
    });

    let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        // Keep `port_tx` alive until the child exits so `wait_ready()` unblocks on early crash.
        let _port_tx_guard = port_tx;
        tokio::select! {
            status = child.wait() => {
                probe_task.abort();
                let code = status.ok().and_then(|s| s.code());
                let _ = event_tx.try_send(DevServerEvent::Exited { code });
            }
            _ = &mut kill_rx => {
                probe_task.abort();
                if let Some(p) = pid {
                    let _ = tokio::task::spawn_blocking(move || kill_process_tree_sync(p)).await;
                }
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
        }
    });

    Ok(DevServerHandle {
        pid,
        kill_tx: Some(kill_tx),
        task: Some(task),
        port_rx,
    })
}

/// Strip ANSI CSI escape sequences from terminal output.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract a local port number from lines like `Loopback: http://localhost:6020/` or `Local: http://127.0.0.1:5173/`.
fn extract_local_port(line: &str) -> Option<u16> {
    let lower = line.to_ascii_lowercase();
    for scheme in ["http://", "https://"] {
        let mut rest = lower.as_str();
        while let Some(idx) = rest.find(scheme) {
            let after = &rest[idx + scheme.len()..];
            for host in ["localhost:", "127.0.0.1:", "0.0.0.0:", "[::1]:"] {
                if let Some(port_part) = after.strip_prefix(host) {
                    let digits: String = port_part
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(port) = digits.parse::<u16>()
                        && port > 0
                    {
                        return Some(port);
                    }
                }
            }
            rest = after;
        }
    }
    None
}
