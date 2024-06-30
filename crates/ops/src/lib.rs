mod node;
pub mod spawn;
mod system;

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
use hai_runtime::{
    quickjs_rusty::{JSContext, RawJSValue},
    QuickVM,
};

#[cfg(not(feature = "web"))]
use self::{node::*, system::*};

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
    let execute_plugin_command = vm
        .context()
        .create_custom_callback(execute_plugin_command)
        .unwrap();

    vm.context()
        .set_global("__hai_pushCommand", receive_command)
        .unwrap();
    vm.context()
        .set_global("__hai_executeNodeCommand", execute_node_command)
        .unwrap();
    vm.context()
        .set_global("__hai_executePluginCommand", execute_plugin_command)
        .unwrap();

    vm.context()
        .eval(
            "\
        globalThis.hai = {\
            pushCommand: __hai_pushCommand,\
            executeNodeCommand: __hai_executeNodeCommand,\
            executePluginCommand: __hai_executePluginCommand,\
        }",
            false,
        )
        .unwrap();
}

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
fn receive_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use hai_runtime::quickjs_rusty::OwnedJsArray;

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
        let node_map = core.node_map().read();

        let node = get_node(&node_map, node_id).map_err(|e| anyhow!(e))?;
        let mut node = node.write();

        if let Some(node) = node.as_command() {
            node.execute(&mut payload)
                .map(|v| v.map(|v| unsafe { v.extract() }))
                .map_err(|e| {
                    let err = anyhow!(e);
                    log::error!("Error executing node command: {:?}", err);
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

#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
fn execute_plugin_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use anyhow::anyhow;

    use hai_core::core::get_core;
    use hai_core::utils::convert::from_js;
    use hai_core::utils::convert::JSValue;

    let plugin_name = JSValue::own(context, &args[0]);
    let plugin_name: &str = from_js(&plugin_name)?;

    let mut payload = JSValue::own(context, &args[1]);

    if payload.is_object() {
        let core = get_core();

        let plugin = core
            .get_plugin(plugin_name)
            .ok_or_else(|| anyhow!("Plugin {} not found", plugin_name))?;

        let mut plugin = plugin.lock();

        if let Some(plugin) = plugin.as_command() {
            plugin
                .execute(&mut payload)
                .map(|v| v.map(|v| unsafe { v.extract() }))
                .map_err(|e| {
                    let err = anyhow!(e);
                    log::error!("Error executing plugin command: {:?}", err);
                    err
                })
        } else {
            log::warn!("Plugin `{}` does not implement Command", plugin_name);
            Err(anyhow!(
                "Plugin `{}` does not implement Command",
                plugin_name
            ))
        }
    } else {
        Err(anyhow!("Payload must be an object"))
    }
}
