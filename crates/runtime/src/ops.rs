use quickjs_rusty::CustomCallback;

use crate::ops::eval::*;
use crate::ops::http::*;
use crate::ops::websocket::*;

mod eval;
mod http;
pub mod websocket;

pub(crate) fn register_ops(context: &quickjs_rusty::Context) {
    register_single(context, "__moyu_eval", moyu_eval);
    register_single(context, "__moyu_fetch", moyu_fetch);
    register_single(context, "__moyu_ws_connect", ws_connect);
    register_single(context, "__moyu_ws_send", ws_send);
    register_single(context, "__moyu_ws_close", ws_close);
}

fn register_single(context: &quickjs_rusty::Context, name: &str, func: CustomCallback) {
    let func = context.create_custom_callback(func).unwrap();
    context.set_global(name, func).unwrap();
}

pub(crate) fn inject_scripts(context: &quickjs_rusty::Context) {
    context
        .eval(include_str!("injections/location.js"), false)
        .unwrap();

    context
        .eval(include_str!("injections/stubs.js"), false)
        .unwrap();

    context
        .eval(include_str!("injections/websocket.js"), false)
        .unwrap();

    context
        .eval(include_str!("injections/fetch.js"), false)
        .unwrap();
    context
        .eval(include_str!("injections/dom.js"), false)
        .unwrap();
}
