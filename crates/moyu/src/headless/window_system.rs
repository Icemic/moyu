//! The window system of a runtime with no window.
//!
//! The engine only needs a window for presenting and for platform input. Everything
//! else — the node tree, plugins, the script runtime, the debug bridge — works from a
//! [`WindowSystem`] alone. This implementation reports a fixed surface and refuses
//! every platform capability it cannot provide, which makes `create_surface` return
//! `Unsupported` and leaves the engine rendering into its own texture.
//!
//! See `rfcs/2026-10-22-headless-runtime.md`.

use std::sync::atomic::{AtomicBool, Ordering};

use moyu_core::base::MoyuCursor;
use moyu_core::core::{
    SurfaceMetrics, WindowSystem, WindowSystemError, WindowSystemResult,
};
use moyu_pal::config::WindowState;

/// A window system that has no window, no input method and no surface.
pub struct HeadlessWindowSystem {
    /// Size the surface reports, in physical pixels.
    surface_size: (u32, u32),
    quit_requested: AtomicBool,
}

impl HeadlessWindowSystem {
    pub fn new(surface_size: (u32, u32)) -> Self {
        Self {
            surface_size,
            quit_requested: AtomicBool::new(false),
        }
    }

    /// Whether the session asked to end.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested.load(Ordering::Relaxed)
    }
}

impl WindowSystem for HeadlessWindowSystem {
    fn surface_metrics(&self) -> SurfaceMetrics {
        SurfaceMetrics {
            physical_width: self.surface_size.0,
            physical_height: self.surface_size.1,
            // Nothing scales a window that does not exist, so the configured size is
            // already the physical one.
            scale_factor: 1.0,
        }
    }

    fn window_position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn window_state(&self) -> WindowState {
        WindowState::Idle
    }

    /// Always false: the session can never gain focus, which is also what keeps
    /// `EditableManager` from asking for an input method.
    fn has_focus(&self) -> bool {
        false
    }

    /// Nothing to redraw on request; the frame loop drives frames on its own.
    fn request_redraw(&self) {}

    fn request_quit(&self) {
        self.quit_requested.store(true, Ordering::Relaxed);
    }

    /// Projects call this unconditionally, so it is accepted and ignored.
    fn set_title(&self, _title: &str) -> WindowSystemResult<()> {
        Ok(())
    }

    fn set_window_state(&self, _state: WindowState) -> WindowSystemResult<()> {
        Err(WindowSystemError::Unsupported)
    }

    fn request_window_size(
        &self,
        _logical_width: f64,
        _logical_height: f64,
    ) -> WindowSystemResult<()> {
        Err(WindowSystemError::Unsupported)
    }

    fn move_to_center(&self) -> WindowSystemResult<()> {
        Err(WindowSystemError::Unsupported)
    }

    fn focus_window(&self) {}

    fn set_cursor(&self, _cursor: MoyuCursor) {}

    fn set_ime_allowed(&self, _allowed: bool) -> WindowSystemResult<()> {
        Err(WindowSystemError::Unsupported)
    }

    fn set_ime_cursor_area(
        &self,
        _x: i32,
        _y: i32,
        _width: u32,
        _height: u32,
    ) -> WindowSystemResult<()> {
        Err(WindowSystemError::Unsupported)
    }

    fn pre_present_notify(&self) {}

    /// `Unsupported` is what tells the engine there is no surface to present to, so it
    /// renders into a texture it owns instead.
    fn create_surface(
        &self,
        _instance: &wgpu::Instance,
    ) -> WindowSystemResult<wgpu::Surface<'static>> {
        Err(WindowSystemError::Unsupported)
    }
}
