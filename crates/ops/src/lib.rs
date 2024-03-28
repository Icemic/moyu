mod node;
pub mod spawn;
mod system;

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
use hai_js_runtime::{prelude::*, utils::IntoV8, *};
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
use hai_runtime::{
    quickjspp::{JSContext, RawJSValue},
    QuickVM,
};

#[cfg(not(feature = "web"))]
use self::{node::*, system::*};

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
pub fn init(handle_scope: &mut HandleScope, global: &Local<Object>) {
    bind_object! {
        to global;
        of handle_scope;
        "hai" => {
            "pushCommand" => receive_command
        }
    }
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
fn receive_command(scope: &mut HandleScope, args: FunctionCallbackArguments, ret: ReturnValue) {
    let command_name = try_from_value_or_throw_exception!(scope, String, args.get(0));
    let command_name = command_name.to_rust_string_lossy(scope);
    let command_args = try_from_value_or_throw_exception!(scope, Array, args.get(1));

    match command_name.as_str() {
        "load_preset" => load_preset(scope, command_args, ret),
        "resize_window" => resize_window(scope, command_args, ret),
        "set_idle" => set_idle(scope, command_args, ret),
        "set_fullscreen" => set_fullscreen(scope, command_args, ret),
        "set_maximized" => set_maximized(scope, command_args, ret),
        "set_minimized" => set_minimized(scope, command_args, ret),
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

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
pub fn init(vm: &QuickVM) {
    let receive_command = vm
        .context()
        .create_custom_callback(receive_command)
        .unwrap();
    let execute_node_command = vm
        .context()
        .create_custom_callback(execute_node_command)
        .unwrap();

    vm.context()
        .set_global("__hai_pushCommand", receive_command)
        .unwrap();
    vm.context()
        .set_global("__hai_executeNodeCommand", execute_node_command)
        .unwrap();

    vm.context()
        .eval(
            "\
        globalThis.hai = {\
            pushCommand: __hai_pushCommand,\
            executeNodeCommand: __hai_executeNodeCommand\
        }",
        )
        .unwrap();
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
fn receive_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use hai_runtime::quickjspp::OwnedJsArray;

    use hai_core::utils::convert::from_js;
    use hai_core::utils::convert::JSValue;

    let command_name = JSValue::own(context, &args[0]);
    let command_name: &str = from_js(&command_name)?;
    let command_args = {
        let command_args = JSValue::own(context, &args[1]);
        OwnedJsArray::try_from_value(command_args)?.raw_elements()
    };
    let command_args = &command_args;

    // info!("command_name: {}", command_name);

    match command_name {
        "resize_window" => resize_window(context, command_args),
        "set_idle" => set_idle(context, command_args),
        "set_fullscreen" => set_fullscreen(context, command_args),
        "set_maximized" => set_maximized(context, command_args),
        "set_minimized" => set_minimized(context, command_args),
        "quit" => quit(context, command_args),
        "create_instance" => create_instance(context, command_args),
        "destroy_instance" => destroy_instance(context, command_args),
        "add_child" => add_child(context, command_args),
        "insert_child" => insert_child(context, command_args),
        "insert_child_before" => insert_child_before(context, command_args),
        "remove_child" => remove_child(context, command_args),
        "remove_child_at" => remove_child_at(context, command_args),
        "move_to" => move_to(context, command_args),
        // "get_translate" => get_translate(context, command_args),
        "update_props" => update_props(context, command_args),
        _ => Err(anyhow::anyhow!("Unknown command '{}'", command_name)),
    }
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
fn execute_node_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use anyhow::anyhow;

    use hai_core::core::get_core;
    use hai_core::utils::convert::from_js;
    use hai_core::utils::convert::JSValue;

    let node_id = JSValue::own(context, &args[0]);
    let node_id: u32 = from_js(&node_id)?;

    let mut payload = JSValue::own(context, &args[1]);

    if payload.is_object() {
        let core = get_core();
        let node_map = core.node_map.clone();
        let node_map = node_map.read();

        let node = get_node(&node_map, node_id).map_err(|e| anyhow!(e))?;
        let mut node = node.write();

        if let Some(node) = node.as_command() {
            node.execute(&mut payload)
                .map(|v| v.map(|v| unsafe { v.extract() }))
                .map_err(|e| {
                    let err = anyhow!(e);
                    log::error!("Error executing command: {:?}", err);
                    err
                })
        } else {
            Err(anyhow!(
                "Node id {} of type `{}` does not implement Command",
                node_id,
                node.node_type()
            ))
        }
    } else {
        Err(anyhow!("Payload must be an object"))
    }
}
