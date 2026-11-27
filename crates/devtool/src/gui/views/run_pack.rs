//! Run, Pack, Android export, and Schema generation view.

use gpui::{AnyElement, Context, div, prelude::*, px};

use crate::gui::DevtoolApp;
use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button};
use crate::gui::ui::card::{card, muted_panel};
use crate::service::android::{ANDROID_FORMATS, ANDROID_PLATFORM};

const PACK_TARGETS: &[&str] = &[
    "windows-amd64",
    "linux-amd64",
    "linux-aarch64",
    "macos-amd64",
    "macos-aarch64",
    "web-universal",
    ANDROID_PLATFORM,
];

pub fn render_run_pack_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let is_framework = app.pack_framework;
    let is_android = !is_framework && app.pack_target == ANDROID_PLATFORM;
    let needs_signing_pw = is_android
        && matches!(
            app.pack_android_format.as_str(),
            "release-apk" | "release-aab"
        );
    let current_target = app.pack_target.clone();
    let current_android_fmt = app.pack_android_format.clone();
    let compress = app.pack_compress;
    let video = app.pack_video_decoder;

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            card(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.text_secondary)
                        .child("Run & Schema"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.0))
                        .child(button(
                            "btn-run-native",
                            i18n.run_native(),
                            ButtonVariant::Primary,
                            theme,
                            cx.listener(|this, _, _, cx| this.start_run(false, cx)),
                        ))
                        .child(button(
                            "btn-run-web",
                            i18n.run_web(),
                            ButtonVariant::Default,
                            theme,
                            cx.listener(|this, _, _, cx| this.start_run(true, cx)),
                        ))
                        .child(button(
                            "btn-gen-cmd-schema",
                            i18n.gen_commands_schema(),
                            ButtonVariant::Default,
                            theme,
                            cx.listener(|this, _, _, cx| this.start_generate_schema(false, cx)),
                        ))
                        .child(button(
                            "btn-gen-ui-schema",
                            i18n.gen_ui_schema(),
                            ButtonVariant::Default,
                            theme,
                            cx.listener(|this, _, _, cx| this.start_generate_schema(true, cx)),
                        )),
                ),
        )
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
                                .child("Pack Configuration"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(6.0))
                                .child(button(
                                    "pack-mode-game",
                                    "Game",
                                    if !is_framework {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Default
                                    },
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.pack_framework = false;
                                        cx.notify();
                                    }),
                                ))
                                .child(button(
                                    "pack-mode-fw",
                                    "Framework",
                                    if is_framework {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Default
                                    },
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.pack_framework = true;
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
                .when(!is_framework, |this| {
                    this.child(
                        muted_panel(theme).child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .gap(px(6.0))
                                .children(PACK_TARGETS.iter().enumerate().map(|(idx, &t)| {
                                    let sel = current_target == t;
                                    button(
                                        ("pack-target", idx),
                                        t,
                                        if sel {
                                            ButtonVariant::Primary
                                        } else {
                                            ButtonVariant::Default
                                        },
                                        theme,
                                        cx.listener(move |this, _, _, cx| {
                                            this.pack_target = t.to_string();
                                            cx.notify();
                                        }),
                                    )
                                })),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(8.0))
                            .when(!is_android, |row| {
                                row.child(button(
                                    "toggle-compress",
                                    if compress { "✓ ZIP Compress" } else { "ZIP Compress" },
                                    if compress {
                                        ButtonVariant::Primary
                                    } else {
                                        ButtonVariant::Default
                                    },
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        this.pack_compress = !this.pack_compress;
                                        cx.notify();
                                    }),
                                ))
                            })
                            .child(button(
                                "toggle-video",
                                if video {
                                    "✓ Video Decoder Plugin"
                                } else {
                                    "Video Decoder Plugin"
                                },
                                if video {
                                    ButtonVariant::Primary
                                } else {
                                    ButtonVariant::Default
                                },
                                theme,
                                cx.listener(|this, _, _, cx| {
                                    this.pack_video_decoder = !this.pack_video_decoder;
                                    cx.notify();
                                }),
                            )),
                    )
                })
                .when(is_android, |this| {
                    this.child(
                        muted_panel(theme)
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .flex_wrap()
                                    .gap(px(6.0))
                                    .children(ANDROID_FORMATS.iter().enumerate().map(
                                        |(idx, &fmt)| {
                                            let sel = current_android_fmt == fmt;
                                            button(
                                                ("android-fmt", idx),
                                                fmt,
                                                if sel {
                                                    ButtonVariant::Primary
                                                } else {
                                                    ButtonVariant::Default
                                                },
                                                theme,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.pack_android_format = fmt.to_string();
                                                    cx.notify();
                                                }),
                                            )
                                        },
                                    )),
                            )
                            .when(needs_signing_pw, |inner| {
                                inner.child(app.android_password_input.clone())
                            }),
                    )
                })
                .child(
                    div().flex().flex_row().gap(px(8.0)).child(button(
                        "btn-start-pack",
                        i18n.start_pack(),
                        ButtonVariant::Primary,
                        theme,
                        cx.listener(|this, _, _, cx| this.start_pack(cx)),
                    )),
                ),
        )
        .into_any_element()
}
