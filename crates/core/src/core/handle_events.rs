use hai_pal::config::WindowState;
use log::{debug, error, info, warn};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::window::Window;

use crate::core::SurfaceSize;
use crate::events::AnimationFrameCallbackEvent;
use crate::user_event::UserEvent;
use crate::utils::dispatch_event::dispatch_event;

use super::{Core, HaiRedrawMode};

impl Core {
    #[inline(always)]
    pub fn handle_events(
        &self,
        event: &Event<UserEvent>,
        window: &Window,
        event_loop: &EventLoopWindowTarget<UserEvent>,
    ) {
        match event {
            &Event::AboutToWait => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                let redraw_mode = self.redraw_mode.load();
                match **redraw_mode {
                    HaiRedrawMode::Auto => {
                        if !self.is_paused.load(Ordering::Relaxed) {
                            window.request_redraw();
                        }
                    }
                    HaiRedrawMode::Dirty => {
                        // skip rendering if not dirty
                        if self.is_dirty.load(Ordering::Relaxed) {
                            self.set_dirty(false);
                            window.request_redraw();
                        }
                    }
                }
            }
            &Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                let mut handled = self.handle_pointer_events(window, event);

                if !handled {
                    match event {
                        WindowEvent::RedrawRequested => {
                            match self.render(window) {
                                Ok(_) => {
                                    // For real browsers, the callback should be executed before rendering,
                                    // but in our asynchronous process, this would cause performance issues.
                                    // Therefore, we only send a message after rendering to let the JavaScript
                                    // environment align with v-sync.
                                    // We convert u128 to u32 here, which is safe because the timestamp is an
                                    // elapsed time (Can't you play a game for 136 years?).
                                    dispatch_event(AnimationFrameCallbackEvent {
                                        timestamp: self.instant.elapsed().as_millis() as u32,
                                    });
                                }
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => {
                                    warn!("surface lost, reconfigure.");
                                    self.refresh();
                                }
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    error!("surface out of memory, quit.");
                                    event_loop.exit();
                                }
                                Err(wgpu::SurfaceError::Outdated) => {
                                    // ignore
                                    warn!("surface outdated, ignored.");
                                }
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => {
                                    error!("surface error: {:?}", e);
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

                            if window.fullscreen().is_some() {
                                self.window_state.store(Arc::new(WindowState::Fullscreen));
                            } else if window.is_maximized() {
                                self.window_state.store(Arc::new(WindowState::Maximized));
                            } else if let Some(true) = window.is_minimized() {
                                self.window_state.store(Arc::new(WindowState::Minimized));
                            } else {
                                self.window_state.store(Arc::new(WindowState::Idle));
                            }

                            debug!("window state changes to {:?}", self.window_state.load());
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            self.stage_size.write().set_scale_factor(*scale_factor);
                        }
                        _ => {}
                    }
                }
            }
            Event::UserEvent(user_event) => match user_event {
                &UserEvent::ResizeWindow(logical_width, logical_height, factor) => {
                    self.resize_window(logical_width, logical_height, factor);
                    self.move_to_center();
                }
                &UserEvent::WindowState(state) => {
                    self.set_window_state(state);
                }
                UserEvent::SetTitle(ref title) => {
                    window.set_title(title);
                }
                &UserEvent::SetCursorIcon(icon) => {
                    window.set_cursor_icon(icon);
                }
                &UserEvent::SetCursorVisible(visible) => {
                    window.set_cursor_visible(visible);
                }
                UserEvent::Quit => {
                    info!("Goodbye.");
                    event_loop.exit();
                }
                UserEvent::Custom(_) => {
                    // do nothing
                }
            },
            _ => {}
        }
    }
}
