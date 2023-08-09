#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
use hai_js_runtime::v8::{HandleScope, Local, Value};

#[cfg(feature = "web")]
pub type JSValue = wasm_bindgen::JsValue;

#[cfg(all(not(feature = "web"), feature = "v8"))]
pub struct JSValue<'a, 'b> {
    pub scope: &'b mut HandleScope<'a>,
    pub value: Local<'b, Value>,
}

#[cfg(all(not(feature = "web"), feature = "v8"))]
impl<'a, 'b> JSValue<'a, 'b> {
    pub fn new(scope: &'b mut HandleScope<'a>, value: Local<'b, Value>) -> Self {
        Self { scope, value }
    }
}

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
use quick_runtime::quickjspp::{JSContext, OwnedJsValue};

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
pub type JSValue = OwnedJsValue;

#[cfg(all(not(feature = "web"), feature = "quickjs"))]
pub fn from_js<'a, T: serde::Deserialize<'a>>(value: &'a JSValue) -> anyhow::Result<T> {
    use anyhow::format_err;
    pub use quick_runtime::quickjspp::serde::{from_js, to_js};

    match from_js(value.context(), value) {
        Ok(v) => Ok(v),
        Err(err) => Err(format_err!(err.to_string())),
    }
}

#[cfg(all(not(feature = "web"), feature = "v8"))]
pub fn from_js<'a, 'b, T: serde::Deserialize<'a>>(
    value: &mut JSValue<'a, 'b>,
) -> anyhow::Result<T> {
    use anyhow::format_err;
    use hai_js_runtime::serde_v8;

    match serde_v8::from_v8(value.scope, value.value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        Err(err) => Err(format_err!(err)),
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
    pub use quick_runtime::quickjspp::serde::to_js;

    match to_js(context, &value) {
        Ok(v) => Ok(v),
        Err(err) => Err(format_err!(err.to_string())),
    }
}

#[cfg(all(not(feature = "web"), feature = "v8"))]
pub fn to_js<'a, 'b, T: serde::Serialize>(
    scope: &'b mut HandleScope<'a>,
    value: &T,
) -> Result<Local<'b, Value>> {
    use anyhow::format_err;
    use hai_js_runtime::serde_v8;

    match serde_v8::to_v8(scope, value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        Err(err) => Err(format_err!(err)),
    }
}

#[allow(dead_code)]
#[cfg(feature = "web")]
pub fn to_js<'a, T: serde::Serialize>(value: &T) -> Result<JSValue, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::to_value(value)
}
