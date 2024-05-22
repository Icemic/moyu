use std::future::Future;
#[cfg(not(feature = "web"))]
use std::sync::Arc;

#[cfg(not(feature = "web"))]
use tokio::runtime::Handle;

#[cfg(not(feature = "web"))]
use crate::visible_hand::{InvisibleHand, VisibleHand};

#[cfg(not(feature = "web"))]
pub type JoinHandle<T> = tokio::task::JoinHandle<T>;

static mut HANDLE: InvisibleHand<Arc<Handle>> = InvisibleHand::new();

#[cfg(not(feature = "web"))]
pub(crate) fn setup_async_runtime() -> VisibleHand<Arc<Handle>> {
    let handle = Arc::new(tokio::runtime::Handle::current());
    unsafe {
        HANDLE.set(handle).expect("Failed to set handle.");
        HANDLE.intervent()
    }
}

#[inline]
#[cfg(not(feature = "web"))]
pub fn get_runtime_handle<'a>() -> &'a std::sync::Arc<Handle> {
    unsafe { HANDLE.get() }
}

#[inline]
#[cfg(not(feature = "web"))]
fn current_handle() -> Arc<tokio::runtime::Handle> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Arc::new(handle),
        Err(_) => get_runtime_handle().clone(),
    }
}

/// Spawn a task.
/// It can be called wherever you want even it is not in the context of a async runtime.
pub fn spawn<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    #[cfg(not(feature = "web"))]
    return current_handle().spawn(future);

    #[cfg(feature = "web")]
    return wasm_bindgen_futures::spawn_local(future);
}

/// Spawn a task which is executed in the current thread.
/// Make sure this function is called in the context of a async runtime.
pub fn spawn_local<T>(future: T) -> JoinHandle<T::Output>
where
    T: Future + 'static,
    T::Output: 'static,
{
    #[cfg(not(feature = "web"))]
    return tokio::task::spawn_local(future);

    #[cfg(feature = "web")]
    return wasm_bindgen_futures::spawn_local(future);
}

pub fn block_on<T: Future>(future: T) -> T::Output {
    #[cfg(not(feature = "web"))]
    return current_handle().block_on(future);

    #[cfg(feature = "web")]
    compile_error!("block_on is not supported in web mode.");
}
