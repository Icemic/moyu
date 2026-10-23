#[cfg(native)]
use crate::visible_hand::VisibleHand;
#[cfg(native)]
use std::sync::Arc;
#[cfg(native)]
use tokio::runtime::Handle;

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether fatal errors skip the modal dialog and only reach the log.
///
/// A runtime with no window has nobody to dismiss a dialog, so it turns this on
/// before anything that can fail; the process still exits with the same code.
static ERROR_DIALOGS_DISABLED: AtomicBool = AtomicBool::new(false);

/// Skip the modal dialog [`show_fatal_error_and_exit`] would show.
pub fn disable_error_dialogs() {
    ERROR_DIALOGS_DISABLED.store(true, Ordering::Relaxed);
}

/// Whether the process runs without a window, as set by [`set_headless`].
static HEADLESS: AtomicBool = AtomicBool::new(false);

/// Record that the host started the engine without a window.
pub fn set_headless(headless: bool) {
    HEADLESS.store(headless, Ordering::Relaxed);
}

/// Whether the host started the engine without a window.
pub fn is_headless() -> bool {
    HEADLESS.load(Ordering::Relaxed)
}

#[cfg(native)]
pub fn setup() -> VisibleHand<Arc<Handle>> {
    use crate::task;

    task::setup_async_runtime()
}

#[cfg(web)]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    // console_error_panic_hook::set_once();
}

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub use android::*;

pub fn show_fatal_error_and_exit(message: &str) -> ! {
    #[cfg(desktop)]
    {
        if ERROR_DIALOGS_DISABLED.load(Ordering::Relaxed) {
            // Nothing can dismiss a dialog, so the log is the only way to report it.
            log::error!("Fatal Error: {}", message);
        } else {
            use native_dialog::{DialogBuilder, MessageLevel};
            let _ = DialogBuilder::message()
                .set_title("Fatal Error")
                .set_text(message)
                .set_level(MessageLevel::Error)
                .alert()
                .show();
        }
    }

    #[cfg(any(mobile, web))]
    {
        log::error!("Fatal Error: {}", message);
    }

    std::process::exit(1);
}
