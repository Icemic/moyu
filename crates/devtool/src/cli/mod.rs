//! Command-line interface frontend powered by `clap`.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};

use crate::config::{DEFAULT_ATTACH_PORT, DEFAULT_WEB_PORT, meta_file, platform_dir};
use crate::model::debug::LogEntry;
use crate::model::tasks::TaskEvent;
use crate::service::debug::{
    DEFAULT_CONNECT_TIMEOUT_MS, DEFAULT_READY_TIMEOUT_MS, DebugHost, SessionOptions,
    start_debug_session,
};
use crate::service::engine::{
    detect_platform, download_platforms, fetch_versions_json, load_config, load_meta,
    resolve_update_platforms, switch_active_version, try_detect_platform,
};
use crate::service::init::{ProjectTemplate, create_project, validate_project_name};
use crate::service::mcp::{collect_node_lines, format_hms_ms, serve_mcp};
use crate::service::node::generate_json_schema;
use crate::service::pack::{PackOptions, pack_project};
use crate::service::project::require_project_root;
use crate::service::run::run_project;
use crate::service::video_decoder::supports_video_decoder;
use crate::service::{format_bytes};

#[derive(Parser)]
#[command(
    name = "moyu",
    version = env!("CARGO_PKG_VERSION"),
    about = "CLI & GUI toolchain for the Moyu Visual Novel Engine"
)]
pub struct Cli {
    /// Run in CLI mode even without a subcommand.
    #[arg(long, global = true)]
    pub no_gui: bool,

    /// Launch GUI mode explicitly.
    #[arg(long, global = true)]
    pub gui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new Moyu project.
    Init {
        name: Option<String>,
        #[arg(long)]
        template: Option<String>,
    },
    /// Download a specific engine version for selected platforms.
    Download {
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long = "platform")]
        platforms: Vec<String>,
        #[arg(long)]
        activate: bool,
    },
    /// Update the engine to the latest version in the current channel.
    Update {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Switch the active engine version.
    Switch { version: Option<String> },
    /// Run the engine in native or web mode.
    Run {
        #[arg(short = 'n', long)]
        native: bool,
        #[arg(short = 'w', long)]
        web: bool,
        #[arg(long, default_value_t = DEFAULT_WEB_PORT)]
        port: u16,
    },
    /// Inspect a running engine through the debug bridge.
    Debug {
        #[command(subcommand)]
        action: DebugCommands,
    },
    /// Serve the engine debug bridge as MCP tools on stdio.
    Mcp,
    /// Package game for distribution.
    Pack {
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        framework: bool,
        #[arg(long)]
        compress: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long = "android-format")]
        android_format: Option<String>,
        #[arg(long)]
        video_decoder: Option<bool>,
    },
    /// Generate JSON Schema from Zod command definitions.
    Schema {
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Generate JSON Schema from Zod UI definitions.
    #[command(name = "ui-schema")]
    UiSchema {
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Args, Clone)]
pub struct DebugSessionArgs {
    /// Debug an engine running in a browser page.
    #[arg(short = 'w', long)]
    pub web: bool,
    /// Port for the web engine server.
    #[arg(long, default_value_t = DEFAULT_WEB_PORT)]
    pub port: u16,
    /// Entry file passed to a native engine.
    #[arg(long)]
    pub entry: Option<String>,
    /// Run the native engine without a window.
    #[arg(
        long,
        default_value_t = true,
        num_args = 0..=1,
        default_missing_value = "true",
        action = clap::ArgAction::Set
    )]
    pub headless: bool,
    /// Show the engine window.
    #[arg(long = "no-headless", overrides_with = "headless")]
    pub no_headless: bool,
    /// Attach to an engine started by hand instead of starting one.
    #[arg(long)]
    pub attach: bool,
    /// Port to listen on in attach mode.
    #[arg(long, default_value_t = DEFAULT_ATTACH_PORT)]
    pub listen: u16,
    /// Milliseconds to wait for the engine to connect.
    #[arg(long, default_value_t = DEFAULT_CONNECT_TIMEOUT_MS)]
    pub timeout: u64,
}

impl DebugSessionArgs {
    fn effective_headless(&self) -> bool {
        !self.no_headless && self.headless
    }

    fn ready_timeout_ms(&self) -> u64 {
        self.timeout.max(DEFAULT_READY_TIMEOUT_MS)
    }

    fn to_options(&self) -> SessionOptions {
        if self.attach {
            SessionOptions {
                attach_port: Some(self.listen),
                ..Default::default()
            }
        } else {
            SessionOptions {
                project_root: None,
                web: self.web,
                port: Some(self.port),
                entry: self.entry.clone(),
                headless: self.effective_headless(),
                attach_port: None,
            }
        }
    }
}

#[derive(Subcommand)]
pub enum DebugCommands {
    /// Print the state snapshot of an engine.
    State {
        #[command(flatten)]
        session: DebugSessionArgs,
    },
    /// Evaluate JavaScript in the engine and print the result.
    Eval {
        #[command(flatten)]
        session: DebugSessionArgs,
        code: String,
        #[arg(long = "eval-timeout", default_value_t = 5000)]
        eval_timeout: u64,
    },
    /// Read engine logs, optionally following new entries.
    Logs {
        #[command(flatten)]
        session: DebugSessionArgs,
        #[arg(long)]
        level: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        since: Option<u64>,
        #[arg(short = 'f', long)]
        follow: bool,
    },
    /// Print the node tree.
    Tree {
        #[command(flatten)]
        session: DebugSessionArgs,
        node: Option<u32>,
        #[arg(long, default_value_t = 8)]
        depth: u32,
    },
    /// Print one node properties and derived state.
    Props {
        #[command(flatten)]
        session: DebugSessionArgs,
        node: u32,
    },
    /// Capture the stage and write it to a file.
    Screenshot {
        #[command(flatten)]
        session: DebugSessionArgs,
        file: PathBuf,
        #[arg(long = "max-width")]
        max_width: Option<u32>,
        #[arg(long = "max-height")]
        max_height: Option<u32>,
    },
}

pub async fn run(cli: Cli) -> i32 {
    let Some(command) = cli.command else {
        use clap::CommandFactory;
        let _ = Cli::command().print_help();
        println!();
        return 0;
    };

    match dispatch(command).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            1
        }
    }
}

async fn dispatch(command: Commands) -> Result<i32> {
    match command {
        Commands::Init { name, template } => {
            cmd_init(name, template)?;
            Ok(0)
        }
        Commands::Download {
            channel,
            version,
            platforms,
            activate,
        } => {
            cmd_download(channel, version, platforms, activate).await?;
            Ok(0)
        }
        Commands::Update { yes } => {
            cmd_update(yes).await?;
            Ok(0)
        }
        Commands::Switch { version } => {
            cmd_switch(version)?;
            Ok(0)
        }
        Commands::Run { native: _, web, port } => run_project(&require_project_root(None)?, web, port).await,
        Commands::Debug { action } => {
            cmd_debug(action).await?;
            Ok(0)
        }
        Commands::Mcp => {
            serve_mcp().await?;
            Ok(0)
        }
        Commands::Pack {
            target,
            framework,
            compress,
            output,
            android_format,
            video_decoder,
        } => {
            let project_root = require_project_root(None)?;
            let include_video_decoder = match video_decoder {
                Some(v) => v,
                None if !framework
                    && target.as_deref().is_some_and(supports_video_decoder)
                    && io::stdin().is_terminal() =>
                {
                    prompt_confirm("Include video playback plugin?", false)?
                }
                None => false,
            };
            let signing_password = if !framework
                && target.as_deref() == Some("android-aarch64")
                && matches!(
                    android_format.as_deref(),
                    Some("release-apk" | "release-aab")
                ) {
                Some(prompt_hidden("Android signing password: ")?)
            } else {
                None
            };
            pack_project(
                &project_root,
                PackOptions {
                    target,
                    framework,
                    compress,
                    output,
                    android_format,
                    include_video_decoder,
                    signing_password,
                },
            )
            .await?;
            Ok(0)
        }
        Commands::Schema { input, output } => {
            let project_root = require_project_root(None)?;
            let in_path = project_root.join(input.unwrap_or_else(|| PathBuf::from("src/commands/commands.ts")));
            let out_path = project_root.join(output.unwrap_or_else(|| PathBuf::from("commands.schema.json")));
            generate_json_schema(&project_root, &in_path, &out_path, "ScenarioCommandSchema").await?;
            eprintln!("success: JSON Schema written to {}", out_path.display());
            Ok(0)
        }
        Commands::UiSchema { input, output } => {
            let project_root = require_project_root(None)?;
            let in_path = project_root.join(input.unwrap_or_else(|| PathBuf::from("src/data/ui.ts")));
            let out_path = project_root.join(output.unwrap_or_else(|| PathBuf::from("ui.schema.json")));
            generate_json_schema(&project_root, &in_path, &out_path, "GameUiSchema").await?;
            eprintln!("success: JSON Schema written to {}", out_path.display());
            Ok(0)
        }
    }
}

fn cmd_init(name_arg: Option<String>, template_arg: Option<String>) -> Result<()> {
    println!("\nMoyu Project Setup\nCreate a new visual novel project\n");
    let project_name = match name_arg {
        Some(n) => n,
        None => prompt_text("Project name", Some("my-visual-novel"))?,
    };
    validate_project_name(&project_name)?;
    let target_dir = std::env::current_dir()?.join(&project_name);

    let tpl_str = match template_arg {
        Some(t) => t,
        None => {
            let idx = prompt_select(
                "Select a template:",
                &[
                    "Basic (Minimal starter template)".to_string(),
                    "Advanced (With save/load, settings, and gallery)".to_string(),
                ],
                0,
            )?;
            if idx == 0 {
                "basic".to_string()
            } else {
                "advanced".to_string()
            }
        }
    };
    let template = match tpl_str.as_str() {
        "basic" => ProjectTemplate::Basic,
        "advanced" => ProjectTemplate::Advanced,
        other => bail!("Unknown template \"{other}\". Use \"basic\" or \"advanced\"."),
    };

    println!("\nProject configuration:");
    println!("  Name:     {project_name}");
    println!("  Template: {tpl_str}");
    println!("  Path:     {}\n", target_dir.display());

    if io::stdin().is_terminal() && !prompt_confirm("Create project?", true)? {
        eprintln!("info: Setup cancelled.");
        return Ok(());
    }

    create_project(&project_name, template, &target_dir)?;
    eprintln!("success: Created project at {}", target_dir.display());
    Ok(())
}

async fn cmd_download(
    channel_arg: Option<String>,
    version_arg: Option<String>,
    platforms_arg: Vec<String>,
    activate_flag: bool,
) -> Result<()> {
    let project_root = require_project_root(None)?;
    let config = load_config(&project_root);
    eprintln!("info: Fetching version manifest...");
    let manifest = fetch_versions_json(&config.cdn_url).await?;

    let channel_names: Vec<String> = manifest.channels.keys().cloned().collect();
    if channel_names.is_empty() {
        bail!("No channels available in the version manifest.");
    }

    let selected_channel = match channel_arg {
        Some(c) => c,
        None => {
            let labels: Vec<String> = channel_names
                .iter()
                .map(|c| {
                    if *c == config.channel {
                        format!("{c} (current)")
                    } else {
                        c.clone()
                    }
                })
                .collect();
            let default_idx = channel_names
                .iter()
                .position(|c| *c == config.channel)
                .unwrap_or(0);
            let idx = prompt_select("Select channel:", &labels, default_idx)?;
            channel_names[idx].clone()
        }
    };

    let channel = manifest
        .channels
        .get(&selected_channel)
        .ok_or_else(|| anyhow::anyhow!("Channel \"{selected_channel}\" not found."))?;

    let mut version_entries: Vec<(&String, &crate::model::engine::VersionEntry)> =
        channel.versions.iter().collect();
    version_entries.sort_by(|a, b| b.1.published_at.cmp(&a.1.published_at));
    if version_entries.is_empty() {
        bail!("No versions available in channel \"{selected_channel}\".");
    }

    let selected_version = match version_arg {
        Some(v) => v,
        None => {
            let labels: Vec<String> = version_entries
                .iter()
                .map(|(name, e)| {
                    let date = e.published_at.get(..10).unwrap_or(&e.published_at);
                    if **name == channel.latest {
                        format!("{name} ({date}) [latest]")
                    } else {
                        format!("{name} ({date})")
                    }
                })
                .collect();
            let default_idx = version_entries
                .iter()
                .position(|(n, _)| **n == channel.latest)
                .unwrap_or(0);
            let idx = prompt_select("Select version:", &labels, default_idx)?;
            version_entries[idx].0.clone()
        }
    };

    let entry = channel
        .versions
        .get(&selected_version)
        .ok_or_else(|| anyhow::anyhow!("Version \"{selected_version}\" not found."))?;
    let available_platforms: Vec<String> = entry.assets.keys().cloned().collect();
    if available_platforms.is_empty() {
        bail!("No platform assets available for version \"{selected_version}\".");
    }

    let selected_platforms = if !platforms_arg.is_empty() {
        platforms_arg
    } else {
        let current = try_detect_platform();
        let default_indices: Vec<usize> = current
            .and_then(|c| available_platforms.iter().position(|p| p == c))
            .into_iter()
            .collect();
        let chosen = prompt_multiselect(
            "Select platforms to download (comma-separated numbers):",
            &available_platforms,
            &default_indices,
        )?;
        chosen
            .into_iter()
            .map(|i| available_platforms[i].clone())
            .collect()
    };

    if selected_platforms.is_empty() {
        eprintln!("warn: No platforms selected. Nothing to download.");
        return Ok(());
    }

    let existing_meta = load_meta(&meta_file(&project_root)).unwrap_or_default();
    let activate = if activate_flag || existing_meta.active.is_none() {
        true
    } else if let Some(active) = &existing_meta.active
        && active.version != selected_version
        && io::stdin().is_terminal()
    {
        prompt_confirm(
            &format!(
                "Switch active version from {} to {selected_version}?",
                active.version
            ),
            false,
        )?
    } else {
        false
    };

    let (tx, rx) = async_channel::unbounded();
    let printer = spawn_progress_printer(rx);
    eprintln!("info: Downloading {selected_version} ({})...", selected_platforms.join(", "));
    download_platforms(
        &project_root,
        &selected_channel,
        &selected_version,
        &manifest,
        &selected_platforms,
        activate,
        Some(&tx),
    )
    .await?;
    drop(tx);
    let _ = printer.await;

    eprintln!("success: Done!");
    Ok(())
}

async fn cmd_update(skip_confirm: bool) -> Result<()> {
    let project_root = require_project_root(None)?;
    let meta = load_meta(&meta_file(&project_root)).unwrap_or_default();
    let config = load_config(&project_root);
    let current_channel = meta
        .active
        .as_ref()
        .map(|a| a.channel.clone())
        .unwrap_or(config.channel);

    eprintln!("info: CDN: {}", config.cdn_url);
    eprintln!("info: Channel: {current_channel}");
    eprintln!("info: Fetching version manifest...");
    let manifest = fetch_versions_json(&config.cdn_url).await?;

    let channel = manifest
        .channels
        .get(&current_channel)
        .ok_or_else(|| anyhow::anyhow!("Channel \"{current_channel}\" not found."))?;
    let target_version = channel.latest.clone();
    let entry = channel
        .versions
        .get(&target_version)
        .ok_or_else(|| anyhow::anyhow!("Version \"{target_version}\" not found."))?;

    let target_platforms = resolve_update_platforms(&meta)?;
    if let Some(existing) = meta.downloads.get(&target_version) {
        let all_match = target_platforms.iter().all(|p| {
            existing.platforms.get(p).is_some_and(|local| {
                entry
                    .assets
                    .get(p)
                    .is_some_and(|remote| local.sha256 == remote.sha256)
            })
        });
        if meta
            .active
            .as_ref()
            .is_some_and(|a| a.version == target_version)
            && all_match
        {
            eprintln!("success: Engine is already up to date ({target_version}). Nothing to download.");
            return Ok(());
        }
    }

    let current_ver = meta
        .active
        .as_ref()
        .map(|a| a.version.as_str())
        .unwrap_or("(none)");
    if !skip_confirm {
        if !io::stdin().is_terminal() {
            bail!("Confirmation required. Re-run with --yes (-y) in non-interactive mode.");
        }
        if !prompt_confirm(
            &format!(
                "Update engine: {current_ver} -> {target_version}?\n  Platforms: {}",
                target_platforms.join(", ")
            ),
            false,
        )? {
            eprintln!("info: Update cancelled.");
            return Ok(());
        }
    }

    let (tx, rx) = async_channel::unbounded();
    let printer = spawn_progress_printer(rx);
    download_platforms(
        &project_root,
        &current_channel,
        &target_version,
        &manifest,
        &target_platforms,
        true,
        Some(&tx),
    )
    .await?;
    drop(tx);
    let _ = printer.await;

    eprintln!("success: Engine updated to {target_version}!");
    for p in &target_platforms {
        if let Ok(dir) = platform_dir(&project_root, &target_version, p) {
            eprintln!("info:   {p}: {}", dir.display());
        }
    }
    Ok(())
}

fn cmd_switch(version_arg: Option<String>) -> Result<()> {
    let project_root = require_project_root(None)?;
    let meta = load_meta(&meta_file(&project_root)).ok_or_else(|| {
        anyhow::anyhow!("No engine versions downloaded yet. Run \"moyu-devtool download\" first.")
    })?;
    if meta.downloads.is_empty() {
        bail!("No engine versions downloaded yet. Run \"moyu-devtool download\" first.");
    }

    let version_names: Vec<String> = meta.downloads.keys().cloned().collect();
    let selected = if let Some(v) = version_arg {
        v
    } else if version_names.len() == 1 {
        let only = &version_names[0];
        if meta.active.as_ref().is_some_and(|a| &a.version == only) {
            eprintln!("info: Only one version available ({only}) and it is already active.");
            return Ok(());
        }
        only.clone()
    } else {
        let labels: Vec<String> = version_names
            .iter()
            .map(|v| {
                let dl = &meta.downloads[v];
                let plats: Vec<&str> = dl.platforms.keys().map(String::as_str).collect();
                let active_mark = if meta.active.as_ref().is_some_and(|a| &a.version == v) {
                    " (active)"
                } else {
                    ""
                };
                format!("{v} [{} | {}{active_mark}]", dl.channel, plats.join(", "))
            })
            .collect();
        let idx = prompt_select("Select engine version to activate:", &labels, 0)?;
        version_names[idx].clone()
    };

    if meta
        .active
        .as_ref()
        .is_some_and(|a| a.version == selected)
    {
        eprintln!("info: {selected} is already the active version.");
        return Ok(());
    }

    let current_platform = detect_platform()?;
    if let Some(dl) = meta.downloads.get(&selected)
        && !dl.platforms.contains_key(current_platform)
    {
        eprintln!(
            "warn: Version {selected} does not include platform \"{current_platform}\".\nNative run may not work. Use \"moyu-devtool download\" to add it."
        );
    }

    switch_active_version(&project_root, &selected)?;
    eprintln!("success: Active version switched to {selected}.");
    Ok(())
}

async fn cmd_debug(action: DebugCommands) -> Result<()> {
    let session_args = match &action {
        DebugCommands::State { session }
        | DebugCommands::Eval { session, .. }
        | DebugCommands::Logs { session, .. }
        | DebugCommands::Tree { session, .. }
        | DebugCommands::Props { session, .. }
        | DebugCommands::Screenshot { session, .. } => session.clone(),
    };

    let mut session = start_debug_session(session_args.to_options(), None).await?;
    let res = run_debug_task(action, &session_args, &session.host).await;
    session.stop().await;
    res
}

async fn run_debug_task(
    action: DebugCommands,
    session_args: &DebugSessionArgs,
    host: &Arc<DebugHost>,
) -> Result<()> {
    host.wait_for_engine(session_args.timeout).await?;

    match action {
        DebugCommands::State { .. } => {
            host.wait_for_ready(session_args.ready_timeout_ms()).await?;
            let val = host.request("engine:state", None).await?;
            println!("{}", serde_json::to_string_pretty(&val)?);
        }
        DebugCommands::Eval {
            code, eval_timeout, ..
        } => {
            host.wait_for_ready(session_args.ready_timeout_ms()).await?;
            let val = host
                .request(
                    "engine:eval",
                    Some(json!({ "code": code, "timeoutMs": eval_timeout })),
                )
                .await?;
            let out = match val.get("value") {
                Some(v) => serde_json::to_string_pretty(v)?,
                None => val
                    .get("repr")
                    .and_then(|v| v.as_str())
                    .unwrap_or("undefined")
                    .to_string(),
            };
            println!("{out}");
            if val.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
                eprintln!("warn: Result exceeded the size limit and is only shown in short form.");
            }
        }
        DebugCommands::Logs {
            level,
            limit,
            since,
            follow,
            ..
        } => {
            let mut rx = host.subscribe_messages();
            let mut payload = serde_json::Map::new();
            if let Some(l) = level {
                payload.insert("level".to_string(), json!(l));
            }
            if let Some(lim) = limit {
                payload.insert("limit".to_string(), json!(lim));
            }
            if let Some(s) = since {
                payload.insert("sinceSeq".to_string(), json!(s));
            }
            payload.insert("subscribe".to_string(), json!(follow));

            let res = host
                .request("engine:logs", Some(Value::Object(payload)))
                .await?;
            let mut last_seq = 0u64;
            if let Some(entries) = res.get("entries").and_then(|v| v.as_array()) {
                for raw in entries {
                    if let Ok(entry) = serde_json::from_value::<LogEntry>(raw.clone()) {
                        print_log_entry(&entry);
                        last_seq = last_seq.max(entry.seq);
                    }
                }
            }
            if !follow {
                return Ok(());
            }
            if let Some(dropped) = res.get("dropped").and_then(|v| v.as_u64())
                && dropped > 0
            {
                eprintln!("warn: {dropped} earlier entries were dropped.");
            }
            eprintln!("info: Following. Press Ctrl+C to stop.");

            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Ok(val) if val.get("type").and_then(|v| v.as_str()) == Some("engine:log") => {
                                if let Some(entry_val) = val.get("entry")
                                    && let Ok(entry) = serde_json::from_value::<LogEntry>(entry_val.clone())
                                    && entry.seq > last_seq
                                {
                                    last_seq = entry.seq;
                                    print_log_entry(&entry);
                                }
                            }
                            Ok(_) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        }
                    }
                    _ = tokio::signal::ctrl_c() => break,
                }
            }
        }
        DebugCommands::Tree { node, depth, .. } => {
            host.wait_for_ready(session_args.ready_timeout_ms()).await?;
            let mut payload = serde_json::Map::new();
            if let Some(id) = node {
                payload.insert("nodeId".to_string(), json!(id));
            }
            payload.insert("depth".to_string(), json!(depth));
            let res = host
                .request("engine:tree", Some(Value::Object(payload)))
                .await?;
            let root: crate::model::debug::NodeSummary = serde_json::from_value(
                res.get("node")
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Missing node in engine:tree response"))?,
            )?;
            let mut lines = Vec::new();
            collect_node_lines(&root, 0, &mut lines);
            println!("{}", lines.join("\n"));
        }
        DebugCommands::Props { node, .. } => {
            host.wait_for_ready(session_args.ready_timeout_ms()).await?;
            let res = host
                .request("engine:props", Some(json!({ "nodeId": node })))
                .await?;
            if let Some(node_val) = res.get("node") {
                println!("{}", serde_json::to_string_pretty(node_val)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        }
        DebugCommands::Screenshot {
            file,
            max_width,
            max_height,
            ..
        } => {
            host.wait_for_ready(session_args.ready_timeout_ms()).await?;
            let mut payload = serde_json::Map::new();
            if let Some(w) = max_width {
                payload.insert("maxWidth".to_string(), json!(w));
            }
            if let Some(h) = max_height {
                payload.insert("maxHeight".to_string(), json!(h));
            }
            let res = host
                .request("engine:screenshot", Some(Value::Object(payload)))
                .await?;
            let data_b64 = res
                .get("data")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Missing screenshot data"))?;
            use base64::Engine as _;
            let bytes = base64::engine::general_purpose::STANDARD.decode(data_b64)?;
            std::fs::write(&file, bytes)?;
            let w = res.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
            let h = res.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
            let fmt = res.get("format").and_then(|v| v.as_str()).unwrap_or("webp");
            eprintln!("info: Wrote {w}x{h} {fmt} to {}", file.display());
        }
    }
    Ok(())
}

fn print_log_entry(entry: &LogEntry) {
    println!(
        "{} {} {}: {}",
        format_hms_ms(entry.timestamp_ms),
        entry.level.to_ascii_uppercase(),
        entry.target,
        entry.message
    );
}

fn spawn_progress_printer(rx: async_channel::Receiver<TaskEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            if let TaskEvent::Progress {
                downloaded_bytes,
                total_bytes,
                ..
            } = ev
            {
                if let Some(total) = total_bytes
                    && total > 0
                {
                    let pct = (downloaded_bytes * 100) / total;
                    eprint!(
                        "\r  Downloading... {} / {} ({pct}%)",
                        format_bytes(downloaded_bytes),
                        format_bytes(total)
                    );
                } else {
                    eprint!("\r  Downloading... {}", format_bytes(downloaded_bytes));
                }
                let _ = io::stderr().flush();
            }
        }
        eprintln!();
    })
}

fn prompt_text(label: &str, default: Option<&str>) -> Result<String> {
    if let Some(d) = default {
        print!("{label} [{d}]: ");
    } else {
        print!("{label}: ");
    }
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();
    if trimmed.is_empty()
        && let Some(d) = default
    {
        return Ok(d.to_string());
    }
    Ok(trimmed)
}

fn prompt_confirm(label: &str, default: bool) -> Result<bool> {
    let hint = if default { "Y/n" } else { "y/N" };
    print!("{label} ({hint}): ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(matches!(trimmed.as_str(), "y" | "yes"))
    }
}

fn prompt_hidden(message: &str) -> Result<String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("Android release signing password must be entered in an interactive terminal.");
    }
    print!("{message}");
    io::stdout().flush()?;

    #[cfg(target_os = "windows")]
    let password = {
        use windows::Win32::System::Console::{
            CONSOLE_MODE, ENABLE_ECHO_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE,
            SetConsoleMode,
        };
        struct ModeGuard {
            handle: windows::Win32::Foundation::HANDLE,
            old_mode: CONSOLE_MODE,
        }
        impl Drop for ModeGuard {
            fn drop(&mut self) {
                unsafe {
                    let _ = SetConsoleMode(self.handle, self.old_mode);
                }
            }
        }
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE)? };
        let mut old_mode = CONSOLE_MODE(0);
        unsafe { GetConsoleMode(handle, &mut old_mode)? };
        let _guard = ModeGuard { handle, old_mode };
        unsafe {
            SetConsoleMode(
                handle,
                CONSOLE_MODE(old_mode.0 & !ENABLE_ECHO_INPUT.0),
            )?;
        }
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        println!();
        line.trim_end_matches(&['\r', '\n'][..]).to_string()
    };

    #[cfg(not(target_os = "windows"))]
    let password = {
        use std::process::{Command, Stdio};
        struct EchoGuard;
        impl Drop for EchoGuard {
            fn drop(&mut self) {
                let _ = Command::new("stty")
                    .arg("echo")
                    .stdin(Stdio::inherit())
                    .status();
            }
        }
        let status = Command::new("stty")
            .arg("-echo")
            .stdin(Stdio::inherit())
            .status()?;
        if !status.success() {
            bail!("Failed to disable terminal echo via stty.");
        }
        let _guard = EchoGuard;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        println!();
        line.trim_end_matches(&['\r', '\n'][..]).to_string()
    };

    Ok(password)
}

fn prompt_select(label: &str, options: &[String], default_idx: usize) -> Result<usize> {
    println!("{label}");
    for (i, opt) in options.iter().enumerate() {
        let marker = if i == default_idx { "*" } else { " " };
        println!("  {marker} [{}] {opt}", i + 1);
    }
    print!("Enter number [{}]: ", default_idx + 1);
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(default_idx);
    }
    let idx: usize = trimmed
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("Invalid selection: {trimmed}"))?;
    if idx == 0 || idx > options.len() {
        bail!("Selection out of range: {idx}");
    }
    Ok(idx - 1)
}

fn prompt_multiselect(
    label: &str,
    options: &[String],
    default_indices: &[usize],
) -> Result<Vec<usize>> {
    println!("{label}");
    for (i, opt) in options.iter().enumerate() {
        let marker = if default_indices.contains(&i) {
            "*"
        } else {
            " "
        };
        println!("  {marker} [{}] {opt}", i + 1);
    }
    let default_str: Vec<String> = default_indices
        .iter()
        .map(|i| (i + 1).to_string())
        .collect();
    print!("Enter numbers [{}]: ", default_str.join(","));
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(default_indices.to_vec());
    }
    let mut selected = Vec::new();
    for part in trimmed.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let idx: usize = p
            .parse::<usize>()
            .map_err(|_| anyhow::anyhow!("Invalid selection: {p}"))?;
        if idx == 0 || idx > options.len() {
            bail!("Selection out of range: {idx}");
        }
        if !selected.contains(&(idx - 1)) {
            selected.push(idx - 1);
        }
    }
    Ok(selected)
}
