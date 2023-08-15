use log::{error, info};

use hai_pal::env::entry_dir;
use hai_pal::url::{resolve_package_from, Url};
use hai_pal::{fs, task};

pub fn module_loader(module_name: &str, _: *mut std::ffi::c_void) -> String {
    let module_name = Url::parse(module_name).unwrap();
    let code = match task::block_on(fs::read(&module_name)) {
        Ok(v) => String::from_utf8(v).unwrap(),
        Err(err) => {
            error!("failed to read '{}': {}", module_name, err.to_string());
            "".to_string()
        }
    };
    info!("module {} loaded", module_name.to_string());
    code
}

pub fn module_normalize(
    module_base_name: &str,
    module_name: &str,
    _: *mut std::ffi::c_void,
) -> String {
    let base_dir = if module_base_name == "." {
        entry_dir()
    } else {
        Url::parse(module_base_name).unwrap()
    };

    let resolved_module_path = resolve_package_from(module_name, base_dir).unwrap();
    let resolved_module_path = resolved_module_path.to_string();

    info!(
        "resolving module {} {} to {}",
        module_base_name, module_name, resolved_module_path
    );

    resolved_module_path
}
