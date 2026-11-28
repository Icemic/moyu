//! Package / Export view: 2x2 target grid and target-specific options (fully localized).

use gpui::{AnyElement, Context, div, prelude::*, px};

use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button, icon_button, sm_button};
use crate::gui::ui::card::{detail_row, panel, section_heading, status_dot};
use crate::gui::ui::dropdown::{menu_item, select_dropdown};
use crate::gui::ui::icon::IconName;
use crate::gui::ui::switch::switch;
use crate::gui::{ActivePopover, DevtoolApp, PackTargetGroup};
use crate::service::android::ANDROID_FORMATS;

const DESKTOP_PLATFORMS: &[(&str, &str)] = &[
    ("windows-amd64", "Windows x64"),
    ("linux-amd64", "Linux x64"),
    ("linux-aarch64", "Linux ARM64"),
    ("macos-amd64", "macOS Intel"),
    ("macos-aarch64", "macOS Apple Silicon"),
];

pub fn render_pack_view(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let group = app.pack_target_group;
    let desktop_plat = app.pack_desktop_platform.clone();
    let desktop_plat_label = DESKTOP_PLATFORMS
        .iter()
        .find(|(id, _)| *id == desktop_plat)
        .map(|(_, l)| *l)
        .unwrap_or(&desktop_plat);
    let plat_open = app.active_popover == Some(ActivePopover::PackPlatform);

    let android_fmt = app.pack_android_format.clone();
    let android_fmt_open = app.active_popover == Some(ActivePopover::PackAndroidFormat);
    let needs_signing_pw = group == PackTargetGroup::Android
        && matches!(android_fmt.as_str(), "release-apk" | "release-aab");
    let compress = app.pack_compress;
    let video = app.pack_video_decoder;

    let desktop_items: Vec<AnyElement> = DESKTOP_PLATFORMS
        .iter()
        .enumerate()
        .map(|(i, &(id, label))| {
            menu_item(
                ("pack-desk-plat", i),
                label,
                desktop_plat == id,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.pack_desktop_platform = id.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let android_items: Vec<AnyElement> = ANDROID_FORMATS
        .iter()
        .enumerate()
        .map(|(i, &fmt)| {
            menu_item(
                ("pack-and-fmt", i),
                i18n.android_format_name(fmt),
                android_fmt == fmt,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.pack_android_format = fmt.to_string();
                    this.active_popover = None;
                    cx.notify();
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
        .mb(px(12.0))
        .border_b_1()
        .border_color(theme.rule)
        .child(
            div()
                .text_size(px(20.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(i18n.tab_pack()),
        )
        .child(button(
            "btn-start-pack",
            i18n.start_pack(),
            ButtonVariant::Primary,
            theme,
            cx.listener(|this, _, _, cx| this.start_pack(cx)),
        ));

    let target_card = |id: &'static str,
                       title: &'static str,
                       desc: &'static str,
                       target_group: PackTargetGroup,
                       cx: &mut Context<DevtoolApp>| {
        let active = group == target_group;
        div()
            .id(id)
            .flex_1()
            .px(px(12.0))
            .py(px(10.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(if active { theme.ring } else { theme.rule })
            .bg(if active {
                theme.accent
            } else {
                gpui::transparent_black()
            })
            .hover(move |s| s.bg(theme.accent))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.pack_target_group = target_group;
                this.active_popover = None;
                cx.notify();
            }))
            .flex()
            .flex_col()
            .gap(px(3.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(if active {
                        theme.accent_foreground
                    } else {
                        theme.foreground
                    })
                    .child(title),
            )
            .child(div().text_size(px(11.0)).text_color(theme.meta).child(desc))
    };

    let targets_panel = panel(theme)
        .flex_1()
        .gap(px(8.0))
        .child(section_heading(i18n.export_target(), None, theme))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(target_card(
                    "pack-grp-desktop",
                    i18n.target_desktop(),
                    i18n.target_desktop_desc(),
                    PackTargetGroup::Desktop,
                    cx,
                ))
                .child(target_card(
                    "pack-grp-web",
                    i18n.target_web(),
                    i18n.target_web_desc(),
                    PackTargetGroup::Web,
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(target_card(
                    "pack-grp-android",
                    i18n.target_android(),
                    i18n.target_android_desc(),
                    PackTargetGroup::Android,
                    cx,
                ))
                .child(target_card(
                    "pack-grp-framework",
                    i18n.target_framework(),
                    i18n.target_framework_desc(),
                    PackTargetGroup::Framework,
                    cx,
                )),
        );

    let options_panel = panel(theme)
        .w(px(440.0))
        .flex_shrink_0()
        .child(section_heading(i18n.options(), None, theme))
        .when(group == PackTargetGroup::Desktop, |p| {
            p.child(
                detail_row(theme)
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.meta)
                            .child(i18n.pack_platform_label()),
                    )
                    .child(select_dropdown(
                        "pack-plat-select",
                        desktop_plat_label.to_string(),
                        plat_open,
                        220.0,
                        true,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_popover(ActivePopover::PackPlatform, cx);
                        }),
                        desktop_items,
                    )),
            )
        })
        .when(group == PackTargetGroup::Android, |p| {
            p.child(
                detail_row(theme)
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.meta)
                            .child(i18n.pack_android_format_label()),
                    )
                    .child(select_dropdown(
                        "pack-and-select",
                        i18n.android_format_name(&android_fmt),
                        android_fmt_open,
                        220.0,
                        true,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_popover(ActivePopover::PackAndroidFormat, cx);
                        }),
                        android_items,
                    )),
            )
            .when(needs_signing_pw, |inner| {
                inner.child(
                    detail_row(theme)
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.meta)
                                .child(i18n.signing_password_label()),
                        )
                        .child(div().w(px(220.0)).child(app.android_password_input.clone())),
                )
            })
            .child(
                detail_row(theme)
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
                                    .text_color(theme.foreground)
                                    .child(i18n.android_config_title()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.meta)
                                    .overflow_hidden()
                                    .child(
                                        app.project
                                            .moyu_summary
                                            .as_ref()
                                            .and_then(|m| m.android_application_id.clone())
                                            .unwrap_or_else(|| "moyu.json · android".to_string()),
                                    ),
                            ),
                    )
                    .child(sm_button(
                        "pack-open-android-cfg",
                        i18n.configure(),
                        ButtonVariant::Secondary,
                        theme,
                        cx.listener(|this, _, _, cx| this.open_android_config_dialog(cx)),
                    )),
            )
        })
        .when(
            matches!(group, PackTargetGroup::Desktop | PackTargetGroup::Web),
            |p| {
                p.child(
                    detail_row(theme)
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.foreground)
                                .child(i18n.zip_archive_label()),
                        )
                        .child(switch(
                            "toggle-compress",
                            compress,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.pack_compress = !this.pack_compress;
                                cx.notify();
                            }),
                        )),
                )
            },
        )
        .when(
            matches!(group, PackTargetGroup::Desktop | PackTargetGroup::Android),
            |p| {
                p.child(
                    detail_row(theme)
                        .child(
                            div()
                                .text_size(px(13.0))
                                .text_color(theme.foreground)
                                .child(i18n.video_decoder_label()),
                        )
                        .child(switch(
                            "toggle-video",
                            video,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.pack_video_decoder = !this.pack_video_decoder;
                                cx.notify();
                            }),
                        )),
                )
            },
        )
        .when(group == PackTargetGroup::Framework, |p| {
            p.child(
                detail_row(theme)
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.meta)
                            .child(i18n.output_format_label()),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(theme.foreground)
                            .child("ZIP"),
                    ),
            )
        })
        .child(
            detail_row(theme)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(theme.meta)
                        .child(i18n.output_dir_label()),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.foreground)
                                .child(".moyu/release"),
                        )
                        .when_some(
                            app.project
                                .root
                                .as_ref()
                                .map(|r| r.join(".moyu").join("release"))
                                .filter(|p| p.is_dir()),
                            |d, release_dir| {
                                d.child(sm_button(
                                    "pack-open-release-dir",
                                    i18n.open_folder(),
                                    ButtonVariant::Secondary,
                                    theme,
                                    cx.listener(move |this, _, _, cx| {
                                        this.open_pack_output_dir(release_dir.clone(), cx);
                                    }),
                                ))
                            },
                        ),
                ),
        );

    let pack_done_banner = app.last_pack_output.clone().map(|out_dir| {
        let out_str = out_dir.display().to_string();
        let open_dir = out_dir.clone();
        div()
            .mb(px(10.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(4.0))
            .bg(theme.card)
            .border_1()
            .border_color(theme.success)
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .min_w_0()
                    .flex_1()
                    .child(status_dot(theme.success))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.foreground)
                                    .child(i18n.pack_complete_title()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.meta)
                                    .overflow_hidden()
                                    .child(out_str),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .flex_shrink_0()
                    .child(sm_button(
                        "pack-open-last-output",
                        i18n.open_output_dir(),
                        ButtonVariant::Primary,
                        theme,
                        cx.listener(move |this, _, _, cx| {
                            this.open_pack_output_dir(open_dir.clone(), cx);
                        }),
                    ))
                    .child(icon_button(
                        "pack-dismiss-banner",
                        IconName::X,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            this.last_pack_output = None;
                            cx.notify();
                        }),
                    )),
            )
    });

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(heading)
        .when_some(pack_done_banner, |d, b| d.child(b))
        .child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(10.0))
                .child(targets_panel)
                .child(options_panel),
        )
        .into_any_element()
}
