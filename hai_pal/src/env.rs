use std::{env, path::PathBuf};

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

pub fn entry_dir() -> PathBuf {
    let entry_dir =
        env::var("HAI_ENTRY").unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());
    PathBuf::from(entry_dir)
}
