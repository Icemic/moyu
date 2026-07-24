use log::debug;
use moyu_pal::config::WindowState;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::core::SurfaceSize;
use crate::events::{
    AnimationFrameCallbackEvent, BeforeUnloadEvent, FocusEvent, FocusEventKind, FullScreenEvent,
    FullscreenEventKind, ResizeEvent,
};
#[cfg(desktop)]
use crate::state::MOUSE_IDENTIFIER;
use crate::utils::dispatch_event::dispatch_event;

use super::Core;

impl Core {
    pub fn handle_about_to_wait(&self, _: &ActiveEventLoop) {
        self.editable.maintain();

        // poll all plugins
        for plugin in self.plugins.iter() {
            plugin.lock().update(false);
        }
    }

    pub fn handle_window_event(
        &self,
        event: &WindowEvent,
        window_id: &WindowId,
        event_loop: &ActiveEventLoop,
    ) {
        let window = self.window();
        if window_id == &window.id() {
            let mut handled = self.handle_pointer_events(window, event);

            if !handled {
                handled = self.handle_keyboard_events(window, event);
            }

            if !handled {
                match event {
                    WindowEvent::RedrawRequested => {
                        if !self.is_paused.load(Ordering::Relaxed) {
                            // poll all plugins
                            for plugin in self.plugins.iter() {
                                plugin.lock().update(true);
                            }

                            #[cfg(native)]
                            if let Some(handles) = self.graphics_thread.load().as_ref() {
                                if handles.0.is_finished() {
                                    // detect graphics thread exit
                                    log::error!("Graphics thread exited unexpectedly.");
                                    event_loop.exit();
                                } else {
                                    // wake up graphics thread
                                    handles.0.thread().unpark();
                                }
                            } else {
                                // keep the loop running until the graphics thread is created
                                window.request_redraw();
                                return;
                            }

                            // We convert u128 to u32 here, which is safe because the timestamp is an
                            // elapsed time (Can't you play a game for 136 years?).
                            dispatch_event(AnimationFrameCallbackEvent {
                                timestamp: self.instant.elapsed().as_millis() as u32,
                            });

                            #[cfg(desktop)]
                            self.handle_pointer_hover(MOUSE_IDENTIFIER, true);

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
                    }
                    WindowEvent::CloseRequested => {
                        dispatch_event(BeforeUnloadEvent {});
                    }
                    WindowEvent::Resized(physical_size) => {
                        let stage_size =
                            SurfaceSize::from_physical_size(physical_size, window.scale_factor());

                        if physical_size.width == 0 || physical_size.height == 0 {
                            // window minimized, stop rendering
                            self.is_paused.store(true, Ordering::Relaxed);
                        } else {
                            self.is_paused.store(false, Ordering::Relaxed);
                            self.resize_stage(stage_size);
                        }

                        let state;

                        if window.fullscreen().is_some() {
                            state = WindowState::Fullscreen;
                        } else if window.is_maximized() {
                            state = WindowState::Maximized;
                        } else if let Some(true) = window.is_minimized() {
                            state = WindowState::Minimized;
                        } else {
                            state = WindowState::Idle;
                        }

                        debug!("window state changes to {:?}", state);

                        let prev_state = *self.window_state.swap(Arc::new(state));

                        // only dispatch event when entering or exiting fullscreen
                        if (state == WindowState::Fullscreen
                            || prev_state == WindowState::Fullscreen)
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
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        self.stage_size.write().set_scale_factor(*scale_factor);
                        self.surface_size.write().set_scale_factor(*scale_factor);
                    }
                    WindowEvent::Focused(focused) => {
                        if !focused {
                            self.editable.blur_active();
                        }
                        dispatch_event(FocusEvent {
                            kind: if *focused {
                                FocusEventKind::Focus
                            } else {
                                FocusEventKind::Blur
                            },
                            target_id: 0,
                        });
                    }
                    WindowEvent::Ime(ime) => {
                        self.editable.handle_ime(ime);
                    }
                    _ => {}
                }
            }
        }

        // FIXME: should be moved
        if self.about_to_quit.load(Ordering::Relaxed) {
            event_loop.exit();
        }
    }
}
