#[cfg(web)]
pub type JSValue = wasm_bindgen::JsValue;
#[cfg(web)]
pub type OwnedJsPromise = web_sys::js_sys::Promise;

#[cfg(all(native, feature = "js_runtime"))]
use moyu_runtime::quickjs_rusty::OwnedJsValue;

#[cfg(all(native, feature = "js_runtime"))]
pub type JSValue = OwnedJsValue;

#[cfg(all(native, feature = "js_runtime"))]
pub fn from_js<T: serde::de::DeserializeOwned>(value: &JSValue) -> anyhow::Result<T> {
    use anyhow::format_err;
    pub use moyu_runtime::quickjs_rusty::serde::from_js;

    match from_js(value.context(), value) {
        Ok(v) => Ok(v),
        Err(err) => Err(format_err!(err.to_string())),
    }
}

#[cfg(web)]
pub fn from_js<'a, T: serde::de::DeserializeOwned>(value: &mut JSValue) -> anyhow::Result<T> {
    use anyhow::anyhow;

    serde_wasm_bindgen::from_value(value.to_owned()).map_err(|e| anyhow!(e.to_string()))
}

#[cfg(all(native, feature = "js_runtime"))]
pub fn to_js<T: serde::Serialize>(value: &T) -> anyhow::Result<OwnedJsValue> {
    use anyhow::format_err;
    use moyu_runtime::get_vm;
    pub use moyu_runtime::quickjs_rusty::serde::to_js;

    // since `to_js` is always called in quickjs thread, it's safe to get context directly.
    let context = unsafe { get_vm().context().context_raw() };

    match to_js(context, &value) {
        Ok(v) => Ok(v),
        Err(err) => Err(format_err!(err.to_string())),
    }
}

#[allow(dead_code)]
#[cfg(web)]
pub fn to_js<'a, T: serde::Serialize>(value: &T) -> anyhow::Result<JSValue> {
    use anyhow::anyhow;

    serde_wasm_bindgen::to_value(value).map_err(|e| anyhow!(e.to_string()).into())
}

#[cfg(all(native, feature = "js_runtime"))]
pub fn create_promise<F, V>(future: F) -> anyhow::Result<JSValue>
where
    F: core::future::Future<Output = Result<V, anyhow::Error>> + Send + 'static,
    V: serde::Serialize + Send + 'static,
{
    use moyu_runtime::get_vm;

    let vm = get_vm();

    vm.create_promise(future)
}

#[cfg(web)]
pub fn create_promise<F, V>(future: F) -> anyhow::Result<JSValue>
where
    F: core::future::Future<Output = Result<V, anyhow::Error>> + 'static,
    V: serde::Serialize,
{
    use wasm_bindgen_futures::future_to_promise;

    let promise = future_to_promise(async move {
        match future.await {
            Ok(value) => Ok(serde_wasm_bindgen::to_value(&value)
                // .map(|v| v.0)
                .unwrap()),
            Err(err) => Err(wasm_bindgen::JsValue::from_str(&err.to_string())),
        }
    });

    let promise = wasm_bindgen::JsValue::from(promise);

    Ok(promise)
}
