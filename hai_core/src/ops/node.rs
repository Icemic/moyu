use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use hai_js_runtime::{prelude::*, *};
#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[cfg(target_arch = "wasm32")]
use crate::web::get_shared_state;
use crate::{
    nodes::{Container, Sprite},
    state::State,
    types::Point,
};

#[cfg(not(target_arch = "wasm32"))]
pub fn create_instance(
    scope: &mut HandleScope,
    args: Local<Array>,
    callback: Option<Local<Function>>,
) {
    let node_type = get_from_v8_array!(scope, args, 0);
    let props = get_from_v8_array!(scope, args, 1);

    check_exist!(scope, node_type);
    check_exist!(scope, props);

    let node_type = try_from_value_or_throw_exception!(scope, String, node_type);
    let props = try_from_value_or_throw_exception!(scope, Object, props);
    let node_type = node_type.to_rust_string_lossy(scope);

    let label = get_from_v8_object!(scope, props, "label");
    let label = try_from_option_value_or_throw_exception!(scope, String, label)
        .and_then(|v| Some(v.to_rust_string_lossy(scope)));
    let src = get_from_v8_object!(scope, props, "src");
    let src = try_from_option_value_or_throw_exception!(scope, String, src)
        .and_then(|v| Some(v.to_rust_string_lossy(scope)));

    let state = get_shared_state!(scope, State);

    match create_instance_inner(state, node_type, label, src) {
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
    let state = get_shared_state();
    create_instance_inner(state, node_type, example.label, example.src)
}

pub fn create_instance_inner(
    state: Arc<Mutex<State>>,
    node_type: std::string::String,
    label: Option<std::string::String>,
    src: Option<std::string::String>,
) -> Result<u32, std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let mut node_map = node_map.lock().unwrap();

    let label = label.unwrap_or_default();

    let node_id;
    match node_type.as_str() {
        "node" => {
            let n = Container::new(label, Default::default(), Default::default());
            node_id = n.id;
            node_map.insert(n.id, Arc::new(Mutex::new(n)));
        }
        "sprite" => {
            let src = src.unwrap_or_default();

            let mut resource_manager = state.resource_manager.lock().unwrap();
            let texture = resource_manager.get_texture(src.clone());
            let n = Sprite::new(src, texture);
            node_id = n.id;
            node_map.insert(n.id, Arc::new(Mutex::new(n)));
        }
        _ => {
            return Err(format!("Unknown nodeType '{}'", node_type));
        }
    };

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

    let state = get_shared_state!(scope, State);

    if let Err(s) = add_child_inner(state, node_id.value() as u32, child_node_id.value() as u32) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn add_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let state = get_shared_state();
    add_child_inner(state, node_id, child_node_id)
}

pub fn add_child_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);
    let child_node = node_map.get(&child_node_id);

    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    if child_node.is_none() {
        return Err(format!("Cannot find node by id {}", child_node_id));
    }

    let mut node = node.unwrap().lock().unwrap();
    let child_node = child_node.unwrap().clone();

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

    let state = get_shared_state!(scope, State);

    if let Err(s) = insert_child_inner(
        state,
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
    let state = get_shared_state();
    insert_child_inner(state, node_id, index, child_node_id)
}

pub fn insert_child_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
    index: usize,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);
    let child_node = node_map.get(&child_node_id);

    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    if child_node.is_none() {
        return Err(format!("Cannot find node by id {}", child_node_id));
    }

    let mut node = node.unwrap().lock().unwrap();
    let child_node = child_node.unwrap().clone();

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

    let state = get_shared_state!(scope, State);

    if let Err(s) = insert_child_before_inner(
        state,
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
    let state = get_shared_state();
    insert_child_before_inner(state, node_id, before_node_id, child_node_id)
}

pub fn insert_child_before_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
    before_node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);
    let before_node = node_map.get(&before_node_id);
    let child_node = node_map.get(&child_node_id);

    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    if before_node.is_none() {
        return Err(format!("Cannot find node by id {}", before_node_id));
    }

    if child_node.is_none() {
        return Err(format!("Cannot find node by id {}", child_node_id));
    }

    let mut node = node.unwrap().lock().unwrap();
    let before_node = before_node.unwrap().clone();
    let child_node = child_node.unwrap().clone();

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

    let state = get_shared_state!(scope, State);

    if let Err(s) = remove_child_inner(state, node_id.value() as u32, child_node_id.value() as u32)
    {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn remove_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let state = get_shared_state();
    remove_child_inner(state, node_id, child_node_id)
}

pub fn remove_child_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);
    let child_node = node_map.get(&child_node_id);

    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    if child_node.is_none() {
        return Err(format!("Cannot find node by id {}", child_node_id));
    }

    let mut node = node.unwrap().lock().unwrap();
    let child_node = child_node.unwrap().clone();

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

    let state = get_shared_state!(scope, State);

    if let Err(s) = remove_child_at_inner(state, node_id.value() as u32, index.value() as usize) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn remove_child_at(node_id: u32, index: usize) -> Result<(), std::string::String> {
    let state = get_shared_state();
    remove_child_at_inner(state, node_id, index)
}

pub fn remove_child_at_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
    index: usize,
) -> Result<(), std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);

    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    let mut node = node.unwrap().lock().unwrap();

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

    let state = get_shared_state!(scope, State);
    if let Err(s) = move_to_inner(
        state,
        node_id.value() as u32,
        x.value() as i32,
        y.value() as i32,
    ) {
        throw_exception!(scope, s);
    }
}

#[wasm_bindgen]
#[cfg(target_arch = "wasm32")]
pub fn move_to(node_id: u32, x: i32, y: i32) -> Result<(), std::string::String> {
    let state = get_shared_state();
    move_to_inner(state, node_id, x, y)
}

pub fn move_to_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
    x: i32,
    y: i32,
) -> Result<(), std::string::String> {
    let node_map = {
        let state = state.lock().unwrap();
        state.node_map.clone()
    };
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);
    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    let mut node = node.unwrap().lock().unwrap();
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

    let state = get_shared_state!(scope, State);

    match get_translate_inner(state, node_id.value() as u32) {
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
    let state = get_shared_state();
    get_translate_inner(state, node_id).and_then(|v| Ok(v.to_vec()))
}

pub fn get_translate_inner(
    state: Arc<Mutex<State>>,
    node_id: u32,
) -> Result<[i32; 2], std::string::String> {
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&node_id);

    if node.is_none() {
        return Err(format!("Cannot find node by id {}", node_id));
    }

    let node = node.unwrap().lock().unwrap();

    let &Point { x, y } = node.translate();

    Ok([x, y])
}
