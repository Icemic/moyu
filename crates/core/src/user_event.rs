use std::ffi::c_void;

use hai_pal::config::WindowState;
use winit::window::CursorIcon;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum UserEvent {
    // logical_width, logical_height, factor
    ResizeWindow(f64, f64, Option<f64>),
    WindowState(WindowState),
    SetTitle(String),
    SetCursorIcon(CursorIcon),
    SetCursorVisible(bool),
    Quit,
    Custom(*mut c_void),
}

unsafe impl Send for UserEvent {}
unsafe impl Sync for UserEvent {}
