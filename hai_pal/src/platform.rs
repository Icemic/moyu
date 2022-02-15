#[cfg(not(target_arch = "wasm32"))]
pub fn setup() {
    // no-op
}

#[cfg(target_arch = "wasm32")]
pub fn setup() {
    console_error_panic_hook::set_once();
}
