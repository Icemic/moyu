//! GPUI single-window application shell and Tokio async bridge.

#[cfg(feature = "gui")]
mod i18n;
#[cfg(feature = "gui")]
mod theme;
#[cfg(feature = "gui")]
mod ui;
#[cfg(feature = "gui")]
mod views;

#[cfg(not(feature = "gui"))]
pub fn run() -> i32 {
    eprintln!(
        "moyu-devtool was compiled without the `gui` feature.\nRebuild with default features (`cargo build -p moyu_devtool`) or pass a CLI subcommand."
    );
    1
}

#[cfg(feature = "gui")]
pub use app::DevtoolApp;

#[cfg(feature = "gui")]
pub fn run() -> i32 {
    app::run()
}

#[cfg(feature = "gui")]
mod app {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::sync::Arc;

    use gpui::{
        App, Bounds, Context, Entity, Global, PathPromptOptions, Window, WindowBounds,
        WindowOptions, div, prelude::*, px, size,
    };

    use crate::config::{
        DEFAULT_CHANNEL, DEFAULT_WEB_PORT, DevtoolGlobalConfig, LocalePreference, ThemePreference,
        load_global_config, meta_file, save_global_config,
    };
    use crate::gui::i18n::I18n;
    use crate::gui::theme::Theme;
    use crate::gui::ui::badge::{BadgeTone, badge};
    use crate::gui::ui::button::{ButtonVariant, button};
    use crate::gui::ui::card::card;
    use crate::gui::ui::log_list::LogListView;
    use crate::gui::ui::progress::progress_bar;
    use crate::gui::ui::text_input::{TextInput, register_key_bindings};
    use crate::gui::views::{engine, project, run_pack, settings};
    use crate::model::DomainModel;
    use crate::model::engine::{EngineEvent, EngineModel};
    use crate::model::env::EnvModel;
    use crate::model::project::{ProjectEvent, ProjectModel};
    use crate::model::tasks::{LogLevel, TaskEvent, TaskLogEntry, TasksModel};
    use crate::service::engine::{
        download_platforms, fetch_versions_json, load_config, load_meta, resolve_update_platforms,
        switch_active_version, try_detect_platform,
    };
    use crate::service::init::{ProjectTemplate, create_project};
    use crate::service::node::{detect_env, generate_json_schema, install_managed_node};
    use crate::service::pack::{PackOptions, pack_project};
    use crate::service::project::{
        check_multi_res_assets, load_project_opened_event, report_multi_res_warnings,
    };
    use crate::service::run::run_project;
    use crate::service::{CancelToken, ServiceReporter, format_bytes, next_task_id};

    pub struct TokioRuntime(pub Arc<tokio::runtime::Runtime>);
    impl Global for TokioRuntime {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum NavTab {
        Project,
        Engine,
        RunPack,
        Debug,
        Settings,
    }

    pub struct DevtoolApp {
        active_tab: NavTab,
        pub global_config: DevtoolGlobalConfig,
        pub project: ProjectModel,
        pub engine: EngineModel,
        pub env: EnvModel,
        pub tasks: TasksModel,
        pub active_cancel: Option<CancelToken>,
        pub active_stage: Option<String>,
        pub active_progress: Option<(u64, Option<u64>)>,
        pub init_name_input: Entity<TextInput>,
        pub init_template: ProjectTemplate,
        pub selected_channel: String,
        pub selected_version: Option<String>,
        pub selected_platforms: BTreeSet<String>,
        pub pack_framework: bool,
        pub pack_target: String,
        pub pack_compress: bool,
        pub pack_video_decoder: bool,
        pub pack_android_format: String,
        pub android_password_input: Entity<TextInput>,
        log_list: Entity<LogListView>,
        logs_collapsed: bool,
    }

    impl DevtoolApp {
        fn new(cx: &mut Context<Self>) -> Self {
            let global_config = load_global_config();
            let i18n = I18n::new(global_config.locale);
            let init_name_input = cx.new(|cx| TextInput::new(i18n.project_name_placeholder(), cx));
            let android_password_input = cx.new(|cx| {
                let mut input = TextInput::new(i18n.android_password_placeholder(), cx);
                input.set_masked(true, cx);
                input
            });
            let log_list = cx.new(|_| LogListView::new(i18n.logs_empty()));

            let default_platform = try_detect_platform().unwrap_or("windows-amd64").to_string();
            let mut selected_platforms = BTreeSet::new();
            selected_platforms.insert(default_platform.clone());

            let mut app = Self {
                active_tab: NavTab::Project,
                global_config,
                project: ProjectModel::default(),
                engine: EngineModel::default(),
                env: EnvModel::default(),
                tasks: TasksModel::default(),
                active_cancel: None,
                active_stage: None,
                active_progress: None,
                init_name_input,
                init_template: ProjectTemplate::Basic,
                selected_channel: DEFAULT_CHANNEL.to_string(),
                selected_version: None,
                selected_platforms,
                pack_framework: false,
                pack_target: default_platform,
                pack_compress: true,
                pack_video_decoder: false,
                pack_android_format: "debug-apk".to_string(),
                android_password_input,
                log_list,
                logs_collapsed: false,
            };

            if let Some(last) = app.global_config.recent_projects.first().cloned()
                && last.join("index.json").is_file()
            {
                app.open_project_path(last, cx);
            } else {
                app.refresh_env(cx);
            }

            app
        }

        pub fn push_log(&self, level: LogLevel, line: impl Into<String>, cx: &mut Context<Self>) {
            let entry = TaskLogEntry {
                task_id: 0,
                level,
                line: line.into(),
            };
            self.log_list.update(cx, |list, cx| list.push(entry, cx));
        }

        fn begin_task(&mut self, label: impl Into<String>, cx: &mut Context<Self>) -> ServiceReporter {
            if let Some(prev) = self.active_cancel.take() {
                prev.cancel();
            }
            let cancel = CancelToken::new();
            let (tx, rx) = async_channel::unbounded();
            let reporter = ServiceReporter::with_channel(next_task_id(), tx, cancel.clone());
            self.active_cancel = Some(cancel);
            self.active_stage = Some(label.into());
            self.active_progress = None;
            cx.notify();

            cx.spawn(async move |this, cx| {
                while let Ok(ev) = rx.recv().await {
                    let _ = this.update(cx, |app, cx| {
                        match &ev {
                            TaskEvent::Stage { stage, .. } => {
                                app.active_stage = Some(stage.clone());
                            }
                            TaskEvent::Progress {
                                downloaded_bytes,
                                total_bytes,
                                ..
                            } => {
                                app.active_progress = Some((*downloaded_bytes, *total_bytes));
                            }
                            TaskEvent::LogLine { id, level, line } => {
                                let entry = TaskLogEntry {
                                    task_id: *id,
                                    level: *level,
                                    line: line.clone(),
                                };
                                app.log_list.update(cx, |list, cx| list.push(entry, cx));
                            }
                            TaskEvent::Started { .. } | TaskEvent::Finished { .. } => {}
                        }
                        app.tasks.apply(ev);
                        cx.notify();
                    });
                }
            })
            .detach();

            reporter
        }

        fn clear_active_task(&mut self, cx: &mut Context<Self>) {
            self.active_cancel = None;
            self.active_stage = None;
            self.active_progress = None;
            cx.notify();
        }

        pub fn cancel_active_task(&mut self, cx: &mut Context<Self>) {
            if let Some(cancel) = self.active_cancel.take() {
                cancel.cancel();
                self.push_log(LogLevel::Warn, "Cancellation requested...", cx);
            }
            cx.notify();
        }

        pub fn open_project_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
            match load_project_opened_event(&path) {
                Ok(ev) => {
                    self.project.apply(ev);
                    let cfg = load_config(&path);
                    self.selected_channel = cfg.channel.clone();
                    self.engine.apply(EngineEvent::ConfigLoaded(cfg));
                    let meta = load_meta(&meta_file(&path)).unwrap_or_default();
                    self.engine.apply(EngineEvent::MetaLoaded(meta));

                    self.global_config.record_recent_project(path.clone());
                    let _ = save_global_config(&self.global_config);
                    self.recheck_multi_res(cx);
                    self.refresh_env(cx);
                    self.push_log(
                        LogLevel::Success,
                        format!("Opened project: {}", path.display()),
                        cx,
                    );
                    cx.notify();
                }
                Err(err) => {
                    self.push_log(LogLevel::Error, format!("{err:#}"), cx);
                }
            }
        }

        pub fn recheck_multi_res(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                return;
            };
            match check_multi_res_assets(&root) {
                Ok(report) => {
                    let (tx, rx) = async_channel::unbounded();
                    let rep = ServiceReporter::with_channel(next_task_id(), tx, CancelToken::new());
                    report_multi_res_warnings(&report, &rep);
                    drop(rep);
                    while let Ok(TaskEvent::LogLine { level, line, .. }) = rx.try_recv() {
                        self.push_log(level, line, cx);
                    }
                    self.project.apply(ProjectEvent::MultiResChecked(report));
                    cx.notify();
                }
                Err(err) => self.push_log(LogLevel::Error, format!("{err:#}"), cx),
            }
        }

        pub fn prompt_open_project(&mut self, cx: &mut Context<Self>) {
            let rx = cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: None,
            });
            cx.spawn(async move |this, cx| {
                if let Ok(Ok(Some(paths))) = rx.await
                    && let Some(dir) = paths.into_iter().next()
                {
                    let _ = this.update(cx, |app, cx| app.open_project_path(dir, cx));
                }
            })
            .detach();
        }

        pub fn prompt_create_project(&mut self, cx: &mut Context<Self>) {
            let name = self.init_name_input.read(cx).text().trim().to_string();
            if name.is_empty() {
                self.push_log(LogLevel::Warn, "Please enter a project name first.", cx);
                return;
            }
            let template = self.init_template;
            let rx = cx.prompt_for_paths(PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: None,
            });
            cx.spawn(async move |this, cx| {
                if let Ok(Ok(Some(paths))) = rx.await
                    && let Some(parent) = paths.into_iter().next()
                {
                    let target_dir = parent.join(&name);
                    let res = create_project(&name, template, &target_dir);
                    let _ = this.update(cx, |app, cx| match res {
                        Ok(()) => {
                            app.push_log(
                                LogLevel::Success,
                                format!("Created project at {}", target_dir.display()),
                                cx,
                            );
                            app.open_project_path(target_dir, cx);
                        }
                        Err(err) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                    });
                }
            })
            .detach();
        }

        pub fn fetch_engine_manifest(&mut self, cx: &mut Context<Self>) {
            let cdn_url = self.engine.config.cdn_url.clone();
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            self.engine.apply(EngineEvent::ManifestFetching);
            self.push_log(LogLevel::Info, format!("Fetching manifest: {cdn_url}"), cx);

            let handle = rt.spawn(async move { fetch_versions_json(&cdn_url).await });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| match res {
                    Ok(Ok(manifest)) => {
                        let first_ch = if manifest.channels.contains_key(&app.selected_channel) {
                            app.selected_channel.clone()
                        } else {
                            manifest
                                .channels
                                .keys()
                                .next()
                                .cloned()
                                .unwrap_or_else(|| DEFAULT_CHANNEL.to_string())
                        };
                        let latest_ver = manifest
                            .channels
                            .get(&first_ch)
                            .map(|c| c.latest.clone());
                        app.selected_channel = first_ch;
                        app.selected_version = latest_ver;
                        app.engine.apply(EngineEvent::ManifestLoaded(manifest));
                        app.push_log(LogLevel::Success, "Fetched engine version manifest.", cx);
                        cx.notify();
                    }
                    Ok(Err(err)) => {
                        app.engine.apply(EngineEvent::Error(format!("{err:#}")));
                        app.push_log(LogLevel::Error, format!("{err:#}"), cx);
                    }
                    Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                });
            })
            .detach();
        }

        pub fn select_channel(&mut self, channel: String, cx: &mut Context<Self>) {
            self.selected_channel = channel.clone();
            self.selected_version = self
                .engine
                .remote_manifest
                .as_ref()
                .and_then(|m| m.channels.get(&channel))
                .map(|c| c.latest.clone());
            cx.notify();
        }

        pub fn switch_local_engine(&mut self, version: &str, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            match switch_active_version(&root, version) {
                Ok(meta) => {
                    self.engine.apply(EngineEvent::MetaLoaded(meta));
                    self.push_log(
                        LogLevel::Success,
                        format!("Switched active engine version to {version}."),
                        cx,
                    );
                    cx.notify();
                }
                Err(err) => self.push_log(LogLevel::Error, format!("{err:#}"), cx),
            }
        }

        pub fn start_engine_download_selected(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let Some(manifest) = self.engine.remote_manifest.clone() else {
                self.push_log(LogLevel::Warn, "Fetch version manifest first.", cx);
                return;
            };
            let Some(version) = self.selected_version.clone() else {
                self.push_log(LogLevel::Warn, "Select a version first.", cx);
                return;
            };
            let platforms: Vec<String> = self.selected_platforms.iter().cloned().collect();
            if platforms.is_empty() {
                self.push_log(LogLevel::Warn, "Select at least one platform.", cx);
                return;
            }
            let channel = self.selected_channel.clone();
            let reporter = self.begin_task(format!("Downloading {version}..."), cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let res = download_platforms(
                    &root,
                    &channel,
                    &version,
                    &manifest,
                    &platforms,
                    true,
                    &reporter,
                )
                .await;
                drop(reporter);
                res
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(meta)) => {
                            app.engine.apply(EngineEvent::MetaLoaded(meta));
                            app.push_log(LogLevel::Success, "Engine download completed.", cx);
                        }
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                    cx.notify();
                });
            })
            .detach();
        }

        pub fn start_engine_update(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let cdn_url = self.engine.config.cdn_url.clone();
            let current_channel = self
                .engine
                .meta
                .active
                .as_ref()
                .map(|a| a.channel.clone())
                .unwrap_or_else(|| self.engine.config.channel.clone());
            let meta_snapshot = self.engine.meta.clone();
            let reporter = self.begin_task(format!("Updating channel {current_channel}..."), cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);

            let handle = rt.spawn(async move {
                let manifest = fetch_versions_json(&cdn_url).await?;
                let target_ver = manifest
                    .channels
                    .get(&current_channel)
                    .ok_or_else(|| anyhow::anyhow!("Channel \"{current_channel}\" not found."))?
                    .latest
                    .clone();
                let platforms = resolve_update_platforms(&meta_snapshot)?;
                let meta = download_platforms(
                    &root,
                    &current_channel,
                    &target_ver,
                    &manifest,
                    &platforms,
                    true,
                    &reporter,
                )
                .await?;
                drop(reporter);
                Ok::<_, anyhow::Error>((manifest, meta, target_ver))
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok((manifest, meta, ver))) => {
                            app.engine.apply(EngineEvent::ManifestLoaded(manifest));
                            app.engine.apply(EngineEvent::MetaLoaded(meta));
                            app.push_log(
                                LogLevel::Success,
                                format!("Updated engine to {ver}."),
                                cx,
                            );
                        }
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                    cx.notify();
                });
            })
            .detach();
        }

        pub fn start_run(&mut self, web: bool, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let label = if web {
                format!("Running Web Server (port {DEFAULT_WEB_PORT})...")
            } else {
                "Running Native Engine...".to_string()
            };
            let reporter = self.begin_task(label, cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let res = run_project(&root, web, DEFAULT_WEB_PORT, &reporter).await;
                drop(reporter);
                res
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(code)) => app.push_log(
                            LogLevel::Info,
                            format!("Run exited with code {code}."),
                            cx,
                        ),
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                });
            })
            .detach();
        }

        pub fn start_pack(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let signing_pw = self.android_password_input.read(cx).text().to_string();
            let options = PackOptions {
                target: (!self.pack_framework).then(|| self.pack_target.clone()),
                framework: self.pack_framework,
                compress: self.pack_compress,
                output: None,
                android_format: (!self.pack_framework
                    && self.pack_target == crate::service::android::ANDROID_PLATFORM)
                    .then(|| self.pack_android_format.clone()),
                include_video_decoder: self.pack_video_decoder,
                signing_password: (!signing_pw.is_empty()).then_some(signing_pw),
            };
            let reporter = self.begin_task("Packaging project...", cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let res = pack_project(&root, options, &reporter).await;
                drop(reporter);
                res
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(())) => {}
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                });
            })
            .detach();
        }

        pub fn start_generate_schema(&mut self, is_ui: bool, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let (in_rel, out_rel, export) = if is_ui {
                ("src/data/ui.ts", "ui.schema.json", "GameUiSchema")
            } else {
                (
                    "src/commands/commands.ts",
                    "commands.schema.json",
                    "ScenarioCommandSchema",
                )
            };
            let in_path = root.join(in_rel);
            let out_path = root.join(out_rel);
            let reporter = self.begin_task(format!("Generating {out_rel}..."), cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let res = generate_json_schema(&root, &in_path, &out_path, export).await;
                drop(reporter);
                res.map(|()| out_path)
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(path)) => app.push_log(
                            LogLevel::Success,
                            format!("JSON Schema written to {}", path.display()),
                            cx,
                        ),
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                });
            })
            .detach();
        }

        pub fn refresh_env(&mut self, cx: &mut Context<Self>) {
            let root = self.project.root.clone();
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn_blocking(move || detect_env(root.as_deref()));
            cx.spawn(async move |this, cx| {
                if let Ok(env_model) = handle.await {
                    let _ = this.update(cx, |app, cx| {
                        app.env = env_model;
                        cx.notify();
                    });
                }
            })
            .detach();
        }

        pub fn start_install_managed_node(&mut self, cx: &mut Context<Self>) {
            let reporter = self.begin_task("Installing managed Node.js LTS...", cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let res = install_managed_node(&reporter).await;
                drop(reporter);
                res
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(ver)) => {
                            app.global_config.managed_node_version = Some(ver);
                            let _ = save_global_config(&app.global_config);
                            app.refresh_env(cx);
                        }
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                });
            })
            .detach();
        }

        pub fn set_theme(&mut self, pref: ThemePreference, window: &Window, cx: &mut Context<Self>) {
            self.global_config.theme = pref;
            let _ = save_global_config(&self.global_config);
            let next = Theme::resolve(pref, window.appearance());
            cx.set_global(next);
            cx.notify();
        }

        pub fn set_locale(&mut self, locale: LocalePreference, cx: &mut Context<Self>) {
            self.global_config.locale = locale;
            let _ = save_global_config(&self.global_config);
            let i18n = I18n::new(locale);
            self.init_name_input
                .update(cx, |i, cx| i.set_placeholder(i18n.project_name_placeholder(), cx));
            self.android_password_input.update(cx, |i, cx| {
                i.set_placeholder(i18n.android_password_placeholder(), cx)
            });
            self.log_list
                .update(cx, |l, cx| l.set_empty_label(i18n.logs_empty(), cx));
            cx.notify();
        }
    }

    impl Render for DevtoolApp {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::global(cx);
            let i18n = I18n::new(self.global_config.locale);
            let active_tab = self.active_tab;

            let nav_items: &[(NavTab, &str)] = &[
                (NavTab::Project, i18n.nav_project()),
                (NavTab::Engine, i18n.nav_engine()),
                (NavTab::RunPack, i18n.nav_run_pack()),
                (NavTab::Debug, i18n.nav_debug()),
                (NavTab::Settings, i18n.nav_settings()),
            ];

            let project_name = self
                .project
                .index_summary
                .as_ref()
                .and_then(|s| s.window_title.clone())
                .or_else(|| {
                    self.project
                        .root
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                })
                .unwrap_or_else(|| i18n.no_project_selected().to_string());

            let sidebar = div()
                .w(px(220.0))
                .h_full()
                .bg(theme.background)
                .p(px(12.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(
                    card(theme)
                        .p(px(12.0))
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(theme.text)
                                .child(project_name),
                        )
                        .child(button(
                            "btn-open-project",
                            i18n.open_project(),
                            ButtonVariant::Primary,
                            theme,
                            cx.listener(|this, _, _, cx| this.prompt_open_project(cx)),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .children(nav_items.iter().map(|&(tab, label)| {
                            let is_active = tab == active_tab;
                            let bg = if is_active {
                                theme.accent_soft
                            } else {
                                gpui::transparent_black()
                            };
                            let fg = if is_active {
                                theme.accent
                            } else {
                                theme.text_secondary
                            };
                            let hover_bg = if is_active {
                                theme.accent_soft
                            } else {
                                theme.control_hover
                            };
                            div()
                                .id(("nav-item", tab as usize))
                                .px(px(10.0))
                                .py(px(7.0))
                                .rounded(px(6.0))
                                .bg(bg)
                                .text_color(fg)
                                .text_size(px(13.0))
                                .cursor_pointer()
                                .hover(move |s| s.bg(hover_bg))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.active_tab = tab;
                                    cx.notify();
                                }))
                                .child(label)
                        })),
                );

            let current_title = nav_items
                .iter()
                .find(|(t, _)| *t == active_tab)
                .map(|(_, l)| *l)
                .unwrap_or("");

            let header = div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(18.0))
                                .text_color(theme.text)
                                .child(current_title),
                        )
                        .child(badge(
                            format!("v{}", env!("CARGO_PKG_VERSION")),
                            BadgeTone::Accent,
                            theme,
                        )),
                )
                .when(self.active_cancel.is_some(), |this| {
                    this.child(button(
                        "btn-cancel-active",
                        i18n.stop_task(),
                        ButtonVariant::Danger,
                        theme,
                        cx.listener(|this, _, _, cx| this.cancel_active_task(cx)),
                    ))
                });

            let page_body = match active_tab {
                NavTab::Project => project::render_project_view(self, theme, i18n, cx),
                NavTab::Engine => engine::render_engine_view(self, theme, i18n, cx),
                NavTab::RunPack => run_pack::render_run_pack_view(self, theme, i18n, cx),
                NavTab::Debug => card(theme)
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.text_secondary)
                            .child(i18n.debug_v2_placeholder()),
                    )
                    .into_any_element(),
                NavTab::Settings => settings::render_settings_view(self, theme, i18n, cx),
            };

            let progress_banner = self.active_stage.clone().map(|stage| {
                let (ratio, detail) = match self.active_progress {
                    Some((dl, Some(total))) if total > 0 => (
                        dl as f32 / total as f32,
                        format!("{} / {}", format_bytes(dl), format_bytes(total)),
                    ),
                    Some((dl, None)) => (0.5, format_bytes(dl)),
                    _ => (0.0, String::new()),
                };
                card(theme)
                    .p(px(10.0))
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .text_size(px(12.0))
                            .child(div().text_color(theme.text).child(stage))
                            .child(div().text_color(theme.text_tertiary).child(detail)),
                    )
                    .child(progress_bar(ratio, theme))
            });

            let logs_collapsed = self.logs_collapsed;
            let logs_panel = card(theme)
                .p(px(12.0))
                .gap(px(6.0))
                .when(!logs_collapsed, |c| c.h(px(170.0)))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .child(i18n.logs_title()),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(6.0))
                                .child(button(
                                    "btn-clear-logs",
                                    i18n.clear_logs(),
                                    ButtonVariant::Ghost,
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.log_list.update(cx, |l, cx| l.clear(cx));
                                    }),
                                ))
                                .child(button(
                                    "btn-toggle-logs",
                                    if logs_collapsed { "▲" } else { "▼" },
                                    ButtonVariant::Ghost,
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.logs_collapsed = !this.logs_collapsed;
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
                .when(!logs_collapsed, |c| {
                    c.child(div().flex_1().w_full().child(self.log_list.clone()))
                });

            let main_body = div()
                .flex_1()
                .h_full()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(header)
                .when_some(progress_banner, |this, banner| this.child(banner))
                .child(div().flex_1().overflow_hidden().child(page_body))
                .child(logs_panel);

            div()
                .size_full()
                .bg(theme.background)
                .text_color(theme.text)
                .flex()
                .flex_row()
                .child(sidebar)
                .child(main_body)
        }
    }

    pub fn run() -> i32 {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize background Tokio runtime");
        let rt = Arc::new(rt);

        gpui_platform::application().run(move |cx: &mut App| {
            let cfg = load_global_config();
            cx.set_global(TokioRuntime(Arc::clone(&rt)));
            cx.set_global(Theme::resolve(cfg.theme, cx.window_appearance()));
            register_key_bindings(cx);

            let i18n = I18n::new(cfg.locale);
            let bounds = Bounds::centered(None, size(px(1100.0), px(740.0)), cx);
            let _ = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some(i18n.app_title().into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    cx.set_global(Theme::resolve(cfg.theme, window.appearance()));
                    cx.new(DevtoolApp::new)
                },
            );
            cx.activate(true);
        });
        0
    }
}
