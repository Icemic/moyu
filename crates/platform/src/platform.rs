#[cfg(not(feature = "web"))]
use crate::visible_hand::VisibleHand;
#[cfg(not(feature = "web"))]
use std::sync::Arc;
#[cfg(not(feature = "web"))]
use tokio::runtime::Handle;

#[cfg(not(feature = "web"))]
pub fn setup() -> VisibleHand<Arc<Handle>> {
    use crate::task;

    task::setup_async_runtime()
}

#[cfg(feature = "web")]
pub fn setup() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    // console_error_panic_hook::set_once();
}

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "android")]
pub use android::*;
