use anyhow::Result;
#[cfg(not(feature = "web"))]
use hai_js_runtime::serde_v8;
#[cfg(not(feature = "web"))]
use hai_js_runtime::v8::{HandleScope, Local, Value};
#[cfg(feature = "web")]
use serde::de::DeserializeOwned;
#[cfg(not(feature = "web"))]
use serde::Deserialize;
use serde::Serialize;
#[cfg(feature = "web")]
pub type JSValue = wasm_bindgen::JsValue;

#[cfg(not(feature = "web"))]
pub struct JSValue<'a, 'b> {
    pub scope: &'b mut HandleScope<'a>,
    pub value: Local<'b, Value>,
}

#[cfg(not(feature = "web"))]
impl<'a, 'b> JSValue<'a, 'b> {
    pub fn new(scope: &'b mut HandleScope<'a>, value: Local<'b, Value>) -> Self {
        Self { scope, value }
    }
}

#[cfg(not(feature = "web"))]
pub fn from_js<'a, 'b, T: Deserialize<'a>>(value: &mut JSValue<'a, 'b>) -> Result<T> {
    use anyhow::format_err;

    match serde_v8::from_v8(value.scope, value.value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        Err(err) => Err(format_err!(err)),
    }
}

#[cfg(feature = "web")]
pub fn from_js<'a, T: DeserializeOwned>(
    value: &mut JSValue,
) -> Result<T, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::from_value(value.to_owned())
}

#[cfg(not(feature = "web"))]
pub fn to_js<'a, 'b, T: Serialize>(
    scope: &'b mut HandleScope<'a>,
    value: &T,
) -> Result<Local<'b, Value>> {
    use anyhow::format_err;

    match serde_v8::to_v8(scope, value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        Err(err) => Err(format_err!(err)),
    }
}

#[allow(dead_code)]
#[cfg(feature = "web")]
pub fn to_js<'a, T: Serialize>(value: &T) -> Result<JSValue, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::to_value(value)
}
