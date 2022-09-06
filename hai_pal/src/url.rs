use anyhow::{Ok, Result};
use node_resolve::resolve_from;
pub use url::*;

pub fn resolve_package_from(target: &str, base_dir: Url) -> Result<Url> {
    let scheme = base_dir.scheme();
    let resolved_file_path = if scheme == "file" {
        let path = base_dir.to_file_path().unwrap();
        Url::from_file_path(resolve_from(target, path)?).unwrap()
    } else if scheme == "https" || scheme == "http" {
        base_dir.join(target)?
    } else {
        return Err(anyhow::format_err!("Unsupported scheme '{}'", scheme));
    };

    Ok(resolved_file_path)
}
