//! Command-line interface frontend powered by `clap`, `cliclack`, and `indicatif`.

mod ui;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};

use crate::config::{DEFAULT_ATTACH_PORT, DEFAULT_WEB_PORT, meta_file, platform_dir};
use crate::model::debug::LogEntry;
use crate::service::android::ANDROID_PLATFORM;
use crate::service::debug::{
    DEFAULT_CONNECT_TIMEOUT_MS, DEFAULT_READY_TIMEOUT_MS, DebugHost, SessionOptions,
    start_debug_session,
};
use crate::service::engine::{
    detect_platform, download_platforms, fetch_versions_json, load_config, load_meta,
    resolve_update_platforms, switch_active_version, try_detect_platform,
};
use crate::service::init::{create_project, validate_project_name};
use crate::service::mcp::{collect_node_lines, format_hms_ms, serve_mcp};
use crate::service::node::generate_json_schema;
use crate::service::pack::{PackOptions, pack_project};
use crate::service::project::require_project_root;
use crate::service::run::run_project;
use crate::service::video_decoder::supports_video_decoder;

use self::ui::SelectItem;

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
    /// Create a new Moyu project from the latest framework template.
    Init { name: Option<String> },
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
        #[arg(
            long,
            num_args = 0..=1,
            default_missing_value = "true",
            require_equals = true
        )]
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
        require_equals = true,
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
                dev_server_port: None,
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
            if err.downcast_ref::<ui::UserCancelled>().is_some() {
                ui::outro_cancel("Cancelled.");
                0
            } else {
                ui::log_error(&format!("{err:#}"));
                1
            }
        }
    }
}

async fn dispatch(command: Commands) -> Result<i32> {
    match command {
        Commands::Init { name } => {
            cmd_init(name).await?;
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
        Commands::Run {
            native: _,
            web,
            port,
        } => {
            let (reporter, printer) = ui::create_cli_reporter();
            let res = run_project(&require_project_root(None)?, web, port, &reporter).await;
            drop(reporter);
            let _ = printer.await;
            res
        }
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
            cmd_pack(
                target,
                framework,
                compress,
                output,
                android_format,
                video_decoder,
            )
            .await?;
            Ok(0)
        }
        Commands::Schema { input, output } => {
            let project_root = require_project_root(None)?;
            let in_path = project_root
                .join(input.unwrap_or_else(|| PathBuf::from("src/commands/commands.ts")));
            let out_path =
                project_root.join(output.unwrap_or_else(|| PathBuf::from("commands.schema.json")));
            ui::with_spinner(
                "Generating commands.schema.json...",
                generate_json_schema(&project_root, &in_path, &out_path, "ScenarioCommandSchema"),
            )
            .await?;
            ui::log_success(&format!("JSON Schema written to {}", out_path.display()));
            Ok(0)
        }
        Commands::UiSchema { input, output } => {
            let project_root = require_project_root(None)?;
            let in_path =
                project_root.join(input.unwrap_or_else(|| PathBuf::from("src/data/ui.ts")));
            let out_path =
                project_root.join(output.unwrap_or_else(|| PathBuf::from("ui.schema.json")));
            ui::with_spinner(
                "Generating ui.schema.json...",
                generate_json_schema(&project_root, &in_path, &out_path, "GameUiSchema"),
            )
            .await?;
            ui::log_success(&format!("JSON Schema written to {}", out_path.display()));
            Ok(0)
        }
    }
}

async fn cmd_init(name_arg: Option<String>) -> Result<()> {
    ui::intro("moyu · init");
    let cwd = std::env::current_dir()?;

    let project_name = match name_arg {
        Some(n) => {
            validate_project_name(&n)?;
            n
        }
        None => {
            let cwd_check = cwd.clone();
            ui::prompt_input(
                "Project name",
                Some("my-visual-novel"),
                Some("my-visual-novel"),
                move |input| {
                    let trimmed = input.trim();
                    if let Err(err) = validate_project_name(trimmed) {
                        return Err(format!("{err}"));
                    }
                    if cwd_check.join(trimmed).exists() {
                        return Err(format!("Directory \"{trimmed}\" already exists."));
                    }
                    Ok(())
                },
            )?
        }
    };
    let target_dir = cwd.join(&project_name);
    ui::log_info(&format!("Target directory: {}", target_dir.display()));

    if ui::is_interactive() && !ui::prompt_confirm("Create project?", true)? {
        ui::outro_cancel("Setup cancelled.");
        return Ok(());
    }

    let (reporter, printer) = ui::create_cli_reporter();
    let res = create_project(&project_name, &target_dir, &reporter).await;
    drop(reporter);
    let _ = printer.await;
    res?;

    ui::outro(&format!(
        "Project \"{project_name}\" created at {}",
        target_dir.display()
    ));
    Ok(())
}

async fn cmd_download(
    channel_arg: Option<String>,
    version_arg: Option<String>,
    platforms_arg: Vec<String>,
    activate_flag: bool,
) -> Result<()> {
    ui::intro("moyu · download");
    let project_root = require_project_root(None)?;
    let config = load_config(&project_root);
    let manifest = ui::with_spinner(
        "Fetching engine version manifest...",
        fetch_versions_json(&config.cdn_url),
    )
    .await?;

    let channel_names: Vec<String> = manifest.channels.keys().cloned().collect();
    if channel_names.is_empty() {
        bail!("No channels available in the version manifest.");
    }

    let selected_channel = match channel_arg {
        Some(c) => c,
        None => {
            let items: Vec<SelectItem> = channel_names
                .iter()
                .map(|c| SelectItem::new(c, if *c == config.channel { "current" } else { "" }))
                .collect();
            let default_idx = channel_names
                .iter()
                .position(|c| *c == config.channel)
                .unwrap_or(0);
            let idx = ui::prompt_select("Select engine channel", &items, default_idx)?;
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
            let items: Vec<SelectItem> = version_entries
                .iter()
                .map(|(name, e)| {
                    let date = e.published_at.get(..10).unwrap_or(&e.published_at);
                    let hint = if **name == channel.latest {
                        format!("{date} · latest")
                    } else {
                        date.to_string()
                    };
                    SelectItem::new(*name, hint)
                })
                .collect();
            let default_idx = version_entries
                .iter()
                .position(|(n, _)| **n == channel.latest)
                .unwrap_or(0);
            let idx = ui::prompt_select("Select engine version", &items, default_idx)?;
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
        let items: Vec<SelectItem> = available_platforms
            .iter()
            .map(|p| {
                let hint = if current == Some(p.as_str()) {
                    "current platform"
                } else {
                    ""
                };
                SelectItem::new(p, hint)
            })
            .collect();
        let default_indices: Vec<usize> = current
            .and_then(|c| available_platforms.iter().position(|p| p == c))
            .into_iter()
            .collect();
        let chosen = ui::prompt_multiselect(
            "Select platforms to download (Space to toggle, Enter to confirm)",
            &items,
            &default_indices,
        )?;
        chosen
            .into_iter()
            .map(|i| available_platforms[i].clone())
            .collect()
    };

    if selected_platforms.is_empty() {
        ui::outro_cancel("No platforms selected. Nothing to download.");
        return Ok(());
    }

    let existing_meta = load_meta(&meta_file(&project_root)).unwrap_or_default();
    let activate = if activate_flag || existing_meta.active.is_none() {
        true
    } else if let Some(active) = &existing_meta.active
        && active.version != selected_version
        && ui::is_interactive()
    {
        ui::prompt_confirm(
            &format!(
                "Switch active version from {} to {selected_version}?",
                active.version
            ),
            false,
        )?
    } else {
        false
    };

    let (reporter, printer) = ui::create_cli_reporter();
    let res = download_platforms(
        &project_root,
        &selected_channel,
        &selected_version,
        &manifest,
        &selected_platforms,
        activate,
        &reporter,
    )
    .await;
    drop(reporter);
    let _ = printer.await;
    res?;

    ui::outro(&format!(
        "Downloaded {selected_version} ({})",
        selected_platforms.join(", ")
    ));
    Ok(())
}

async fn cmd_update(skip_confirm: bool) -> Result<()> {
    ui::intro("moyu · update");
    let project_root = require_project_root(None)?;
    let meta = load_meta(&meta_file(&project_root)).unwrap_or_default();
    let config = load_config(&project_root);
    let current_channel = meta
        .active
        .as_ref()
        .map(|a| a.channel.clone())
        .unwrap_or(config.channel);

    ui::log_info(&format!(
        "Channel: {current_channel}  ·  CDN: {}",
        config.cdn_url
    ));
    let manifest = ui::with_spinner(
        "Fetching engine version manifest...",
        fetch_versions_json(&config.cdn_url),
    )
    .await?;

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
            ui::outro(&format!("Engine is already up to date ({target_version})."));
            return Ok(());
        }
    }

    let current_ver = meta
        .active
        .as_ref()
        .map(|a| a.version.as_str())
        .unwrap_or("(none)");
    if !skip_confirm {
        if !ui::is_interactive() {
            bail!("Confirmation required. Re-run with --yes (-y) in non-interactive mode.");
        }
        if !ui::prompt_confirm(
            &format!(
                "Update engine {current_ver} -> {target_version} ({})?",
                target_platforms.join(", ")
            ),
            true,
        )? {
            ui::outro_cancel("Update cancelled.");
            return Ok(());
        }
    }

    let (reporter, printer) = ui::create_cli_reporter();
    let res = download_platforms(
        &project_root,
        &current_channel,
        &target_version,
        &manifest,
        &target_platforms,
        true,
        &reporter,
    )
    .await;
    drop(reporter);
    let _ = printer.await;
    res?;

    for p in &target_platforms {
        if let Ok(dir) = platform_dir(&project_root, &target_version, p) {
            ui::log_info(&format!("{p}: {}", dir.display()));
        }
    }
    ui::outro(&format!("Engine updated to {target_version}!"));
    Ok(())
}

fn cmd_switch(version_arg: Option<String>) -> Result<()> {
    ui::intro("moyu · switch");
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
            ui::outro(&format!(
                "Only one version available ({only}) and it is already active."
            ));
            return Ok(());
        }
        only.clone()
    } else {
        let active_idx = meta
            .active
            .as_ref()
            .and_then(|a| version_names.iter().position(|v| v == &a.version))
            .unwrap_or(0);
        let items: Vec<SelectItem> = version_names
            .iter()
            .map(|v| {
                let dl = &meta.downloads[v];
                let plats: Vec<&str> = dl.platforms.keys().map(String::as_str).collect();
                let active_mark = if meta.active.as_ref().is_some_and(|a| &a.version == v) {
                    " · active"
                } else {
                    ""
                };
                SelectItem::new(
                    v,
                    format!("{} · {}{active_mark}", dl.channel, plats.join(", ")),
                )
            })
            .collect();
        let idx = ui::prompt_select("Select engine version to activate", &items, active_idx)?;
        version_names[idx].clone()
    };

    if meta.active.as_ref().is_some_and(|a| a.version == selected) {
        ui::outro(&format!("{selected} is already the active version."));
        return Ok(());
    }

    let current_platform = detect_platform()?;
    if let Some(dl) = meta.downloads.get(&selected)
        && !dl.platforms.contains_key(current_platform)
    {
        ui::log_warn(&format!(
            "Version {selected} does not include platform \"{current_platform}\". Native run may not work."
        ));
    }

    switch_active_version(&project_root, &selected)?;
    ui::outro(&format!("Active engine version switched to {selected}."));
    Ok(())
}

async fn cmd_pack(
    mut target: Option<String>,
    mut framework: bool,
    compress: bool,
    output: Option<PathBuf>,
    mut android_format: Option<String>,
    video_decoder: Option<bool>,
) -> Result<()> {
    ui::intro("moyu · pack");
    let project_root = require_project_root(None)?;

    if !framework && target.is_none() && ui::is_interactive() {
        let group_items = [
            SelectItem::new("Desktop", "Windows, macOS, Linux"),
            SelectItem::new("Web", "Browser static bundle"),
            SelectItem::new("Android", "APK, AAB, Android Studio project"),
            SelectItem::new("Framework", "Moyu framework ZIP archive"),
        ];
        match ui::prompt_select("Select export target", &group_items, 0)? {
            0 => {
                let desktop_platforms = [
                    "windows-amd64",
                    "macos-aarch64",
                    "macos-amd64",
                    "linux-amd64",
                    "linux-aarch64",
                ];
                let current = try_detect_platform();
                let default_idx = current
                    .and_then(|c| desktop_platforms.iter().position(|&p| p == c))
                    .unwrap_or(0);
                let plat_items: Vec<SelectItem> = desktop_platforms
                    .iter()
                    .map(|&p| SelectItem::new(p, if current == Some(p) { "current" } else { "" }))
                    .collect();
                let idx = ui::prompt_select("Select desktop platform", &plat_items, default_idx)?;
                target = Some(desktop_platforms[idx].to_string());
            }
            1 => {
                target = Some("web-universal".to_string());
            }
            2 => {
                target = Some(ANDROID_PLATFORM.to_string());
                if android_format.is_none() {
                    let fmt_keys = ["debug-apk", "release-apk", "release-aab", "android-project"];
                    let fmt_items = [
                        SelectItem::new("debug-apk", "Debug APK (no signing password needed)"),
                        SelectItem::new("release-apk", "Signed Release APK"),
                        SelectItem::new("release-aab", "Signed Release App Bundle (AAB)"),
                        SelectItem::new("android-project", "Export Android Gradle project"),
                    ];
                    let idx = ui::prompt_select("Select Android output format", &fmt_items, 0)?;
                    android_format = Some(fmt_keys[idx].to_string());
                }
            }
            _ => {
                framework = true;
            }
        }
    }

    let include_video_decoder = match video_decoder {
        Some(v) => v,
        None if !framework
            && target.as_deref().is_some_and(supports_video_decoder)
            && ui::is_interactive() =>
        {
            ui::prompt_confirm("Include video playback plugin?", false)?
        }
        None => false,
    };

    let signing_password = if !framework
        && target.as_deref() == Some(ANDROID_PLATFORM)
        && matches!(
            android_format.as_deref(),
            Some("release-apk" | "release-aab")
        ) {
        Some(ui::prompt_password("Android signing password")?)
    } else {
        None
    };

    let (reporter, printer) = ui::create_cli_reporter();
    let res = pack_project(
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
        &reporter,
    )
    .await;
    drop(reporter);
    let _ = printer.await;
    res?;

    ui::outro("Packaging complete!");
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
