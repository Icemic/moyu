use std::future::Future;
#[cfg(native)]
use std::sync::Arc;

#[cfg(native)]
use tokio::runtime::Handle;

#[cfg(native)]
use crate::visible_hand::{InvisibleHand, VisibleHand};

#[cfg(native)]
pub type JoinHandle<T> = tokio::task::JoinHandle<T>;

#[cfg(native)]
static HANDLE: InvisibleHand<Arc<Handle>> = InvisibleHand::new();

#[cfg(native)]
pub(crate) fn setup_async_runtime() -> VisibleHand<Arc<Handle>> {
    let handle = Arc::new(tokio::runtime::Handle::current());
    HANDLE.set(handle).expect("Failed to set handle.");
    HANDLE.intervent()
}

#[inline]
#[cfg(native)]
pub fn get_runtime_handle<'a>() -> &'a std::sync::Arc<Handle> {
    HANDLE.get()
}

#[inline]
#[cfg(native)]
fn current_handle() -> Arc<tokio::runtime::Handle> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Arc::new(handle),
        Err(_) => get_runtime_handle().clone(),
    }
}

/// Spawn a task.
/// It can be called wherever you want even it is not in the context of a async runtime.
#[cfg(native)]
pub fn spawn<T>(future: T)
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    current_handle().spawn(future);
}

/// Spawn a task.
/// It can be called wherever you want even it is not in the context of a async runtime.
#[cfg(web)]
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
    #[cfg(native)]
    tokio::task::spawn_local(future);

    #[cfg(web)]
    wasm_bindgen_futures::spawn_local(async move {
        future.await;
    });
}

pub fn block_on<T: Future>(future: T) -> T::Output {
    #[cfg(native)]
    return current_handle().block_on(future);

    #[cfg(web)]
    unimplemented!("block_on is not supported in web mode.");
}

/// Block on a future without a runtime. Use it before the runtime is set up.
pub fn block_on_without_runtime<T: Future>(future: T) -> T::Output {
    pollster::block_on(future)
}

/// Check if the current thread is the main thread. \
/// In native mode, it checks if the current thread name is "main". \
/// In web mode, it always returns true. Since web mode does not have a concept of threads (for now).
pub fn is_main_thread() -> bool {
    #[cfg(native)]
    return std::thread::current().name() == Some("main");

    #[cfg(web)]
    return true;
}

#[cfg(native)]
pub struct TimeoutHandle(tokio::task::JoinHandle<()>);
#[cfg(web)]
pub struct TimeoutHandle(i32);

pub fn set_timeout<F>(duration: std::time::Duration, callback: F) -> TimeoutHandle
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    #[cfg(native)]
    {
        let handle = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            callback.await;
        });
        TimeoutHandle(handle)
    }

    #[cfg(web)]
    {
        use wasm_bindgen::JsCast;

        let closure = wasm_bindgen::closure::Closure::once_into_js(move || {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = callback.await;
            });
        });
        let id = web_sys::window()
            .expect("no global `window` exists")
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.unchecked_ref(),
                duration.as_millis() as i32,
            )
            .expect("should register `setTimeout` OK");
        TimeoutHandle(id)
    }
}

pub fn clear_timeout(handle: &TimeoutHandle) {
    #[cfg(native)]
    {
        handle.0.abort();
    }

    #[cfg(web)]
    {
        web_sys::window()
            .expect("no global `window` exists")
            .clear_timeout_with_handle(handle.0);
    }
}
