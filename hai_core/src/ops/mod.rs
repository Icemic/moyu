mod node;
mod system;

#[cfg(not(feature = "web"))]
use hai_js_runtime::{prelude::*, utils::IntoV8, *};
#[cfg(not(feature = "web"))]
use log::debug;

#[cfg(not(feature = "web"))]
use self::{node::*, system::*};

#[cfg(not(feature = "web"))]
pub fn init(handle_scope: &mut HandleScope, global: &Local<Object>) {
    bind_object! {
        to global;
        of handle_scope;
        "hai" => {
            "pushCommand" => receive_command
        }
    }
}

#[cfg(not(feature = "web"))]
// (name: string, args: [...], callback?: (err: Error, returnValue: any) => void) => void
fn receive_command(scope: &mut HandleScope, args: FunctionCallbackArguments, ret: ReturnValue) {
    let command_name = try_from_value_or_throw_exception!(scope, String, args.get(0));
    let command_name = command_name.to_rust_string_lossy(scope);
    let command_args = try_from_value_or_throw_exception!(scope, Array, args.get(1));

    match command_name.as_str() {
        "test" => debug!("command_name test!"),
        "load_preset" => load_preset(scope, command_args, ret),
        "resize_window" => resize_window(scope, command_args, ret),
        "quit" => quit(scope, command_args, ret),
        "create_instance" => create_instance(scope, command_args, ret),
        "add_child" => add_child(scope, command_args, ret),
        "insert_child" => insert_child(scope, command_args, ret),
        "insert_child_before" => insert_child_before(scope, command_args, ret),
        "remove_child" => remove_child(scope, command_args, ret),
        "remove_child_at" => remove_child_at(scope, command_args, ret),
        "move_to" => move_to(scope, command_args, ret),
        // "get_translate" => get_translate(scope, command_args, ret),
        "update_props" => update_props(scope, command_args, ret),
        _ => {
            let error_message: Local<String> =
                format!("Unknown command '{}'", command_name).into_v8(scope);
            let error = Exception::error(scope, error_message);
            scope.throw_exception(error);
        }
    }
}
