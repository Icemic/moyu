//! FishFlow-styled panel, section heading, detail row, status dot, and chip helpers.

use gpui::{AnimationExt, AnyElement, Div, ElementId, Hsla, SharedString, div, prelude::*, px};

use crate::gui::theme::Theme;
use crate::gui::ui::motion::pulse_animation;

pub fn panel(theme: Theme) -> Div {
    div()
        .bg(theme.card)
        .text_color(theme.card_foreground)
        .border_1()
        .border_color(theme.border)
        .rounded(px(4.0))
        .px(px(14.0))
        .py(px(10.0))
        .flex()
        .flex_col()
        .min_w_0()
}

pub fn muted_panel(theme: Theme) -> Div {
    div()
        .bg(theme.background)
        .border_1()
        .border_color(theme.rule)
        .rounded(px(4.0))
        .px(px(10.0))
        .py(px(8.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .min_w_0()
}

pub fn section_heading(
    title: impl Into<SharedString>,
    right: Option<AnyElement>,
    theme: Theme,
) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .min_h(px(30.0))
        .pb(px(6.0))
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.card_foreground)
                .child(title.into()),
        )
        .when_some(right, |d, el| d.child(el))
}

pub fn detail_row(theme: Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .min_h(px(38.0))
        .py(px(6.0))
        .border_t_1()
        .border_color(theme.rule)
}

pub fn status_dot(color: Hsla) -> Div {
    div().size(px(6.0)).rounded_full().bg(color).flex_shrink_0()
}

/// Gentle pulsing status dot for starting / waiting / busy indicators.
pub fn pulsing_status_dot(id: impl Into<ElementId>, color: Hsla) -> AnyElement {
    status_dot(color)
        .with_animation(id, pulse_animation(), |dot, delta| dot.opacity(delta))
        .into_any_element()
}

pub fn chip(label: impl Into<SharedString>, theme: Theme) -> Div {
    div()
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(3.0))
        .bg(theme.secondary)
        .text_color(theme.secondary_foreground)
        .text_size(px(12.0))
        .child(label.into())
}
