use anyhow::Result;
#[cfg(web)]
use wasm_bindgen::prelude::wasm_bindgen;

use moyu_core::core::get_core;
#[cfg(native)]
use moyu_core::utils::convert::{JSValue, from_js};
#[cfg(native)]
use moyu_macros::moyu_bindgen;
#[cfg(native)]
use moyu_runtime::quickjs_rusty::{JSContext, RawJSValue};

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn focus_editable(node_id: u32) -> Result<(), std::string::String> {
    get_core().editable().focus(node_id);
    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn blur_editable(node_id: u32) -> Result<(), std::string::String> {
    get_core().editable().blur(node_id);
    Ok(())
}
