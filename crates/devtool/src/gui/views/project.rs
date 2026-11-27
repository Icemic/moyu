//! Project overview view: manifest summary, multi-res preflight, recent projects, and `init`.

use gpui::{AnyElement, Context, div, prelude::*, px};

use crate::gui::DevtoolApp;
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::badge::{BadgeTone, badge};
use crate::gui::ui::button::{ButtonVariant, button};
use crate::gui::ui::card::{card, muted_panel};
use crate::service::init::ProjectTemplate;

pub fn render_project_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let project_path = app
        .project
        .root
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| i18n.no_project_selected().to_string());

    let multi_res_badge = match &app.project.multi_res_report {
        Some(r) if r.is_clean() => badge("Multi-Res OK", BadgeTone::Success, theme),
        Some(_) => badge("Multi-Res Warnings", BadgeTone::Warn, theme),
        None => badge("Idle", BadgeTone::Neutral, theme),
    };

    let summary_lines: Vec<String> = if let Some(idx) = &app.project.index_summary {
        let mut lines = Vec::new();
        if let Some(t) = &idx.window_title {
            lines.push(format!("Window Title: {t}"));
        }
        if let Some(e) = &idx.entry_filename {
            lines.push(format!("Entry: {e}"));
        }
        if let Some(s) = &idx.stage_size {
            lines.push(format!("Stage Size: {s}"));
        }
        if let Some(m) = &app.project.moyu_summary {
            if let Some(ch) = &m.engine_channel {
                lines.push(format!("Engine Channel (moyu.json): {ch}"));
            }
            if let Some(app_id) = &m.android_application_id {
                lines.push(format!("Android ID: {app_id}"));
            }
        }
        if let Some(fw) = &app.project.framework_summary
            && let Some(name) = &fw.name
        {
            lines.push(format!(
                "Framework: {name} ({})",
                fw.version.as_deref().unwrap_or("?")
            ));
        }
        lines
    } else {
        Vec::new()
    };

    let recent_list = app.global_config.recent_projects.clone();
    let tpl = app.init_template;

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
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
                                .text_color(theme.text)
                                .child(project_path),
                        )
                        .child(multi_res_badge),
                )
                .when(!summary_lines.is_empty(), |this| {
                    this.child(
                        muted_panel(theme).children(summary_lines.into_iter().map(|line| {
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .child(line)
                        })),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(button(
                            "proj-open-btn",
                            i18n.open_project(),
                            ButtonVariant::Primary,
                            theme,
                            cx.listener(|this, _, _, cx| this.prompt_open_project(cx)),
                        ))
                        .child(button(
                            "proj-recheck-btn",
                            i18n.recheck_assets(),
                            ButtonVariant::Default,
                            theme,
                            cx.listener(|this, _, _, cx| this.recheck_multi_res(cx)),
                        )),
                ),
        )
        .child(
            card(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .child(i18n.create_project_title()),
                )
                .child(app.init_name_input.clone())
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(button(
                            "tpl-basic",
                            "Basic",
                            if tpl == ProjectTemplate::Basic {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.init_template = ProjectTemplate::Basic;
                                cx.notify();
                            }),
                        ))
                        .child(button(
                            "tpl-advanced",
                            "Advanced",
                            if tpl == ProjectTemplate::Advanced {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.init_template = ProjectTemplate::Advanced;
                                cx.notify();
                            }),
                        ))
                        .child(button(
                            "btn-create-project",
                            i18n.pick_parent_and_create(),
                            ButtonVariant::Primary,
                            theme,
                            cx.listener(|this, _, _, cx| this.prompt_create_project(cx)),
                        )),
                ),
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
                            .id(("recent-proj-row", idx))
                            .px(px(8.0))
                            .py(px(5.0))
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
        .into_any_element()
}
