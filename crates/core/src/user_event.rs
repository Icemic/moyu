#[allow(dead_code)]
#[derive(Debug)]
pub enum UserEvent {
    // logical_width, logical_height, factor
    ResizeWindow(f64, f64, Option<f64>),
    SetTitle(String),
    Quit,
}

unsafe impl Send for UserEvent {}
unsafe impl Sync for UserEvent {}
