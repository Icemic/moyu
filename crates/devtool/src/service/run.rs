//! Engine execution service for native and web modes (`run`).

use std::path::Path;
use std::process::Stdio;

use anyhow::{Result, bail};
use tokio::process::Command;

use crate::config::{DEV_SERVER_PORT, meta_file, platform_dir};
use crate::service::engine::{detect_platform, load_meta, native_executable_name};
use crate::service::project::{check_multi_res_assets, print_multi_res_warnings};
use crate::service::server::{require_web_engine_assets, start_static_file_server};

pub async fn run_project(project_root: &Path, web: bool, port: u16) -> Result<i32> {
    let report = check_multi_res_assets(project_root)?;
    print_multi_res_warnings(&report);

    let meta = load_meta(&meta_file(project_root));
    let active = meta
        .and_then(|m| m.active)
        .ok_or_else(|| anyhow::anyhow!("No active engine version. Run \"moyu-devtool download\" first."))?;

    if !web {
        let platform = detect_platform()?;
        let native_dir = platform_dir(project_root, &active.version, platform)?;
        run_native(project_root, &native_dir, platform).await
    } else {
        let web_path = require_web_engine_assets(project_root, &active.version)?;
        run_web(project_root, &web_path, port).await?;
        Ok(0)
    }
}

async fn run_native(project_root: &Path, native_dir: &Path, platform: &str) -> Result<i32> {
    let exe_name = native_executable_name(platform)
        .ok_or_else(|| anyhow::anyhow!("Unsupported native platform: {platform}"))?;
    let engine_path = native_dir.join(exe_name);
    if !engine_path.is_file() {
        bail!(
            "Engine binary not found at {}\nThe engine files may be corrupted. Run \"moyu-devtool download\" to re-download.",
            engine_path.display()
        );
    }

    eprintln!("info: Starting native engine: {}", engine_path.display());
    eprintln!("info: Working directory: {}", project_root.display());

    let entry_url = format!("http://localhost:{DEV_SERVER_PORT}/index.json");
    let mut child = Command::new(&engine_path)
        .args(["--entry", &entry_url])
        .current_dir(project_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let status = tokio::select! {
        res = child.wait() => res?,
        _ = tokio::signal::ctrl_c() => {
            let _ = child.kill().await;
            return Ok(130);
        }
    };

    Ok(status.code().unwrap_or(1))
}

async fn run_web(project_root: &Path, web_path: &Path, port: u16) -> Result<()> {
    let server = start_static_file_server(project_root, web_path, port).await?;

    eprintln!("success: Web engine server running at {}", server.url);
    eprintln!("info: Serving files from:");
    eprintln!("info:   1. {} (engine)", web_path.display());
    eprintln!("info:   2. {} (project)", project_root.display());
    eprintln!("info: Press Ctrl+C to stop.\n");

    let _ = tokio::signal::ctrl_c().await;
    eprintln!("\ninfo: Shutting down server...");
    server.close();
    Ok(())
}
