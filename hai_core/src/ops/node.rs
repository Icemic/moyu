use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::{prelude::*, *};
use hai_pal::sync::{RwLock, RwLockReadGuard};
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::core::get_shared_state;
use crate::traits::{JSValue, Node, UpdateProps};
use crate::{
    nodes::{Container, Sprite},
    types::Point,
};

#[inline]
fn get_node<'a>(
    node_map: &'a RwLockReadGuard<HashMap<u32, Arc<RwLock<dyn Node>>>>,
    node_id: u32,
) -> Result<&'a Arc<RwLock<dyn Node>>, std::string::String> {
    let node = node_map.get(&node_id);

    if let Some(node) = node {
        Ok(node)
    } else {
        return Err(format!("Cannot find node by id {}", node_id));
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_instance(
    scope: &mut HandleScope,
    args: Local<Array>,
    callback: Option<Local<Function>>,
) {
    let node_type = get_from_v8_array!(scope, args, 0);
    let label = get_from_v8_array!(scope, args, 1);
    let props = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, node_type);
    check_exist!(scope, props);

    let node_type = try_from_value_or_throw_exception!(scope, String, node_type);
    let node_type = node_type.to_rust_string_lossy(scope);
    let label = try_from_option_value_or_throw_exception!(scope, String, label)
        .and_then(|v| Some(v.to_rust_string_lossy(scope)));
    let props = JSValue::new(scope, props);

    match create_instance_inner(node_type, label, props) {
        Ok(node_id) => {
            // call callback function to return node id
            if callback.is_some() {
                let global_this = scope.get_current_context().global(scope);
                let callback = callback.unwrap();
                let node_id = node_id.into_v8(scope);
                let node_id: Local<Value> = node_id.into();
                callback.call(scope, global_this.into(), &[node_id]);
            }
        }
        Err(s) => throw_exception!(scope, s),
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize, Deserialize)]
pub struct CreateInstanceProps {
    label: Option<String>,
    src: Option<String>,
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn create_instance(node_type: String, props: JsValue) -> Result<u32, std::string::String> {
    let example: CreateInstanceProps = serde_wasm_bindgen::from_value(props).unwrap();
    create_instance_inner(node_type, example.label, props)
}

pub fn create_instance_inner(
    node_type: std::string::String,
    label: Option<std::string::String>,
    mut props: JSValue,
) -> Result<u32, std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let mut node_map = node_map.write();

    let label = label.unwrap_or_default();

    let node_id;
    let node: Arc<RwLock<dyn Node>>;
    match node_type.as_str() {
        "container" => {
            let n = Container::new(label);
            node_id = n.id;
            node = Arc::new(RwLock::new(n));
        }
        "sprite" => {
            let n = Sprite::new(label);
            node_id = n.id;
            node = Arc::new(RwLock::new(n));
        }
        _ => {
            return Err(format!("Unknown nodeType '{}'", node_type));
        }
    };

    node_map.insert(node_id, node.clone());

    drop(state);

    let mut node = node.write();

    Node::update_properties(&mut *node, &mut props);
    UpdateProps::update_properties(&mut *node, &mut props);

    Ok(node_id)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn add_child(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let child_node_id = get_from_v8_array!(scope, args, 1);

    check_exist!(scope, node_id);
    check_exist!(scope, child_node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let child_node_id = try_from_value_or_throw_exception!(scope, Number, child_node_id);

    if let Err(s) = add_child_inner(node_id.value() as u32, child_node_id.value() as u32) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn add_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    add_child_inner(node_id, child_node_id)
}

pub fn add_child_inner(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.add_child(child_node);

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn insert_child(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let index = get_from_v8_array!(scope, args, 1);
    let child_node_id = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, node_id);
    check_exist!(scope, index);
    check_exist!(scope, child_node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let index = try_from_value_or_throw_exception!(scope, Number, index);
    let child_node_id = try_from_value_or_throw_exception!(scope, Number, child_node_id);

    if let Err(s) = insert_child_inner(
        node_id.value() as u32,
        index.value() as usize,
        child_node_id.value() as u32,
    ) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn insert_child(
    node_id: u32,
    index: usize,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    insert_child_inner(node_id, index, child_node_id)
}

pub fn insert_child_inner(
    node_id: u32,
    index: usize,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.insert_child(index, child_node);

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn insert_child_before(
    scope: &mut HandleScope,
    args: Local<Array>,
    _: Option<Local<Function>>,
) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let before_node_id = get_from_v8_array!(scope, args, 1);
    let child_node_id = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, node_id);
    check_exist!(scope, before_node_id);
    check_exist!(scope, child_node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let before_node_id = try_from_value_or_throw_exception!(scope, Number, before_node_id);
    let child_node_id = try_from_value_or_throw_exception!(scope, Number, child_node_id);

    if let Err(s) = insert_child_before_inner(
        node_id.value() as u32,
        before_node_id.value() as u32,
        child_node_id.value() as u32,
    ) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn insert_child_before(
    node_id: u32,
    before_node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    insert_child_before_inner(node_id, before_node_id, child_node_id)
}

pub fn insert_child_before_inner(
    node_id: u32,
    before_node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let before_node = get_node(&node_map, before_node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let before_node = before_node.clone();
    let child_node = child_node.clone();

    node.insert_child_before(before_node, child_node);

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remove_child(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let child_node_id = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, node_id);
    check_exist!(scope, child_node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let child_node_id = try_from_value_or_throw_exception!(scope, Number, child_node_id);

    if let Err(s) = remove_child_inner(node_id.value() as u32, child_node_id.value() as u32) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn remove_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    remove_child_inner(node_id, child_node_id)
}

pub fn remove_child_inner(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.remove_child(child_node).unwrap();

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remove_child_at(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let index = get_from_v8_array!(scope, args, 1);

    check_exist!(scope, node_id);
    check_exist!(scope, index);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let index = try_from_value_or_throw_exception!(scope, Number, index);

    if let Err(s) = remove_child_at_inner(node_id.value() as u32, index.value() as usize) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn remove_child_at(node_id: u32, index: usize) -> Result<(), std::string::String> {
    remove_child_at_inner(node_id, index)
}

pub fn remove_child_at_inner(node_id: u32, index: usize) -> Result<(), std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;

    let mut node = node.write();

    node.remove_child_at(index).unwrap();

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn move_to(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let x = get_from_v8_array!(scope, args, 1);
    let y = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, node_id);
    check_exist!(scope, x);
    check_exist!(scope, y);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let x = try_from_value_or_throw_exception!(scope, Number, x);
    let y = try_from_value_or_throw_exception!(scope, Number, y);

    if let Err(s) = move_to_inner(node_id.value() as u32, x.value(), y.value()) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn move_to(node_id: u32, x: f64, y: f64) -> Result<(), std::string::String> {
    move_to_inner(node_id, x, y)
}

pub fn move_to_inner(node_id: u32, x: f64, y: f64) -> Result<(), std::string::String> {
    let node_map = {
        let state = get_shared_state();

        state.node_map.clone()
    };
    let node_map = node_map.read();
    let node = get_node(&node_map, node_id)?;

    let mut node = node.write();
    node.move_to(x, y);

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_translate(
    scope: &mut HandleScope,
    args: Local<Array>,
    callback: Option<Local<Function>>,
) {
    let node_id = get_from_v8_array!(scope, args, 0);

    check_exist!(scope, node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);

    match get_translate_inner(node_id.value() as u32) {
        Ok([x, y]) => {
            // call callback function to return node id
            if callback.is_some() {
                let global_this = scope.get_current_context().global(scope);
                let callback = callback.unwrap();
                let x = x.into_v8(scope);
                let x: Local<Value> = x.into();
                let y = y.into_v8(scope);
                let y: Local<Value> = y.into();
                callback.call(scope, global_this.into(), &[x, y]);
            }
        }
        Err(s) => throw_exception!(scope, s),
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn get_translate(node_id: u32) -> Result<Vec<i32>, std::string::String> {
    get_translate_inner(node_id).and_then(|v| Ok(v.to_vec()))
}

pub fn get_translate_inner(node_id: u32) -> Result<[f64; 2], std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let node = node.write();

    let &Point { x, y } = node.translate();

    Ok([x, y])
}

#[cfg(not(target_arch = "wasm32"))]
pub fn update_props(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let props = get_from_v8_array!(scope, args, 1);

    check_exist!(scope, node_id);
    check_exist!(scope, props);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);

    let props = JSValue::new(scope, props);

    if let Err(s) = update_props_inner(node_id.value() as u32, props) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn update_props(node_id: u32, props: JsValue) -> Result<(), std::string::String> {
    update_props_inner(node_id, JsValue)
}

pub fn update_props_inner(node_id: u32, mut props: JSValue) -> Result<(), std::string::String> {
    let state = get_shared_state();
    let node_map = state.node_map.clone();

    drop(state);

    let node_map = node_map.read();
    let node = get_node(&node_map, node_id)?;
    let mut node = node.write();

    // set node props
    Node::update_properties(&mut *node, &mut props);

    // set props
    UpdateProps::update_properties(&mut *node, &mut props);

    Ok(())
}
