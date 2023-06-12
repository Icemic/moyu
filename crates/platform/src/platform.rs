#[cfg(not(feature = "web"))]
pub fn setup() {
    use crate::task;

    task::setup_async_runtime();
}

#[cfg(feature = "web")]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    // console_error_panic_hook::set_once();
}
