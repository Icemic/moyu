//! Compact status badge / tag component.

use gpui::{SharedString, div, prelude::*, px};

use crate::gui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BadgeTone {
    Neutral,
    Accent,
    Success,
    Warn,
    Danger,
}

pub fn badge(label: impl Into<SharedString>, tone: BadgeTone, theme: Theme) -> impl IntoElement {
    let (bg, fg) = match tone {
        BadgeTone::Neutral => (theme.control_idle, theme.text_secondary),
        BadgeTone::Accent => (theme.accent_soft, theme.accent),
        BadgeTone::Success => (theme.surface_muted, theme.success),
        BadgeTone::Warn => (theme.surface_muted, theme.warn),
        BadgeTone::Danger => (theme.surface_muted, theme.danger),
    };

    div()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(bg)
        .text_color(fg)
        .text_size(px(12.0))
        .child(label.into())
}
