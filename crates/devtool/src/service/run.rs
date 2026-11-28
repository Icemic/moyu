//! Engine execution service for native and web modes (`run`).

use std::path::Path;
use std::process::Stdio;

use anyhow::{Result, bail};
use tokio::process::Command;

use crate::config::{DEV_SERVER_PORT, meta_file, platform_dir};
use crate::service::ServiceReporter;
use crate::service::engine::{detect_platform, load_meta, native_executable_name};
use crate::service::project::{check_multi_res_assets, report_multi_res_warnings};
use crate::service::server::{require_web_engine_assets, start_static_file_server};

pub async fn run_project(
    project_root: &Path,
    web: bool,
    port: u16,
    reporter: &ServiceReporter,
) -> Result<i32> {
    let report = check_multi_res_assets(project_root)?;
    report_multi_res_warnings(&report, reporter);

    let meta = load_meta(&meta_file(project_root));
    let active = meta.and_then(|m| m.active).ok_or_else(|| {
        anyhow::anyhow!("No active engine version. Run \"moyu-devtool download\" first.")
    })?;

    if !web {
        let platform = detect_platform()?;
        let native_dir = platform_dir(project_root, &active.version, platform)?;
        run_native(project_root, &native_dir, platform, reporter).await
    } else {
        let web_path = require_web_engine_assets(project_root, &active.version)?;
        run_web(project_root, &web_path, port, reporter).await?;
        Ok(0)
    }
}

async fn run_native(
    project_root: &Path,
    native_dir: &Path,
    platform: &str,
    reporter: &ServiceReporter,
) -> Result<i32> {
    let exe_name = native_executable_name(platform)
        .ok_or_else(|| anyhow::anyhow!("Unsupported native platform: {platform}"))?;
    let engine_path = native_dir.join(exe_name);
    if !engine_path.is_file() {
        bail!(
            "Engine binary not found at {}\nThe engine files may be corrupted. Run \"moyu-devtool download\" to re-download.",
            engine_path.display()
        );
    }

    reporter.info(format!("Starting native engine: {}", engine_path.display()));
    reporter.info(format!("Working directory: {}", project_root.display()));

    let entry_url = format!("http://localhost:{DEV_SERVER_PORT}/index.json");
    let mut child = Command::new(&engine_path)
        .args(["--entry", &entry_url])
        .current_dir(project_root)
        .kill_on_drop(true)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let status = tokio::select! {
        res = child.wait() => res?,
        _ = reporter.cancel.cancelled() => {
            let _ = child.kill().await;
            return Ok(0);
        }
        _ = wait_shutdown_signal() => {
            let _ = child.kill().await;
            return Ok(130);
        }
    };

    Ok(status.code().unwrap_or(1))
}

async fn run_web(
    project_root: &Path,
    web_path: &Path,
    port: u16,
    reporter: &ServiceReporter,
) -> Result<()> {
    let server = start_static_file_server(project_root, web_path, port).await?;

    reporter.stage(server.url.clone());
    reporter.success(format!("Web engine server running at {}", server.url));
    reporter.info("Serving files from:");
    reporter.info(format!("  1. {} (engine)", web_path.display()));
    reporter.info(format!("  2. {} (project)", project_root.display()));
    reporter.info("Press Ctrl+C to stop.");

    tokio::select! {
        _ = reporter.cancel.cancelled() => {}
        _ = wait_shutdown_signal() => {}
    }
    reporter.info("Shutting down server...");
    server.close();
    Ok(())
}

async fn wait_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
