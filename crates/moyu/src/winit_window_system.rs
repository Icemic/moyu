//! The default window system: winit's window, input method and surface capabilities
//! behind the engine's [`WindowSystem`] boundary.
//!
//! Everything winit-specific the engine needs lives here: the event loop and window
//! creation, the conversion of winit events into [`WindowSystemEvent`], and the
//! platform extras (Windows icon, Web canvas, Android activity). `moyu_core` and
//! `moyu_nodes` never see a winit type.

use std::sync::Arc;

use anyhow::anyhow;
use moyu_core::base::MoyuCursor;
use moyu_core::core::{
    ImeEvent, KeyInput, KeyboardModifiers, PointerButton, SurfaceMetrics, TouchPhase, WindowSystem,
    WindowSystemError, WindowSystemEvent, WindowSystemResult,
};
use moyu_core::events::{KeyboardEventKind, KeyboardLocation, WheelEventDeltaMode};
use moyu_pal::config::{WindowState, get_engine_config};
use moyu_pal::sync::Mutex;
#[cfg(desktop)]
use winit::dpi::LogicalSize;
use winit::dpi::{PhysicalPosition, PhysicalSize, Size};
use winit::event::{
    ElementState, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase,
    WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window};

use crate::entry::ApplicationInitEvent;

/// Create the winit event loop the default runtime runs on.
#[allow(dead_code)]
pub fn create_eventloop(
    #[cfg(android)] app: winit::platform::android::activity::AndroidApp,
) -> EventLoop<ApplicationInitEvent> {
    // create main thread infinity loop
    #[cfg(not(android))]
    {
        EventLoop::with_user_event().build().unwrap()
    }

    #[cfg(android)]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        EventLoop::with_user_event()
            .with_android_app(app)
            .build()
            .unwrap()
    }
}

/// Create the default runtime's window and apply the configured window options.
pub fn create_window(event_loop: &ActiveEventLoop, #[cfg(web)] element_id: &str) -> Arc<Window> {
    let env = get_engine_config();
    // create window
    let mut builder = Window::default_attributes()
        .with_inner_size(Size::Logical(env.initial_surface_size.as_tuple().into()))
        .with_resizable(env.window_resizable)
        .with_visible(false)
        .with_active(true)
        .with_title(&env.window_title);

    // On the web the page owns the canvas size, so there is no minimum size to enforce there.
    #[cfg(native)]
    {
        builder = builder.with_min_inner_size(PhysicalSize::new(400, 300));
    }

    match env.window_state {
        moyu_pal::config::WindowState::Maximized => {
            builder = builder.with_maximized(true);
        }
        moyu_pal::config::WindowState::Minimized => {
            log::warn!("You should not start with a minimized window.");
        }
        moyu_pal::config::WindowState::Fullscreen => {
            builder =
                builder.with_fullscreen(Some(Fullscreen::Borderless(event_loop.primary_monitor())));
        }
        _ => {
            // idle
        }
    };

    let window = event_loop
        .create_window(builder)
        .expect("Failed to create window.");

    // web target only
    // add a canvas element to dom as 'window'
    #[cfg(web)]
    {
        use winit::platform::web::WindowExtWebSys;

        let host = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|document| document.get_element_by_id(element_id))
            .unwrap_or_else(|| {
                panic!("couldn't find the element to mount the canvas: {element_id}")
            });

        let canvas = window
            .canvas()
            .expect("Failed to get canvas from winit window.");

        // The mount element plays the role of the window on the web: it owns the size and the
        // canvas fills it. winit observes the resulting canvas box and reports it as a resize,
        // so the surface follows without the engine measuring anything itself.
        // `display: block` keeps the canvas from adding a baseline gap to its parent.
        let style = canvas.style();
        let _ = style.set_property("display", "block");
        let _ = style.set_property("width", "100%");
        let _ = style.set_property("height", "100%");

        host.append_child(&web_sys::Element::from(canvas))
            .expect("couldn't append canvas to the mount element");
    }

    Arc::new(window)
}

/// The winit-backed window system used by the default `moyu` runtime.
pub struct WinitWindowSystem {
    window: Arc<Window>,
    event_proxy: EventLoopProxy<ApplicationInitEvent>,
    /// Modifier keys currently down. Kept here so a keyboard event carries a snapshot
    /// without asking the core.
    modifiers: Mutex<KeyboardModifiers>,
}

impl WinitWindowSystem {
    pub fn new(window: Arc<Window>, event_proxy: EventLoopProxy<ApplicationInitEvent>) -> Self {
        Self {
            window,
            event_proxy,
            modifiers: Mutex::new(KeyboardModifiers::default()),
        }
    }

    /// The native window, for the runtime's own needs.
    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }

    /// Show or hide the window. Only the default runtime needs this.
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    /// Convert a winit window event into the engine's window system event.
    ///
    /// Returns `None` for events the engine does not consume, including synthetic
    /// keyboard input and the events the runtime handles itself.
    pub fn convert_window_event(&self, event: &WindowEvent) -> Option<WindowSystemEvent> {
        match event {
            WindowEvent::CursorMoved { position, .. } => Some(WindowSystemEvent::CursorMoved {
                physical_x: position.x,
                physical_y: position.y,
            }),
            WindowEvent::CursorLeft { .. } => Some(WindowSystemEvent::CursorLeft),
            WindowEvent::MouseInput { button, state, .. } => {
                from_winit_button(*button).map(|button| WindowSystemEvent::PointerButton {
                    button,
                    pressed: *state == ElementState::Pressed,
                })
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y, mode) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        (*x as f64, *y as f64, WheelEventDeltaMode::Line)
                    }
                    MouseScrollDelta::PixelDelta(position) => {
                        (position.x, position.y, WheelEventDeltaMode::Pixel)
                    }
                };

                Some(WindowSystemEvent::Wheel {
                    delta_x,
                    delta_y,
                    mode,
                })
            }
            WindowEvent::Touch(touch) => Some(WindowSystemEvent::Touch {
                identifier: touch.id as u32,
                phase: match touch.phase {
                    WinitTouchPhase::Started => TouchPhase::Start,
                    WinitTouchPhase::Moved => TouchPhase::Move,
                    WinitTouchPhase::Ended => TouchPhase::End,
                    WinitTouchPhase::Cancelled => TouchPhase::Cancel,
                },
                physical_x: touch.location.x,
                physical_y: touch.location.y,
            }),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if *is_synthetic {
                    return None;
                }

                // The Android back gesture reports as a keyboard key; the engine treats
                // it as a close request.
                #[cfg(android)]
                if event.state.is_pressed()
                    && matches!(&event.logical_key, Key::Named(NamedKey::BrowserBack))
                {
                    return Some(WindowSystemEvent::CloseRequested);
                }

                Some(WindowSystemEvent::Key {
                    input: key_input_from_winit(event, *self.modifiers.lock()),
                })
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let snapshot = keyboard_modifiers_from_winit(&modifiers.state());
                *self.modifiers.lock() = snapshot;
                Some(WindowSystemEvent::ModifiersChanged {
                    modifiers: snapshot,
                })
            }
            WindowEvent::Ime(ime) => Some(WindowSystemEvent::Ime {
                event: ime_event_from_winit(ime),
            }),
            WindowEvent::Focused(focused) => Some(WindowSystemEvent::Focused(*focused)),
            WindowEvent::Resized(physical_size) => Some(WindowSystemEvent::Resized {
                physical_width: physical_size.width,
                physical_height: physical_size.height,
                scale_factor: self.window.scale_factor(),
            }),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Some(WindowSystemEvent::ScaleFactorChanged {
                    scale_factor: *scale_factor,
                })
            }
            WindowEvent::CloseRequested => Some(WindowSystemEvent::CloseRequested),
            WindowEvent::RedrawRequested => Some(WindowSystemEvent::Frame),
            _ => None,
        }
    }
}

impl WindowSystem for WinitWindowSystem {
    fn surface_metrics(&self) -> SurfaceMetrics {
        let size = self.window.inner_size();

        SurfaceMetrics {
            physical_width: size.width,
            physical_height: size.height,
            scale_factor: self.window.scale_factor(),
        }
    }

    fn window_position(&self) -> (i32, i32) {
        let position = self.window.inner_position().unwrap_or_default();
        (position.x, position.y)
    }

    fn window_state(&self) -> WindowState {
        let window = &self.window;

        if window.fullscreen().is_some() {
            WindowState::Fullscreen
        } else if window.is_maximized() {
            WindowState::Maximized
        } else if let Some(true) = window.is_minimized() {
            WindowState::Minimized
        } else {
            WindowState::Idle
        }
    }

    fn has_focus(&self) -> bool {
        self.window.has_focus()
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn request_quit(&self) {
        if let Err(error) = self.event_proxy.send_event(ApplicationInitEvent::Quit) {
            log::warn!("failed to request quit: {error}");
        }
    }

    fn set_title(&self, title: &str) -> WindowSystemResult<()> {
        self.window.set_title(title);
        Ok(())
    }

    fn set_window_state(&self, state: WindowState) -> WindowSystemResult<()> {
        let window = &self.window;
        let is_fullscreen = window.fullscreen().is_some();

        match state {
            WindowState::Idle => {
                window.set_maximized(false);
                window.set_minimized(false);
                if is_fullscreen {
                    window.set_fullscreen(None);
                }
            }
            WindowState::Maximized => {
                if is_fullscreen {
                    window.set_fullscreen(None);
                }
                window.set_maximized(true);
            }
            WindowState::Minimized => {
                if is_fullscreen {
                    window.set_fullscreen(None);
                }
                window.set_minimized(true);
            }
            WindowState::Fullscreen => {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }

        Ok(())
    }

    fn request_window_size(
        &self,
        logical_width: f64,
        logical_height: f64,
    ) -> WindowSystemResult<()> {
        // Requesting a size clears the window state, so it is saved and restored around
        // the request; the saved fullscreen remembers the monitor it was on.
        #[cfg(desktop)]
        {
            let window = &self.window;
            let window_fullscreen = window.fullscreen();
            let window_minimized = window.is_minimized();
            let window_maximized = window.is_maximized();

            if logical_width > 0. && logical_height > 0. {
                let size = Size::Logical(LogicalSize::new(logical_width, logical_height));
                let _ = window.request_inner_size(size);

                window.set_minimized(window_minimized.unwrap_or(false));
                window.set_maximized(window_maximized);

                if window_fullscreen.is_some() {
                    window.set_fullscreen(None);
                    window.set_fullscreen(window_fullscreen);
                }
            }

            Ok(())
        }

        #[cfg(not(desktop))]
        {
            let _ = (logical_width, logical_height);
            Err(WindowSystemError::Unsupported)
        }
    }

    fn move_to_center(&self) -> WindowSystemResult<()> {
        #[cfg(desktop)]
        {
            let window = &self.window;

            if let Some(monitor) = window.current_monitor() {
                let monitor_size = monitor.size();
                let window_size = window.outer_size();

                window.set_outer_position(PhysicalPosition {
                    x: monitor_size.width.saturating_sub(window_size.width) as f64 / 2.
                        + monitor.position().x as f64,
                    y: monitor_size.height.saturating_sub(window_size.height) as f64 / 2.
                        + monitor.position().y as f64,
                });
            }

            Ok(())
        }

        #[cfg(not(desktop))]
        {
            Err(WindowSystemError::Unsupported)
        }
    }

    fn focus_window(&self) {
        self.window.focus_window();
    }

    fn set_cursor(&self, cursor: MoyuCursor) {
        match cursor {
            MoyuCursor::Visible(cursor) => {
                self.window.set_cursor(cursor);
                self.window.set_cursor_visible(true);
            }
            MoyuCursor::Hidden => {
                self.window.set_cursor_visible(false);
            }
        }
    }

    fn set_ime_allowed(&self, allowed: bool) -> WindowSystemResult<()> {
        self.window.set_ime_allowed(allowed);
        Ok(())
    }

    fn set_ime_cursor_area(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> WindowSystemResult<()> {
        self.window.set_ime_cursor_area(
            PhysicalPosition::new(x, y),
            PhysicalSize::new(width, height),
        );
        Ok(())
    }

    fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    fn create_surface(
        &self,
        instance: &wgpu::Instance,
    ) -> WindowSystemResult<wgpu::Surface<'static>> {
        instance
            .create_surface(self.window.clone())
            .map_err(|error| {
                WindowSystemError::Platform(anyhow!("failed to create surface: {error}"))
            })
    }
}

/// Convert a winit IME event into the engine's IME event.
///
/// The preedit cursor position winit reports is not used: the candidate window is
/// placed from the editable's caret rectangle instead.
fn ime_event_from_winit(ime: &winit::event::Ime) -> ImeEvent {
    match ime {
        winit::event::Ime::Enabled => ImeEvent::Enabled,
        winit::event::Ime::Disabled => ImeEvent::Disabled,
        winit::event::Ime::Preedit(text, _) => ImeEvent::Preedit(text.clone()),
        winit::event::Ime::Commit(text) => ImeEvent::Commit(text.clone()),
    }
}

/// Convert winit modifier state into the engine's modifier snapshot.
fn keyboard_modifiers_from_winit(state: &ModifiersState) -> KeyboardModifiers {
    KeyboardModifiers {
        ctrl: state.control_key(),
        shift: state.shift_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}

/// Convert a winit key event into the engine's keyboard input.
///
/// `key` keeps the JavaScript name (a named key's debug name, or the character
/// itself) and `code` the physical key name. `text` is only carried for the keys
/// that type input, so named keys such as Enter and Backspace contribute no text.
fn key_input_from_winit(event: &KeyEvent, modifiers: KeyboardModifiers) -> KeyInput {
    let kind = if event.state.is_pressed() {
        KeyboardEventKind::KeyDown
    } else {
        KeyboardEventKind::KeyUp
    };

    let location = match event.location {
        winit::keyboard::KeyLocation::Standard => KeyboardLocation::Standard,
        winit::keyboard::KeyLocation::Left => KeyboardLocation::Left,
        winit::keyboard::KeyLocation::Right => KeyboardLocation::Right,
        winit::keyboard::KeyLocation::Numpad => KeyboardLocation::Numpad,
    };

    let key = match event.logical_key.clone() {
        Key::Named(named_key) => {
            if named_key == NamedKey::Space {
                " ".to_string()
            } else {
                format!("{:?}", named_key)
            }
        }
        Key::Character(c) => c.to_string(),
        Key::Unidentified(native_key) => {
            log::warn!("Unidentified key {:?} is not supported", native_key);
            "Unidentified".to_string()
        }
        Key::Dead(_) => {
            log::warn!("Dead key is not supported");
            "".to_string()
        }
    };

    let code = match event.physical_key {
        winit::keyboard::PhysicalKey::Code(key_code) => format!("{:?}", key_code),
        winit::keyboard::PhysicalKey::Unidentified(native_key_code) => {
            log::warn!(
                "Unidentified key code {:?} is not supported",
                native_key_code
            );
            "Unidentified".to_string()
        }
    };

    let text = match &event.logical_key {
        Key::Character(_) => event.text.as_ref().map(|text| text.to_string()),
        Key::Named(NamedKey::Space) => Some(" ".to_string()),
        _ => None,
    };

    KeyInput {
        kind,
        key,
        code,
        location,
        repeat: event.repeat,
        modifiers,
        text,
    }
}

/// Map a winit mouse button to the engine's vocabulary. DOM only defines the five
/// standard buttons, so any other physical button yields `None` and is ignored.
fn from_winit_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        MouseButton::Back => Some(PointerButton::Back),
        MouseButton::Forward => Some(PointerButton::Forward),
        MouseButton::Other(_) => None,
    }
}
