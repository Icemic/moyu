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
pub fn spawn_runtime_with_core<F>(
    _core: &Arc<Core>,
    on_load: F,
) -> Result<moyu_pal::visible_hand::VisibleHand<Arc<moyu_runtime::QuickVM>>>
where
    F: FnOnce() + Send + 'static,
{
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

    on_load();

    Ok(vm_handle)
}

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(web)]
pub fn spawn_runtime_with_core<F>(_: &Arc<Core>, on_load: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    use log::debug;
    use moyu_pal::config::{AutorunMode, get_engine_config};
    use moyu_pal::dir::entry_dir;

    wasm_bindgen_futures::spawn_local(async move {
        debug!("Injecting entry script.");
        let window = web_sys::window().expect("Cannot get global `window` object.");
        let document = window.document().expect("No document found.");

        let config = get_engine_config();

        if config.autorun == AutorunMode::All {
            use wasm_bindgen::JsCast;
            use wasm_bindgen::prelude::Closure;

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

            root_script
                .add_event_listener_with_callback(
                    "load",
                    Closure::once_into_js(on_load).as_ref().unchecked_ref(),
                )
                .unwrap();

            body.append_child(&root_script).unwrap();
        }
    });

    Ok(())
}
