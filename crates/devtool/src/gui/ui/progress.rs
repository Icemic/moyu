//! Horizontal progress bar aligned with FishFlow theme tokens.

use gpui::{div, prelude::*, px, relative};

use crate::gui::theme::Theme;

#[allow(dead_code)]
pub fn progress_bar(ratio: f32, theme: Theme) -> impl IntoElement {
    let clamped = ratio.clamp(0.0, 1.0);
    div()
        .w_full()
        .h(px(4.0))
        .rounded(px(2.0))
        .bg(theme.muted)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(relative(clamped))
                .rounded(px(2.0))
                .bg(theme.ring),
        )
}
