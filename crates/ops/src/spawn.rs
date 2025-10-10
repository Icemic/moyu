use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use moyu_core::core::Core;

pub type SpawnRuntimeCallback =
    Box<dyn (FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(native)]
pub fn spawn_runtime_with_core(
    _core: &Arc<Core>,
) -> Result<moyu_pal::visible_hand::VisibleHand<Arc<moyu_runtime::QuickVM>>> {
    use moyu_runtime::{get_vm, setup_vm};

    let vm_handle = setup_vm();

    let vm = get_vm();

    crate::init(vm);

    if let Err(err) = vm.prepare_entry() {
        return Err(anyhow::anyhow!(
            "Fatal error: failed to run from entry. {}",
            err.to_string()
        ));
    }

    Ok(vm_handle)
}

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(web)]
pub fn spawn_runtime_with_core(_: &Arc<Core>) -> Result<()> {
    use log::debug;
    use moyu_pal::config::{AutorunMode, get_engine_config};
    use moyu_pal::dir::entry_dir;

    wasm_bindgen_futures::spawn_local(async move {
        debug!("Injecting entry script.");
        let window = web_sys::window().expect("Cannot get global `window` object.");
        let document = window.document().expect("No document found.");

        let config = get_engine_config();

        if config.autorun == AutorunMode::All {
            let body = document.body().expect("No body found.");

            let root_script = document
                .create_element("script")
                .expect("Cannot create script element.");
            root_script
                .set_attribute(
                    "src",
                    entry_dir()
                        .join(&get_engine_config().entry_filename)
                        .unwrap()
                        .as_str(),
                )
                .unwrap();
            root_script.set_attribute("type", "module").unwrap();

            body.append_child(&root_script).unwrap();
        }
    });

    Ok(())
}
