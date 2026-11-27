//! Horizontal progress bar aligned with Gallery track and accent tokens.

use gpui::{div, prelude::*, px, relative};

use crate::gui::theme::Theme;

#[allow(dead_code)]
pub fn progress_bar(ratio: f32, theme: Theme) -> impl IntoElement {
    let clamped = ratio.clamp(0.0, 1.0);
    div()
        .w_full()
        .h(px(6.0))
        .rounded(px(4.0))
        .bg(theme.track)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(relative(clamped))
                .rounded(px(4.0))
                .bg(theme.accent),
        )
}
