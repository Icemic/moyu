//! GPUI single-window application shell and Tokio async bridge.

#[cfg(feature = "gui")]
mod i18n;
#[cfg(feature = "gui")]
mod theme;
#[cfg(feature = "gui")]
mod ui;

#[cfg(not(feature = "gui"))]
pub fn run() -> i32 {
    eprintln!(
        "moyu-devtool was compiled without the `gui` feature.\nRebuild with default features (`cargo build -p moyu_devtool`) or pass a CLI subcommand."
    );
    1
}

#[cfg(feature = "gui")]
pub fn run() -> i32 {
    app::run()
}

#[cfg(feature = "gui")]
mod app {
    use std::path::PathBuf;
    use std::sync::Arc;

    use gpui::{
        App, Bounds, Context, Entity, Global, PathPromptOptions, Window, WindowBounds,
        WindowOptions, div, prelude::*, px, size,
    };

    use crate::config::{
        DevtoolGlobalConfig, LocalePreference, ThemePreference, load_global_config,
        save_global_config,
    };
    use crate::gui::i18n::I18n;
    use crate::gui::theme::Theme;
    use crate::gui::ui::badge::{BadgeTone, badge};
    use crate::gui::ui::button::{ButtonVariant, button};
    use crate::gui::ui::card::card;
    use crate::gui::ui::log_list::LogListView;
    use crate::gui::ui::text_input::{TextInput, register_key_bindings};
    use crate::model::DomainModel;
    use crate::model::project::ProjectModel;
    use crate::model::tasks::{LogLevel, TaskLogEntry};
    use crate::service::project::{check_multi_res_assets, load_project_opened_event};

    #[allow(dead_code)]
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

    struct DevtoolApp {
        active_tab: NavTab,
        global_config: DevtoolGlobalConfig,
        project: ProjectModel,
        ime_input: Entity<TextInput>,
        log_list: Entity<LogListView>,
    }

    impl DevtoolApp {
        fn new(cx: &mut Context<Self>) -> Self {
            let global_config = load_global_config();
            let i18n = I18n::new(global_config.locale);
            let ime_input = cx.new(|cx| TextInput::new(i18n.search_placeholder(), cx));
            let log_list = cx.new(|_| LogListView::new(i18n.logs_empty()));

            let mut app = Self {
                active_tab: NavTab::Project,
                global_config,
                project: ProjectModel::default(),
                ime_input,
                log_list,
            };

            if let Some(last) = app.global_config.recent_projects.first().cloned()
                && last.join("index.json").is_file()
            {
                app.open_project_path(last, cx);
            }

            app
        }

        fn push_log(&self, level: LogLevel, line: impl Into<String>, cx: &mut Context<Self>) {
            let entry = TaskLogEntry {
                task_id: 0,
                level,
                line: line.into(),
            };
            self.log_list.update(cx, |list, cx| list.push(entry, cx));
        }

        fn open_project_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
            match load_project_opened_event(&path) {
                Ok(ev) => {
                    self.project.apply(ev);
                    if let Ok(report) = check_multi_res_assets(&path) {
                        self.project
                            .apply(crate::model::project::ProjectEvent::MultiResChecked(report));
                    }
                    self.global_config.record_recent_project(path.clone());
                    let _ = save_global_config(&self.global_config);
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

        fn prompt_open_project(&mut self, cx: &mut Context<Self>) {
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
                        app.open_project_path(dir, cx);
                    });
                }
            })
            .detach();
        }

        fn set_theme(&mut self, pref: ThemePreference, window: &Window, cx: &mut Context<Self>) {
            self.global_config.theme = pref;
            let _ = save_global_config(&self.global_config);
            let next = Theme::resolve(pref, window.appearance());
            cx.set_global(next);
            cx.notify();
        }

        fn set_locale(&mut self, locale: LocalePreference, cx: &mut Context<Self>) {
            self.global_config.locale = locale;
            let _ = save_global_config(&self.global_config);
            let i18n = I18n::new(locale);
            self.ime_input
                .update(cx, |input, cx| input.set_placeholder(i18n.search_placeholder(), cx));
            self.log_list
                .update(cx, |list, cx| list.set_empty_label(i18n.logs_empty(), cx));
            cx.notify();
        }
    }

    impl Render for DevtoolApp {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(6.0))
                        .child(button(
                            "theme-light",
                            i18n.theme_light(),
                            if self.global_config.theme == ThemePreference::Light {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, window, cx| {
                                this.set_theme(ThemePreference::Light, window, cx)
                            }),
                        ))
                        .child(button(
                            "theme-dark",
                            i18n.theme_dark(),
                            if self.global_config.theme == ThemePreference::Dark {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, window, cx| {
                                this.set_theme(ThemePreference::Dark, window, cx)
                            }),
                        ))
                        .child(button(
                            "theme-system",
                            i18n.theme_system(),
                            if self.global_config.theme == ThemePreference::System {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, window, cx| {
                                this.set_theme(ThemePreference::System, window, cx)
                            }),
                        ))
                        .child(button(
                            "locale-toggle",
                            match self.global_config.locale {
                                LocalePreference::ZhCn => "中文",
                                LocalePreference::EnUs => "EN",
                            },
                            ButtonVariant::Ghost,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                let next = match this.global_config.locale {
                                    LocalePreference::ZhCn => LocalePreference::EnUs,
                                    LocalePreference::EnUs => LocalePreference::ZhCn,
                                };
                                this.set_locale(next, cx);
                            }),
                        )),
                );

            let project_path_text = self
                .project
                .root
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| i18n.no_project_selected().to_string());

            let multi_res_badge = match &self.project.multi_res_report {
                Some(r) if r.is_clean() => badge("Multi-Res OK", BadgeTone::Success, theme),
                Some(_) => badge("Multi-Res Warnings", BadgeTone::Warn, theme),
                None => badge("Idle", BadgeTone::Neutral, theme),
            };

            let recent_list = self.global_config.recent_projects.clone();
            let main_body = div()
                .flex_1()
                .h_full()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(header)
                .child(
                    card(theme)
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(theme.text_secondary)
                                        .child(project_path_text),
                                )
                                .child(multi_res_badge),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_tertiary)
                                .child(i18n.stage_placeholder_hint()),
                        )
                        .child(self.ime_input.clone()),
                )
                .when(!recent_list.is_empty(), |this| {
                    this.child(
                        card(theme)
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(theme.text_secondary)
                                    .child(i18n.recent_projects()),
                            )
                            .children(recent_list.into_iter().enumerate().map(|(idx, p)| {
                                let label = p.display().to_string();
                                div()
                                    .id(("recent-proj", idx))
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .rounded(px(4.0))
                                    .bg(theme.control_idle)
                                    .hover(move |s| s.bg(theme.control_hover))
                                    .text_size(px(12.0))
                                    .text_color(theme.text)
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_project_path(p.clone(), cx);
                                    }))
                                    .child(label)
                            })),
                    )
                })
                .child(
                    card(theme)
                        .flex_1()
                        .min_h(px(140.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.text_secondary)
                                .child(i18n.logs_title()),
                        )
                        .child(div().flex_1().w_full().child(self.log_list.clone())),
                );

            let _ = window;
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
            let bounds = Bounds::centered(None, size(px(1080.0), px(720.0)), cx);
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
