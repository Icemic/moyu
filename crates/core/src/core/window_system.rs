//! The window system boundary: platform events submitted to the engine and the
//! platform capabilities the engine uses back.
//!
//! A window system owns the platform event loop and the native window or canvas. It
//! converts raw platform events into [`WindowSystemEvent`] and submits them through
//! `Core::handle_window_system_event`, and implements [`WindowSystem`] so the core and
//! the graphics layer can control the window, read surface metrics, drive IME and
//! create the render surface.
//!
//! Core and nodes only use the types in this module, so neither depends on a specific
//! windowing library. The default `moyu` runtime implements this boundary with winit.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Error;
use cursor_icon::CursorIcon;
use moyu_pal::config::WindowState;
use wgpu::{Instance, Surface};

use crate::base::{CursorImage, MoyuCursor};
use crate::core::input::{KeyInput, KeyboardModifiers, PointerButton, TouchPhase};
use crate::events::WheelEventDeltaMode;

/// Physical size and scale factor of the current surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceMetrics {
    pub physical_width: u32,
    pub physical_height: u32,
    pub scale_factor: f64,
}

/// Why a [`WindowSystem`] call could not be completed.
#[derive(Debug)]
pub enum WindowSystemError {
    /// The window system cannot execute this operation at all.
    Unsupported,
    /// The platform call was attempted and failed.
    Platform(Error),
}

impl std::fmt::Display for WindowSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "the window system does not support this operation"),
            Self::Platform(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for WindowSystemError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unsupported => None,
            Self::Platform(error) => Some(error.as_ref()),
        }
    }
}

/// Result of a [`WindowSystem`] call.
pub type WindowSystemResult<T> = Result<T, WindowSystemError>;

/// The window, input method and render surface capabilities the engine uses.
///
/// Implementations must be safe to call from the engine's main thread and from the
/// graphics update and render threads, and must handle their own platform threading
/// rules. A method must not call back into the core.
pub trait WindowSystem: Send + Sync + 'static {
    /// Physical size and scale factor of the current surface.
    fn surface_metrics(&self) -> SurfaceMetrics;
    /// Position of the window's top-left corner on the desktop, in physical pixels.
    fn window_position(&self) -> (i32, i32);
    /// Current window state.
    fn window_state(&self) -> WindowState;
    /// Whether the window currently has input focus.
    fn has_focus(&self) -> bool;

    /// Ask for the next frame to be rendered.
    fn request_redraw(&self);
    /// Ask the window system to end the session.
    fn request_quit(&self);
    /// Set the window title.
    fn set_title(&self, title: &str) -> WindowSystemResult<()>;
    /// Change the window state.
    fn set_window_state(&self, state: WindowState) -> WindowSystemResult<()>;
    /// Request a new logical inner size for the window.
    fn request_window_size(
        &self,
        logical_width: f64,
        logical_height: f64,
    ) -> WindowSystemResult<()>;
    /// Move the window to the center of its monitor.
    fn move_to_center(&self) -> WindowSystemResult<()>;
    /// Give the window input focus.
    fn focus_window(&self);
    /// Show the cursor for the current state.
    ///
    /// The window system holds the project's images (from
    /// [`Self::set_custom_cursors`]) and picks the one for this state itself; the
    /// engine does not resolve the state for it.
    fn set_cursor(&self, cursor: MoyuCursor);
    /// Hand the project's cursor images to the window system, replacing the current
    /// set.
    ///
    /// Called after loading at startup, and again whenever a script hands over a new
    /// set. An empty map removes every custom cursor. The window system installs the
    /// images at its next event callback and keeps them from then on; the engine
    /// re-applies the current cursor once that happened.
    fn set_custom_cursors(&self, cursors: HashMap<CursorIcon, Arc<CursorImage>>);

    /// Allow or forbid the platform input method.
    fn set_ime_allowed(&self, allowed: bool) -> WindowSystemResult<()>;
    /// Set the physical rectangle the IME candidate window points at.
    fn set_ime_cursor_area(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> WindowSystemResult<()>;

    /// Notify the platform that a frame is about to be presented.
    fn pre_present_notify(&self);
    /// Create the render surface from the given instance.
    fn create_surface(&self, instance: &Instance) -> WindowSystemResult<Surface<'static>>;
}

/// An event a window system submits to the core.
#[derive(Debug, Clone)]
pub enum WindowSystemEvent {
    /// The pointer moved; coordinates are physical and relative to the surface.
    CursorMoved { physical_x: f64, physical_y: f64 },
    /// The pointer left the surface.
    CursorLeft,
    /// A pointer button was pressed or released.
    PointerButton {
        button: PointerButton,
        pressed: bool,
    },
    /// The wheel scrolled.
    Wheel {
        delta_x: f64,
        delta_y: f64,
        mode: WheelEventDeltaMode,
    },
    /// A touch point changed phase; coordinates are physical and relative to the surface.
    Touch {
        identifier: u32,
        phase: TouchPhase,
        physical_x: f64,
        physical_y: f64,
    },

    /// A keyboard event.
    Key { input: KeyInput },
    /// The modifier keys changed.
    ModifiersChanged { modifiers: KeyboardModifiers },
    /// An input method event.
    Ime { event: ImeEvent },

    /// The window gained or lost input focus.
    Focused(bool),
    /// The surface was resized, in physical pixels.
    Resized {
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
    },
    /// The device scale factor changed.
    ScaleFactorChanged { scale_factor: f64 },

    /// The session resumed after a suspend.
    Resumed,
    /// The session is about to be suspended.
    Suspended,
    /// The window close was requested.
    CloseRequested,
    /// A frame can be rendered.
    Frame,
    /// The event loop is idle.
    Idle,
}

/// An input method event.
#[derive(Debug, Clone)]
pub enum ImeEvent {
    /// The input method became available.
    Enabled,
    /// The input method is no longer available.
    Disabled,
    /// Text being composed, not yet committed.
    Preedit(String),
    /// Composed text committed to the editable.
    Commit(String),
}
