//! Modal dialogs: Settings, Create Project, and Download Engine Version.

use gpui::{AnyElement, Context, MouseButton, div, prelude::*, px};

use crate::config::{LocalePreference, ThemePreference};
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button, icon_button, sm_button};
use crate::gui::ui::dropdown::{menu_item, select_dropdown};
use crate::gui::ui::icon::IconName;
use crate::gui::{ActivePopover, DevtoolApp};

fn modal_shell(
    id: usize,
    title: &str,
    theme: Theme,
    body: AnyElement,
    footer: AnyElement,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    div()
        .id(("modal-backdrop", id))
        .absolute()
        .inset_0()
        .bg(theme.backdrop)
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.close_dialog(cx);
            }),
        )
        .child(
            div()
                .id(("modal-box", id))
                .w(px(420.0))
                .max_w(px(520.0))
                .rounded(px(6.0))
                .bg(theme.background)
                .text_color(theme.foreground)
                .border_1()
                .border_color(theme.rule)
                .shadow_lg()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        if this.active_popover.is_some() {
                            this.active_popover = None;
                            cx.notify();
                        }
                    }),
                )
                .child(
                    div()
                        .p(px(16.0))
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(15.0))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_color(theme.foreground)
                                        .child(title.to_string()),
                                )
                                .child(icon_button(
                                    ("modal-close", id),
                                    IconName::X,
                                    theme,
                                    cx.listener(|this, _, _, cx| this.close_dialog(cx)),
                                )),
                        )
                        .child(body),
                )
                .child(
                    div()
                        .px(px(16.0))
                        .py(px(12.0))
                        .rounded_b(px(6.0))
                        .bg(theme.card)
                        .border_t_1()
                        .border_color(theme.rule)
                        .flex()
                        .flex_row()
                        .justify_end()
                        .gap(px(8.0))
                        .child(footer),
                ),
        )
        .into_any_element()
}

pub fn render_settings_dialog(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let cur_theme = app.global_config.theme;
    let cur_locale = app.global_config.locale;
    let theme_open = app.active_popover == Some(ActivePopover::SettingsTheme);
    let locale_open = app.active_popover == Some(ActivePopover::SettingsLocale);

    let theme_options = [
        ThemePreference::System,
        ThemePreference::Light,
        ThemePreference::Dark,
        ThemePreference::SolarizedLight,
        ThemePreference::OneDark,
    ];
    let theme_items: Vec<AnyElement> = theme_options
        .into_iter()
        .enumerate()
        .map(|(i, pref)| {
            menu_item(
                ("dlg-theme-opt", i),
                i18n.theme_name(pref),
                cur_theme == pref,
                theme,
                cx.listener(move |this, _, window, cx| {
                    this.active_popover = None;
                    this.set_theme(pref, window, cx);
                }),
            )
            .into_any_element()
        })
        .collect();

    let locale_options = [LocalePreference::ZhCn, LocalePreference::EnUs];
    let locale_items: Vec<AnyElement> = locale_options
        .into_iter()
        .enumerate()
        .map(|(i, loc)| {
            menu_item(
                ("dlg-locale-opt", i),
                i18n.locale_name(loc),
                cur_locale == loc,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.active_popover = None;
                    this.set_locale(loc, cx);
                }),
            )
            .into_any_element()
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .child(
            div()
                .border_t_1()
                .border_color(theme.rule)
                .min_h(px(44.0))
                .py(px(6.0))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.foreground)
                        .child(i18n.theme_label()),
                )
                .child(select_dropdown(
                    "dlg-theme-select",
                    i18n.theme_name(cur_theme),
                    theme_open,
                    180.0,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::SettingsTheme, cx);
                    }),
                    theme_items,
                )),
        )
        .child(
            div()
                .border_t_1()
                .border_color(theme.rule)
                .min_h(px(44.0))
                .py(px(6.0))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.foreground)
                        .child(i18n.language_label()),
                )
                .child(select_dropdown(
                    "dlg-locale-select",
                    i18n.locale_name(cur_locale),
                    locale_open,
                    180.0,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::SettingsLocale, cx);
                    }),
                    locale_items,
                )),
        )
        .into_any_element();

    let footer = button(
        "dlg-settings-done",
        i18n.done(),
        ButtonVariant::Secondary,
        theme,
        cx.listener(|this, _, _, cx| this.close_dialog(cx)),
    )
    .into_any_element();

    modal_shell(1, i18n.settings(), theme, body, footer, cx)
}

pub fn render_create_project_dialog(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .child(i18n.project_name()),
                )
                .child(app.init_name_input.clone()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.meta)
                        .child(i18n.parent_directory()),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(div().flex_1().child(app.init_dir_input.clone()))
                        .child(button(
                            "dlg-pick-parent-dir",
                            i18n.browse(),
                            ButtonVariant::Secondary,
                            theme,
                            cx.listener(|this, _, _, cx| this.prompt_pick_create_parent(cx)),
                        )),
                ),
        )
        .into_any_element();

    let footer = div()
        .flex()
        .flex_row()
        .gap(px(8.0))
        .child(button(
            "dlg-create-cancel",
            i18n.cancel(),
            ButtonVariant::Secondary,
            theme,
            cx.listener(|this, _, _, cx| this.close_dialog(cx)),
        ))
        .child(button(
            "dlg-create-submit",
            i18n.create_project_submit(),
            ButtonVariant::Primary,
            theme,
            cx.listener(|this, _, _, cx| this.submit_create_project(cx)),
        ))
        .into_any_element();

    modal_shell(2, i18n.new_project(), theme, body, footer, cx)
}

pub fn render_download_engine_dialog(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
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
    let fetching = app.engine.fetching_manifest;

    let ch_open = app.active_popover == Some(ActivePopover::DownloadChannel);
    let ver_open = app.active_popover == Some(ActivePopover::DownloadVersion);

    let ch_items: Vec<AnyElement> = remote_channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let ch_clone = ch.clone();
            menu_item(
                ("dlg-ch-opt", i),
                ch.clone(),
                *ch == selected_channel,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.active_popover = None;
                    this.select_channel(ch_clone.clone(), cx);
                }),
            )
            .into_any_element()
        })
        .collect();

    let ver_items: Vec<AnyElement> = remote_versions
        .iter()
        .enumerate()
        .map(|(i, (ver, date, is_latest))| {
            let label = if *is_latest {
                format!("{ver} ({date}) · latest")
            } else {
                format!("{ver} ({date})")
            };
            let ver_clone = ver.clone();
            menu_item(
                ("dlg-ver-opt", i),
                label,
                selected_version.as_deref() == Some(ver.as_str()),
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.selected_version = Some(ver_clone.clone());
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let body =
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .when(remote_channels.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(div().text_size(px(12.0)).text_color(theme.meta).child(
                            if fetching {
                                i18n.fetching_manifest()
                            } else {
                                i18n.load_manifest_hint()
                            },
                        ))
                        .child(sm_button(
                            "dlg-fetch-manifest",
                            i18n.fetch_manifest(),
                            ButtonVariant::Secondary,
                            theme,
                            cx.listener(|this, _, _, cx| this.fetch_engine_manifest(cx)),
                        )),
                )
            })
            .when(!remote_channels.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
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
                                        .text_color(theme.meta)
                                        .child(i18n.engine_channel_label()),
                                )
                                .child(sm_button(
                                    "dlg-refetch-manifest",
                                    i18n.fetch_manifest(),
                                    ButtonVariant::Ghost,
                                    theme,
                                    cx.listener(|this, _, _, cx| this.fetch_engine_manifest(cx)),
                                )),
                        )
                        .child(select_dropdown(
                            "dlg-ch-select",
                            selected_channel.clone(),
                            ch_open,
                            388.0,
                            false,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_popover(ActivePopover::DownloadChannel, cx);
                            }),
                            ch_items,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .child(i18n.version_label()),
                        )
                        .child(select_dropdown(
                            "dlg-ver-select",
                            selected_version.clone().unwrap_or_else(|| "-".to_string()),
                            ver_open,
                            388.0,
                            false,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_popover(ActivePopover::DownloadVersion, cx);
                            }),
                            ver_items,
                        )),
                )
                .when(!available_platforms.is_empty(), |inner| {
                    inner.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.meta)
                                    .child(i18n.platforms_label()),
                            )
                            .child(
                                div().flex().flex_row().flex_wrap().gap(px(6.0)).children(
                                    available_platforms.into_iter().enumerate().map(
                                        |(idx, plat)| {
                                            let checked = selected_platforms.contains(&plat);
                                            let plat_clone = plat.clone();
                                            sm_button(
                                                ("dlg-plat", idx),
                                                if checked { format!("✓ {plat}") } else { plat },
                                                if checked {
                                                    ButtonVariant::Primary
                                                } else {
                                                    ButtonVariant::Secondary
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
                            ),
                    )
                })
            })
            .into_any_element();

    let footer = div()
        .flex()
        .flex_row()
        .gap(px(8.0))
        .child(button(
            "dlg-dl-cancel",
            i18n.cancel(),
            ButtonVariant::Secondary,
            theme,
            cx.listener(|this, _, _, cx| this.close_dialog(cx)),
        ))
        .child(button(
            "dlg-dl-submit",
            i18n.download_submit(),
            ButtonVariant::Primary,
            theme,
            cx.listener(|this, _, _, cx| {
                this.close_dialog(cx);
                this.start_engine_download_selected(cx);
            }),
        ))
        .into_any_element();

    modal_shell(3, i18n.download_engine_title(), theme, body, footer, cx)
}
