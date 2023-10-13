use anyhow::Result;
#[cfg(not(feature = "web"))]
use node_resolve::resolve_from;
pub use url::*;

pub fn resolve_package_from(target: &str, base_dir: Url) -> Result<Url> {
    let scheme = base_dir.scheme();

    #[cfg(not(feature = "web"))]
    if scheme == "file" {
        let path = base_dir.join("./").unwrap().to_file_path().unwrap();
        return Ok(Url::from_file_path(resolve_from(target, path)?).unwrap());
    }

    if scheme == "https" || scheme == "http" {
        Ok(base_dir.join(target)?)
    } else {
        Err(anyhow::format_err!("Unsupported scheme '{}'", scheme))
    }
}
