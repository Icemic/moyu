#[cfg(feature = "web")]
pub type JSValue = wasm_bindgen::JsValue;

use hai_runtime::quickjs_rusty::OwnedJsPromise;
#[cfg(all(not(feature = "web"), feature = "quickjs"))]
use hai_runtime::quickjs_rusty::{JSContext, OwnedJsValue};

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
pub type JSValue = OwnedJsValue;

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
pub fn from_js<'a, T: serde::Deserialize<'a>>(value: &'a JSValue) -> anyhow::Result<T> {
    use anyhow::format_err;
    pub use hai_runtime::quickjs_rusty::serde::from_js;

    match from_js(value.context(), value) {
        Ok(v) => Ok(v),
        Err(err) => Err(format_err!(err.to_string())),
    }
}

#[cfg(feature = "web")]
pub fn from_js<'a, T: erde::de::DeserializeOwned>(
    value: &mut JSValue,
) -> Result<T, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::from_value(value.to_owned())
}

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
pub fn to_js<T: serde::Serialize>(
    context: *mut JSContext,
    value: &T,
) -> anyhow::Result<OwnedJsValue> {
    use anyhow::format_err;
    pub use hai_runtime::quickjs_rusty::serde::to_js;

    match to_js(context, &value) {
        Ok(v) => Ok(v),
        Err(err) => Err(format_err!(err.to_string())),
    }
}

#[allow(dead_code)]
#[cfg(feature = "web")]
pub fn to_js<'a, T: serde::Serialize>(value: &T) -> Result<JSValue, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::to_value(value)
}

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
pub fn create_promise<F, V>(future: F) -> anyhow::Result<OwnedJsPromise>
where
    F: core::future::Future<Output = Result<V, anyhow::Error>> + Send + 'static,
    V: serde::Serialize + Send,
{
    use hai_pal::task::get_runtime_handle;
    use hai_runtime::get_vm;

    let vm = get_vm();
    let (promise, resolve, reject) =
        OwnedJsPromise::with_resolvers(vm.context()).expect("Failed to create promise");

    get_runtime_handle().spawn(async move {
        match future.await {
            Ok(value) => {
                resolve
                    .call(vec![to_js(vm.context().context_raw(), &value).unwrap()])
                    .expect("Failed to resolve promise");
            }
            Err(err) => {
                reject
                    .call(vec![
                        to_js(vm.context().context_raw(), &err.to_string()).unwrap()
                    ])
                    .expect("Failed to reject promise");
            }
        }
    });

    Ok(promise)
}
