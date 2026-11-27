//! Engine version management view: active version summary, local versions, update, and download dialog.

use gpui::{AnyElement, Context, div, prelude::*, px};

use crate::gui::DevtoolApp;
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button, sm_button};
use crate::gui::ui::card::{chip, detail_row, panel, section_heading, status_dot};

pub fn render_engine_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let active = app.engine.meta.active.clone();
    let active_platforms: Vec<String> = active
        .as_ref()
        .and_then(|a| app.engine.meta.downloads.get(&a.version))
        .map(|d| d.platforms.keys().cloned().collect())
        .unwrap_or_default();

    let local_downloads: Vec<(String, String, Vec<String>, bool)> = app
        .engine
        .meta
        .downloads
        .iter()
        .map(|(ver, dl)| {
            let is_active = active.as_ref().is_some_and(|a| a.version == *ver);
            (
                ver.clone(),
                dl.channel.clone(),
                dl.platforms.keys().cloned().collect(),
                is_active,
            )
        })
        .collect();

    let heading = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .pb(px(12.0))
        .mb(px(12.0))
        .border_b_1()
        .border_color(theme.rule)
        .child(
            div()
                .text_size(px(20.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(i18n.tab_engine()),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(button(
                    "eng-update-btn",
                    i18n.update_latest(),
                    ButtonVariant::Secondary,
                    theme,
                    cx.listener(|this, _, _, cx| this.start_engine_update(cx)),
                ))
                .child(button(
                    "eng-open-download-dlg",
                    i18n.download_version(),
                    ButtonVariant::Primary,
                    theme,
                    cx.listener(|this, _, _, cx| this.open_download_engine_dialog(cx)),
                )),
        );

    let active_panel = panel(theme)
        .w(px(360.0))
        .flex_shrink_0()
        .child(section_heading(
            i18n.current_active(),
            Some(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(status_dot(if active.is_some() {
                        theme.success
                    } else {
                        theme.meta
                    }))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.meta)
                            .child(if active.is_some() {
                                i18n.activated()
                            } else {
                                "-"
                            }),
                    )
                    .into_any_element(),
            ),
            theme,
        ))
        .child(
            div()
                .py(px(8.0))
                .text_size(px(22.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(
                    active
                        .as_ref()
                        .map(|a| a.version.clone())
                        .unwrap_or_else(|| "-".to_string()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(5.0))
                .when_some(active.as_ref(), |d, a| {
                    d.child(chip(a.channel.clone(), theme))
                })
                .children(active_platforms.into_iter().map(|p| chip(p, theme))),
        )
        .child(
            div()
                .mt(px(12.0))
                .pt(px(8.0))
                .border_t_1()
                .border_color(theme.rule)
                .text_size(px(11.0))
                .text_color(theme.meta)
                .child(format!("CDN: {}", app.engine.config.cdn_url)),
        );

    let installed_panel = panel(theme)
        .flex_1()
        .child(section_heading(i18n.local_versions(), None, theme))
        .when(local_downloads.is_empty(), |p| {
            p.child(
                detail_row(theme).child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .child(i18n.no_local_versions()),
                ),
            )
        })
        .children(local_downloads.into_iter().enumerate().map(
            |(idx, (ver, ch, plats, is_active))| {
                let ver_clone = ver.clone();
                let subtitle = format!("{ch} · {}", plats.join("、"));
                detail_row(theme)
                    .min_h(px(48.0))
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
                                    .child(ver),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.meta)
                                    .child(subtitle),
                            ),
                    )
                    .child(if is_active {
                        div()
                            .h(px(26.0))
                            .px(px(9.0))
                            .rounded(px(4.0))
                            .bg(theme.accent)
                            .text_color(theme.accent_foreground)
                            .text_size(px(12.0))
                            .flex()
                            .items_center()
                            .child(i18n.current_version())
                            .into_any_element()
                    } else {
                        sm_button(
                            ("activate-ver", idx),
                            i18n.switch_version(),
                            ButtonVariant::Secondary,
                            theme,
                            cx.listener(move |this, _, _, cx| {
                                this.switch_local_engine(&ver_clone, cx);
                            }),
                        )
                        .into_any_element()
                    })
            },
        ));

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(heading)
        .child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(10.0))
                .child(active_panel)
                .child(installed_panel),
        )
        .into_any_element()
}
