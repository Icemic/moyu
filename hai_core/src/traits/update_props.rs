use anyhow::Result;
use serde::Deserialize;

pub trait UpdateProps {
    fn update_properties(&mut self, _props: &mut JSValue) {
        // defaults to do nothing
    }
}

#[cfg(target_arch = "wasm32")]
type JSValue = wasm_bindgen::JsValue;

#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::v8::{HandleScope, Local, Value};
#[cfg(not(target_arch = "wasm32"))]
pub struct JSValue<'a, 'b> {
    scope: &'b mut HandleScope<'a>,
    value: Local<'b, Value>,
}
#[cfg(not(target_arch = "wasm32"))]
impl<'a, 'b> JSValue<'a, 'b> {
    pub fn new(scope: &'b mut HandleScope<'a>, value: Local<'b, Value>) -> Self {
        Self { scope, value }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn parse_props<T>(props: JSValue) -> Result<T> {
    serde_wasm_bindgen::from_value(props)?
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_props<'a, 'b, T: Deserialize<'a>>(props: &mut JSValue<'a, 'b>) -> Result<T> {
    use anyhow::format_err;
    use hai_js_runtime::serde_v8;

    match serde_v8::from_v8(props.scope, props.value) {
        Ok(v) => Ok(v),
        Err(serde_v8::Error::Message(msg)) => Err(format_err!(msg)),
        _ => Err(format_err!("error occurs when parsing incoming props.")),
    }
}
