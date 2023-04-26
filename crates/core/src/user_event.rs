#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum UserEvent {
    // logical_width, logical_height, factor
    ResizeWindow(f64, f64, Option<f64>),
    WindowState(WindowState),
    SetTitle(String),
    Quit,
}

unsafe impl Send for UserEvent {}
unsafe impl Sync for UserEvent {}

#[derive(Debug, Clone, Copy)]
pub enum WindowState {
    Idle,
    Maximized,
    Minimized,
    Fullscreen,
}
