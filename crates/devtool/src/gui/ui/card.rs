//! Surface card / panel container using Gallery borderless elevation levels.

use gpui::{Div, div, prelude::*, px};

use crate::gui::theme::Theme;

pub fn card(theme: Theme) -> Div {
    div()
        .bg(theme.surface)
        .rounded(px(8.0))
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(10.0))
}

#[allow(dead_code)]
pub fn muted_panel(theme: Theme) -> Div {
    div()
        .bg(theme.surface_muted)
        .rounded(px(6.0))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
}
