mod node;
mod system;

use hai_js_runtime::{prelude::*, utils::IntoV8, *};
use log::debug;

use self::{node::*, system::*};

pub fn init(handle_scope: &mut HandleScope, global: &Local<Object>) {
    bind_object! {
        to global;
        of handle_scope;
        "hai" => {
            "pushCommand" => receive_command
        }
    }
}

// (name: string, args: [...], callback?: (err: Error, returnValue: any) => void) => void
fn receive_command(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let command_name = try_from_value_or_throw_exception!(scope, String, args.get(0));
    let command_name = command_name.to_rust_string_lossy(scope);
    let command_args = try_from_value_or_throw_exception!(scope, Array, args.get(1));
    let command_callback = try_from_option_value_or_throw_exception!(scope, Function, args.get(2));

    match command_name.as_str() {
        "test" => debug!("command_name test!"),
        "load_preset" => load_preset(scope, command_args, None),
        "resize_window" => resize_window(scope, command_args, None),
        "quit" => quit(scope, command_args, None),
        "create_instance" => create_instance(scope, command_args, command_callback),
        "add_child" => add_child(scope, command_args, None),
        "insert_child" => insert_child(scope, command_args, None),
        "insert_child_before" => insert_child_before(scope, command_args, None),
        "remove_child" => remove_child(scope, command_args, None),
        "remove_child_at" => remove_child_at(scope, command_args, None),
        _ => {
            let error_message: Local<String> =
                format!("Unknown command '{}'", command_name).into_v8(scope);
            let error = Exception::error(scope, error_message);
            scope.throw_exception(error);
        }
    }
}
