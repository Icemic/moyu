//! Full modal editors for `index.json` (Runtime Config) and `moyu.json` (Engine & Android Config).

use gpui::{AnyElement, Context, Entity, MouseButton, div, prelude::*, px};

use crate::gui::i18n::I18n;
use crate::gui::theme::Theme;
use crate::gui::ui::button::{ButtonVariant, button, icon_button, sm_button};
use crate::gui::ui::dropdown::{menu_item, select_dropdown};
use crate::gui::ui::icon::{IconName, icon};
use crate::gui::ui::motion::{animate_modal_backdrop, animate_modal_panel};
use crate::gui::ui::switch::switch;
use crate::gui::ui::text_input::TextInput;
use crate::gui::{ActivePopover, DevtoolApp};
use crate::service::android::AndroidConfig;
use crate::service::project::{
    CursorEntryForm, EngineProjectConfigForm, FontEntryForm, RuntimeConfigForm,
};

pub struct FontRowEditor {
    pub path: Entity<TextInput>,
    pub alias: Entity<TextInput>,
    pub kind: String, // "" | "cjk" | "western"
}

impl FontRowEditor {
    pub fn from_entry(entry: &FontEntryForm, cx: &mut Context<DevtoolApp>) -> Self {
        let path_val = entry.path.clone();
        let alias_val = entry.alias.clone();
        Self {
            path: cx.new(|cx| {
                let mut i = TextInput::new("fonts/default.otf", cx);
                i.set_text(path_val, cx);
                i
            }),
            alias: cx.new(|cx| {
                let mut i = TextInput::new("Alias", cx);
                i.set_text(alias_val, cx);
                i
            }),
            kind: entry.kind.clone(),
        }
    }
}

pub struct CursorRowEditor {
    pub state: Entity<TextInput>,
    pub src: Entity<TextInput>,
    pub hotspot_x: Entity<TextInput>,
    pub hotspot_y: Entity<TextInput>,
}

impl CursorRowEditor {
    pub fn from_entry(entry: &CursorEntryForm, cx: &mut Context<DevtoolApp>) -> Self {
        let state_val = entry.state.clone();
        let src_val = entry.src.clone();
        let hx_val = entry.hotspot_x.clone();
        let hy_val = entry.hotspot_y.clone();
        Self {
            state: cx.new(|cx| {
                let mut i = TextInput::new("default", cx);
                i.set_text(state_val, cx);
                i
            }),
            src: cx.new(|cx| {
                let mut i = TextInput::new("cursors/default.png", cx);
                i.set_text(src_val, cx);
                i
            }),
            hotspot_x: cx.new(|cx| {
                let mut i = TextInput::new("X", cx);
                i.set_text(hx_val, cx);
                i
            }),
            hotspot_y: cx.new(|cx| {
                let mut i = TextInput::new("Y", cx);
                i.set_text(hy_val, cx);
                i
            }),
        }
    }
}

/// Encapsulated editor state for `index.json` (all 25 runtime fields).
pub struct RuntimeConfigEditors {
    pub window_title: Entity<TextInput>,
    pub app_name: Entity<TextInput>,
    pub initial_surface_size: Entity<TextInput>,
    pub stage_size: Entity<TextInput>,
    pub window_state: String,
    pub window_resizable: bool,
    pub background_color: Entity<TextInput>,
    pub fonts: Vec<FontRowEditor>,
    // Multi-res
    pub multi_res_mode: String,
    pub multi_res_fixed_scale: Entity<TextInput>,
    pub multi_res_scales: Entity<TextInput>,
    pub multi_res_policy: String,
    // Features & platform
    pub enable_msaa: bool,
    pub enable_mipmaps: bool,
    pub enable_gamepads: bool,
    pub skip_splash: bool,
    pub show_fps: bool,
    pub steam_enabled: bool,
    pub steam_app_id: Entity<TextInput>,
    pub steam_required: bool,
    pub steam_restart_through_client: bool,
    pub cursor_enabled: bool,
    pub cursors: Vec<CursorRowEditor>,
    // Advanced
    pub advanced_open: bool,
    pub entry_filename: Entity<TextInput>,
    pub entry: Entity<TextInput>,
    pub autorun: String,
    pub backend: String,
    pub present_mode: String,
    pub desired_maximum_frame_latency: Entity<TextInput>,
    pub params: Entity<TextInput>,
    pub save_error: Option<String>,
}

impl RuntimeConfigEditors {
    pub fn new(cx: &mut Context<DevtoolApp>) -> Self {
        let default_font = FontRowEditor::from_entry(&FontEntryForm::default(), cx);
        Self {
            window_title: cx.new(|cx| TextInput::new("My Visual Novel", cx)),
            app_name: cx.new(|cx| TextInput::new("moyu", cx)),
            initial_surface_size: cx.new(|cx| TextInput::new("1280x720", cx)),
            stage_size: cx.new(|cx| TextInput::new("1920x1080", cx)),
            window_state: "idle".to_string(),
            window_resizable: false,
            background_color: cx.new(|cx| TextInput::new("transparent", cx)),
            fonts: vec![default_font],
            multi_res_mode: "off".to_string(),
            multi_res_fixed_scale: cx.new(|cx| TextInput::new("2", cx)),
            multi_res_scales: cx.new(|cx| TextInput::new("1.5, 2", cx)),
            multi_res_policy: "quality".to_string(),
            enable_msaa: false,
            enable_mipmaps: false,
            enable_gamepads: false,
            skip_splash: false,
            show_fps: false,
            steam_enabled: false,
            steam_app_id: cx.new(|cx| TextInput::new("480", cx)),
            steam_required: false,
            steam_restart_through_client: false,
            cursor_enabled: false,
            cursors: Vec::new(),
            advanced_open: false,
            entry_filename: cx.new(|cx| TextInput::new("index.js", cx)),
            entry: cx.new(|cx| TextInput::new("", cx)),
            autorun: "all".to_string(),
            backend: "auto".to_string(),
            present_mode: "recommended".to_string(),
            desired_maximum_frame_latency: cx.new(|cx| TextInput::new("2", cx)),
            params: cx.new(|cx| TextInput::new("", cx)),
            save_error: None,
        }
    }

    pub fn load_from(&mut self, form: &RuntimeConfigForm, cx: &mut Context<DevtoolApp>) {
        self.window_title
            .update(cx, |i, cx| i.set_text(form.window_title.clone(), cx));
        self.app_name
            .update(cx, |i, cx| i.set_text(form.app_name.clone(), cx));
        self.initial_surface_size.update(cx, |i, cx| {
            i.set_text(form.initial_surface_size.clone(), cx)
        });
        self.stage_size
            .update(cx, |i, cx| i.set_text(form.stage_size.clone(), cx));
        self.window_state = form.window_state.clone();
        self.window_resizable = form.window_resizable;
        self.background_color
            .update(cx, |i, cx| i.set_text(form.background_color.clone(), cx));
        self.fonts = if form.fonts.is_empty() {
            vec![FontRowEditor::from_entry(&FontEntryForm::default(), cx)]
        } else {
            form.fonts
                .iter()
                .map(|f| FontRowEditor::from_entry(f, cx))
                .collect()
        };

        self.multi_res_mode = form.multi_res_mode.clone();
        self.multi_res_fixed_scale.update(cx, |i, cx| {
            i.set_text(form.multi_res_fixed_scale.clone(), cx)
        });
        self.multi_res_scales
            .update(cx, |i, cx| i.set_text(form.multi_res_scales.clone(), cx));
        self.multi_res_policy = form.multi_res_policy.clone();

        self.enable_msaa = form.enable_msaa;
        self.enable_mipmaps = form.enable_mipmaps;
        self.enable_gamepads = form.enable_gamepads;
        self.skip_splash = form.skip_splash;
        self.show_fps = form.show_fps;
        self.steam_enabled = form.steam_enabled;
        self.steam_app_id
            .update(cx, |i, cx| i.set_text(form.steam_app_id.clone(), cx));
        self.steam_required = form.steam_required;
        self.steam_restart_through_client = form.steam_restart_through_client;
        self.cursor_enabled = form.cursor_enabled;
        self.cursors = form
            .cursors
            .iter()
            .map(|c| CursorRowEditor::from_entry(c, cx))
            .collect();

        self.entry_filename
            .update(cx, |i, cx| i.set_text(form.entry_filename.clone(), cx));
        self.entry
            .update(cx, |i, cx| i.set_text(form.entry.clone(), cx));
        self.autorun = form.autorun.clone();
        self.backend = form.backend.clone();
        self.present_mode = form.present_mode.clone();
        self.desired_maximum_frame_latency.update(cx, |i, cx| {
            i.set_text(form.desired_maximum_frame_latency.clone(), cx)
        });
        self.params
            .update(cx, |i, cx| i.set_text(form.params.clone(), cx));
        self.advanced_open = false;
        self.save_error = None;
    }

    pub fn to_form(&self, cx: &Context<DevtoolApp>) -> RuntimeConfigForm {
        RuntimeConfigForm {
            window_title: self.window_title.read(cx).text().to_string(),
            app_name: self.app_name.read(cx).text().to_string(),
            initial_surface_size: self.initial_surface_size.read(cx).text().to_string(),
            stage_size: self.stage_size.read(cx).text().to_string(),
            window_state: self.window_state.clone(),
            window_resizable: self.window_resizable,
            background_color: self.background_color.read(cx).text().to_string(),
            fonts: self
                .fonts
                .iter()
                .map(|f| FontEntryForm {
                    path: f.path.read(cx).text().to_string(),
                    alias: f.alias.read(cx).text().to_string(),
                    kind: f.kind.clone(),
                })
                .collect(),
            multi_res_mode: self.multi_res_mode.clone(),
            multi_res_fixed_scale: self.multi_res_fixed_scale.read(cx).text().to_string(),
            multi_res_scales: self.multi_res_scales.read(cx).text().to_string(),
            multi_res_policy: self.multi_res_policy.clone(),
            enable_msaa: self.enable_msaa,
            enable_mipmaps: self.enable_mipmaps,
            enable_gamepads: self.enable_gamepads,
            skip_splash: self.skip_splash,
            show_fps: self.show_fps,
            steam_enabled: self.steam_enabled,
            steam_app_id: self.steam_app_id.read(cx).text().to_string(),
            steam_required: self.steam_required,
            steam_restart_through_client: self.steam_restart_through_client,
            cursor_enabled: self.cursor_enabled,
            cursors: self
                .cursors
                .iter()
                .map(|c| CursorEntryForm {
                    state: c.state.read(cx).text().to_string(),
                    src: c.src.read(cx).text().to_string(),
                    hotspot_x: c.hotspot_x.read(cx).text().to_string(),
                    hotspot_y: c.hotspot_y.read(cx).text().to_string(),
                })
                .collect(),
            entry_filename: self.entry_filename.read(cx).text().to_string(),
            entry: self.entry.read(cx).text().to_string(),
            autorun: self.autorun.clone(),
            backend: self.backend.clone(),
            present_mode: self.present_mode.clone(),
            desired_maximum_frame_latency: self
                .desired_maximum_frame_latency
                .read(cx)
                .text()
                .to_string(),
            params: self.params.read(cx).text().to_string(),
        }
    }
}

/// Encapsulated editor state for `moyu.json`'s `engine` section.
pub struct EngineConfigEditors {
    pub channel: String,
    pub cdn_url: Entity<TextInput>,
    pub version: Entity<TextInput>,
    pub advanced_open: bool,
    pub save_error: Option<String>,
}

impl EngineConfigEditors {
    pub fn new(cx: &mut Context<DevtoolApp>) -> Self {
        Self {
            channel: crate::config::DEFAULT_CHANNEL.to_string(),
            cdn_url: cx.new(|cx| TextInput::new(crate::config::DEFAULT_CDN_URL, cx)),
            version: cx.new(|cx| TextInput::new("", cx)),
            advanced_open: false,
            save_error: None,
        }
    }

    pub fn load_from(&mut self, form: &EngineProjectConfigForm, cx: &mut Context<DevtoolApp>) {
        self.channel = form.channel.clone();
        self.cdn_url
            .update(cx, |i, cx| i.set_text(form.cdn_url.clone(), cx));
        self.version
            .update(cx, |i, cx| i.set_text(form.version.clone(), cx));
        self.advanced_open = !form.cdn_url.is_empty() || !form.version.is_empty();
        self.save_error = None;
    }

    pub fn to_form(&self, cx: &Context<DevtoolApp>) -> EngineProjectConfigForm {
        EngineProjectConfigForm {
            channel: self.channel.clone(),
            cdn_url: self.cdn_url.read(cx).text().to_string(),
            version: self.version.read(cx).text().to_string(),
        }
    }
}

/// Encapsulated editor state for `moyu.json`'s `android` section.
pub struct AndroidConfigEditors {
    pub app_name: Entity<TextInput>,
    pub application_id: Entity<TextInput>,
    pub version_name: Entity<TextInput>,
    pub version_code: Entity<TextInput>,
    pub orientation: String,
    pub icon_source: Entity<TextInput>,
    pub icon_bg_color: Entity<TextInput>,
    pub foreground_padding: Entity<TextInput>,
    pub keystore_path: Entity<TextInput>,
    pub key_alias: Entity<TextInput>,
    pub advanced_open: bool,
    pub save_error: Option<String>,
}

impl AndroidConfigEditors {
    pub fn new(cx: &mut Context<DevtoolApp>) -> Self {
        Self {
            app_name: cx.new(|cx| TextInput::new("My Visual Novel", cx)),
            application_id: cx.new(|cx| TextInput::new("com.example.game", cx)),
            version_name: cx.new(|cx| TextInput::new("1.0.0", cx)),
            version_code: cx.new(|cx| TextInput::new("1", cx)),
            orientation: "landscape".to_string(),
            icon_source: cx.new(|cx| TextInput::new("assets/icon.png", cx)),
            icon_bg_color: cx.new(|cx| TextInput::new("#ffffff", cx)),
            foreground_padding: cx.new(|cx| TextInput::new("0.18", cx)),
            keystore_path: cx.new(|cx| TextInput::new("release.keystore", cx)),
            key_alias: cx.new(|cx| TextInput::new("release", cx)),
            advanced_open: false,
            save_error: None,
        }
    }

    pub fn load_from(&mut self, cfg: &AndroidConfig, cx: &mut Context<DevtoolApp>) {
        self.app_name
            .update(cx, |i, cx| i.set_text(cfg.app_name.clone(), cx));
        self.application_id
            .update(cx, |i, cx| i.set_text(cfg.application_id.clone(), cx));
        self.version_name
            .update(cx, |i, cx| i.set_text(cfg.version_name.clone(), cx));
        self.version_code
            .update(cx, |i, cx| i.set_text(cfg.version_code.to_string(), cx));
        self.orientation = cfg.orientation.clone();
        self.icon_source
            .update(cx, |i, cx| i.set_text(cfg.icon_source.clone(), cx));
        self.icon_bg_color
            .update(cx, |i, cx| i.set_text(cfg.icon_bg_color.clone(), cx));
        self.foreground_padding.update(cx, |i, cx| {
            i.set_text(format!("{}", cfg.foreground_padding), cx)
        });
        self.keystore_path
            .update(cx, |i, cx| i.set_text(cfg.keystore_path.clone(), cx));
        self.key_alias
            .update(cx, |i, cx| i.set_text(cfg.key_alias.clone(), cx));
        self.advanced_open = false;
        self.save_error = None;
    }

    pub fn to_config(&self, cx: &Context<DevtoolApp>) -> anyhow::Result<AndroidConfig> {
        let version_code: u32 = self
            .version_code
            .read(cx)
            .text()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Android versionCode must be a positive integer."))?;
        let foreground_padding: f64 = self
            .foreground_padding
            .read(cx)
            .text()
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Foreground padding must be a number (0.0 to 0.5)."))?;
        Ok(AndroidConfig {
            app_name: self.app_name.read(cx).text().to_string(),
            application_id: self.application_id.read(cx).text().to_string(),
            version_name: self.version_name.read(cx).text().to_string(),
            version_code,
            orientation: self.orientation.clone(),
            icon_source: self.icon_source.read(cx).text().to_string(),
            icon_bg_color: self.icon_bg_color.read(cx).text().to_string(),
            foreground_padding,
            keystore_path: self.keystore_path.read(cx).text().to_string(),
            key_alias: self.key_alias.read(cx).text().to_string(),
        })
    }
}

fn config_modal_shell(
    id: usize,
    width: f32,
    title: &str,
    subtitle: &str,
    error: Option<String>,
    theme: Theme,
    body: AnyElement,
    footer: AnyElement,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let panel = div()
        .id(("cfg-modal-box", id))
        .w(px(width))
        .max_w(px(width))
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
                .px(px(16.0))
                .pt(px(14.0))
                .pb(px(10.0))
                .border_b_1()
                .border_color(theme.rule)
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
                                .text_size(px(15.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.foreground)
                                .child(title.to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.meta)
                                .child(subtitle.to_string()),
                        ),
                )
                .child(icon_button(
                    ("cfg-modal-close", id),
                    IconName::X,
                    theme,
                    cx.listener(|this, _, _, cx| this.close_dialog(cx)),
                )),
        )
        .when_some(error, |p, err| {
            p.child(
                div()
                    .mx(px(16.0))
                    .mt(px(10.0))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .bg(theme.card)
                    .border_1()
                    .border_color(theme.destructive)
                    .text_size(px(12.0))
                    .text_color(theme.destructive)
                    .child(err),
            )
        })
        .child(
            div()
                .id(("cfg-modal-scroll", id))
                .max_h(px(520.0))
                .overflow_y_scroll()
                .px(px(16.0))
                .py(px(12.0))
                .child(body),
        )
        .child(
            div()
                .px(px(16.0))
                .py(px(10.0))
                .rounded_b(px(6.0))
                .bg(theme.card)
                .border_t_1()
                .border_color(theme.rule)
                .flex()
                .flex_row()
                .justify_end()
                .gap(px(8.0))
                .child(footer),
        );

    let backdrop = div()
        .id(("cfg-modal-backdrop", id))
        .absolute()
        .inset_0()
        .bg(theme.backdrop)
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| this.close_dialog(cx)),
        )
        .child(animate_modal_panel(("cfg-modal-panel-anim", id), panel));

    animate_modal_backdrop(("cfg-modal-backdrop-anim", id), backdrop).into_any_element()
}

fn form_section_title(title: &str, theme: Theme) -> gpui::Div {
    div()
        .pt(px(8.0))
        .pb(px(4.0))
        .text_size(px(12.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.meta)
        .child(title.to_string())
}

fn form_row(label: &str, control: AnyElement, theme: Theme) -> gpui::Div {
    div()
        .min_h(px(36.0))
        .py(px(4.0))
        .border_t_1()
        .border_color(theme.rule)
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .text_size(px(13.0))
                .text_color(theme.foreground)
                .flex_shrink_0()
                .child(label.to_string()),
        )
        .child(control)
}

fn toggle_btn(
    id: &'static str,
    enabled: bool,
    theme: Theme,
    _i18n: I18n,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    switch(id, enabled, theme, on_click).into_any_element()
}

fn collapsible_header(
    id: &'static str,
    label: &str,
    open: bool,
    theme: Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .mt(px(8.0))
        .py(px(6.0))
        .px(px(8.0))
        .rounded(px(4.0))
        .bg(theme.card)
        .border_1()
        .border_color(theme.rule)
        .hover(move |s| s.bg(theme.accent))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(label.to_string()),
        )
        .child(icon(
            if open {
                IconName::ChevronUp
            } else {
                IconName::ChevronDown
            },
            14.0,
            theme.meta,
        ))
}

pub fn render_runtime_config_dialog(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let rt = &app.rt_cfg;
    let ctrl_w = 280.0;

    let win_states = ["idle", "maximized", "minimized", "fullscreen"];
    let win_state_items: Vec<AnyElement> = win_states
        .iter()
        .enumerate()
        .map(|(i, &st)| {
            menu_item(
                ("rt-ws-opt", i),
                st,
                rt.window_state == st,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.rt_cfg.window_state = st.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let mr_modes = ["off", "auto", "fixed"];
    let mr_mode_items: Vec<AnyElement> = mr_modes
        .iter()
        .enumerate()
        .map(|(i, &m)| {
            menu_item(
                ("rt-mrm-opt", i),
                m,
                rt.multi_res_mode == m,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.rt_cfg.multi_res_mode = m.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let mr_policies = ["quality", "balanced"];
    let mr_policy_items: Vec<AnyElement> = mr_policies
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            menu_item(
                ("rt-mrp-opt", i),
                p,
                rt.multi_res_policy == p,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.rt_cfg.multi_res_policy = p.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let autorun_opts = ["all", "native-only"];
    let autorun_items: Vec<AnyElement> = autorun_opts
        .iter()
        .enumerate()
        .map(|(i, &a)| {
            menu_item(
                ("rt-ar-opt", i),
                a,
                rt.autorun == a,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.rt_cfg.autorun = a.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let backend_opts = ["auto", "vulkan", "metal", "dx12", "webgpu", "gles"];
    let backend_items: Vec<AnyElement> = backend_opts
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            menu_item(
                ("rt-be-opt", i),
                b,
                rt.backend == b,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.rt_cfg.backend = b.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let pm_opts = ["recommended", "autovsync", "autonovsync"];
    let pm_items: Vec<AnyElement> = pm_opts
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            menu_item(
                ("rt-pm-opt", i),
                p,
                rt.present_mode == p,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.rt_cfg.present_mode = p.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        // --- Section 1: Window & Basics ---
        .child(form_section_title(i18n.cfg_sec_window(), theme))
        .child(form_row(
            i18n.window_title_label(),
            div()
                .w(px(ctrl_w))
                .child(rt.window_title.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.cfg_app_name(),
            div()
                .w(px(ctrl_w))
                .child(rt.app_name.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.window_size_label(),
            div()
                .w(px(ctrl_w))
                .child(rt.initial_surface_size.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.stage_size_label(),
            div()
                .w(px(ctrl_w))
                .child(rt.stage_size.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.cfg_window_state(),
            select_dropdown(
                "rt-ws-select",
                rt.window_state.clone(),
                app.active_popover == Some(ActivePopover::RuntimeWindowState),
                ctrl_w,
                true,
                theme,
                cx.listener(|this, _, _, cx| {
                    this.toggle_popover(ActivePopover::RuntimeWindowState, cx);
                }),
                win_state_items,
            ),
            theme,
        ))
        .child(form_row(
            i18n.cfg_window_resizable(),
            toggle_btn(
                "rt-toggle-resizable",
                rt.window_resizable,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.window_resizable = !this.rt_cfg.window_resizable;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .child(form_row(
            i18n.cfg_background_color(),
            div()
                .w(px(ctrl_w))
                .child(rt.background_color.clone())
                .into_any_element(),
            theme,
        ))
        .child(
            div()
                .py(px(6.0))
                .border_t_1()
                .border_color(theme.rule)
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
                                .text_size(px(13.0))
                                .text_color(theme.foreground)
                                .child(i18n.cfg_font_file()),
                        )
                        .child(sm_button(
                            "rt-add-font",
                            format!("+ {}", i18n.cfg_add_font()),
                            ButtonVariant::Secondary,
                            theme,
                            cx.listener(|this, _, _, cx| {
                                let row = FontRowEditor::from_entry(
                                    &FontEntryForm {
                                        path: String::new(),
                                        alias: String::new(),
                                        kind: String::new(),
                                    },
                                    cx,
                                );
                                this.rt_cfg.fonts.push(row);
                                cx.notify();
                            }),
                        )),
                )
                .children({
                    let font_count = rt.fonts.len();
                    rt.fonts
                        .iter()
                        .enumerate()
                        .map(|(idx, f)| {
                            let kind_label = match f.kind.as_str() {
                                "cjk" => "CJK",
                                "western" => "Western",
                                _ => i18n.cfg_font_kind_auto(),
                            };
                            let kind_opts = [
                                ("", i18n.cfg_font_kind_auto()),
                                ("cjk", "CJK"),
                                ("western", "Western"),
                            ];
                            let kind_items: Vec<AnyElement> = kind_opts
                                .iter()
                                .enumerate()
                                .map(|(opt_i, &(val, label))| {
                                    menu_item(
                                        ("rt-fk-opt", idx * 10 + opt_i),
                                        label,
                                        f.kind == val,
                                        theme,
                                        cx.listener(move |this, _, _, cx| {
                                            if let Some(entry) = this.rt_cfg.fonts.get_mut(idx) {
                                                entry.kind = val.to_string();
                                            }
                                            this.active_popover = None;
                                            cx.notify();
                                        }),
                                    )
                                    .into_any_element()
                                })
                                .collect();
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(div().flex_1().min_w_0().child(f.path.clone()))
                                .child(div().w(px(110.0)).flex_shrink_0().child(f.alias.clone()))
                                .child(select_dropdown(
                                    ("rt-font-kind", idx),
                                    kind_label,
                                    app.active_popover == Some(ActivePopover::RuntimeFontKind(idx)),
                                    96.0,
                                    true,
                                    theme,
                                    cx.listener(move |this, _, _, cx| {
                                        this.toggle_popover(
                                            ActivePopover::RuntimeFontKind(idx),
                                            cx,
                                        );
                                    }),
                                    kind_items,
                                ))
                                .when(font_count > 1, |r| {
                                    r.child(
                                        div()
                                            .id(("rt-font-del", idx))
                                            .size(px(26.0))
                                            .flex_shrink_0()
                                            .rounded(px(4.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(theme.destructive.opacity(0.14)))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if this.rt_cfg.fonts.len() > 1
                                                    && idx < this.rt_cfg.fonts.len()
                                                {
                                                    this.rt_cfg.fonts.remove(idx);
                                                    cx.notify();
                                                }
                                            }))
                                            .child(icon(IconName::X, 13.0, theme.destructive)),
                                    )
                                })
                        })
                        .collect::<Vec<_>>()
                }),
        )
        // --- Section 2: Multi-Resolution Assets ---
        .child(form_section_title(i18n.multi_res_title(), theme))
        .child(form_row(
            i18n.cfg_multi_res_mode(),
            select_dropdown(
                "rt-mrm-select",
                rt.multi_res_mode.clone(),
                app.active_popover == Some(ActivePopover::RuntimeMultiResMode),
                ctrl_w,
                true,
                theme,
                cx.listener(|this, _, _, cx| {
                    this.toggle_popover(ActivePopover::RuntimeMultiResMode, cx);
                }),
                mr_mode_items,
            ),
            theme,
        ))
        .when(rt.multi_res_mode == "fixed", |d| {
            d.child(form_row(
                i18n.cfg_multi_res_fixed(),
                div()
                    .w(px(ctrl_w))
                    .child(rt.multi_res_fixed_scale.clone())
                    .into_any_element(),
                theme,
            ))
        })
        .when(rt.multi_res_mode == "auto", |d| {
            d.child(form_row(
                i18n.cfg_multi_res_scales(),
                div()
                    .w(px(ctrl_w))
                    .child(rt.multi_res_scales.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_multi_res_policy(),
                select_dropdown(
                    "rt-mrp-select",
                    rt.multi_res_policy.clone(),
                    app.active_popover == Some(ActivePopover::RuntimeMultiResPolicy),
                    ctrl_w,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::RuntimeMultiResPolicy, cx);
                    }),
                    mr_policy_items,
                ),
                theme,
            ))
        })
        // --- Section 3: Features & Platform (including MSAA & Mipmaps) ---
        .child(form_section_title(i18n.cfg_sec_features(), theme))
        .child(form_row(
            "MSAA (enableMSAA)",
            toggle_btn(
                "rt-toggle-msaa",
                rt.enable_msaa,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.enable_msaa = !this.rt_cfg.enable_msaa;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .child(form_row(
            "Mipmaps (enableMipmaps)",
            toggle_btn(
                "rt-toggle-mipmaps",
                rt.enable_mipmaps,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.enable_mipmaps = !this.rt_cfg.enable_mipmaps;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .child(form_row(
            i18n.cfg_enable_gamepads(),
            toggle_btn(
                "rt-toggle-gamepads",
                rt.enable_gamepads,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.enable_gamepads = !this.rt_cfg.enable_gamepads;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .child(form_row(
            i18n.cfg_skip_splash(),
            toggle_btn(
                "rt-toggle-splash",
                rt.skip_splash,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.skip_splash = !this.rt_cfg.skip_splash;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .child(form_row(
            i18n.cfg_show_fps(),
            toggle_btn(
                "rt-toggle-fps",
                rt.show_fps,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.show_fps = !this.rt_cfg.show_fps;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .child(form_row(
            i18n.cfg_steam_enable(),
            toggle_btn(
                "rt-toggle-steam",
                rt.steam_enabled,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.steam_enabled = !this.rt_cfg.steam_enabled;
                    cx.notify();
                }),
            ),
            theme,
        ))
        .when(rt.steam_enabled, |d| {
            d.child(form_row(
                "Steam App ID",
                div()
                    .w(px(ctrl_w))
                    .child(rt.steam_app_id.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_steam_required(),
                toggle_btn(
                    "rt-toggle-steam-req",
                    rt.steam_required,
                    theme,
                    i18n,
                    cx.listener(|this, _, _, cx| {
                        this.rt_cfg.steam_required = !this.rt_cfg.steam_required;
                        cx.notify();
                    }),
                ),
                theme,
            ))
            .child(form_row(
                i18n.cfg_steam_restart(),
                toggle_btn(
                    "rt-toggle-steam-rst",
                    rt.steam_restart_through_client,
                    theme,
                    i18n,
                    cx.listener(|this, _, _, cx| {
                        this.rt_cfg.steam_restart_through_client =
                            !this.rt_cfg.steam_restart_through_client;
                        cx.notify();
                    }),
                ),
                theme,
            ))
        })
        .child(form_row(
            i18n.cfg_cursor(),
            toggle_btn(
                "rt-toggle-cursor",
                rt.cursor_enabled,
                theme,
                i18n,
                cx.listener(|this, _, _, cx| {
                    this.rt_cfg.cursor_enabled = !this.rt_cfg.cursor_enabled;
                    if this.rt_cfg.cursor_enabled && this.rt_cfg.cursors.is_empty() {
                        let row = CursorRowEditor::from_entry(&CursorEntryForm::default(), cx);
                        this.rt_cfg.cursors.push(row);
                    }
                    cx.notify();
                }),
            ),
            theme,
        ))
        .when(rt.cursor_enabled, |d| {
            d.child(
                div()
                    .pb(px(6.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .children(
                        rt.cursors
                            .iter()
                            .enumerate()
                            .map(|(idx, c)| {
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div().w(px(110.0)).flex_shrink_0().child(c.state.clone()),
                                    )
                                    .child(div().flex_1().min_w_0().child(c.src.clone()))
                                    .child(
                                        div()
                                            .w(px(48.0))
                                            .flex_shrink_0()
                                            .child(c.hotspot_x.clone()),
                                    )
                                    .child(
                                        div()
                                            .w(px(48.0))
                                            .flex_shrink_0()
                                            .child(c.hotspot_y.clone()),
                                    )
                                    .child(
                                        div()
                                            .id(("rt-cursor-del", idx))
                                            .size(px(26.0))
                                            .flex_shrink_0()
                                            .rounded(px(4.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(theme.destructive.opacity(0.14)))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if idx < this.rt_cfg.cursors.len() {
                                                    this.rt_cfg.cursors.remove(idx);
                                                    cx.notify();
                                                }
                                            }))
                                            .child(icon(IconName::X, 13.0, theme.destructive)),
                                    )
                            })
                            .collect::<Vec<_>>(),
                    )
                    .child(div().flex().flex_row().justify_end().child(sm_button(
                        "rt-add-cursor",
                        format!("+ {}", i18n.cfg_add_cursor()),
                        ButtonVariant::Secondary,
                        theme,
                        cx.listener(|this, _, _, cx| {
                            let default_state = if this.rt_cfg.cursors.is_empty() {
                                "default"
                            } else {
                                "pointer"
                            };
                            let row = CursorRowEditor::from_entry(
                                &CursorEntryForm {
                                    state: default_state.to_string(),
                                    src: String::new(),
                                    hotspot_x: String::new(),
                                    hotspot_y: String::new(),
                                },
                                cx,
                            );
                            this.rt_cfg.cursors.push(row);
                            cx.notify();
                        }),
                    ))),
            )
        })
        // --- Section 4: Advanced Rendering & Entry (collapsible) ---
        .child(collapsible_header(
            "rt-toggle-advanced",
            i18n.cfg_sec_advanced(),
            rt.advanced_open,
            theme,
            cx.listener(|this, _, _, cx| {
                this.rt_cfg.advanced_open = !this.rt_cfg.advanced_open;
                cx.notify();
            }),
        ))
        .when(rt.advanced_open, |d| {
            d.child(form_row(
                i18n.entry_script_label(),
                div()
                    .w(px(ctrl_w))
                    .child(rt.entry_filename.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_entry_redirect(),
                div()
                    .w(px(ctrl_w))
                    .child(rt.entry.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_autorun(),
                select_dropdown(
                    "rt-ar-select",
                    rt.autorun.clone(),
                    app.active_popover == Some(ActivePopover::RuntimeAutorun),
                    ctrl_w,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::RuntimeAutorun, cx);
                    }),
                    autorun_items,
                ),
                theme,
            ))
            .child(form_row(
                i18n.cfg_backend(),
                select_dropdown(
                    "rt-be-select",
                    rt.backend.clone(),
                    app.active_popover == Some(ActivePopover::RuntimeBackend),
                    ctrl_w,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::RuntimeBackend, cx);
                    }),
                    backend_items,
                ),
                theme,
            ))
            .child(form_row(
                i18n.cfg_present_mode(),
                select_dropdown(
                    "rt-pm-select",
                    rt.present_mode.clone(),
                    app.active_popover == Some(ActivePopover::RuntimePresentMode),
                    ctrl_w,
                    true,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_popover(ActivePopover::RuntimePresentMode, cx);
                    }),
                    pm_items,
                ),
                theme,
            ))
            .child(form_row(
                i18n.cfg_frame_latency(),
                div()
                    .w(px(ctrl_w))
                    .child(rt.desired_maximum_frame_latency.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_params(),
                div()
                    .w(px(ctrl_w))
                    .child(rt.params.clone())
                    .into_any_element(),
                theme,
            ))
        })
        .into_any_element();

    let footer = div()
        .flex()
        .flex_row()
        .gap(px(8.0))
        .child(button(
            "dlg-rt-cancel",
            i18n.cancel(),
            ButtonVariant::Secondary,
            theme,
            cx.listener(|this, _, _, cx| this.close_dialog(cx)),
        ))
        .child(button(
            "dlg-rt-save",
            i18n.save(),
            ButtonVariant::Primary,
            theme,
            cx.listener(|this, _, _, cx| this.save_runtime_config_dialog(cx)),
        ))
        .into_any_element();

    config_modal_shell(
        10,
        640.0,
        i18n.runtime_config_title(),
        "index.json",
        rt.save_error.clone(),
        theme,
        body,
        footer,
        cx,
    )
}

pub fn render_engine_config_dialog(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let eng = &app.eng_cfg;
    let ctrl_w = 240.0;
    let channels = app.available_config_channels();
    let ch_items: Vec<AnyElement> = channels
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let sel = eng.channel == ch;
            let ch_clone = ch.clone();
            menu_item(
                ("eng-cfg-ch-opt", i),
                ch,
                sel,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.eng_cfg.channel = ch_clone.clone();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(form_row(
            i18n.engine_channel_label(),
            select_dropdown(
                "eng-cfg-ch-select",
                eng.channel.clone(),
                app.active_popover == Some(ActivePopover::EngineCfgChannel),
                ctrl_w,
                true,
                theme,
                cx.listener(|this, _, _, cx| {
                    this.toggle_popover(ActivePopover::EngineCfgChannel, cx);
                }),
                ch_items,
            ),
            theme,
        ))
        .child(collapsible_header(
            "eng-cfg-toggle-adv",
            i18n.cfg_engine_advanced(),
            eng.advanced_open,
            theme,
            cx.listener(|this, _, _, cx| {
                this.eng_cfg.advanced_open = !this.eng_cfg.advanced_open;
                cx.notify();
            }),
        ))
        .when(eng.advanced_open, |d| {
            d.child(form_row(
                "CDN URL",
                div()
                    .w(px(ctrl_w))
                    .child(eng.cdn_url.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_engine_pin_version(),
                div()
                    .w(px(ctrl_w))
                    .child(eng.version.clone())
                    .into_any_element(),
                theme,
            ))
        })
        .into_any_element();

    let footer = div()
        .flex()
        .flex_row()
        .gap(px(8.0))
        .child(button(
            "dlg-eng-cancel",
            i18n.cancel(),
            ButtonVariant::Secondary,
            theme,
            cx.listener(|this, _, _, cx| this.close_dialog(cx)),
        ))
        .child(button(
            "dlg-eng-save",
            i18n.save(),
            ButtonVariant::Primary,
            theme,
            cx.listener(|this, _, _, cx| this.save_engine_config_dialog(cx)),
        ))
        .into_any_element();

    config_modal_shell(
        11,
        460.0,
        i18n.engine_config_title(),
        "moyu.json · engine",
        eng.save_error.clone(),
        theme,
        body,
        footer,
        cx,
    )
}

pub fn render_android_config_dialog(
    app: &DevtoolApp,
    theme: Theme,
    i18n: I18n,
    cx: &mut Context<DevtoolApp>,
) -> AnyElement {
    let and = &app.and_cfg;
    let ctrl_w = 260.0;

    let orient_opts = ["landscape", "portrait", "sensor"];
    let orient_items: Vec<AnyElement> = orient_opts
        .iter()
        .enumerate()
        .map(|(i, &o)| {
            menu_item(
                ("and-ori-opt", i),
                o,
                and.orientation == o,
                theme,
                cx.listener(move |this, _, _, cx| {
                    this.and_cfg.orientation = o.to_string();
                    this.active_popover = None;
                    cx.notify();
                }),
            )
            .into_any_element()
        })
        .collect();

    let body = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(form_section_title(i18n.cfg_android_basic(), theme))
        .child(form_row(
            i18n.cfg_android_app_name(),
            div()
                .w(px(ctrl_w))
                .child(and.app_name.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            "Application ID",
            div()
                .w(px(ctrl_w))
                .child(and.application_id.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.cfg_android_version_name(),
            div()
                .w(px(ctrl_w))
                .child(and.version_name.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.cfg_android_version_code(),
            div()
                .w(px(ctrl_w))
                .child(and.version_code.clone())
                .into_any_element(),
            theme,
        ))
        .child(form_row(
            i18n.cfg_android_orientation(),
            select_dropdown(
                "and-ori-select",
                and.orientation.clone(),
                app.active_popover == Some(ActivePopover::AndroidCfgOrientation),
                ctrl_w,
                true,
                theme,
                cx.listener(|this, _, _, cx| {
                    this.toggle_popover(ActivePopover::AndroidCfgOrientation, cx);
                }),
                orient_items,
            ),
            theme,
        ))
        .child(collapsible_header(
            "and-cfg-toggle-adv",
            i18n.cfg_android_icon_signing(),
            and.advanced_open,
            theme,
            cx.listener(|this, _, _, cx| {
                this.and_cfg.advanced_open = !this.and_cfg.advanced_open;
                cx.notify();
            }),
        ))
        .when(and.advanced_open, |d| {
            d.child(form_row(
                i18n.cfg_android_icon_source(),
                div()
                    .w(px(ctrl_w))
                    .child(and.icon_source.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_android_icon_bg(),
                div()
                    .w(px(ctrl_w))
                    .child(and.icon_bg_color.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_android_icon_padding(),
                div()
                    .w(px(ctrl_w))
                    .child(and.foreground_padding.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_android_keystore(),
                div()
                    .w(px(ctrl_w))
                    .child(and.keystore_path.clone())
                    .into_any_element(),
                theme,
            ))
            .child(form_row(
                i18n.cfg_android_key_alias(),
                div()
                    .w(px(ctrl_w))
                    .child(and.key_alias.clone())
                    .into_any_element(),
                theme,
            ))
        })
        .into_any_element();

    let footer = div()
        .flex()
        .flex_row()
        .gap(px(8.0))
        .child(button(
            "dlg-and-cancel",
            i18n.cancel(),
            ButtonVariant::Secondary,
            theme,
            cx.listener(|this, _, _, cx| this.close_dialog(cx)),
        ))
        .child(button(
            "dlg-and-save",
            i18n.save(),
            ButtonVariant::Primary,
            theme,
            cx.listener(|this, _, _, cx| this.save_android_config_dialog(cx)),
        ))
        .into_any_element();

    config_modal_shell(
        12,
        520.0,
        i18n.android_config_title(),
        "moyu.json · android",
        and.save_error.clone(),
        theme,
        body,
        footer,
        cx,
    )
}
