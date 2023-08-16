use std::future::Future;
#[cfg(not(feature = "web"))]
use std::sync::Arc;

use once_cell::sync::OnceCell;

#[cfg(not(feature = "web"))]
use tokio::runtime::Handle;

#[cfg(not(feature = "web"))]
pub type JoinHandle<T> = tokio::task::JoinHandle<T>;

#[cfg(not(feature = "web"))]
pub(crate) fn setup_async_runtime() {
    use std::ffi::c_void;

    let handle = Arc::new(tokio::runtime::Handle::current());
    let p = Arc::into_raw(handle) as *const c_void as usize;
    HANDLE.set(p).expect("Failed to set handle.");
}

static HANDLE: OnceCell<usize> = OnceCell::new();

#[inline]
#[cfg(not(feature = "web"))]
pub fn get_runtime_handle() -> std::sync::Arc<Handle> {
    use std::ffi::c_void;

    let p = *HANDLE.get().unwrap() as *const c_void;
    let ptr = p as *const Handle;
    let r = unsafe { Arc::from_raw(ptr) };
    let r_cloned = r.clone();

    // keep ptr leaked
    std::mem::forget(r);

    r_cloned
}

#[inline]
#[cfg(not(feature = "web"))]
fn current_handle() -> Arc<tokio::runtime::Handle> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Arc::new(handle),
        Err(_) => get_runtime_handle(),
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
