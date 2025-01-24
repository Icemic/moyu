use anyhow::Result;
#[cfg(native)]
use node_resolve::resolve_from;
pub use url::*;

pub fn resolve_package_from(target: &str, base_dir: Url) -> Result<Url> {
    let scheme = base_dir.scheme();

    #[cfg(native)]
    if scheme == "file" {
        // implicit adding `./` to target if it's a relative path
        // this will disable resolve to node_modules
        // TODO: add support for node_modules via `node:` prefix
        let target =
            if target.starts_with("./") || target.starts_with("../") || target.starts_with("/") {
                target.to_string()
            } else {
                format!("./{}", target)
            };

        let path = base_dir.join("./").unwrap().to_file_path().unwrap();
        return Ok(Url::from_file_path(resolve_from(&target, path)?).unwrap());
    }

    if scheme == "https" || scheme == "http" {
        Ok(base_dir.join(target)?)
    } else {
        Err(anyhow::format_err!("Unsupported scheme '{}'", scheme))
    }
}
