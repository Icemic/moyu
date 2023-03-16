#[cfg(not(feature = "web"))]
pub fn setup() {
    // no-op
}

#[cfg(feature = "web")]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    // console_error_panic_hook::set_once();
}
