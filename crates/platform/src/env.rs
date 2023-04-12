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

    if entry_dir.starts_with("http://")
        || entry_dir.starts_with("https://")
        || entry_dir.starts_with("file://")
    {
        Url::parse(&entry_dir).unwrap()
    } else if cfg!(not(feature = "web")) && !entry_dir.contains("://") {
        #[cfg(not(feature = "web"))]
        get_entry_dir_local(entry_dir)
    } else {
        unimplemented!("unsupported entry '{}'.", entry_dir);
    }
}

#[cfg(not(feature = "web"))]
fn get_entry_dir_local(entry_dir: String) -> Url {
    let local_path = env::current_dir().unwrap();
    let local_path = local_path.join(entry_dir);
    if local_path.is_file() {
        Url::from_file_path(&local_path).unwrap()
    } else {
        Url::from_directory_path(&local_path).unwrap()
    }
}
