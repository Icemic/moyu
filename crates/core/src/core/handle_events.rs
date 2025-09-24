use log::debug;
use moyu_pal::config::WindowState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use crate::core::SurfaceSize;
use crate::events::{
    AnimationFrameCallbackEvent, FocusEvent, FocusEventKind, FullScreenEvent, FullscreenEventKind,
    ResizeEvent,
};
use crate::utils::dispatch_event::dispatch_event;

use super::Core;

impl Core {
    pub fn handle_events(
        &self,
        event: &Event<()>,
        window: &Window,
        event_loop: &EventLoopWindowTarget<()>,
    ) {
        match event {
            &Event::AboutToWait => {
                // poll all plugins
                for plugin in self.plugins.lock().values_mut() {
                    plugin.lock().update(false);
                }
            }
            &Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let mut handled = self.handle_pointer_events(window, event);

                if !handled {
                    handled = self.handle_keyboard_events(window, event);
                }

                if !handled {
                    match event {
                        WindowEvent::RedrawRequested => {
                            // poll all plugins
                            for plugin in self.plugins.lock().values_mut() {
                                plugin.lock().update(true);
                            }

                            #[cfg(native)]
                            if let Some(handle) = self.graphics_thread.load().as_ref() {
                                if handle.is_finished() {
                                    // detect graphics thread exit
                                    log::error!("Graphics thread exited unexpectedly.");
                                    event_loop.exit();
                                } else {
                                    // wake up graphics thread
                                    handle.thread().unpark();
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

                            #[cfg(native)]
                            if let Some(vm) = moyu_runtime::try_get_vm() {
                                vm.tick();
                            }

                            #[cfg(web)]
                            if let Some(graphics) = self.graphics.load().as_ref() {
                                if let Err(err) = graphics.render() {
                                    log::error!(
                                        "Error occurs on rendering, terminate graphics thread: {:?}",
                                        err
                                    );
                                }
                            }
                        }
                        WindowEvent::CloseRequested => event_loop.exit(),
                        WindowEvent::Resized(physical_size) => {
                            let stage_size = SurfaceSize::from_physical_size(
                                physical_size,
                                window.scale_factor(),
                            );

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
                        }
                        WindowEvent::Focused(focused) => {
                            dispatch_event(FocusEvent {
                                kind: if *focused {
                                    FocusEventKind::Focus
                                } else {
                                    FocusEventKind::Blur
                                },
                                target_id: 0,
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        if self.about_to_quit.load(Ordering::Relaxed) {
            event_loop.exit();
        }
    }
}
