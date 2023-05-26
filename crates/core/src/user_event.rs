use std::ffi::c_void;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum UserEvent {
    // logical_width, logical_height, factor
    ResizeWindow(f64, f64, Option<f64>),
    WindowState(WindowState),
    SetTitle(String),
    Quit,
    Custom(*mut c_void),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Idle,
    Maximized,
    Minimized,
    Fullscreen,
}
