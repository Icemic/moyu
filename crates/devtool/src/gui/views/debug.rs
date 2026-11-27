//! V2 Visual Debugger view: session bar, collapsible TreeView, props inspector, eval, screenshot, and logs.

use std::collections::BTreeSet;

use gpui::{AnyElement, Context, div, img, prelude::*, px};

use crate::gui::DevtoolApp;
use crate::gui::theme::Theme;
use crate::gui::ui::badge::{BadgeTone, badge};
use crate::gui::ui::button::{ButtonVariant, button};
use crate::gui::ui::card::{card, muted_panel};
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
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let running = app.debug_session.is_some();
    let mode = app.debug_mode;
    let headless = app.debug_headless;

    let status_badge = match &app.debug.status {
        DebugConnectionStatus::Stopped => badge("Stopped", BadgeTone::Neutral, theme),
        DebugConnectionStatus::Listening { port } => {
            badge(format!("Listening :{port}"), BadgeTone::Warn, theme)
        }
        DebugConnectionStatus::Connected => badge("Connected", BadgeTone::Accent, theme),
        DebugConnectionStatus::Ready => badge("Ready", BadgeTone::Success, theme),
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
        .unwrap_or_else(|| "Select a node in the tree to inspect its properties.".to_string());

    let eval_output = app
        .debug_eval_output
        .clone()
        .unwrap_or_else(|| "No evaluation result yet.".to_string());

    let level_filter = app.debug_log_level_filter.clone();
    let filtered_logs: Vec<String> = app
        .debug
        .logs
        .iter()
        .rev()
        .filter(|entry| level_filter == "ALL" || entry.level.eq_ignore_ascii_case(&level_filter))
        .take(40)
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

    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .size_full()
        .child(
            card(theme)
                .p(px(12.0))
                .gap(px(8.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(status_badge)
                                .child(button(
                                    "dbg-mode-native",
                                    "Native",
                                    if mode == SessionMode::Native {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Default
                                    },
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.debug_mode = SessionMode::Native;
                                        cx.notify();
                                    }),
                                ))
                                .child(button(
                                    "dbg-mode-web",
                                    "Web",
                                    if mode == SessionMode::Web {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Default
                                    },
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.debug_mode = SessionMode::Web;
                                        cx.notify();
                                    }),
                                ))
                                .child(button(
                                    "dbg-mode-attach",
                                    "Attach",
                                    if mode == SessionMode::Attach {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Default
                                    },
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.debug_mode = SessionMode::Attach;
                                        cx.notify();
                                    }),
                                ))
                                .when(mode == SessionMode::Native, |row| {
                                    row.child(button(
                                        "dbg-toggle-headless",
                                        if headless { "✓ Headless" } else { "Windowed" },
                                        ButtonVariant::Default,
                                        theme,
                                        cx.listener(|this, _, _, cx| {
                                            this.debug_headless = !this.debug_headless;
                                            cx.notify();
                                        }),
                                    ))
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(6.0))
                                .child(if running {
                                    button(
                                        "dbg-stop",
                                        "Stop Session",
                                        ButtonVariant::Danger,
                                        theme,
                                        cx.listener(|this, _, _, cx| this.stop_debug_session(cx)),
                                    )
                                    .into_any_element()
                                } else {
                                    button(
                                        "dbg-start",
                                        "Start Session",
                                        ButtonVariant::Primary,
                                        theme,
                                        cx.listener(|this, _, _, cx| this.start_debug_session(cx)),
                                    )
                                    .into_any_element()
                                })
                                .when(running, |row| {
                                    row.child(button(
                                        "dbg-refresh-tree",
                                        "Refresh Tree",
                                        ButtonVariant::Default,
                                        theme,
                                        cx.listener(|this, _, _, cx| this.debug_fetch_tree(cx)),
                                    ))
                                    .child(button(
                                        "dbg-screenshot",
                                        "Screenshot",
                                        ButtonVariant::Default,
                                        theme,
                                        cx.listener(|this, _, _, cx| {
                                            this.debug_capture_screenshot(cx)
                                        }),
                                    ))
                                    .child(button(
                                        "dbg-sub-logs",
                                        "Fetch Logs",
                                        ButtonVariant::Default,
                                        theme,
                                        cx.listener(|this, _, _, cx| this.debug_subscribe_logs(cx)),
                                    ))
                                }),
                        ),
                )
                .when_some(endpoint_hint, |this, ep| {
                    let ep_copy = ep.clone();
                    this.child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_muted)
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.text_secondary)
                                    .child(ep),
                            )
                            .child(button(
                                "dbg-copy-ep",
                                "Copy",
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
                }),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_row()
                .gap(px(10.0))
                .overflow_hidden()
                .child(
                    card(theme)
                        .w(px(300.0))
                        .h_full()
                        .p(px(12.0))
                        .overflow_hidden()
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.text_secondary)
                                .child("Node Tree"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .overflow_hidden()
                                .when(tree_rows.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text_tertiary)
                                            .child("Start a session and click Refresh Tree."),
                                    )
                                })
                                .children(tree_rows.into_iter().take(60).map(|row| {
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
                                        .pl(px(8.0 + row.depth as f32 * 14.0))
                                        .pr(px(6.0))
                                        .py(px(3.0))
                                        .rounded(px(4.0))
                                        .bg(if is_sel {
                                            theme.accent_soft
                                        } else {
                                            gpui::transparent_black()
                                        })
                                        .hover(move |s| {
                                            s.bg(if is_sel {
                                                theme.accent_soft
                                            } else {
                                                theme.control_hover
                                            })
                                        })
                                        .text_size(px(12.0))
                                        .text_color(if is_sel { theme.accent } else { theme.text })
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if row.has_children
                                                && this.debug_selected_node == Some(node_id)
                                                && !this.debug_collapsed_nodes.remove(&node_id) {
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
                        .h_full()
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .overflow_hidden()
                        .child(
                            card(theme)
                                .p(px(12.0))
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(theme.text_secondary)
                                        .child("Node Properties & Eval"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .gap(px(6.0))
                                        .child(div().flex_1().child(app.debug_eval_input.clone()))
                                        .child(button(
                                            "dbg-run-eval",
                                            "Eval",
                                            ButtonVariant::Primary,
                                            theme,
                                            cx.listener(|this, _, _, cx| this.debug_run_eval(cx)),
                                        )),
                                )
                                .child(
                                    muted_panel(theme).child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text)
                                            .child(format!("Eval: {eval_output}")),
                                    ),
                                )
                                .child(
                                    muted_panel(theme).max_h(px(120.0)).overflow_hidden().child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.text_secondary)
                                            .child(props_text),
                                    ),
                                ),
                        )
                        .when_some(screenshot_img, |this, image| {
                            this.child(
                                card(theme)
                                    .p(px(10.0))
                                    .gap(px(6.0))
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
                                                    .child(screenshot_meta.unwrap_or_else(|| {
                                                        "Screenshot".to_string()
                                                    })),
                                            )
                                            .child(button(
                                                "dbg-save-shot",
                                                "Save",
                                                ButtonVariant::Default,
                                                theme,
                                                cx.listener(|this, _, _, cx| {
                                                    this.debug_save_screenshot(cx)
                                                }),
                                            )),
                                    )
                                    .child(img(image).h(px(140.0))),
                            )
                        })
                        .child(
                            card(theme)
                                .flex_1()
                                .p(px(10.0))
                                .gap(px(6.0))
                                .overflow_hidden()
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
                                                .child("Engine Logs"),
                                        )
                                        .child(
                                            div().flex().flex_row().gap(px(4.0)).children(
                                                ["ALL", "ERROR", "WARN", "INFO", "DEBUG"]
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(i, &lvl)| {
                                                        let active = level_filter == lvl;
                                                        button(
                                                            ("dbg-lvl", i),
                                                            lvl,
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
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .overflow_hidden()
                                        .children(filtered_logs.into_iter().map(|line| {
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(theme.text_secondary)
                                                .child(line)
                                        })),
                                ),
                        ),
                ),
        )
        .into_any_element()
}
