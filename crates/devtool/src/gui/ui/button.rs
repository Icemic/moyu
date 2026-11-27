//! Gallery-styled Button component (Default / Primary / Ghost / Danger).

use gpui::{ClickEvent, ElementId, SharedString, Window, div, prelude::*, px};

use crate::gui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButtonVariant {
    Default,
    Primary,
    Ghost,
    Danger,
}

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let (bg, hover_bg, active_bg, fg) = match variant {
        ButtonVariant::Default => (
            theme.control_idle,
            theme.control_hover,
            theme.control_press,
            theme.text,
        ),
        ButtonVariant::Primary => (
            theme.accent,
            theme.accent_hover,
            theme.accent_press,
            theme.on_accent,
        ),
        ButtonVariant::Ghost => (
            gpui::transparent_black(),
            theme.control_hover,
            theme.control_press,
            theme.text_secondary,
        ),
        ButtonVariant::Danger => (
            theme.danger,
            theme.danger_hover,
            theme.danger,
            theme.on_danger,
        ),
    };

    div()
        .id(id)
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(bg)
        .text_color(fg)
        .text_size(px(13.0))
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .active(move |s| s.bg(active_bg))
        .on_click(on_click)
        .child(label.into())
}
