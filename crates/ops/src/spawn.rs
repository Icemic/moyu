use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hai_core::core::Core;

pub type SpawnRuntimeCallback =
    Box<dyn (FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
pub fn spawn_runtime_with_core(
    _core: &Arc<Core>,
    spawn_callback: Option<SpawnRuntimeCallback>,
) -> hai_pal::visible_hand::VisibleHand<Arc<hai_runtime::QuickVM>> {
    use hai_runtime::{get_vm, setup_vm};
    use log::error;

    let (sender, receiver) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("quickjs".to_string())
        .spawn(move || {
            let _vm_handle = setup_vm();

            sender.send(_vm_handle).unwrap();

            let handle = hai_pal::task::get_runtime_handle();

            if let Some(spawn_callback) = spawn_callback {
                let async_callback = spawn_callback();
                handle.spawn(async_callback);
            }

            let vm = get_vm();

            // returns 'Could not convert value to string' error randomly
            // not sure why but it's not a big deal (...I think), just ignore it.
            if let Err(err) = vm
                .context()
                .eval("console.info('Hello %s!', 'World')", false)
            {
                error!("{:?}", err);
            };

            crate::init(&vm);

            if let Err(err) = vm.prepare_entry() {
                error!("{:?}", err);
            };

            vm.block_on_ticking();
        })
        .ok();

    receiver.recv().unwrap()
}

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(feature = "web")]
pub fn spawn_runtime_with_core(_: &Arc<Core>, spawn_callback: Option<SpawnRuntimeCallback>) {
    use log::debug;

    if let Some(spawn_callback) = spawn_callback {
        let async_callback = spawn_callback();
        wasm_bindgen_futures::spawn_local(async move {
            async_callback.await;
        });
    }

    wasm_bindgen_futures::spawn_local(async move {
        debug!("Injecting entry script.");
        let window = web_sys::window().expect("Cannot get global `window` object.");
        let document = window.document().expect("No document found.");
        let body = document.body().expect("No body found.");

        let root_script = document
            .create_element("script")
            .expect("Cannot create script element.");
        root_script
            .set_attribute("src", hai_pal::config::entry_dir().as_str())
            .unwrap();
        root_script.set_attribute("type", "module").unwrap();

        body.append_child(&root_script).unwrap();
    });
}
