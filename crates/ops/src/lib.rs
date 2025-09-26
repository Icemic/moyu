pub mod node;
pub mod spawn;

use moyu_core::utils::convert::JSValue;
#[cfg(native)]
use moyu_runtime::{
    quickjs_rusty::{JSContext, RawJSValue},
    QuickVM,
};
#[cfg(web)]
use wasm_bindgen::prelude::wasm_bindgen;

use self::node::*;

#[cfg(native)]
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
        .set_global("__moyu_pushCommand", receive_command)
        .unwrap();
    vm.context()
        .set_global("__moyu_executeNodeCommand", execute_node_command)
        .unwrap();
    vm.context()
        .set_global("__moyu_executePluginCommand", execute_plugin_command)
        .unwrap();

    vm.context()
        .eval(
            "\
        globalThis.moyu = {\
            pushCommand: __moyu_pushCommand,\
            executeNodeCommand: __moyu_executeNodeCommand,\
            executePluginCommand: __moyu_executePluginCommand,\
        }",
            false,
        )
        .unwrap();
}

#[cfg(native)]
fn receive_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use moyu_runtime::quickjs_rusty::OwnedJsArray;

    use moyu_core::utils::convert::from_js;
    use moyu_core::utils::convert::JSValue;

    let command_name = JSValue::own(context, &args[0]);
    let command_name = from_js::<String>(&command_name)?;
    let command_name = command_name.as_str();

    let command_args = {
        let command_args = JSValue::own(context, &args[1]);
        OwnedJsArray::try_from_value(command_args)?.raw_elements()
    };
    let command_args = command_args.as_slice();

    // info!("command_name: {}", command_name);

    match command_name {
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

#[cfg(native)]
fn execute_node_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use moyu_core::utils::convert::from_js;
    use moyu_core::utils::convert::JSValue;

    let node_id = JSValue::own(context, &args[0]);
    let node_id: u32 = from_js(&node_id)?;

    let mut payload = JSValue::own(context, &args[1]);

    execute_node_command_inner(node_id, &mut payload).map(|v| v.map(|v| unsafe { v.extract() }))
}

#[cfg(web)]
#[cfg_attr(web, wasm_bindgen)]
pub fn execute_node_command(
    node_id: u32,
    mut payload: JSValue,
) -> Result<JSValue, std::string::String> {
    let ret = execute_node_command_inner(node_id, &mut payload).map_err(|e| e.to_string())?;
    Ok(ret.unwrap_or(JSValue::undefined()))
}

#[inline]
fn execute_node_command_inner(
    node_id: u32,
    payload: &mut JSValue,
) -> anyhow::Result<Option<JSValue>> {
    use anyhow::anyhow;
    use moyu_core::core::get_core;

    if payload.is_object() {
        let core = get_core();
        let node_map = core.node_map();

        let node = get_node(&node_map, node_id).map_err(|e| anyhow!(e))?;
        let mut node = node.write();

        if let Some(node) = node.as_command() {
            node.execute(payload).map_err(|e| {
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

#[cfg(native)]
fn execute_plugin_command(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> anyhow::Result<Option<RawJSValue>> {
    use moyu_core::utils::convert::from_js;
    use moyu_core::utils::convert::JSValue;

    let plugin_name = JSValue::own(context, &args[0]);
    let plugin_name = from_js::<String>(&plugin_name)?;
    let plugin_name = plugin_name.as_str();

    let mut payload = JSValue::own(context, &args[1]);

    execute_plugin_command_inner(plugin_name, &mut payload)
        .map(|v| v.map(|v| unsafe { v.extract() }))
}

#[cfg(web)]
#[cfg_attr(web, wasm_bindgen)]
pub fn execute_plugin_command(
    plugin_name: &str,
    mut payload: JSValue,
) -> Result<JSValue, std::string::String> {
    let ret = execute_plugin_command_inner(plugin_name, &mut payload).map_err(|e| e.to_string())?;
    Ok(ret.unwrap_or(JSValue::undefined()))
}

#[inline]
fn execute_plugin_command_inner(
    plugin_name: &str,
    payload: &mut JSValue,
) -> anyhow::Result<Option<JSValue>> {
    use anyhow::anyhow;
    use moyu_core::core::get_core;

    if payload.is_object() {
        let core = get_core();

        let plugin = core
            .get_plugin(plugin_name)
            .ok_or_else(|| anyhow!("Plugin {} not found", plugin_name))?;

        let mut plugin = plugin.lock();

        if let Some(plugin) = plugin.as_command() {
            plugin.execute(payload).map_err(|e| {
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
