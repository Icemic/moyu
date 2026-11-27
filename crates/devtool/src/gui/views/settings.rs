//! Environment detection, one-click managed Node LTS install, and appearance settings.

use gpui::{AnyElement, Context, div, prelude::*, px};

use crate::config::{LocalePreference, ThemePreference};
use crate::gui::DevtoolApp;
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::badge::{BadgeTone, badge};
use crate::gui::ui::button::{ButtonVariant, button};
use crate::gui::ui::card::{card, muted_panel};

pub fn render_settings_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let (node_badge, node_detail) = match &app.env.node {
        Some(info) => (
            badge(
                format!(
                    "{} ({})",
                    info.version,
                    if info.is_managed { "managed" } else { "system" }
                ),
                BadgeTone::Success,
                theme,
            ),
            info.executable.display().to_string(),
        ),
        None => (
            badge("Node Not Found", BadgeTone::Warn, theme),
            "Not found in PATH or ~/.moyu/node/".to_string(),
        ),
    };

    let pms: Vec<String> = app
        .env
        .available_package_managers
        .iter()
        .map(|pm| pm.binary_name().to_string())
        .collect();
    let project_pm = app
        .env
        .package_manager
        .map(|pm| pm.binary_name())
        .unwrap_or("(no project)");
    let jiti_status = app
        .env
        .jiti_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "not found".to_string());

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
                                .child(i18n.env_title()),
                        )
                        .child(node_badge),
                )
                .child(
                    muted_panel(theme)
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .child(format!("Node Executable: {node_detail}")),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .child(format!(
                                    "Project PM: {project_pm} | Available: {}",
                                    if pms.is_empty() {
                                        "(none)".to_string()
                                    } else {
                                        pms.join(", ")
                                    }
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.text_secondary)
                                .child(format!("jiti: {jiti_status}")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(button(
                            "btn-redetect-env",
                            i18n.redetect_env(),
                            ButtonVariant::Default,
                            theme,
                            cx.listener(|this, _, _, cx| this.refresh_env(cx)),
                        ))
                        .child(button(
                            "btn-install-node",
                            i18n.install_managed_node(),
                            ButtonVariant::Primary,
                            theme,
                            cx.listener(|this, _, _, cx| this.start_install_managed_node(cx)),
                        )),
                ),
        )
        .child(
            card(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .child(i18n.theme_label()),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(button(
                            "set-theme-light",
                            i18n.theme_light(),
                            if app.global_config.theme == ThemePreference::Light {
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
                            "set-theme-dark",
                            i18n.theme_dark(),
                            if app.global_config.theme == ThemePreference::Dark {
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
                            "set-theme-system",
                            i18n.theme_system(),
                            if app.global_config.theme == ThemePreference::System {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, window, cx| {
                                this.set_theme(ThemePreference::System, window, cx)
                            }),
                        )),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.text)
                        .child(i18n.language_label()),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(button(
                            "set-lang-zh",
                            "简体中文",
                            if app.global_config.locale == LocalePreference::ZhCn {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.set_locale(LocalePreference::ZhCn, cx)
                            }),
                        ))
                        .child(button(
                            "set-lang-en",
                            "English",
                            if app.global_config.locale == LocalePreference::EnUs {
                                ButtonVariant::Primary
                            } else {
                                ButtonVariant::Default
                            },
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.set_locale(LocalePreference::EnUs, cx)
                            }),
                        )),
                ),
        )
        .into_any_element()
}
