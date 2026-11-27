//! GPUI single-window application shell, custom titlebar, and Tokio async bridge.

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
pub use app::{ActivePopover, DevtoolApp, PackTargetGroup, RunMode};

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
        App, Bounds, Context, Entity, Global, MouseButton, PathPromptOptions, Window, WindowBounds,
        WindowControlArea, WindowOptions, deferred, div, img, point, prelude::*, px, size,
    };

    use crate::config::{
        DEFAULT_CHANNEL, DEFAULT_WEB_PORT, DevtoolGlobalConfig, LocalePreference, ThemePreference,
        load_global_config, meta_file, save_global_config,
    };
    use crate::gui::i18n::I18n;
    use crate::gui::theme::Theme;
    use crate::gui::ui::button::{ButtonVariant, icon_button, sm_button};
    use crate::gui::ui::card::status_dot;
    use crate::gui::ui::dropdown::{menu_divider, menu_section_label, popover_container};
    use crate::gui::ui::icon::{DevtoolAssets, IconName, icon};
    use crate::gui::ui::log_list::LogListView;
    use crate::gui::ui::progress::progress_bar;
    use crate::gui::ui::text_input::{TextInput, register_key_bindings};
    use crate::gui::views::{debug, development, dialogs, engine, pack};
    use crate::model::DomainModel;
    use crate::model::debug::{DebugEvent, DebugModel, LogEntry, NodeSummary, SessionMode};
    use crate::model::engine::{EngineEvent, EngineModel};
    use crate::model::env::EnvModel;
    use crate::model::project::{ProjectEvent, ProjectModel};
    use crate::model::tasks::{LogLevel, TaskEvent, TaskLogEntry, TasksModel};
    use crate::service::debug::{DebugSession, SessionOptions, start_debug_session};
    use crate::service::dev_server::{
        DevServerEvent, DevServerHandle, DevServerStatus, start_dev_server,
    };
    use crate::service::editor::{
        DetectedEditor, detect_editors, open_in_editor, open_in_file_manager, open_url_in_browser,
    };
    use crate::service::engine::{
        download_platforms, fetch_versions_json, load_config, load_meta, resolve_update_platforms,
        switch_active_version, try_detect_platform,
    };
    use crate::service::init::{create_project, validate_project_name};
    use crate::service::node::{
        detect_env, generate_json_schema, install_managed_node, run_project_install,
        uninstall_managed_node,
    };
    use crate::service::pack::{PackOptions, pack_project};
    use crate::service::project::{
        ProjectConfigPatch, check_multi_res_assets, load_index_summary, load_project_opened_event,
        report_multi_res_warnings, save_project_config,
    };
    use crate::service::{CancelToken, ServiceReporter, format_bytes, next_task_id};

    const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/app-icon.png");

    pub struct TokioRuntime(pub Arc<tokio::runtime::Runtime>);
    impl Global for TokioRuntime {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum NavTab {
        Development,
        Engine,
        Pack,
        Debug,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ActivePopover {
        ProjectMenu,
        EditorMenu,
        RunModeMenu,
        ConfigChannel,
        SettingsTheme,
        SettingsLocale,
        DownloadChannel,
        DownloadVersion,
        PackPlatform,
        PackAndroidFormat,
        DebugMode,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ActiveDialog {
        Settings,
        CreateProject,
        DownloadEngine,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RunMode {
        Desktop,
        Browser,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PackTargetGroup {
        Desktop,
        Web,
        Android,
        Framework,
    }

    pub struct DevtoolApp {
        app_icon: Arc<gpui::Image>,
        active_tab: NavTab,
        pub active_popover: Option<ActivePopover>,
        pub active_dialog: Option<ActiveDialog>,
        pub global_config: DevtoolGlobalConfig,
        pub project: ProjectModel,
        pub engine: EngineModel,
        pub env: EnvModel,
        pub tasks: TasksModel,
        pub active_cancel: Option<CancelToken>,
        pub active_stage: Option<String>,
        pub active_progress: Option<(u64, Option<u64>)>,
        pub status_message: String,
        pub status_is_error: bool,
        // Editor detection & launch
        pub detected_editors: Vec<DetectedEditor>,
        pub selected_editor_id: Option<String>,
        // Unified run/debug preview mode (shares `debug_session` with Debugger tab)
        pub run_mode: RunMode,
        // Inline project config editing
        pub config_editing: bool,
        pub config_channel_draft: String,
        pub config_save_error: Option<String>,
        pub cfg_title_input: Entity<TextInput>,
        pub cfg_entry_input: Entity<TextInput>,
        pub cfg_window_size_input: Entity<TextInput>,
        pub cfg_stage_size_input: Entity<TextInput>,
        // Create project dialog
        pub init_name_input: Entity<TextInput>,
        pub init_dir_input: Entity<TextInput>,
        // Engine download dialog
        pub selected_channel: String,
        pub selected_version: Option<String>,
        pub selected_platforms: BTreeSet<String>,
        // Pack view
        pub pack_target_group: PackTargetGroup,
        pub pack_desktop_platform: String,
        pub pack_compress: bool,
        pub pack_video_decoder: bool,
        pub pack_android_format: String,
        pub android_password_input: Entity<TextInput>,
        // Project Dev Server (independent from one-shot task slot)
        pub dev_server_status: DevServerStatus,
        dev_server_handle: Option<Arc<tokio::sync::Mutex<DevServerHandle>>>,
        dev_server_gen: u64,
        // V2 Visual Debugger
        pub debug: DebugModel,
        pub debug_session: Option<Arc<tokio::sync::Mutex<DebugSession>>>,
        debug_session_gen: u64,
        pub debug_mode: SessionMode,
        pub debug_headless: bool,
        pub debug_collapsed_nodes: BTreeSet<u32>,
        pub debug_selected_node: Option<u32>,
        pub debug_selected_props: Option<serde_json::Value>,
        pub debug_eval_input: Entity<TextInput>,
        pub debug_eval_output: Option<String>,
        pub debug_log_level_filter: String,
        pub debug_screenshot_bytes: Option<Vec<u8>>,
        pub debug_screenshot_image: Option<Arc<gpui::Image>>,
        pub debug_screenshot_meta: Option<String>,
        // Bottom output drawer
        log_list: Entity<LogListView>,
        output_open: bool,
    }

    impl DevtoolApp {
        fn new(cx: &mut Context<Self>) -> Self {
            let global_config = load_global_config();
            let i18n = I18n::new(global_config.locale);
            let app_icon = Arc::new(gpui::Image::from_bytes(
                gpui::ImageFormat::Png,
                APP_ICON_PNG.to_vec(),
            ));

            let init_name_input = cx.new(|cx| TextInput::new(i18n.project_name_placeholder(), cx));
            let default_parent = std::env::current_dir()
                .ok()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let init_dir_input = cx.new(|cx| {
                let mut input = TextInput::new(i18n.parent_directory(), cx);
                input.set_text(default_parent, cx);
                input
            });
            let cfg_title_input = cx.new(|cx| TextInput::new(i18n.window_title_label(), cx));
            let cfg_entry_input = cx.new(|cx| TextInput::new("./main.js", cx));
            let cfg_window_size_input = cx.new(|cx| TextInput::new("1280x720", cx));
            let cfg_stage_size_input = cx.new(|cx| TextInput::new("1920x1080", cx));
            let android_password_input = cx.new(|cx| {
                let mut input = TextInput::new(i18n.android_password_placeholder(), cx);
                input.set_masked(true, cx);
                input
            });
            let debug_eval_input = cx.new(|cx| TextInput::new(i18n.dbg_eval_placeholder(), cx));
            let log_list = cx.new(|_| LogListView::new(i18n.logs_empty()));

            let default_platform = try_detect_platform().unwrap_or("windows-amd64").to_string();
            let mut selected_platforms = BTreeSet::new();
            selected_platforms.insert(default_platform.clone());

            let preferred_editor = global_config.preferred_editor.clone();

            let mut app = Self {
                app_icon,
                active_tab: NavTab::Development,
                active_popover: None,
                active_dialog: None,
                global_config,
                project: ProjectModel::default(),
                engine: EngineModel::default(),
                env: EnvModel::default(),
                tasks: TasksModel::default(),
                active_cancel: None,
                active_stage: None,
                active_progress: None,
                status_message: i18n.ready_status().to_string(),
                status_is_error: false,
                detected_editors: Vec::new(),
                selected_editor_id: preferred_editor,
                run_mode: RunMode::Desktop,
                config_editing: false,
                config_channel_draft: DEFAULT_CHANNEL.to_string(),
                config_save_error: None,
                cfg_title_input,
                cfg_entry_input,
                cfg_window_size_input,
                cfg_stage_size_input,
                init_name_input,
                init_dir_input,
                selected_channel: DEFAULT_CHANNEL.to_string(),
                selected_version: None,
                selected_platforms,
                pack_target_group: PackTargetGroup::Desktop,
                pack_desktop_platform: default_platform,
                pack_compress: true,
                pack_video_decoder: false,
                pack_android_format: "debug-apk".to_string(),
                android_password_input,
                dev_server_status: DevServerStatus::Stopped,
                dev_server_handle: None,
                dev_server_gen: 0,
                debug: DebugModel::default(),
                debug_session: None,
                debug_session_gen: 0,
                debug_mode: SessionMode::Native,
                debug_headless: false,
                debug_collapsed_nodes: BTreeSet::new(),
                debug_selected_node: None,
                debug_selected_props: None,
                debug_eval_input,
                debug_eval_output: None,
                debug_log_level_filter: "ALL".to_string(),
                debug_screenshot_bytes: None,
                debug_screenshot_image: None,
                debug_screenshot_meta: None,
                log_list,
                output_open: false,
            };

            app.refresh_editors(cx);
            if let Some(last) = app.global_config.recent_projects.first().cloned()
                && last.join("index.json").is_file()
            {
                app.open_project_path(last, cx);
            } else {
                app.refresh_env(cx);
            }

            app
        }

        pub fn toggle_popover(&mut self, pop: ActivePopover, cx: &mut Context<Self>) {
            self.active_popover = if self.active_popover == Some(pop) {
                None
            } else {
                Some(pop)
            };
            cx.notify();
        }

        pub fn close_dialog(&mut self, cx: &mut Context<Self>) {
            self.active_popover = None;
            self.active_dialog = None;
            cx.notify();
        }

        pub fn open_create_project_dialog(&mut self, cx: &mut Context<Self>) {
            self.active_popover = None;
            self.active_dialog = Some(ActiveDialog::CreateProject);
            cx.notify();
        }

        pub fn open_download_engine_dialog(&mut self, cx: &mut Context<Self>) {
            self.active_popover = None;
            self.active_dialog = Some(ActiveDialog::DownloadEngine);
            if self.engine.remote_manifest.is_none() && !self.engine.fetching_manifest {
                self.fetch_engine_manifest(cx);
            }
            cx.notify();
        }

        pub fn push_log(
            &mut self,
            level: LogLevel,
            line: impl Into<String>,
            cx: &mut Context<Self>,
        ) {
            let text = line.into();
            self.status_message = text.clone();
            self.status_is_error = matches!(level, LogLevel::Error);
            let entry = TaskLogEntry {
                task_id: 0,
                level,
                line: text,
            };
            self.log_list.update(cx, |list, cx| list.push(entry, cx));
            cx.notify();
        }

        fn begin_task(
            &mut self,
            label: impl Into<String>,
            cx: &mut Context<Self>,
        ) -> ServiceReporter {
            if let Some(prev) = self.active_cancel.take() {
                prev.cancel();
            }
            let cancel = CancelToken::new();
            let (tx, rx) = async_channel::unbounded();
            let reporter = ServiceReporter::with_channel(next_task_id(), tx, cancel.clone());
            let label_str = label.into();
            self.active_cancel = Some(cancel);
            self.active_stage = Some(label_str.clone());
            self.active_progress = None;
            self.status_message = label_str;
            self.status_is_error = false;
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
                                app.status_message = line.clone();
                                app.status_is_error = matches!(level, LogLevel::Error);
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

        pub fn current_editor(&self) -> Option<&DetectedEditor> {
            if let Some(sel) = &self.selected_editor_id
                && let Some(found) = self.detected_editors.iter().find(|e| e.id == sel)
            {
                return Some(found);
            }
            self.detected_editors.first()
        }

        pub fn select_editor(&mut self, id: &str, cx: &mut Context<Self>) {
            self.selected_editor_id = Some(id.to_string());
            self.global_config.preferred_editor = Some(id.to_string());
            let _ = save_global_config(&self.global_config);
            self.active_popover = None;
            cx.notify();
        }

        pub fn refresh_editors(&mut self, cx: &mut Context<Self>) {
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn_blocking(detect_editors);
            cx.spawn(async move |this, cx| {
                if let Ok(editors) = handle.await {
                    let _ = this.update(cx, |app, cx| {
                        app.detected_editors = editors;
                        if let Some(sel) = &app.selected_editor_id
                            && !app.detected_editors.iter().any(|e| e.id == sel)
                        {
                            app.selected_editor_id =
                                app.detected_editors.first().map(|e| e.id.to_string());
                        }
                        cx.notify();
                    });
                }
            })
            .detach();
        }

        pub fn open_project_folder(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            match open_in_file_manager(&root) {
                Ok(()) => self.push_log(
                    LogLevel::Info,
                    format!("Opened folder: {}", root.display()),
                    cx,
                ),
                Err(err) => self.push_log(LogLevel::Error, format!("{err:#}"), cx),
            }
        }

        pub fn launch_selected_editor(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let Some(editor) = self.current_editor().cloned() else {
                self.push_log(LogLevel::Warn, "No supported code editor detected.", cx);
                return;
            };
            match open_in_editor(&editor, &root) {
                Ok(()) => self.push_log(
                    LogLevel::Info,
                    format!("Opened {} in {}.", root.display(), editor.name),
                    cx,
                ),
                Err(err) => self.push_log(LogLevel::Error, format!("{err:#}"), cx),
            }
        }

        pub fn available_config_channels(&self) -> Vec<String> {
            let mut channels = vec!["stable".to_string(), "dev".to_string()];
            if let Some(manifest) = &self.engine.remote_manifest {
                for key in manifest.channels.keys() {
                    if !channels.contains(key) {
                        channels.push(key.clone());
                    }
                }
            }
            if !self.config_channel_draft.is_empty()
                && !channels.contains(&self.config_channel_draft)
            {
                channels.push(self.config_channel_draft.clone());
            }
            channels
        }

        pub fn begin_config_edit(&mut self, cx: &mut Context<Self>) {
            let index = self.project.index_summary.as_ref();
            let title = index
                .and_then(|s| s.window_title.clone())
                .unwrap_or_default();
            let entry = index
                .and_then(|s| s.entry_filename.clone())
                .unwrap_or_else(|| "./main.js".to_string());
            let win_size = index
                .and_then(|s| s.initial_surface_size.clone())
                .unwrap_or_else(|| "1280x720".to_string());
            let stage_size = index
                .and_then(|s| s.stage_size.clone())
                .unwrap_or_else(|| "1920x1080".to_string());
            let channel = self
                .project
                .moyu_summary
                .as_ref()
                .and_then(|m| m.engine_channel.clone())
                .unwrap_or_else(|| self.engine.config.channel.clone());

            self.cfg_title_input
                .update(cx, |i, cx| i.set_text(title, cx));
            self.cfg_entry_input
                .update(cx, |i, cx| i.set_text(entry, cx));
            self.cfg_window_size_input
                .update(cx, |i, cx| i.set_text(win_size, cx));
            self.cfg_stage_size_input
                .update(cx, |i, cx| i.set_text(stage_size, cx));
            self.config_channel_draft = channel;
            self.config_save_error = None;
            self.config_editing = true;
            cx.notify();
        }

        pub fn cancel_config_edit(&mut self, cx: &mut Context<Self>) {
            self.config_editing = false;
            self.config_save_error = None;
            cx.notify();
        }

        pub fn save_config_edit(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                return;
            };
            let patch = ProjectConfigPatch {
                window_title: self.cfg_title_input.read(cx).text().to_string(),
                entry_filename: self.cfg_entry_input.read(cx).text().to_string(),
                initial_surface_size: self.cfg_window_size_input.read(cx).text().to_string(),
                stage_size: self.cfg_stage_size_input.read(cx).text().to_string(),
                engine_channel: self.config_channel_draft.clone(),
            };
            match save_project_config(&root, &patch) {
                Ok(()) => {
                    if let Ok(ev) = load_project_opened_event(&root) {
                        self.project.apply(ev);
                    }
                    let cfg = load_config(&root);
                    self.selected_channel = cfg.channel.clone();
                    self.engine.apply(EngineEvent::ConfigLoaded(cfg));
                    self.config_editing = false;
                    self.config_save_error = None;
                    self.push_log(LogLevel::Success, "Project configuration saved.", cx);
                }
                Err(err) => {
                    let msg = format!("{err:#}");
                    self.config_save_error = Some(msg.clone());
                    self.push_log(LogLevel::Error, msg, cx);
                }
            }
            cx.notify();
        }

        pub fn remove_recent_project(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
            if self.global_config.remove_recent_project(path) {
                let _ = save_global_config(&self.global_config);
                cx.notify();
            }
        }

        pub fn open_project_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
            self.active_popover = None;
            if !path.is_dir() || !path.join("index.json").is_file() {
                if self.global_config.remove_recent_project(&path) {
                    let _ = save_global_config(&self.global_config);
                }
                let i18n = I18n::new(self.global_config.locale);
                self.push_log(
                    LogLevel::Warn,
                    i18n.project_not_found_removed(&path.display().to_string()),
                    cx,
                );
                return;
            }

            self.stop_debug_session(cx);
            self.stop_dev_server(cx);
            self.config_editing = false;
            self.config_save_error = None;

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
                    self.active_tab = NavTab::Development;
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
                    let clean = report.is_clean();
                    let (tx, rx) = async_channel::unbounded();
                    let rep = ServiceReporter::with_channel(next_task_id(), tx, CancelToken::new());
                    report_multi_res_warnings(&report, &rep);
                    drop(rep);
                    while let Ok(TaskEvent::LogLine { level, line, .. }) = rx.try_recv() {
                        self.push_log(level, line, cx);
                    }
                    self.project.apply(ProjectEvent::MultiResChecked(report));
                    if clean {
                        self.push_log(LogLevel::Info, "Multi-resolution asset check passed.", cx);
                    }
                    cx.notify();
                }
                Err(err) => self.push_log(LogLevel::Error, format!("{err:#}"), cx),
            }
        }

        pub fn prompt_open_project(&mut self, cx: &mut Context<Self>) {
            self.active_popover = None;
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

        pub fn prompt_pick_create_parent(&mut self, cx: &mut Context<Self>) {
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
                    let _ = this.update(cx, |app, cx| {
                        app.init_dir_input
                            .update(cx, |i, cx| i.set_text(dir.display().to_string(), cx));
                        cx.notify();
                    });
                }
            })
            .detach();
        }

        pub fn submit_create_project(&mut self, cx: &mut Context<Self>) {
            let name = self.init_name_input.read(cx).text().trim().to_string();
            let parent_text = self.init_dir_input.read(cx).text().trim().to_string();
            if name.is_empty() {
                self.push_log(LogLevel::Warn, "Please enter a project name.", cx);
                return;
            }
            if let Err(err) = validate_project_name(&name) {
                self.push_log(LogLevel::Error, format!("{err:#}"), cx);
                return;
            }
            if parent_text.is_empty() {
                self.push_log(LogLevel::Warn, "Please choose a parent directory.", cx);
                return;
            }
            let target_dir = PathBuf::from(parent_text).join(&name);
            if target_dir.exists() {
                self.push_log(
                    LogLevel::Error,
                    format!("Directory \"{}\" already exists.", target_dir.display()),
                    cx,
                );
                return;
            }

            self.active_dialog = None;
            self.init_name_input.update(cx, |i, cx| i.set_text("", cx));
            let reporter = self.begin_task(format!("Creating project {name}..."), cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let dir_for_task = target_dir.clone();
            let name_for_task = name.clone();
            let handle = rt.spawn(async move {
                let res = create_project(&name_for_task, &dir_for_task, &reporter).await;
                drop(reporter);
                res
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(created)) => {
                            if let Some(ver) = created.managed_node_version {
                                app.global_config.managed_node_version = Some(ver);
                                let _ = save_global_config(&app.global_config);
                            }
                            app.open_project_path(target_dir, cx);
                        }
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                });
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
                        let latest_ver = manifest.channels.get(&first_ch).map(|c| c.latest.clone());
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
                    &root, &channel, &version, &manifest, &platforms, true, &reporter,
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

        pub fn set_run_mode(&mut self, mode: RunMode, cx: &mut Context<Self>) {
            self.run_mode = mode;
            self.debug_mode = match mode {
                RunMode::Desktop => SessionMode::Native,
                RunMode::Browser => SessionMode::Web,
            };
            self.active_popover = None;
            cx.notify();
        }

        pub fn set_debug_mode(&mut self, mode: SessionMode, cx: &mut Context<Self>) {
            self.debug_mode = mode;
            match mode {
                SessionMode::Native => self.run_mode = RunMode::Desktop,
                SessionMode::Web => self.run_mode = RunMode::Browser,
                SessionMode::Attach => {}
            }
            self.active_popover = None;
            cx.notify();
        }

        pub fn toggle_run_preview(&mut self, cx: &mut Context<Self>) {
            if self.debug_session.is_some() {
                self.stop_debug_session(cx);
                return;
            }
            self.debug_mode = match self.run_mode {
                RunMode::Desktop => SessionMode::Native,
                RunMode::Browser => SessionMode::Web,
            };
            self.start_debug_session(cx);
        }

        pub fn start_pack(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let signing_pw = self.android_password_input.read(cx).text().to_string();
            let (framework, target, android_format) = match self.pack_target_group {
                PackTargetGroup::Desktop => (false, Some(self.pack_desktop_platform.clone()), None),
                PackTargetGroup::Web => (false, Some("web-universal".to_string()), None),
                PackTargetGroup::Android => (
                    false,
                    Some(crate::service::android::ANDROID_PLATFORM.to_string()),
                    Some(self.pack_android_format.clone()),
                ),
                PackTargetGroup::Framework => (true, None, None),
            };
            let options = PackOptions {
                target,
                framework,
                compress: self.pack_compress,
                output: None,
                android_format,
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

        pub fn uninstall_managed_node_action(&mut self, cx: &mut Context<Self>) {
            match uninstall_managed_node() {
                Ok(()) => {
                    self.global_config.managed_node_version = None;
                    let _ = save_global_config(&self.global_config);
                    self.refresh_env(cx);
                    self.push_log(LogLevel::Success, "Uninstalled managed Node.js LTS.", cx);
                }
                Err(err) => self.push_log(LogLevel::Error, format!("{err:#}"), cx),
            }
        }

        pub fn start_install_deps(&mut self, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            let reporter = self.begin_task("Installing project dependencies...", cx);
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let res = run_project_install(&root, &reporter).await;
                drop(reporter);
                res
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    app.clear_active_task(cx);
                    match res {
                        Ok(Ok(())) => {
                            app.refresh_env(cx);
                            app.push_log(LogLevel::Success, "Project dependencies installed.", cx);
                        }
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                });
            })
            .detach();
        }

        pub fn set_theme(
            &mut self,
            pref: ThemePreference,
            window: &Window,
            cx: &mut Context<Self>,
        ) {
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
            self.init_name_input.update(cx, |i, cx| {
                i.set_placeholder(i18n.project_name_placeholder(), cx)
            });
            self.init_dir_input
                .update(cx, |i, cx| i.set_placeholder(i18n.parent_directory(), cx));
            self.android_password_input.update(cx, |i, cx| {
                i.set_placeholder(i18n.android_password_placeholder(), cx)
            });
            self.debug_eval_input.update(cx, |i, cx| {
                i.set_placeholder(i18n.dbg_eval_placeholder(), cx)
            });
            self.log_list
                .update(cx, |l, cx| l.set_empty_label(i18n.logs_empty(), cx));
            cx.notify();
        }

        pub fn start_dev_server_action(&mut self, cx: &mut Context<Self>) {
            self.start_dev_server_internal(false, cx);
        }

        fn start_dev_server_internal(&mut self, launch_debug_after: bool, cx: &mut Context<Self>) {
            let Some(root) = self.project.root.clone() else {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            };
            if self.dev_server_status != DevServerStatus::Stopped {
                return;
            }

            self.dev_server_gen = self.dev_server_gen.wrapping_add(1);
            let gen_id = self.dev_server_gen;
            self.dev_server_status = DevServerStatus::Starting;
            self.push_log(LogLevel::Info, "Starting dev server...", cx);

            let (ev_tx, ev_rx) = async_channel::unbounded();
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);

            cx.spawn(async move |this, cx| {
                while let Ok(ev) = ev_rx.recv().await {
                    let mut should_break = false;
                    let _ = this.update(cx, |app, cx| {
                        if app.dev_server_gen != gen_id {
                            should_break = true;
                            return;
                        }
                        match ev {
                            DevServerEvent::Log { level, line } => {
                                let entry = TaskLogEntry {
                                    task_id: 0,
                                    level,
                                    line,
                                };
                                app.log_list.update(cx, |list, cx| list.push(entry, cx));
                                cx.notify();
                            }
                            DevServerEvent::Ready { port } => {
                                app.dev_server_status = DevServerStatus::Running { port };
                                app.push_log(
                                    LogLevel::Success,
                                    format!("Dev server ready at http://localhost:{port}"),
                                    cx,
                                );
                            }
                            DevServerEvent::Exited { code } => {
                                app.dev_server_handle = None;
                                app.dev_server_status = DevServerStatus::Stopped;
                                let msg = match code {
                                    Some(0) => "Dev server stopped.".to_string(),
                                    Some(c) => format!("Dev server exited with code {c}."),
                                    None => "Dev server exited.".to_string(),
                                };
                                app.push_log(
                                    if code == Some(0) {
                                        LogLevel::Info
                                    } else {
                                        LogLevel::Warn
                                    },
                                    msg,
                                    cx,
                                );
                                should_break = true;
                            }
                        }
                    });
                    if should_break {
                        break;
                    }
                }
            })
            .detach();

            let rt_clone = Arc::clone(&rt);
            let handle = rt.spawn(async move { start_dev_server(&root, ev_tx).await });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let maybe_handle = match res {
                    Ok(Ok(srv)) => Some(Arc::new(tokio::sync::Mutex::new(srv))),
                    Ok(Err(err)) => {
                        let _ = this.update(cx, |app, cx| {
                            if app.dev_server_gen == gen_id {
                                app.dev_server_status = DevServerStatus::Stopped;
                                app.push_log(LogLevel::Error, format!("{err:#}"), cx);
                            }
                        });
                        return;
                    }
                    Err(err) => {
                        let _ = this.update(cx, |app, cx| {
                            if app.dev_server_gen == gen_id {
                                app.dev_server_status = DevServerStatus::Stopped;
                                app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx);
                            }
                        });
                        return;
                    }
                };

                let Some(srv_arc) = maybe_handle else {
                    return;
                };
                let mut still_active = false;
                let _ = this.update(cx, |app, cx| {
                    if app.dev_server_gen == gen_id {
                        app.dev_server_handle = Some(Arc::clone(&srv_arc));
                        still_active = true;
                        cx.notify();
                    }
                });
                if !still_active {
                    rt_clone.spawn(async move {
                        let mut guard = srv_arc.lock().await;
                        guard.stop().await;
                    });
                    return;
                }

                if launch_debug_after {
                    let wait_handle = rt_clone.spawn(async move {
                        let guard = srv_arc.lock().await;
                        guard.wait_ready(30_000).await
                    });
                    let ready_res = wait_handle.await;
                    let _ = this.update(cx, |app, cx| {
                        if app.dev_server_gen != gen_id {
                            return;
                        }
                        match ready_res {
                            Ok(Ok(port)) => {
                                app.dev_server_status = DevServerStatus::Running { port };
                                app.launch_debug_session_with_port(Some(port), cx);
                            }
                            Ok(Err(err)) => {
                                app.push_log(LogLevel::Error, format!("{err:#}"), cx);
                            }
                            Err(err) => {
                                app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx);
                            }
                        }
                    });
                }
            })
            .detach();
        }

        pub fn stop_dev_server(&mut self, cx: &mut Context<Self>) {
            self.dev_server_gen = self.dev_server_gen.wrapping_add(1);
            let was_active = self.dev_server_handle.is_some()
                || self.dev_server_status != DevServerStatus::Stopped;
            if let Some(handle_arc) = self.dev_server_handle.take() {
                let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
                rt.spawn(async move {
                    let mut guard = handle_arc.lock().await;
                    guard.stop().await;
                });
            }
            self.dev_server_status = DevServerStatus::Stopped;
            if was_active {
                self.push_log(LogLevel::Info, "Dev server stopped.", cx);
            }
            cx.notify();
        }

        pub fn start_debug_session(&mut self, cx: &mut Context<Self>) {
            if self.debug_mode == SessionMode::Attach {
                self.launch_debug_session_with_port(None, cx);
                return;
            }
            if self.project.root.is_none() {
                self.push_log(LogLevel::Warn, "Open a project first.", cx);
                return;
            }

            match self.dev_server_status {
                DevServerStatus::Running { port } => {
                    self.launch_debug_session_with_port(Some(port), cx);
                }
                DevServerStatus::Starting => {
                    let Some(srv_arc) = self.dev_server_handle.clone() else {
                        self.start_dev_server_internal(true, cx);
                        return;
                    };
                    let gen_id = self.dev_server_gen;
                    let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
                    let wait_handle = rt.spawn(async move {
                        let guard = srv_arc.lock().await;
                        guard.wait_ready(30_000).await
                    });
                    cx.spawn(async move |this, cx| {
                        let res = wait_handle.await;
                        let _ = this.update(cx, |app, cx| {
                            if app.dev_server_gen != gen_id {
                                return;
                            }
                            match res {
                                Ok(Ok(port)) => {
                                    app.dev_server_status = DevServerStatus::Running { port };
                                    app.launch_debug_session_with_port(Some(port), cx);
                                }
                                Ok(Err(err)) => {
                                    app.push_log(LogLevel::Error, format!("{err:#}"), cx);
                                }
                                Err(err) => {
                                    app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx);
                                }
                            }
                        });
                    })
                    .detach();
                }
                DevServerStatus::Stopped => {
                    self.start_dev_server_internal(true, cx);
                }
            }
        }

        fn launch_debug_session_with_port(
            &mut self,
            dev_server_port: Option<u16>,
            cx: &mut Context<Self>,
        ) {
            self.stop_debug_session(cx);
            let opts = match self.debug_mode {
                SessionMode::Attach => SessionOptions {
                    attach_port: Some(crate::config::DEFAULT_ATTACH_PORT),
                    ..Default::default()
                },
                SessionMode::Web => {
                    let Some(root) = self.project.root.clone() else {
                        self.push_log(LogLevel::Warn, "Open a project first.", cx);
                        return;
                    };
                    SessionOptions {
                        project_root: Some(root),
                        web: true,
                        port: Some(DEFAULT_WEB_PORT),
                        dev_server_port,
                        headless: false,
                        ..Default::default()
                    }
                }
                SessionMode::Native => {
                    let Some(root) = self.project.root.clone() else {
                        self.push_log(LogLevel::Warn, "Open a project first.", cx);
                        return;
                    };
                    SessionOptions {
                        project_root: Some(root),
                        web: false,
                        dev_server_port,
                        headless: self.debug_headless,
                        ..Default::default()
                    }
                }
            };

            self.debug_session_gen = self.debug_session_gen.wrapping_add(1);
            let session_gen = self.debug_session_gen;

            let (ev_tx, ev_rx) = async_channel::unbounded();
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);

            cx.spawn(async move |this, cx| {
                while let Ok(ev) = ev_rx.recv().await {
                    let mut should_break = false;
                    let _ = this.update(cx, |app, cx| {
                        if app.debug_session_gen != session_gen {
                            should_break = true;
                            return;
                        }
                        if matches!(ev, DebugEvent::SessionStopped) {
                            app.debug_session = None;
                            app.debug_selected_node = None;
                            app.debug_selected_props = None;
                            app.debug.apply(DebugEvent::SessionStopped);
                            app.push_log(
                                LogLevel::Info,
                                "Engine exited; debug session stopped.",
                                cx,
                            );
                            cx.notify();
                            should_break = true;
                            return;
                        }
                        if app.debug.apply(ev) {
                            cx.notify();
                        }
                    });
                    if should_break {
                        break;
                    }
                }
            })
            .detach();

            let handle = rt.spawn(async move { start_debug_session(opts, Some(ev_tx)).await });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| {
                    if app.debug_session_gen != session_gen {
                        return;
                    }
                    match res {
                        Ok(Ok(session)) => {
                            if session.is_child_finished() {
                                app.debug_session = None;
                                app.debug.apply(DebugEvent::SessionStopped);
                                cx.notify();
                                return;
                            }
                            let web_url = session.url.clone();
                            app.debug_session = Some(Arc::new(tokio::sync::Mutex::new(session)));
                            app.push_log(LogLevel::Success, "Debug session started.", cx);
                            if let Some(url) = web_url {
                                match open_url_in_browser(&url) {
                                    Ok(()) => app.push_log(
                                        LogLevel::Info,
                                        format!("Opened browser: {url}"),
                                        cx,
                                    ),
                                    Err(err) => app.push_log(
                                        LogLevel::Warn,
                                        format!("Failed to open browser: {err:#}"),
                                        cx,
                                    ),
                                }
                            }
                        }
                        Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                        Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                    }
                    cx.notify();
                });
            })
            .detach();
        }

        pub fn stop_debug_session(&mut self, cx: &mut Context<Self>) {
            self.debug_session_gen = self.debug_session_gen.wrapping_add(1);
            if let Some(session_arc) = self.debug_session.take() {
                let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
                rt.spawn(async move {
                    let mut guard = session_arc.lock().await;
                    guard.stop().await;
                });
                self.debug.apply(DebugEvent::SessionStopped);
                self.debug_selected_node = None;
                self.debug_selected_props = None;
                self.push_log(LogLevel::Info, "Debug session stopped.", cx);
                cx.notify();
            }
        }

        /// Synchronously terminate any running engine child process and dev server before window/app exit.
        pub fn stop_debug_session_sync(&mut self, cx: &mut Context<Self>) {
            if let Some(cancel) = self.active_cancel.take() {
                cancel.cancel();
            }
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            if let Some(session_arc) = self.debug_session.take() {
                rt.block_on(async move {
                    let mut guard = session_arc.lock().await;
                    guard.stop().await;
                });
                self.debug.apply(DebugEvent::SessionStopped);
            }
            if let Some(dev_arc) = self.dev_server_handle.take() {
                rt.block_on(async move {
                    let mut guard = dev_arc.lock().await;
                    guard.stop().await;
                });
                self.dev_server_status = DevServerStatus::Stopped;
            }
        }

        pub fn debug_fetch_tree(&mut self, cx: &mut Context<Self>) {
            let Some(session_arc) = self.debug_session.clone() else {
                return;
            };
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let host = session_arc.lock().await.host.clone();
                let res = host
                    .request("engine:tree", Some(serde_json::json!({ "depth": 8 })))
                    .await?;
                let node: NodeSummary = serde_json::from_value(
                    res.get("node")
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Missing node in engine:tree response"))?,
                )?;
                Ok::<_, anyhow::Error>(node)
            });

            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| match res {
                    Ok(Ok(node)) => {
                        app.debug.apply(DebugEvent::TreeUpdated(node));
                        cx.notify();
                    }
                    Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                    Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                });
            })
            .detach();
        }

        pub fn debug_select_node(&mut self, node_id: u32, cx: &mut Context<Self>) {
            self.debug_selected_node = Some(node_id);
            cx.notify();
            let Some(session_arc) = self.debug_session.clone() else {
                return;
            };
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let host = session_arc.lock().await.host.clone();
                host.request(
                    "engine:props",
                    Some(serde_json::json!({ "nodeId": node_id })),
                )
                .await
            });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| match res {
                    Ok(Ok(val)) => {
                        app.debug_selected_props = Some(val.get("node").cloned().unwrap_or(val));
                        cx.notify();
                    }
                    Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                    Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                });
            })
            .detach();
        }

        pub fn debug_run_eval(&mut self, cx: &mut Context<Self>) {
            let code = self.debug_eval_input.read(cx).text().trim().to_string();
            if code.is_empty() {
                return;
            }
            let Some(session_arc) = self.debug_session.clone() else {
                self.push_log(LogLevel::Warn, "Start a debug session first.", cx);
                return;
            };
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let host = session_arc.lock().await.host.clone();
                host.request(
                    "engine:eval",
                    Some(serde_json::json!({ "code": code, "timeoutMs": 5000 })),
                )
                .await
            });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| match res {
                    Ok(Ok(val)) => {
                        let out = match val.get("value") {
                            Some(v) => serde_json::to_string_pretty(v).unwrap_or_default(),
                            None => val
                                .get("repr")
                                .and_then(|v| v.as_str())
                                .unwrap_or("undefined")
                                .to_string(),
                        };
                        app.debug_eval_output = Some(out);
                        cx.notify();
                    }
                    Ok(Err(err)) => {
                        app.debug_eval_output = Some(format!("Error: {err:#}"));
                        cx.notify();
                    }
                    Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                });
            })
            .detach();
        }

        pub fn debug_subscribe_logs(&mut self, cx: &mut Context<Self>) {
            let Some(session_arc) = self.debug_session.clone() else {
                return;
            };
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let host = session_arc.lock().await.host.clone();
                let res = host
                    .request(
                        "engine:logs",
                        Some(serde_json::json!({ "subscribe": true })),
                    )
                    .await?;
                let mut entries = Vec::new();
                if let Some(arr) = res.get("entries").and_then(|v| v.as_array()) {
                    for raw in arr {
                        if let Ok(entry) = serde_json::from_value::<LogEntry>(raw.clone()) {
                            entries.push(entry);
                        }
                    }
                }
                Ok::<_, anyhow::Error>(entries)
            });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| match res {
                    Ok(Ok(entries)) => {
                        for e in entries {
                            app.debug.apply(DebugEvent::LogReceived(e));
                        }
                        cx.notify();
                    }
                    Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                    Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                });
            })
            .detach();
        }

        pub fn debug_capture_screenshot(&mut self, cx: &mut Context<Self>) {
            let Some(session_arc) = self.debug_session.clone() else {
                return;
            };
            let rt = Arc::clone(&cx.global::<TokioRuntime>().0);
            let handle = rt.spawn(async move {
                let host = session_arc.lock().await.host.clone();
                let res = host
                    .request(
                        "engine:screenshot",
                        Some(serde_json::json!({ "maxWidth": 1280, "maxHeight": 720 })),
                    )
                    .await?;
                let b64 = res
                    .get("data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing screenshot data"))?;
                use base64::Engine as _;
                let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
                let w = res.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                let h = res.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                let fmt = res
                    .get("format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("webp")
                    .to_string();
                Ok::<_, anyhow::Error>((bytes, format!("{w}x{h} {fmt}")))
            });
            cx.spawn(async move |this, cx| {
                let res = handle.await;
                let _ = this.update(cx, |app, cx| match res {
                    Ok(Ok((bytes, meta))) => {
                        let image = Arc::new(gpui::Image::from_bytes(
                            gpui::ImageFormat::Webp,
                            bytes.clone(),
                        ));
                        app.debug_screenshot_bytes = Some(bytes);
                        app.debug_screenshot_image = Some(image);
                        app.debug_screenshot_meta = Some(meta);
                        cx.notify();
                    }
                    Ok(Err(err)) => app.push_log(LogLevel::Error, format!("{err:#}"), cx),
                    Err(err) => app.push_log(LogLevel::Error, format!("Task panic: {err}"), cx),
                });
            })
            .detach();
        }

        pub fn debug_save_screenshot(&mut self, cx: &mut Context<Self>) {
            let Some(bytes) = self.debug_screenshot_bytes.clone() else {
                return;
            };
            let base_dir = self.project.root.clone().unwrap_or_else(std::env::temp_dir);
            let dir = base_dir.join(".moyu").join("screenshots");
            let filename = format!("shot-{}.webp", chrono::Utc::now().format("%Y%m%d%H%M%S"));
            let path = dir.join(filename);
            match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, bytes)) {
                Ok(()) => self.push_log(
                    LogLevel::Success,
                    format!("Saved screenshot to {}", path.display()),
                    cx,
                ),
                Err(err) => self.push_log(LogLevel::Error, format!("{err}"), cx),
            }
        }

        fn render_titlebar(
            &self,
            window: &Window,
            theme: Theme,
            i18n: I18n,
            cx: &mut Context<Self>,
        ) -> impl IntoElement {
            let is_maximized = window.is_maximized();
            let app_title = i18n.app_title();

            #[cfg(target_os = "macos")]
            {
                let _ = cx;
                let is_fullscreen = window.is_fullscreen();
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex_shrink_0()
                    .bg(theme.background)
                    .border_b_1()
                    .border_color(theme.rule)
                    .flex()
                    .flex_row()
                    .items_center()
                    .window_control_area(WindowControlArea::Drag)
                    .when(!is_fullscreen, |d| {
                        d.child(div().w(px(74.0)).flex_shrink_0())
                    })
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(6.0))
                            .child(img(self.app_icon.clone()).size(px(16.0)))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(app_title),
                            ),
                    )
                    .when(!is_fullscreen, |d| {
                        d.child(div().w(px(74.0)).flex_shrink_0())
                    })
            }

            #[cfg(not(target_os = "macos"))]
            {
                div()
                    .h(px(32.0))
                    .w_full()
                    .flex_shrink_0()
                    .bg(theme.background)
                    .border_b_1()
                    .border_color(theme.rule)
                    .flex()
                    .flex_row()
                    .items_center()
                    // Left title + middle spacer form one native drag/double-click caption area.
                    // Do NOT attach manual double-click `zoom_window()` here on Windows, because
                    // `WindowControlArea::Drag` (`HTCAPTION`) already toggles maximize/restore natively.
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .pl(px(10.0))
                            .window_control_area(WindowControlArea::Drag)
                            .child(img(self.app_icon.clone()).size(px(18.0)))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child(app_title),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .h_full()
                            .flex_shrink_0()
                            .child(
                                div()
                                    .id("win-btn-min")
                                    .w(px(46.0))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(move |s| s.bg(theme.muted))
                                    .window_control_area(WindowControlArea::Min)
                                    .when(!cfg!(windows), |d| {
                                        d.on_click(|_, window, _| window.minimize_window())
                                    })
                                    .child(icon(IconName::Minus, 16.0, theme.foreground)),
                            )
                            .child(
                                div()
                                    .id("win-btn-max")
                                    .w(px(46.0))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(move |s| s.bg(theme.muted))
                                    .window_control_area(WindowControlArea::Max)
                                    .when(!cfg!(windows), |d| {
                                        d.on_click(|_, window, _| window.zoom_window())
                                    })
                                    .child(icon(
                                        if is_maximized {
                                            IconName::Copy
                                        } else {
                                            IconName::Square
                                        },
                                        12.0,
                                        theme.foreground,
                                    )),
                            )
                            .child(
                                div()
                                    .id("win-btn-close")
                                    .w(px(46.0))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .hover(move |s| s.bg(theme.destructive))
                                    .window_control_area(WindowControlArea::Close)
                                    .when(!cfg!(windows), |d| {
                                        d.on_click(cx.listener(|this, _, window, cx| {
                                            this.stop_debug_session_sync(cx);
                                            window.remove_window();
                                        }))
                                    })
                                    .child(icon(IconName::X, 16.0, theme.foreground)),
                            ),
                    )
            }
        }

        fn render_topbar(
            &self,
            theme: Theme,
            i18n: I18n,
            cx: &mut Context<Self>,
        ) -> impl IntoElement {
            let active_tab = self.active_tab;
            let project_menu_open = self.active_popover == Some(ActivePopover::ProjectMenu);

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

            let current_root = self.project.root.clone();
            let recent_entries: Vec<(String, PathBuf, bool)> = self
                .global_config
                .recent_projects
                .iter()
                .map(|p| {
                    let is_cur = current_root.as_ref() == Some(p);
                    let name = if is_cur {
                        project_name.clone()
                    } else {
                        load_index_summary(p)
                            .ok()
                            .and_then(|s| s.window_title)
                            .or_else(|| p.file_name().map(|n| n.to_string_lossy().to_string()))
                            .unwrap_or_else(|| p.display().to_string())
                    };
                    (name, p.clone(), is_cur)
                })
                .collect();

            let tabs: &[(NavTab, &str)] = &[
                (NavTab::Development, i18n.tab_development()),
                (NavTab::Debug, i18n.tab_debug()),
                (NavTab::Engine, i18n.tab_engine()),
                (NavTab::Pack, i18n.tab_pack()),
            ];

            div()
                .relative()
                .h(px(36.0))
                .w_full()
                .flex_shrink_0()
                .px(px(8.0))
                .bg(theme.muted)
                .flex()
                .flex_row()
                .items_end()
                .gap(px(8.0))
                // Bottom separator sits behind the active tab so the active tab merges seamlessly
                // with the `theme.background` content area below.
                .child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(px(1.0))
                        .bg(theme.rule),
                )
                .child(
                    div()
                        .relative()
                        .flex_shrink_0()
                        .pb(px(3.0))
                        .child(
                            div()
                                .id("btn-project-picker")
                                .h(px(28.0))
                                .px(px(8.0))
                                .rounded(px(4.0))
                                .bg(if project_menu_open {
                                    theme.accent
                                } else {
                                    gpui::transparent_black()
                                })
                                .hover(move |s| {
                                    s.bg(theme.accent).text_color(theme.accent_foreground)
                                })
                                .text_size(px(13.0))
                                .line_height(px(18.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation();
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_popover(ActivePopover::ProjectMenu, cx);
                                }))
                                .child(icon(IconName::Folder, 14.0, theme.meta))
                                .child(div().max_w(px(160.0)).overflow_hidden().child(project_name))
                                .child(icon(IconName::ChevronDown, 14.0, theme.meta)),
                        )
                        .when(project_menu_open, |d| {
                            d.child(
                                deferred(
                                    popover_container("popover-project-menu", 320.0, theme)
                                        .absolute()
                                        .top(px(33.0))
                                        .left_0()
                                        .max_h(px(360.0))
                                        .overflow_y_scroll()
                                        .child(menu_section_label(i18n.recent_projects(), theme))
                                        .when(recent_entries.is_empty(), |m| {
                                            m.child(
                                                div()
                                                    .px(px(8.0))
                                                    .py(px(6.0))
                                                    .text_size(px(12.0))
                                                    .text_color(theme.meta)
                                                    .child("-"),
                                            )
                                        })
                                        .children(recent_entries.into_iter().enumerate().map(
                                            |(idx, (name, path, is_cur))| {
                                                let path_str = path.display().to_string();
                                                let open_path = path.clone();
                                                let remove_path = path;
                                                div()
                                                    .id(("recent-proj-item", idx))
                                                    .px(px(8.0))
                                                    .py(px(6.0))
                                                    .rounded(px(3.0))
                                                    .bg(if is_cur {
                                                        theme.accent
                                                    } else {
                                                        gpui::transparent_black()
                                                    })
                                                    .hover(move |s| {
                                                        s.bg(theme.accent)
                                                            .text_color(theme.accent_foreground)
                                                    })
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap(px(10.0))
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                        cx.stop_propagation();
                                                    })
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.open_project_path(
                                                            open_path.clone(),
                                                            cx,
                                                        );
                                                    }))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .gap(px(2.0))
                                                            .min_w_0()
                                                            .flex_1()
                                                            .child(
                                                                div()
                                                                    .text_size(px(13.0))
                                                                    .font_weight(
                                                                        gpui::FontWeight::MEDIUM,
                                                                    )
                                                                    .child(name),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(11.0))
                                                                    .text_color(theme.meta)
                                                                    .overflow_hidden()
                                                                    .child(path_str),
                                                            ),
                                                    )
                                                    .when(is_cur, |r| {
                                                        r.child(icon(
                                                            IconName::Check,
                                                            14.0,
                                                            theme.meta,
                                                        ))
                                                    })
                                                    .when(!is_cur, |r| {
                                                        r.child(
                                                            div()
                                                                .id(("recent-proj-remove", idx))
                                                                .w(px(22.0))
                                                                .h(px(22.0))
                                                                .flex_shrink_0()
                                                                .rounded(px(3.0))
                                                                .flex()
                                                                .items_center()
                                                                .justify_center()
                                                                .cursor_pointer()
                                                                .hover(move |s| {
                                                                    s.bg(theme
                                                                        .destructive
                                                                        .opacity(0.16))
                                                                })
                                                                .on_mouse_down(
                                                                    MouseButton::Left,
                                                                    |_, _, cx| {
                                                                        cx.stop_propagation();
                                                                    },
                                                                )
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        cx.stop_propagation();
                                                                        this.remove_recent_project(
                                                                            &remove_path,
                                                                            cx,
                                                                        );
                                                                    },
                                                                ))
                                                                .child(icon(
                                                                    IconName::X,
                                                                    13.0,
                                                                    theme.destructive,
                                                                )),
                                                        )
                                                    })
                                            },
                                        ))
                                        .child(menu_divider(theme))
                                        .child(
                                            div()
                                                .id("menu-new-project")
                                                .h(px(30.0))
                                                .px(px(8.0))
                                                .rounded(px(3.0))
                                                .hover(move |s| {
                                                    s.bg(theme.accent)
                                                        .text_color(theme.accent_foreground)
                                                })
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(8.0))
                                                .text_size(px(13.0))
                                                .cursor_pointer()
                                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                    cx.stop_propagation();
                                                })
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.open_create_project_dialog(cx);
                                                }))
                                                .child(icon(IconName::Plus, 14.0, theme.meta))
                                                .child(i18n.new_project()),
                                        )
                                        .child(
                                            div()
                                                .id("menu-open-project")
                                                .h(px(30.0))
                                                .px(px(8.0))
                                                .rounded(px(3.0))
                                                .hover(move |s| {
                                                    s.bg(theme.accent)
                                                        .text_color(theme.accent_foreground)
                                                })
                                                .flex()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(8.0))
                                                .text_size(px(13.0))
                                                .cursor_pointer()
                                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                    cx.stop_propagation();
                                                })
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.prompt_open_project(cx);
                                                }))
                                                .child(icon(IconName::FolderOpen, 14.0, theme.meta))
                                                .child(i18n.open_existing_project()),
                                        ),
                                )
                                .with_priority(5),
                            )
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .h_full()
                        .flex()
                        .flex_row()
                        .items_end()
                        .gap(px(2.0))
                        .children(tabs.iter().map(|&(tab, label)| {
                            let is_active = tab == active_tab;
                            div()
                                .id(("top-tab", tab as usize))
                                .relative()
                                .h(px(31.0))
                                .px(px(14.0))
                                .pb(px(1.0))
                                .rounded_t(px(4.0))
                                .bg(if is_active {
                                    theme.background
                                } else {
                                    gpui::transparent_black()
                                })
                                .text_color(if is_active {
                                    theme.foreground
                                } else {
                                    theme.meta
                                })
                                .text_size(px(13.0))
                                .line_height(px(18.0))
                                .font_weight(if is_active {
                                    gpui::FontWeight::MEDIUM
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .when(!is_active, |d| {
                                    d.hover(move |s| {
                                        s.bg(theme.accent).text_color(theme.accent_foreground)
                                    })
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.active_tab = tab;
                                    this.active_popover = None;
                                    cx.notify();
                                }))
                                .child(label)
                        })),
                )
                .child(
                    div().pb(px(3.0)).flex_shrink_0().child(
                        div()
                            .id("btn-open-settings")
                            .h(px(28.0))
                            .px(px(8.0))
                            .rounded(px(4.0))
                            .hover(move |s| s.bg(theme.accent).text_color(theme.accent_foreground))
                            .text_size(px(13.0))
                            .line_height(px(18.0))
                            .text_color(theme.foreground)
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(5.0))
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.active_popover = None;
                                this.active_dialog = Some(ActiveDialog::Settings);
                                cx.notify();
                            }))
                            .child(icon(IconName::Settings, 14.0, theme.meta))
                            .child(i18n.settings()),
                    ),
                )
        }
    }

    impl Render for DevtoolApp {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::global(cx);
            let i18n = I18n::new(self.global_config.locale);

            let page_body = match self.active_tab {
                NavTab::Development => development::render_development_view(self, theme, i18n, cx),
                NavTab::Engine => engine::render_engine_view(self, theme, i18n, cx),
                NavTab::Pack => pack::render_pack_view(self, theme, i18n, cx),
                NavTab::Debug => debug::render_debug_view(self, theme, i18n, cx),
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
                div()
                    .w_full()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(theme.card)
                    .border_b_1()
                    .border_color(theme.rule)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .child(div().text_color(theme.foreground).child(stage))
                                    .child(div().text_color(theme.meta).child(detail)),
                            )
                            .when(self.active_cancel.is_some(), |row| {
                                row.child(sm_button(
                                    "btn-cancel-active",
                                    i18n.stop_task(),
                                    ButtonVariant::Secondary,
                                    theme,
                                    cx.listener(|this, _, _, cx| this.cancel_active_task(cx)),
                                ))
                            }),
                    )
                    .child(progress_bar(ratio, theme))
            });

            let output_open = self.output_open;
            let status_color = if self.status_is_error {
                theme.destructive
            } else {
                theme.success
            };

            let output_drawer = output_open.then(|| {
                div()
                    .h(px(150.0))
                    .w_full()
                    .flex_shrink_0()
                    .bg(theme.card)
                    .border_t_1()
                    .border_color(theme.rule)
                    .p(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.foreground)
                                    .child(i18n.task_output()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(px(4.0))
                                    .child(sm_button(
                                        "btn-clear-logs",
                                        i18n.clear_logs(),
                                        ButtonVariant::Ghost,
                                        theme,
                                        cx.listener(|this, _, _, cx| {
                                            this.log_list.update(cx, |l, cx| l.clear(cx));
                                        }),
                                    ))
                                    .child(icon_button(
                                        "btn-close-output",
                                        IconName::X,
                                        theme,
                                        cx.listener(|this, _, _, cx| {
                                            this.output_open = false;
                                            cx.notify();
                                        }),
                                    )),
                            ),
                    )
                    .child(div().flex_1().w_full().child(self.log_list.clone()))
            });

            let statusbar = div()
                .h(px(28.0))
                .w_full()
                .flex_shrink_0()
                .px(px(8.0))
                .bg(theme.card)
                .border_t_1()
                .border_color(theme.rule)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .min_w_0()
                        .flex_1()
                        .child(status_dot(status_color))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .overflow_hidden()
                                .child(self.status_message.clone()),
                        ),
                )
                .child(
                    div()
                        .id("btn-toggle-output")
                        .h(px(22.0))
                        .px(px(6.0))
                        .rounded(px(3.0))
                        .hover(move |s| s.bg(theme.accent).text_color(theme.accent_foreground))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.output_open = !this.output_open;
                            cx.notify();
                        }))
                        .child(i18n.tasks_and_output())
                        .child(icon(
                            if output_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronUp
                            },
                            14.0,
                            theme.meta,
                        )),
                );

            let modal = match self.active_dialog {
                Some(ActiveDialog::Settings) => {
                    Some(dialogs::render_settings_dialog(self, theme, i18n, cx))
                }
                Some(ActiveDialog::CreateProject) => {
                    Some(dialogs::render_create_project_dialog(self, theme, i18n, cx))
                }
                Some(ActiveDialog::DownloadEngine) => Some(dialogs::render_download_engine_dialog(
                    self, theme, i18n, cx,
                )),
                None => None,
            };

            div()
                .relative()
                .size_full()
                .bg(theme.background)
                .text_color(theme.foreground)
                .flex()
                .flex_col()
                .when(self.active_popover.is_some(), |d| {
                    d.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.active_popover = None;
                            cx.notify();
                        }),
                    )
                })
                .child(self.render_titlebar(window, theme, i18n, cx))
                .child(self.render_topbar(theme, i18n, cx))
                .when_some(progress_banner, |d, b| d.child(b))
                .child(
                    div()
                        .flex_1()
                        .w_full()
                        .p(px(12.0))
                        .overflow_hidden()
                        .child(page_body),
                )
                .when_some(output_drawer, |d, drawer| d.child(drawer))
                .child(statusbar)
                .when_some(modal, |d, m| d.child(m))
        }
    }

    pub fn run() -> i32 {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to initialize background Tokio runtime");
        let rt = Arc::new(rt);

        gpui_platform::application()
            .with_assets(DevtoolAssets)
            .run(move |cx: &mut App| {
                let cfg = load_global_config();
                cx.set_global(TokioRuntime(Arc::clone(&rt)));
                cx.set_global(Theme::resolve(cfg.theme, cx.window_appearance()));
                register_key_bindings(cx);

                let i18n = I18n::new(cfg.locale);
                let bounds = Bounds::centered(None, size(px(1160.0), px(740.0)), cx);
                let _ = cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some(i18n.app_title().into()),
                            appears_transparent: true,
                            traffic_light_position: Some(point(px(12.0), px(9.0))),
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        cx.set_global(Theme::resolve(cfg.theme, window.appearance()));
                        let entity = cx.new(DevtoolApp::new);
                        let close_entity = entity.clone();
                        window.on_window_should_close(cx, move |_window, cx| {
                            close_entity.update(cx, |this, cx| {
                                this.stop_debug_session_sync(cx);
                            });
                            true
                        });
                        let quit_entity = entity.clone();
                        cx.on_app_quit(move |cx| {
                            quit_entity.update(cx, |this, cx| {
                                this.stop_debug_session_sync(cx);
                            });
                            async {}
                        })
                        .detach();
                        entity
                    },
                );
                cx.activate(true);
            });
        0
    }
}
