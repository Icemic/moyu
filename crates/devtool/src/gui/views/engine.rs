//! Engine version management view: CDN manifest, platform download, update, and switch.

use gpui::{AnyElement, Context, div, prelude::*, px};

use crate::gui::DevtoolApp;
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::badge::{BadgeTone, badge};
use crate::gui::ui::button::{ButtonVariant, button};
use crate::gui::ui::card::{card, muted_panel};

pub fn render_engine_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let active_text = app
        .engine
        .meta
        .active
        .as_ref()
        .map(|a| format!("{} ({})", a.version, a.channel))
        .unwrap_or_else(|| "(none)".to_string());

    let local_downloads: Vec<(String, String, Vec<String>, bool)> = app
        .engine
        .meta
        .downloads
        .iter()
        .map(|(ver, dl)| {
            let is_active = app
                .engine
                .meta
                .active
                .as_ref()
                .is_some_and(|a| a.version == *ver);
            (
                ver.clone(),
                dl.channel.clone(),
                dl.platforms.keys().cloned().collect(),
                is_active,
            )
        })
        .collect();

    let remote_channels: Vec<String> = app
        .engine
        .remote_manifest
        .as_ref()
        .map(|m| m.channels.keys().cloned().collect())
        .unwrap_or_default();

    let mut remote_versions: Vec<(String, String, bool)> = Vec::new();
    let mut available_platforms: Vec<String> = Vec::new();

    if let Some(manifest) = &app.engine.remote_manifest
        && let Some(ch) = manifest.channels.get(&app.selected_channel)
    {
        let mut entries: Vec<(&String, &crate::model::engine::VersionEntry)> =
            ch.versions.iter().collect();
        entries.sort_by(|a, b| b.1.published_at.cmp(&a.1.published_at));
        for (ver, entry) in entries.into_iter().take(12) {
            let date = entry.published_at.get(..10).unwrap_or(&entry.published_at);
            remote_versions.push((ver.clone(), date.to_string(), *ver == ch.latest));
        }
        if let Some(sel_ver) = &app.selected_version
            && let Some(entry) = ch.versions.get(sel_ver)
        {
            available_platforms = entry.assets.keys().cloned().collect();
        }
    }

    let selected_channel = app.selected_channel.clone();
    let selected_version = app.selected_version.clone();
    let selected_platforms = app.selected_platforms.clone();

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
                                .child(format!("Active: {active_text}")),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(8.0))
                                .child(button(
                                    "eng-fetch-btn",
                                    i18n.fetch_versions(),
                                    ButtonVariant::Default,
                                    theme,
                                    cx.listener(|this, _, _, cx| this.fetch_engine_manifest(cx)),
                                ))
                                .child(button(
                                    "eng-update-btn",
                                    i18n.update_latest(),
                                    ButtonVariant::Primary,
                                    theme,
                                    cx.listener(|this, _, _, cx| this.start_engine_update(cx)),
                                )),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.text_tertiary)
                        .child(format!("CDN: {}", app.engine.config.cdn_url)),
                ),
        )
        .child(
            card(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.text_secondary)
                        .child(i18n.local_versions()),
                )
                .when(local_downloads.is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(px(12.0))
                            .text_color(theme.text_tertiary)
                            .child("No local engine versions downloaded yet."),
                    )
                })
                .children(local_downloads.into_iter().enumerate().map(
                    |(idx, (ver, ch, plats, is_active))| {
                        let ver_clone = ver.clone();
                        div()
                            .id(("local-ver", idx))
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(theme.surface_muted)
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
                                            .text_size(px(13.0))
                                            .text_color(theme.text)
                                            .child(ver.clone()),
                                    )
                                    .child(badge(ch, BadgeTone::Neutral, theme))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text_tertiary)
                                            .child(plats.join(", ")),
                                    ),
                            )
                            .child(if is_active {
                                badge(i18n.active_badge(), BadgeTone::Accent, theme)
                                    .into_any_element()
                            } else {
                                button(
                                    ("activate-ver", idx),
                                    i18n.activate(),
                                    ButtonVariant::Default,
                                    theme,
                                    cx.listener(move |this, _, _, cx| {
                                        this.switch_local_engine(&ver_clone, cx);
                                    }),
                                )
                                .into_any_element()
                            })
                    },
                )),
        )
        .when(!remote_channels.is_empty(), |this| {
            this.child(
                card(theme)
                    .child(div().flex().flex_row().flex_wrap().gap(px(6.0)).children(
                        remote_channels.into_iter().enumerate().map(|(idx, ch)| {
                            let is_sel = ch == selected_channel;
                            let ch_clone = ch.clone();
                            button(
                                ("ch-chip", idx),
                                ch,
                                if is_sel {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Default
                                },
                                theme,
                                cx.listener(move |this, _, _, cx| {
                                    this.select_channel(ch_clone.clone(), cx);
                                }),
                            )
                        }),
                    ))
                    .child(muted_panel(theme).child(
                        div().flex().flex_row().flex_wrap().gap(px(6.0)).children(
                            remote_versions.into_iter().enumerate().map(
                                |(idx, (ver, date, is_latest))| {
                                    let is_sel = selected_version.as_deref() == Some(ver.as_str());
                                    let label = if is_latest {
                                        format!("{ver} ({date}) *")
                                    } else {
                                        format!("{ver} ({date})")
                                    };
                                    let ver_clone = ver.clone();
                                    button(
                                        ("ver-chip", idx),
                                        label,
                                        if is_sel {
                                            ButtonVariant::Primary
                                        } else {
                                            ButtonVariant::Default
                                        },
                                        theme,
                                        cx.listener(move |this, _, _, cx| {
                                            this.selected_version = Some(ver_clone.clone());
                                            cx.notify();
                                        }),
                                    )
                                },
                            ),
                        ),
                    ))
                    .when(!available_platforms.is_empty(), |inner| {
                        inner
                            .child(
                                div().flex().flex_row().flex_wrap().gap(px(6.0)).children(
                                    available_platforms.into_iter().enumerate().map(
                                        |(idx, plat)| {
                                            let checked = selected_platforms.contains(&plat);
                                            let plat_clone = plat.clone();
                                            button(
                                                ("plat-chip", idx),
                                                if checked { format!("✓ {plat}") } else { plat },
                                                if checked {
                                                    ButtonVariant::Primary
                                                } else {
                                                    ButtonVariant::Default
                                                },
                                                theme,
                                                cx.listener(move |this, _, _, cx| {
                                                    if !this.selected_platforms.remove(&plat_clone)
                                                    {
                                                        this.selected_platforms
                                                            .insert(plat_clone.clone());
                                                    }
                                                    cx.notify();
                                                }),
                                            )
                                        },
                                    ),
                                ),
                            )
                            .child(div().flex().flex_row().gap(px(8.0)).child(button(
                                "btn-download-selected",
                                i18n.download_selected(),
                                ButtonVariant::Primary,
                                theme,
                                cx.listener(|this, _, _, cx| {
                                    this.start_engine_download_selected(cx)
                                }),
                            )))
                    }),
            )
        })
        .into_any_element()
}
