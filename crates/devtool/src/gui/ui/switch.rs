//! Animated pill toggle switch primitive styled with FishFlow theme tokens.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ClickEvent, ElementId, Hsla, IntoElement, RenderOnce, Window,
    div, prelude::*, px,
};

use crate::gui::theme::Theme;
use crate::gui::ui::motion::ease_out_cubic;

const SWITCH_DURATION_MS: u64 = 130;
const THUMB_TRAVEL_PX: f32 = 16.0;

struct SwitchAnimState {
    last_checked: bool,
    anim_gen: u64,
}

fn lerp_hsla(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0.0, 1.0);
    Hsla {
        h: a.h + (b.h - a.h) * t,
        s: a.s + (b.s - a.s) * t,
        l: a.l + (b.l - a.l) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    theme: Theme,
    on_click: Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

/// Standard 36×20 pill toggle switch with smooth thumb slide and track color animation on toggle.
pub fn switch(
    id: impl Into<ElementId>,
    checked: bool,
    theme: Theme,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Switch {
    Switch {
        id: id.into(),
        checked,
        theme,
        on_click: Box::new(on_click),
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let theme = self.theme;
        let state_id: ElementId = (self.id.clone(), "state").into();

        // Track prior state per switch ID so initial mount rests at final position
        // without false sliding, while user toggles trigger a smooth 130ms transition.
        let state = window.use_keyed_state(state_id, cx, |_, _| SwitchAnimState {
            last_checked: checked,
            anim_gen: 0,
        });

        let anim_gen = state.update(cx, |st, _| {
            if st.last_checked != checked {
                st.last_checked = checked;
                st.anim_gen = st.anim_gen.wrapping_add(1);
            }
            st.anim_gen
        });

        let off_track = theme.input;
        let on_track = theme.primary;
        let off_thumb = theme.background;
        let on_thumb = theme.primary_foreground;
        let on_click = self.on_click;

        let track = div()
            .id(self.id.clone())
            .w(px(36.0))
            .h(px(20.0))
            .p(px(2.0))
            .rounded_full()
            .flex()
            .flex_row()
            .items_center()
            .flex_shrink_0()
            .cursor_pointer()
            .hover(|s| s.opacity(0.9))
            .active(|s| s.opacity(0.78))
            .on_click(move |ev, win, cx| (on_click)(ev, win, cx));

        if anim_gen == 0 {
            let track_bg = if checked { on_track } else { off_track };
            let thumb_bg = if checked { on_thumb } else { off_thumb };
            let ml = if checked { THUMB_TRAVEL_PX } else { 0.0 };
            return track
                .bg(track_bg)
                .child(
                    div()
                        .ml(px(ml))
                        .size(px(16.0))
                        .rounded_full()
                        .bg(thumb_bg)
                        .shadow_sm(),
                )
                .into_any_element();
        }

        let (from_track, to_track, from_thumb, to_thumb, from_x, to_x) = if checked {
            (
                off_track,
                on_track,
                off_thumb,
                on_thumb,
                0.0,
                THUMB_TRAVEL_PX,
            )
        } else {
            (
                on_track,
                off_track,
                on_thumb,
                off_thumb,
                THUMB_TRAVEL_PX,
                0.0,
            )
        };

        let anim_id: ElementId = (self.id, format!("anim-{anim_gen}")).into();
        track
            .with_animation(
                anim_id,
                Animation::new(Duration::from_millis(SWITCH_DURATION_MS))
                    .with_easing(ease_out_cubic),
                move |el, delta| {
                    let x = from_x + (to_x - from_x) * delta;
                    el.bg(lerp_hsla(from_track, to_track, delta)).child(
                        div()
                            .ml(px(x))
                            .size(px(16.0))
                            .rounded_full()
                            .bg(lerp_hsla(from_thumb, to_thumb, delta))
                            .shadow_sm(),
                    )
                },
            )
            .into_any_element()
    }
}
