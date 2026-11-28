//! Compact status badge / tag component.

use gpui::{Div, SharedString, div, prelude::*, px};

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

#[allow(dead_code)]
pub fn badge(label: impl Into<SharedString>, tone: BadgeTone, theme: Theme) -> Div {
    let (bg, fg) = match tone {
        BadgeTone::Neutral => (theme.secondary, theme.meta),
        BadgeTone::Accent => (theme.accent, theme.info),
        BadgeTone::Success => (theme.muted, theme.success),
        BadgeTone::Warn => (theme.muted, theme.warning),
        BadgeTone::Danger => (theme.muted, theme.destructive),
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
