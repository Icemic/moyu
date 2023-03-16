#[cfg(not(feature = "web"))]
use std::env;
use url::Url;

#[cfg(not(feature = "web"))]
pub fn setup() {
    use dotenv::dotenv;
    // load custom env from .env file
    dotenv().ok();
}

#[cfg(feature = "web")]
pub fn setup() {
    // unimplemented!("env module not implemented for wasm target");
}

pub fn entry_dir() -> Url {
    #[cfg(not(feature = "web"))]
    let entry_dir =
        env::var("HAI_ENTRY").unwrap_or(env::current_dir().unwrap().to_str().unwrap().to_string());
    #[cfg(feature = "web")]
    let entry_dir = "http://localhost:3020/examples/bunnyMark/index.ts".to_string();
    Url::parse(&entry_dir).unwrap()
}
