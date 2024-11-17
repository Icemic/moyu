use std::future::Future;
#[cfg(not(feature = "web"))]
use std::sync::Arc;

#[cfg(not(feature = "web"))]
use tokio::runtime::Handle;

#[cfg(not(feature = "web"))]
use crate::visible_hand::{InvisibleHand, VisibleHand};

#[cfg(not(feature = "web"))]
pub type JoinHandle<T> = tokio::task::JoinHandle<T>;

#[cfg(not(feature = "web"))]
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
#[cfg(not(feature = "web"))]
pub fn spawn<T>(future: T)
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    current_handle().spawn(future);
}

/// Spawn a task.
/// It can be called wherever you want even it is not in the context of a async runtime.
#[cfg(feature = "web")]
pub fn spawn<T>(future: T)
where
    T: Future + 'static,
    T::Output: 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        future.await;
    });
}

/// Spawn a task which is executed in the current thread.
/// Make sure this function is called in the context of a async runtime.
pub fn spawn_local<T>(future: T)
where
    T: Future + 'static,
    T::Output: 'static,
{
    #[cfg(not(feature = "web"))]
    tokio::task::spawn_local(future);

    #[cfg(feature = "web")]
    wasm_bindgen_futures::spawn_local(async move {
        future.await;
    });
}

pub fn block_on<T: Future>(future: T) -> T::Output {
    #[cfg(not(feature = "web"))]
    return current_handle().block_on(future);

    #[cfg(feature = "web")]
    unimplemented!("block_on is not supported in web mode.");
}

/// Block on a future without a runtime. Use it before the runtime is set up.
pub fn block_on_without_runtime<T: Future>(future: T) -> T::Output {
    pollster::block_on(future)
}
