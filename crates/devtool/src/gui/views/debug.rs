//! V2 Visual Debugger view styled with FishFlow tokens (localized, scrollable tree & JSON).

use std::collections::BTreeSet;

use gpui::{AnyElement, Context, div, img, prelude::*, px};

use crate::gui::{ActivePopover, DevtoolApp};
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button, sm_button};
use crate::gui::ui::card::{muted_panel, panel, section_heading, status_dot};
use crate::gui::ui::dropdown::{menu_item, select_dropdown};
use crate::model::debug::{DebugConnectionStatus, NodeSummary, SessionMode};

struct FlatTreeRow {
    id: u32,
    node_type: String,
    label: String,
    visible: bool,
    depth: usize,
    has_children: bool,
    collapsed: bool,
}

fn flatten_tree(
    node: &NodeSummary,
    depth: usize,
    collapsed_ids: &BTreeSet<u32>,
    out: &mut Vec<FlatTreeRow>,
) {
    let has_children = !node.children.is_empty();
    let collapsed = collapsed_ids.contains(&node.id);
    out.push(FlatTreeRow {
        id: node.id,
        node_type: node.node_type.clone(),
        label: node.label.clone(),
        visible: node.visible,
        depth,
        has_children,
        collapsed,
    });
    if !collapsed {
        for child in &node.children {
            flatten_tree(child, depth + 1, collapsed_ids, out);
        }
    }
}

pub fn render_debug_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let running = app.debug_session.is_some();
    let mode = app.debug_mode;
    let headless = app.debug_headless;
    let mode_open = app.active_popover == Some(ActivePopover::DebugMode);

    let (status_color, status_text) = match &app.debug.status {
        DebugConnectionStatus::Stopped => (theme.meta, i18n.dbg_status_stopped().to_string()),
        DebugConnectionStatus::Listening { port } => {
            (theme.warning, i18n.dbg_status_listening(*port))
        }
        DebugConnectionStatus::Connected => (theme.info, i18n.dbg_status_connected().to_string()),
        DebugConnectionStatus::Ready => (theme.success, i18n.dbg_status_ready().to_string()),
    };

    let endpoint_hint = app
        .debug
        .attach_endpoint
        .as_ref()
        .map(|ep| format!("MOYU_ENGINE_DEBUG_WS={ep}"))
        .or_else(|| app.debug.web_url.clone());

    let mut tree_rows = Vec::new();
    if let Some(root) = &app.debug.tree {
        flatten_tree(root, 0, &app.debug_collapsed_nodes, &mut tree_rows);
    }
    let selected_node = app.debug_selected_node;
    let props_text = app
        .debug_selected_props
        .as_ref()
        .and_then(|v| serde_json::to_string_pretty(v).ok())
        .unwrap_or_else(|| i18n.dbg_props_empty().to_string());

    let eval_output = app
        .debug_eval_output
        .clone()
        .unwrap_or_else(|| i18n.dbg_eval_empty().to_string());

    let level_filter = app.debug_log_level_filter.clone();
    let filtered_logs: Vec<String> = app
        .debug
        .logs
        .iter()
        .rev()
        .filter(|entry| level_filter == "ALL" || entry.level.eq_ignore_ascii_case(&level_filter))
        .take(200)
        .map(|e| {
            format!(
                "[{}] {}: {}",
                e.level.to_ascii_uppercase(),
                e.target,
                e.message
            )
        })
        .collect();

    let screenshot_img = app.debug_screenshot_image.clone();
    let screenshot_meta = app.debug_screenshot_meta.clone();

    let mode_items: Vec<AnyElement> = [SessionMode::Native, SessionMode::Web, SessionMode::Attach]
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            menu_item(
                ("dbg-mode-opt", i),
                i18n.dbg_mode_name(m),
                mode == m,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.set_debug_mode(m, cx);
                }),
            )
            .into_any_element()
        })
        .collect();

    let heading = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pb(px(12.0))
        .mb(px(10.0))
        .border_b_1()
        .border_color(theme.rule)
        .child(
            div()
                .text_size(px(20.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(i18n.tab_debug()),
        )
        .child(if running {
            button(
                "dbg-stop",
                i18n.dbg_stop_session(),
                ButtonVariant::Danger,
                theme,
                cx.listener(|this, _, _, cx| this.stop_debug_session(cx)),
            )
            .into_any_element()
        } else {
            button(
                "dbg-start",
                i18n.dbg_start_session(),
                ButtonVariant::Primary,
                theme,
                cx.listener(|this, _, _, cx| this.start_debug_session(cx)),
            )
            .into_any_element()
        });

    let session_bar = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        .mb(px(10.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .child(i18n.dbg_session_mode_label()),
                )
                .child(select_dropdown(
                    "dbg-mode-select",
                    i18n.dbg_mode_name(mode),
                    mode_open,
                    150.0,
                    false,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::DebugMode, cx);
                    }),
                    mode_items,
                ))
                .when(mode == SessionMode::Native, |row| {
                    row.child(sm_button(
                        "dbg-toggle-headless",
                        if headless {
                            format!("✓ {}", i18n.dbg_headless())
                        } else {
                            i18n.dbg_windowed().to_string()
                        },
                        if headless {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Secondary
                        },
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.debug_headless = !this.debug_headless;
                            cx.notify();
                        }),
                    ))
                })
                .child(
                    div()
                        .ml(px(4.0))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(status_dot(status_color))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .child(status_text),
                        ),
                ),
        )
        .when(running, |row| {
            row.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(6.0))
                    .child(sm_button(
                        "dbg-refresh-tree",
                        i18n.dbg_refresh_tree(),
                        ButtonVariant::Secondary,
                        theme,
                        cx.listener(|this, _, _, cx| this.debug_fetch_tree(cx)),
                    ))
                    .child(sm_button(
                        "dbg-screenshot",
                        i18n.dbg_screenshot(),
                        ButtonVariant::Secondary,
                        theme,
                        cx.listener(|this, _, _, cx| this.debug_capture_screenshot(cx)),
                    ))
                    .child(sm_button(
                        "dbg-sub-logs",
                        i18n.dbg_fetch_logs(),
                        ButtonVariant::Secondary,
                        theme,
                        cx.listener(|this, _, _, cx| this.debug_subscribe_logs(cx)),
                    )),
            )
        });

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(heading)
        .child(session_bar)
        .when_some(endpoint_hint, |this, ep| {
            let ep_copy = ep.clone();
            this.child(
                div()
                    .mb(px(8.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(10.0))
                    .py(px(5.0))
                    .rounded(px(4.0))
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.rule)
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.meta)
                            .child(ep),
                    )
                    .child(sm_button(
                        "dbg-copy-ep",
                        i18n.dbg_copy(),
                        ButtonVariant::Ghost,
                        theme,
                        cx.listener(move |this, _, _, cx| {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                ep_copy.clone(),
                            ));
                            this.push_log(
                                crate::model::tasks::LogLevel::Info,
                                "Copied debug endpoint to clipboard.",
                                cx,
                            );
                        }),
                    )),
            )
        })
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_row()
                .gap(px(10.0))
                .overflow_hidden()
                .child(
                    panel(theme)
                        .w(px(320.0))
                        .h_full()
                        .overflow_hidden()
                        .child(section_heading(i18n.dbg_node_tree(), None, theme))
                        .child(
                            div()
                                .id("dbg-tree-scroll")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .pt(px(4.0))
                                .border_t_1()
                                .border_color(theme.rule)
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .when(tree_rows.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .py(px(6.0))
                                            .text_size(px(12.0))
                                            .text_color(theme.meta)
                                            .child(i18n.dbg_tree_empty()),
                                    )
                                })
                                .children(tree_rows.into_iter().map(|row| {
                                    let is_sel = selected_node == Some(row.id);
                                    let node_id = row.id;
                                    let prefix = if row.has_children {
                                        if row.collapsed { "▸ " } else { "▾ " }
                                    } else {
                                        "  "
                                    };
                                    let label_part = if row.label.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" \"{}\"", row.label)
                                    };
                                    let hidden_part = if row.visible { "" } else { " (hidden)" };
                                    let text = format!(
                                        "{prefix}#{} {}{label_part}{hidden_part}",
                                        row.id, row.node_type
                                    );
                                    div()
                                        .id(("tree-node", row.id as usize))
                                        .min_h(px(26.0))
                                        .pl(px(6.0 + row.depth as f32 * 14.0))
                                        .pr(px(8.0))
                                        .py(px(3.0))
                                        .rounded(px(3.0))
                                        .bg(if is_sel {
                                            theme.accent
                                        } else {
                                            gpui::transparent_black()
                                        })
                                        .hover(move |s| s.bg(theme.accent))
                                        .text_size(px(12.0))
                                        .text_color(if is_sel {
                                            theme.accent_foreground
                                        } else {
                                            theme.foreground
                                        })
                                        .flex()
                                        .items_center()
                                        .flex_shrink_0()
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if row.has_children
                                                && this.debug_selected_node == Some(node_id)
                                                && !this.debug_collapsed_nodes.remove(&node_id)
                                            {
                                                this.debug_collapsed_nodes.insert(node_id);
                                            }
                                            this.debug_select_node(node_id, cx);
                                        }))
                                        .child(text)
                                })),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .overflow_hidden()
                        .child(
                            panel(theme)
                                .flex_1()
                                .min_h(px(180.0))
                                .gap(px(8.0))
                                .overflow_hidden()
                                .child(section_heading(i18n.dbg_props_and_eval(), None, theme))
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(div().flex_1().child(app.debug_eval_input.clone()))
                                        .child(button(
                                            "dbg-run-eval",
                                            i18n.dbg_eval_run(),
                                            ButtonVariant::Primary,
                                            theme,
                                            cx.listener(|this, _, _, cx| this.debug_run_eval(cx)),
                                        )),
                                )
                                .child(
                                    muted_panel(theme).max_h(px(64.0)).child(
                                        div()
                                            .id("dbg-eval-scroll")
                                            .overflow_y_scroll()
                                            .text_size(px(12.0))
                                            .text_color(theme.foreground)
                                            .child(eval_output),
                                    ),
                                )
                                .child(
                                    muted_panel(theme)
                                        .flex_1()
                                        .min_h_0()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .id("dbg-props-scroll")
                                                .flex_1()
                                                .min_h_0()
                                                .overflow_y_scroll()
                                                .text_size(px(12.0))
                                                .text_color(theme.meta)
                                                .child(props_text),
                                        ),
                                ),
                        )
                        .when_some(screenshot_img, |this, image| {
                            this.child(
                                panel(theme)
                                    .flex_shrink_0()
                                    .gap(px(6.0))
                                    .child(section_heading(
                                        screenshot_meta
                                            .unwrap_or_else(|| i18n.dbg_screenshot().to_string()),
                                        Some(
                                            sm_button(
                                                "dbg-save-shot",
                                                i18n.dbg_save_screenshot(),
                                                ButtonVariant::Secondary,
                                                theme,
                                                cx.listener(|this, _, _, cx| {
                                                    this.debug_save_screenshot(cx)
                                                }),
                                            )
                                            .into_any_element(),
                                        ),
                                        theme,
                                    ))
                                    .child(img(image).h(px(130.0))),
                            )
                        })
                        .child(
                            panel(theme)
                                .h(px(170.0))
                                .flex_shrink_0()
                                .gap(px(6.0))
                                .overflow_hidden()
                                .child(section_heading(
                                    i18n.dbg_engine_logs(),
                                    Some(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .gap(px(4.0))
                                            .children(
                                                ["ALL", "ERROR", "WARN", "INFO", "DEBUG"]
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, &lvl)| {
                                                        let active = level_filter == lvl;
                                                        sm_button(
                                                            ("dbg-lvl", i),
                                                            i18n.dbg_log_level_label(lvl),
                                                            if active {
                                                                ButtonVariant::Primary
                                                            } else {
                                                                ButtonVariant::Ghost
                                                            },
                                                            theme,
                                                            cx.listener(move |this, _, _, cx| {
                                                                this.debug_log_level_filter =
                                                                    lvl.to_string();
                                                                cx.notify();
                                                            }),
                                                        )
                                                    }),
                                            )
                                            .into_any_element(),
                                    ),
                                    theme,
                                ))
                                .child(
                                    div()
                                        .id("dbg-logs-scroll")
                                        .flex_1()
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .pt(px(4.0))
                                        .border_t_1()
                                        .border_color(theme.rule)
                                        .flex()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .children(filtered_logs.into_iter().map(|line| {
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(theme.meta)
                                                .flex_shrink_0()
                                                .child(line)
                                        })),
                                ),
                        ),
                ),
        )
        .into_any_element()
}
