#[derive(Debug)]
pub enum UserEvent {
    // logical_width, logical_height, factor
    ResizeWindow(f64, f64, Option<f64>),
}
