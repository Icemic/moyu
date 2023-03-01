use anyhow::format_err;
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::serde_v8;
#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::v8::{HandleScope, Local, Value};
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
type JSValue = wasm_bindgen::JsValue;

#[cfg(not(target_arch = "wasm32"))]
pub struct JSValue<'a, 'b> {
    pub scope: &'b mut HandleScope<'a>,
    pub value: Local<'b, Value>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a, 'b> JSValue<'a, 'b> {
    pub fn new(scope: &'b mut HandleScope<'a>, value: Local<'b, Value>) -> Self {
        Self { scope, value }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn from_js<'a, 'b, T: Deserialize<'a>>(
    scope: &'b mut HandleScope<'a>,
    value: Local<'b, Value>,
) -> Result<T> {
    match serde_v8::from_v8(scope, value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        Err(err) => Err(format_err!(err)),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn from_js<T>(value: JSValue) -> Result<T> {
    serde_wasm_bindgen::from_value(value)?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn to_js<'a, 'b, T: Serialize>(
    scope: &'b mut HandleScope<'a>,
    value: T,
) -> Result<Local<'b, Value>> {
    match serde_v8::to_v8(scope, value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        Err(err) => Err(format_err!(err)),
    }
}
