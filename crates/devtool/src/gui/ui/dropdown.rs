//! FishFlow-styled Popover & SelectDropdown primitives.
//!
//! All triggers stop `MouseDown` propagation so clicking a trigger while open
//! cleanly toggles closed instead of closing-on-mousedown and reopening-on-click.

use gpui::{
    AnyElement, ClickEvent, Div, ElementId, MouseButton, SharedString, Window, deferred, div,
    prelude::*, px,
};

use crate::gui::theme::Theme;
use crate::gui::ui::icon::{IconName, icon};

/// Floating popover container styled after FishFlow's `DropdownMenuContent`.
pub fn popover_container(
    id: impl Into<ElementId>,
    min_width: f32,
    theme: Theme,
) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .min_w(px(min_width))
        .p(px(4.0))
        .rounded(px(4.0))
        .bg(theme.popover)
        .text_color(theme.popover_foreground)
        .border_1()
        .border_color(theme.rule)
        .shadow_md()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

pub fn menu_section_label(label: impl Into<SharedString>, theme: Theme) -> Div {
    div()
        .px(px(8.0))
        .py(px(4.0))
        .text_size(px(11.0))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.muted_foreground)
        .child(label.into())
}

pub fn menu_divider(theme: Theme) -> Div {
    div().my(px(4.0)).h(px(1.0)).w_full().bg(theme.rule)
}

pub fn menu_item(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .h(px(28.0))
        .px(px(8.0))
        .rounded(px(3.0))
        .bg(if selected {
            theme.accent
        } else {
            gpui::transparent_black()
        })
        .text_color(if selected {
            theme.accent_foreground
        } else {
            theme.popover_foreground
        })
        .hover(move |s| s.bg(theme.accent).text_color(theme.accent_foreground))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(10.0))
        .text_size(px(13.0))
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(label.into())
        .when(selected, |row| {
            row.child(icon(IconName::Check, 14.0, theme.meta))
        })
}

/// Reusable single-select dropdown matching FishFlow's `SelectDropdown`.
#[allow(clippy::too_many_arguments)]
pub fn select_dropdown(
    id: impl Into<ElementId>,
    current_label: impl Into<SharedString>,
    is_open: bool,
    width: f32,
    align_right: bool,
    theme: Theme,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    menu_items: Vec<AnyElement>,
) -> AnyElement {
    let trigger_id: ElementId = id.into();
    div()
        .relative()
        .w(px(width))
        .flex_shrink_0()
        .child(
            div()
                .id(trigger_id)
                .h(px(30.0))
                .w_full()
                .px(px(10.0))
                .rounded(px(4.0))
                .bg(theme.background)
                .border_1()
                .border_color(if is_open { theme.ring } else { theme.input })
                .text_color(theme.foreground)
                .text_size(px(13.0))
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .cursor_pointer()
                .hover(move |s| s.bg(theme.accent))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(on_toggle)
                .child(div().overflow_hidden().child(current_label.into()))
                .child(icon(IconName::ChevronDown, 14.0, theme.meta)),
        )
        .when(is_open, |d| {
            let mut popup = popover_container("select-popup-menu", width, theme)
                .absolute()
                .top(px(34.0))
                .max_h(px(240.0))
                .overflow_y_scroll();
            popup = if align_right {
                popup.right_0()
            } else {
                popup.left_0()
            };
            d.child(deferred(popup.children(menu_items)).with_priority(5))
        })
        .into_any_element()
}
