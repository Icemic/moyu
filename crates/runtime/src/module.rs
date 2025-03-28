use anyhow::Result;
use log::info;

use doufu_pal::config::entry_dir;
use doufu_pal::url::{resolve_package_from, Url};
use doufu_pal::{fs, task};

pub fn module_loader(module_name: &str, _: *mut std::ffi::c_void) -> Result<String> {
    let module_name = Url::parse(module_name)?;
    let code = match task::block_on_without_runtime(fs::read(&module_name)) {
        Ok(v) => String::from_utf8(v)?,
        Err(err) => {
            return Err(anyhow::anyhow!(
                "failed to read '{}': {}",
                module_name,
                err.to_string()
            ));
        }
    };
    info!("module {} loaded", module_name.to_string());
    Ok(code)
}

pub fn module_normalize(
    module_base_name: &str,
    module_name: &str,
    _: *mut std::ffi::c_void,
) -> Result<String> {
    let base_dir = if module_base_name == "." {
        entry_dir()
    } else {
        Url::parse(module_base_name)?
    };

    if let Ok(resolved_module_path) = resolve_package_from(module_name, base_dir) {
        let resolved_module_path = resolved_module_path.to_string();

        info!(
            "resolving module {} {} to {}",
            module_base_name, module_name, resolved_module_path
        );

        Ok(resolved_module_path)
    } else {
        Err(anyhow::anyhow!(
            "failed to resolve module {} {}",
            module_base_name,
            module_name
        ))
    }
}
