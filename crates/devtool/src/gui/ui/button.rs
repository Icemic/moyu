//! FishFlow-styled Button primitives with orthogonal `ButtonVariant` and `ButtonSize`.

use gpui::{ClickEvent, ElementId, SharedString, Window, div, prelude::*, px};

use crate::gui::theme::Theme;
use crate::gui::ui::icon::{IconName, icon};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Outline,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonSize {
    /// 26px tall, 12px text — form toggles, panel header actions, inline controls.
    Sm,
    /// 32px tall, 13px text — primary toolbar and modal actions.
    Md,
}

pub fn button_sized(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let (bg, hover_bg, fg, border_color) = match variant {
        ButtonVariant::Primary => (
            theme.primary,
            theme.primary_hover,
            theme.primary_foreground,
            gpui::transparent_black(),
        ),
        ButtonVariant::Secondary => (
            theme.secondary,
            theme.secondary_hover,
            theme.secondary_foreground,
            gpui::transparent_black(),
        ),
        ButtonVariant::Outline => (
            theme.background,
            theme.accent,
            theme.foreground,
            theme.input,
        ),
        ButtonVariant::Ghost => (
            gpui::transparent_black(),
            theme.accent,
            theme.foreground,
            gpui::transparent_black(),
        ),
        ButtonVariant::Danger => (
            theme.destructive,
            theme.destructive_hover,
            theme.destructive_foreground,
            gpui::transparent_black(),
        ),
    };

    let (height, pad_x, text_sz) = match size {
        ButtonSize::Sm => (26.0, 9.0, 12.0),
        ButtonSize::Md => (32.0, 11.0, 13.0),
    };

    div()
        .id(id)
        .h(px(height))
        .px(px(pad_x))
        .rounded(px(4.0))
        .border_1()
        .border_color(border_color)
        .bg(bg)
        .text_color(fg)
        .text_size(px(text_sz))
        .font_weight(gpui::FontWeight::MEDIUM)
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_click(on_click)
        .child(label.into())
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    button_sized(id, label, variant, ButtonSize::Md, theme, on_click)
}

pub fn sm_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    button_sized(id, label, variant, ButtonSize::Sm, theme, on_click)
}

pub fn icon_button(
    id: impl Into<ElementId>,
    icon_name: IconName,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(26.0))
        .rounded(px(4.0))
        .bg(gpui::transparent_black())
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .cursor_pointer()
        .hover(move |s| s.bg(theme.accent))
        .on_click(on_click)
        .child(icon(icon_name, 14.0, theme.foreground))
}
