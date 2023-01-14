#[cfg(not(target_arch = "wasm32"))]
pub fn setup() {
    // no-op
}

#[cfg(target_arch = "wasm32")]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    // console_error_panic_hook::set_once();
}
