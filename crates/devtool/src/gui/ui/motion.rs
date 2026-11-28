//! Reusable GPUI motion tokens and mount-driven animation helpers.

use std::time::Duration;

use gpui::{Animation, AnimationExt, ElementId, Styled, prelude::*, px};

pub const POPOVER_DURATION_MS: u64 = 110;
pub const MODAL_BACKDROP_DURATION_MS: u64 = 120;
pub const MODAL_PANEL_DURATION_MS: u64 = 150;
pub const DRAWER_DURATION_MS: u64 = 140;
pub const PULSE_DURATION_MS: u64 = 1200;

/// Smooth cubic ease-out curve (`0.0..=1.0`).
pub fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

/// Subtle fade + 4px downward slide entrance for floating popovers and dropdown menus.
pub fn animate_popover_in<E>(id: impl Into<ElementId>, el: E) -> impl IntoElement
where
    E: Styled + IntoElement + 'static,
{
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(POPOVER_DURATION_MS)).with_easing(ease_out_cubic),
        |el, delta| el.opacity(delta).mt(px((1.0 - delta) * 4.0)),
    )
}

/// Subtle fade-in for modal backdrop scrims.
pub fn animate_modal_backdrop<E>(id: impl Into<ElementId>, el: E) -> impl IntoElement
where
    E: Styled + IntoElement + 'static,
{
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(MODAL_BACKDROP_DURATION_MS))
            .with_easing(ease_out_cubic),
        |el, delta| el.opacity(delta),
    )
}

/// Subtle fade + 6px upward float entrance for modal dialog panels.
pub fn animate_modal_panel<E>(id: impl Into<ElementId>, el: E) -> impl IntoElement
where
    E: Styled + IntoElement + 'static,
{
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(MODAL_PANEL_DURATION_MS)).with_easing(ease_out_cubic),
        |el, delta| el.opacity(delta).mt(px((1.0 - delta) * 6.0)),
    )
}

/// Smooth height expand + fade-in for collapsible drawers.
pub fn animate_drawer_in<E>(id: impl Into<ElementId>, target_height: f32, el: E) -> impl IntoElement
where
    E: Styled + IntoElement + 'static,
{
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(DRAWER_DURATION_MS)).with_easing(ease_out_cubic),
        move |el, delta| el.h(px(target_height * delta)).opacity(delta),
    )
}

/// Repeating gentle opacity pulse for active/waiting indicators.
pub fn pulse_animation() -> Animation {
    Animation::new(Duration::from_millis(PULSE_DURATION_MS))
        .repeat()
        .with_easing(gpui::pulsating_between(0.35, 1.0))
}
