#[cfg(not(target_arch = "wasm32"))]
pub fn setup() {
    use dotenv::dotenv;
    // load custom env from .env file
    dotenv().ok();
}

#[cfg(target_arch = "wasm32")]
pub fn setup() {
    // unimplemented!("env module not implemented for wasm target");
}
