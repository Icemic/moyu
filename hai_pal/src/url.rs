use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use node_resolve::resolve_from;
pub use url::*;

pub fn resolve_package_from(target: &str, base_dir: Url) -> Result<Url> {
    let scheme = base_dir.scheme();

    #[cfg(not(target_arch = "wasm32"))]
    if scheme == "file" {
        let path = base_dir.to_file_path().unwrap();
        return Ok(Url::from_file_path(resolve_from(target, path)?).unwrap());
    }

    if scheme == "https" || scheme == "http" {
        return Ok(base_dir.join(target)?);
    } else {
        return Err(anyhow::format_err!("Unsupported scheme '{}'", scheme));
    };
}
