//! Main Development workspace: project header, editor/run split buttons, inline config editor,
//! multi-res asset check, environment panel, and Schema generation.

use gpui::{AnyElement, Context, MouseButton, deferred, div, prelude::*, px};

use crate::gui::{ActivePopover, DevtoolApp, RunMode};
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button, sm_button};
use crate::gui::ui::card::{detail_row, panel, section_heading, status_dot};
use crate::gui::ui::dropdown::{
    menu_divider, menu_item, menu_section_label, popover_container, select_dropdown,
};
use crate::gui::ui::icon::{IconName, icon};
use crate::service::editor::LauncherKind;

pub fn render_development_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let Some(root) = &app.project.root else {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                panel(theme)
                    .w(px(380.0))
                    .p(px(24.0))
                    .gap(px(14.0))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(i18n.no_project_selected()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.0))
                            .child(button(
                                "empty-open-proj",
                                i18n.open_existing_project(),
                                ButtonVariant::Primary,
                                theme,
                                cx.listener(|this, _, _, cx| this.prompt_open_project(cx)),
                            ))
                            .child(button(
                                "empty-new-proj",
                                i18n.new_project(),
                                ButtonVariant::Secondary,
                                theme,
                                cx.listener(|this, _, _, cx| this.open_create_project_dialog(cx)),
                            )),
                    ),
            )
            .into_any_element();
    };

    let project_title = app
        .project
        .index_summary
        .as_ref()
        .and_then(|s| s.window_title.clone())
        .or_else(|| root.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| root.display().to_string());
    let project_path = root.display().to_string();

    let selected_editor = app.current_editor().cloned();
    let has_editor = selected_editor.is_some();
    let editor_label = match &selected_editor {
        Some(ed) => i18n.open_with_editor(ed.name),
        None => i18n.no_editor_detected().to_string(),
    };
    let editor_menu_open = app.active_popover == Some(ActivePopover::EditorMenu);

    let is_running = app.debug_session.is_some();
    let run_mode = app.run_mode;
    let run_label = if is_running {
        i18n.stop_running()
    } else {
        match run_mode {
            RunMode::Desktop => i18n.run_desktop(),
            RunMode::Browser => i18n.run_browser(),
        }
    };
    let run_menu_open = app.active_popover == Some(ActivePopover::RunModeMenu);
    let running_web_url = app.debug.web_url.clone();

    let intro = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .pb(px(12.0))
        .mb(px(12.0))
        .border_b_1()
        .border_color(theme.rule)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .min_w_0()
                .child(
                    div()
                        .text_size(px(20.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(project_title),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .child(project_path)
                        .child(
                            div()
                                .id("btn-open-project-folder")
                                .h(px(22.0))
                                .px(px(6.0))
                                .rounded(px(4.0))
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0))
                                .flex_shrink_0()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .cursor_pointer()
                                .hover(move |s| {
                                    s.bg(theme.accent).text_color(theme.accent_foreground)
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.open_project_folder(cx);
                                }))
                                .child(icon(IconName::FolderOpen, 13.0, theme.meta))
                                .child(i18n.open_folder()),
                        )
                        .when_some(running_web_url, |d, url| {
                            d.child(
                                div()
                                    .px(px(6.0))
                                    .py(px(1.0))
                                    .rounded(px(3.0))
                                    .bg(theme.secondary)
                                    .text_color(theme.success)
                                    .child(url),
                            )
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .flex_shrink_0()
                .child(render_editor_split_button(
                    app,
                    &editor_label,
                    has_editor,
                    editor_menu_open,
                    theme,
                    i18n,
                    cx,
                ))
                .child(render_run_split_button(
                    run_label,
                    run_mode,
                    is_running,
                    run_menu_open,
                    theme,
                    i18n,
                    cx,
                )),
        );

    let left_col = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(render_config_panel(app, theme, i18n, cx))
        .child(render_multi_res_panel(app, theme, i18n, cx));

    let right_col = div()
        .w(px(380.0))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(render_env_panel(app, theme, i18n, cx))
        .child(render_schema_panel(theme, i18n, cx));

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(intro)
        .child(
            div()
                .flex_1()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(10.0))
                .child(left_col)
                .child(right_col),
        )
        .into_any_element()
}

fn launcher_icon(kind: LauncherKind) -> IconName {
    match kind {
        LauncherKind::Editor => IconName::Code,
        LauncherKind::Terminal => IconName::Terminal,
        LauncherKind::Ai => IconName::Sparkles,
    }
}

fn render_editor_split_button(
    app: &DevtoolApp,
    label: &str,
    has_editor: bool,
    menu_open: bool,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let selected = app.current_editor();
    let selected_id = selected.map(|e| e.id);
    let main_icon = selected
        .map(|e| launcher_icon(e.kind))
        .unwrap_or(IconName::Code);
    let editors = app.detected_editors.clone();

    let groups: Vec<(&'static str, Vec<(usize, crate::service::editor::DetectedEditor)>)> = [
        (LauncherKind::Editor, i18n.detected_editors()),
        (LauncherKind::Terminal, i18n.detected_terminals()),
        (LauncherKind::Ai, i18n.detected_ai_tools()),
    ]
    .into_iter()
    .filter_map(|(kind, heading)| {
        let items: Vec<_> = editors
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == kind)
            .map(|(i, e)| (i, e.clone()))
            .collect();
        (!items.is_empty()).then_some((heading, items))
    })
    .collect();

    div()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .child(
            div()
                .id("btn-open-editor-main")
                .h(px(32.0))
                .px(px(11.0))
                .rounded_l(px(4.0))
                .bg(theme.secondary)
                .text_color(if has_editor {
                    theme.secondary_foreground
                } else {
                    theme.muted_foreground
                })
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .when(has_editor, |d| {
                    d.cursor_pointer()
                        .hover(move |s| s.bg(theme.secondary_hover))
                        .on_click(cx.listener(|this, _, _, cx| this.launch_selected_editor(cx)))
                })
                .child(icon(
                    main_icon,
                    14.0,
                    if has_editor {
                        theme.secondary_foreground
                    } else {
                        theme.muted_foreground
                    },
                ))
                .child(label.to_string()),
        )
        .child(
            div()
                .id("btn-open-editor-menu")
                .h(px(32.0))
                .w(px(28.0))
                .rounded_r(px(4.0))
                .border_l_1()
                .border_color(theme.rule)
                .bg(theme.secondary)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(theme.secondary_hover))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_popover(ActivePopover::EditorMenu, cx);
                }))
                .child(icon(IconName::ChevronDown, 14.0, theme.secondary_foreground)),
        )
        .when(menu_open, |d| {
            let mut popup = popover_container("popover-editor-menu", 210.0, theme)
                .absolute()
                .top(px(36.0))
                .right_0()
                .max_h(px(380.0))
                .overflow_y_scroll();

            if groups.is_empty() {
                popup = popup.child(
                    div()
                        .px(px(8.0))
                        .py(px(6.0))
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .child(i18n.no_editor_detected()),
                );
            } else {
                for (g_idx, (heading, items)) in groups.into_iter().enumerate() {
                    if g_idx > 0 {
                        popup = popup.child(menu_divider(theme));
                    }
                    popup = popup.child(menu_section_label(heading, theme));
                    for (i, ed) in items {
                        let is_cur = selected_id == Some(ed.id);
                        let ed_id = ed.id;
                        popup = popup.child(menu_item(
                            ("editor-opt", i),
                            ed.name,
                            is_cur,
                            theme,
                            cx.listener(move |this, _, _, cx| {
                                this.select_editor(ed_id, cx);
                            }),
                        ));
                    }
                }
            }

            popup = popup
                .child(menu_divider(theme))
                .child(menu_item(
                    "editor-opt-redetect",
                    i18n.redetect(),
                    false,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.active_popover = None;
                        this.refresh_editors(cx);
                    }),
                ));

            d.child(deferred(popup).with_priority(5))
        })
        .into_any_element()
}

fn render_run_split_button(
    label: &str,
    run_mode: RunMode,
    is_running: bool,
    menu_open: bool,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let main_bg = if is_running {
        theme.destructive
    } else {
        theme.primary
    };
    let main_hover = if is_running {
        theme.destructive_hover
    } else {
        theme.primary_hover
    };
    let main_fg = if is_running {
        theme.destructive_foreground
    } else {
        theme.primary_foreground
    };

    div()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .child(
            div()
                .id("btn-run-main")
                .h(px(32.0))
                .px(px(11.0))
                .rounded_l(px(4.0))
                .bg(main_bg)
                .text_color(main_fg)
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .cursor_pointer()
                .hover(move |s| s.bg(main_hover))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_run_preview(cx)))
                .child(icon(
                    if is_running {
                        IconName::Square
                    } else {
                        IconName::Play
                    },
                    13.0,
                    main_fg,
                ))
                .child(label.to_string()),
        )
        .child(
            div()
                .id("btn-run-mode-menu")
                .h(px(32.0))
                .w(px(28.0))
                .rounded_r(px(4.0))
                .border_l_1()
                .border_color(theme.rule)
                .bg(main_bg)
                .flex()
                .items_center()
                .justify_center()
                .when(!is_running, |d| {
                    d.cursor_pointer()
                        .hover(move |s| s.bg(main_hover))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_popover(ActivePopover::RunModeMenu, cx);
                        }))
                })
                .child(icon(
                    IconName::ChevronDown,
                    14.0,
                    if is_running {
                        theme.muted_foreground
                    } else {
                        main_fg
                    },
                )),
        )
        .when(menu_open && !is_running, |d| {
            d.child(
                deferred(
                    popover_container("popover-run-mode", 150.0, theme)
                        .absolute()
                        .top(px(36.0))
                        .right_0()
                        .child(menu_item(
                            "run-opt-desktop",
                            i18n.run_desktop(),
                            run_mode == RunMode::Desktop,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.set_run_mode(RunMode::Desktop, cx);
                            }),
                        ))
                        .child(menu_item(
                            "run-opt-browser",
                            i18n.run_browser(),
                            run_mode == RunMode::Browser,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.set_run_mode(RunMode::Browser, cx);
                            }),
                        )),
                )
                .with_priority(5),
            )
        })
        .into_any_element()
}

fn render_config_panel(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let editing = app.config_editing;
    let index = app.project.index_summary.as_ref();
    let window_title = index
        .and_then(|s| s.window_title.as_deref())
        .unwrap_or("-");
    let entry_filename = index
        .and_then(|s| s.entry_filename.as_deref())
        .unwrap_or("-");
    let window_size = index
        .and_then(|s| s.initial_surface_size.as_deref())
        .unwrap_or("-");
    let stage_size = index
        .and_then(|s| s.stage_size.as_deref())
        .unwrap_or("-");
    let engine_channel = app
        .project
        .moyu_summary
        .as_ref()
        .and_then(|m| m.engine_channel.as_deref())
        .unwrap_or(&app.engine.config.channel);
    let framework_name = app
        .project
        .framework_summary
        .as_ref()
        .and_then(|f| f.name.clone().or_else(|| f.title.clone()));

    let channel_options = app.available_config_channels();
    let edit_channel = app.config_channel_draft.clone();
    let ch_menu_open = app.active_popover == Some(ActivePopover::ConfigChannel);
    let config_err = app.config_save_error.clone();

    let ch_items: Vec<AnyElement> = channel_options
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let sel = ch == edit_channel;
            let ch_clone = ch.clone();
            menu_item(
                ("cfg-ch-opt", i),
                ch,
                sel,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.config_channel_draft = ch_clone.clone();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let actions = if editing {
        div()
            .flex()
            .flex_row()
            .gap(px(6.0))
            .child(sm_button(
                "btn-config-cancel",
                i18n.cancel(),
                ButtonVariant::Secondary,
                theme,
                cx.listener(|this, _, _, cx| this.cancel_config_edit(cx)),
            ))
            .child(sm_button(
                "btn-config-save",
                i18n.save(),
                ButtonVariant::Primary,
                theme,
                cx.listener(|this, _, _, cx| this.save_config_edit(cx)),
            ))
            .into_any_element()
    } else {
        sm_button(
            "btn-config-edit",
            i18n.edit(),
            ButtonVariant::Secondary,
            theme,
            cx.listener(|this, _, _, cx| this.begin_config_edit(cx)),
        )
        .into_any_element()
    };

    let config_row = |label: &str, value_el: AnyElement| {
        detail_row(theme)
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme.meta)
                    .flex_shrink_0()
                    .child(label.to_string()),
            )
            .child(value_el)
    };

    let value_text = |v: &str| {
        div()
            .text_size(px(13.0))
            .text_color(theme.foreground)
            .child(v.to_string())
            .into_any_element()
    };

    panel(theme)
        .child(section_heading(
            i18n.project_config(),
            Some(actions),
            theme,
        ))
        .when_some(config_err, |p, err| {
            p.child(
                div()
                    .mb(px(8.0))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.destructive)
                    .text_size(px(12.0))
                    .text_color(theme.destructive)
                    .child(err),
            )
        })
        .child(config_row(
            i18n.window_title_label(),
            if editing {
                div()
                    .w(px(260.0))
                    .child(app.cfg_title_input.clone())
                    .into_any_element()
            } else {
                value_text(window_title)
            },
        ))
        .child(config_row(
            i18n.entry_script_label(),
            if editing {
                div()
                    .w(px(260.0))
                    .child(app.cfg_entry_input.clone())
                    .into_any_element()
            } else {
                value_text(entry_filename)
            },
        ))
        .child(config_row(
            i18n.window_size_label(),
            if editing {
                div()
                    .w(px(260.0))
                    .child(app.cfg_window_size_input.clone())
                    .into_any_element()
            } else {
                value_text(window_size)
            },
        ))
        .child(config_row(
            i18n.stage_size_label(),
            if editing {
                div()
                    .w(px(260.0))
                    .child(app.cfg_stage_size_input.clone())
                    .into_any_element()
            } else {
                value_text(stage_size)
            },
        ))
        .child(config_row(
            i18n.engine_channel_label(),
            if editing {
                select_dropdown(
                    "cfg-ch-select",
                    edit_channel,
                    ch_menu_open,
                    260.0,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::ConfigChannel, cx);
                    }),
                    ch_items,
                )
            } else {
                value_text(engine_channel)
            },
        ))
        .when_some(framework_name, |p, fw| {
            p.child(config_row(i18n.framework_label(), value_text(&fw)))
        })
        .into_any_element()
}

fn render_multi_res_panel(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let (dot_color, status_text) = match &app.project.multi_res_report {
        Some(r) if r.is_clean() => (theme.success, i18n.check_passed().to_string()),
        Some(r) => (
            theme.warning,
            i18n.multi_res_issues(
                r.all_unused_when_off,
                r.unused_by_scale.len(),
                r.missing_base.len(),
            ),
        ),
        None => (theme.meta, "-".to_string()),
    };

    panel(theme)
        .child(section_heading(
            i18n.multi_res_title(),
            Some(
                sm_button(
                    "btn-recheck-assets",
                    i18n.recheck(),
                    ButtonVariant::Secondary,
                    theme,
                    cx.listener(|this, _, _, cx| this.recheck_multi_res(cx)),
                )
                .into_any_element(),
            ),
            theme,
        ))
        .child(
            detail_row(theme).child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(status_dot(dot_color))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.meta)
                            .child(status_text),
                    ),
            ),
        )
        .into_any_element()
}

fn render_env_panel(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let (node_found, node_status, node_path) = match &app.env.node {
        Some(n) => {
            let src = if n.is_managed {
                i18n.node_source_managed()
            } else {
                i18n.node_source_system()
            };
            (
                true,
                format!("{} · {src}", n.version),
                n.executable.display().to_string(),
            )
        }
        None => (false, i18n.not_found_label().to_string(), "-".to_string()),
    };

    let pm_label = app
        .env
        .package_manager
        .map(|pm| match pm {
            crate::model::env::PackageManager::Npm => "npm",
            crate::model::env::PackageManager::Yarn => "Yarn",
            crate::model::env::PackageManager::Pnpm => "pnpm",
        })
        .unwrap_or("-");

    let jiti_found = app.env.jiti_path.is_some();

    panel(theme)
        .child(section_heading(
            i18n.dev_env_title(),
            Some(
                sm_button(
                    "btn-redetect-env",
                    i18n.redetect(),
                    ButtonVariant::Secondary,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.refresh_env(cx);
                        this.refresh_editors(cx);
                    }),
                )
                .into_any_element(),
            ),
            theme,
        ))
        .child(
            detail_row(theme)
                .min_h(px(46.0))
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
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child("Node.js"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.meta)
                                .overflow_hidden()
                                .child(node_path),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .flex_shrink_0()
                        .child(status_dot(if node_found {
                            theme.success
                        } else {
                            theme.destructive
                        }))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .child(node_status),
                        ),
                ),
        )
        .child(
            detail_row(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.meta)
                        .child(i18n.project_pm_label()),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.foreground)
                        .child(pm_label),
                ),
        )
        .child(
            detail_row(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.meta)
                        .child("jiti"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(status_dot(if jiti_found {
                            theme.success
                        } else {
                            theme.warning
                        }))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .child(if jiti_found {
                                    i18n.found_label()
                                } else {
                                    i18n.not_found_label()
                                }),
                        ),
                ),
        )
        .child(
            div()
                .pt(px(8.0))
                .border_t_1()
                .border_color(theme.rule)
                .flex()
                .flex_row()
                .justify_end()
                .child(sm_button(
                    "btn-install-node",
                    i18n.install_managed_node(),
                    ButtonVariant::Secondary,
                    theme,
                    cx.listener(|this, _, _, cx| this.start_install_managed_node(cx)),
                )),
        )
        .into_any_element()
}

fn render_schema_panel(
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    panel(theme)
        .child(section_heading(i18n.schema_title(), None, theme))
        .child(
            detail_row(theme)
                .min_h(px(46.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(i18n.scenario_commands()),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.meta)
                                .child("commands.schema.json"),
                        ),
                )
                .child(sm_button(
                    "btn-gen-cmd-schema",
                    i18n.generate(),
                    ButtonVariant::Secondary,
                    theme,
                    cx.listener(|this, _, _, cx| this.start_generate_schema(false, cx)),
                )),
        )
        .child(
            detail_row(theme)
                .min_h(px(46.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(i18n.game_ui()),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.meta)
                                .child("ui.schema.json"),
                        ),
                )
                .child(sm_button(
                    "btn-gen-ui-schema",
                    i18n.generate(),
                    ButtonVariant::Secondary,
                    theme,
                    cx.listener(|this, _, _, cx| this.start_generate_schema(true, cx)),
                )),
        )
        .into_any_element()
}
