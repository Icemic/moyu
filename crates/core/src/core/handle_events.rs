use log::debug;
use moyu_pal::config::WindowState;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::core::{SurfaceSize, WindowSystemEvent};
use crate::events::{
    AnimationFrameCallbackEvent, BeforeUnloadEvent, FocusEvent, FocusEventKind, FullScreenEvent,
    FullscreenEventKind, ResizeEvent,
};
#[cfg(desktop)]
use crate::state::MOUSE_IDENTIFIER;
use crate::utils::dispatch_event::dispatch_event;

use super::Core;
#[cfg(desktop)]
use super::input::{DispatchRecord, MouseEventSource};

impl Core {
    /// Handle one event from the window system.
    pub fn handle_window_system_event(&self, event: WindowSystemEvent) {
        match event {
            WindowSystemEvent::CursorMoved {
                physical_x,
                physical_y,
            } => self.handle_cursor_moved(physical_x, physical_y),
            WindowSystemEvent::CursorLeft => self.handle_cursor_left(),
            WindowSystemEvent::PointerButton { button, pressed } => {
                self.handle_pointer_button(button, pressed);
            }
            WindowSystemEvent::Wheel {
                delta_x,
                delta_y,
                mode,
            } => self.handle_wheel(delta_x, delta_y, mode),
            WindowSystemEvent::Touch {
                identifier,
                phase,
                physical_x,
                physical_y,
            } => self.handle_touch(identifier, phase, physical_x, physical_y),
            WindowSystemEvent::Key { input } => {
                self.editable.handle_keyboard_input(&input);
                super::input::dispatch_keyboard_event(input);
            }
            WindowSystemEvent::ModifiersChanged { modifiers } => {
                self.modifiers_state.store(Arc::new(modifiers));
            }
            WindowSystemEvent::Ime { event } => {
                self.editable.handle_ime(&event);
            }
            WindowSystemEvent::Focused(focused) => {
                if !focused {
                    self.editable.blur_active();
                }
                dispatch_event(FocusEvent {
                    kind: if focused {
                        FocusEventKind::Focus
                    } else {
                        FocusEventKind::Blur
                    },
                    target_id: 0,
                });
            }
            WindowSystemEvent::Resized {
                physical_width,
                physical_height,
                scale_factor,
            } => self.handle_resized(physical_width, physical_height, scale_factor),
            WindowSystemEvent::ScaleFactorChanged { scale_factor } => {
                self.stage_size.write().set_scale_factor(scale_factor);
                self.surface_size.write().set_scale_factor(scale_factor);
            }
            WindowSystemEvent::CloseRequested => {
                dispatch_event(BeforeUnloadEvent {});
            }
            WindowSystemEvent::Resumed => {
                self.reconfigure_surface_with_window();
            }
            WindowSystemEvent::Suspended => {
                self.release_render_surface();
            }
            WindowSystemEvent::Frame => self.handle_frame(),
            WindowSystemEvent::Idle => self.handle_idle(),
        }
    }

    /// Run the periodic work of a renderable frame.
    fn handle_frame(&self) {
        if self.is_paused.load(Ordering::Relaxed) {
            return;
        }

        // poll all plugins
        for plugin in self.plugins.iter() {
            plugin.lock().update(true);
        }

        #[cfg(native)]
        if !self.caller_driven_graphics.load(Ordering::Relaxed) {
            if let Some(handles) = self.graphics_thread.load().as_ref() {
                if handles.0.is_finished() {
                    // detect graphics thread exit
                    log::error!("Graphics thread exited unexpectedly.");
                    self.window_system.request_quit();
                } else {
                    // wake up graphics thread
                    handles.0.thread().unpark();
                }
            } else {
                // keep the loop running until the graphics thread is created
                self.window_system.request_redraw();
                return;
            }
        }

        // We convert u128 to u32 here, which is safe because the timestamp is an
        // elapsed time (Can't you play a game for 136 years?).
        dispatch_event(AnimationFrameCallbackEvent {
            timestamp: self.instant.elapsed().as_millis() as u32,
        });

        #[cfg(desktop)]
        self.handle_pointer_hover(
            MOUSE_IDENTIFIER,
            true,
            MouseEventSource::Synthetic,
            &mut DispatchRecord::disabled(),
        );

        #[cfg(native)]
        if let Some(vm) = moyu_runtime::try_get_vm() {
            vm.tick();
        }

        #[cfg(web)]
        if let Some(graphics) = self.graphics.load().as_ref() {
            if let Err(err) = graphics.update() {
                log::error!(
                    "Error occurs on graphic updating, terminate graphic updating thread: {:?}",
                    err
                );
            }
            if let Err(err) = graphics.render(false) {
                log::error!(
                    "Error occurs on graphic rendering, terminate graphic rendering thread: {:?}",
                    err
                );
            }
        }
    }

    /// Run the work of an idle event loop turn.
    fn handle_idle(&self) {
        self.editable.maintain();

        // poll all plugins
        for plugin in self.plugins.iter() {
            plugin.lock().update(false);
        }
    }

    /// Apply a surface resize and report the resulting window state.
    fn handle_resized(&self, physical_width: u32, physical_height: u32, scale_factor: f64) {
        let stage_size =
            SurfaceSize::from_physical_size(physical_width, physical_height, scale_factor);

        if physical_width == 0 || physical_height == 0 {
            // window minimized, stop rendering
            self.is_paused.store(true, Ordering::Relaxed);
        } else {
            self.is_paused.store(false, Ordering::Relaxed);
            self.resize_stage(stage_size);
        }

        let state = self.window_system.window_state();

        debug!("window state changes to {:?}", state);

        let prev_state = *self.window_state.swap(Arc::new(state));

        // only dispatch event when entering or exiting fullscreen
        if (state == WindowState::Fullscreen || prev_state == WindowState::Fullscreen)
            && state != prev_state
        {
            dispatch_event(FullScreenEvent {
                kind: FullscreenEventKind::Change,
            });
        }

        let (width, height) = stage_size.logical_size();

        // minimized windows have zero width and height, do not dispatch resize event
        if state != WindowState::Minimized {
            dispatch_event(ResizeEvent { width, height });
        }
    }
}
