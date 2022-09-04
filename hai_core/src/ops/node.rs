use hai_js_runtime::{prelude::*, *};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::{
    nodes::{Container, Sprite},
    state::State,
    types::Point,
};

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

    let state = get_shared_state!(scope, State);
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let mut node_map = node_map.lock().unwrap();

    let label = get_from_v8_object!(scope, props, "label");
    let label = try_from_option_value_or_throw_exception!(scope, String, label);

    let label = label
        .and_then(|v| Some(v.to_rust_string_lossy(scope)))
        .unwrap_or_default();

    let mut node_id = 0;
    match node_type.as_str() {
        "node" => {
            let n = Container::new(label, Default::default(), Default::default());
            node_id = n.id;
            node_map.insert(n.id, Arc::new(Mutex::new(n)));
        }
        "sprite" => {
            let src = get_from_v8_object!(scope, props, "src");
            let src = try_from_option_value_or_throw_exception!(scope, String, src);
            let src = src
                .and_then(|v| Some(v.to_rust_string_lossy(scope)))
                .unwrap_or_default();

            let mut resource_manager = state.resource_manager.lock().unwrap();
            let texture = resource_manager.get_texture(src.clone());
            let n = Sprite::new(src, texture);
            node_id = n.id;
            node_map.insert(n.id, Arc::new(Mutex::new(n)));
        }
        _ => {
            throw_exception!(scope, format!("Unknown nodeType '{}'", node_type));
        }
    };

    // call callback function to return node id
    if callback.is_some() {
        let global_this = scope.get_current_context().global(scope);
        let callback = callback.unwrap();
        let node_id = node_id.into_v8(scope);
        let node_id: Local<Value> = node_id.into();
        callback.call(scope, global_this.into(), &[node_id]);
    }
}

pub fn add_child(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let child_node_id = get_from_v8_array!(scope, args, 1);

    check_exist!(scope, node_id);
    check_exist!(scope, child_node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let child_node_id = try_from_value_or_throw_exception!(scope, Number, child_node_id);

    let state = get_shared_state!(scope, State);
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));
    let child_node = node_map.get(&(child_node_id.value() as u32));

    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    if child_node.is_none() {
        throw_exception!(
            scope,
            format!("Cannot find node by id {}", child_node_id.value())
        );
        return;
    }

    let mut node = node.unwrap().lock().unwrap();
    let child_node = child_node.unwrap().clone();

    node.add_child(child_node);
}

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
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));
    let child_node = node_map.get(&(child_node_id.value() as u32));

    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    if child_node.is_none() {
        throw_exception!(
            scope,
            format!("Cannot find node by id {}", child_node_id.value())
        );
        return;
    }

    let mut node = node.unwrap().lock().unwrap();
    let index = index.value() as usize;
    let child_node = child_node.unwrap().clone();

    node.insert_child(index, child_node);
}

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
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));
    let before_node = node_map.get(&(before_node_id.value() as u32));
    let child_node = node_map.get(&(child_node_id.value() as u32));

    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    if before_node.is_none() {
        throw_exception!(
            scope,
            format!("Cannot find node by id {}", before_node_id.value())
        );
        return;
    }

    if child_node.is_none() {
        throw_exception!(
            scope,
            format!("Cannot find node by id {}", child_node_id.value())
        );
        return;
    }

    let mut node = node.unwrap().lock().unwrap();
    let before_node = before_node.unwrap().clone();
    let child_node = child_node.unwrap().clone();

    node.insert_child_before(before_node, child_node);
}

pub fn remove_child(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let child_node_id = get_from_v8_array!(scope, args, 2);

    check_exist!(scope, node_id);
    check_exist!(scope, child_node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let child_node_id = try_from_value_or_throw_exception!(scope, Number, child_node_id);

    let state = get_shared_state!(scope, State);
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));
    let child_node = node_map.get(&(child_node_id.value() as u32));

    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    if child_node.is_none() {
        throw_exception!(
            scope,
            format!("Cannot find node by id {}", child_node_id.value())
        );
        return;
    }

    let mut node = node.unwrap().lock().unwrap();
    let child_node = child_node.unwrap().clone();

    node.remove_child(child_node).unwrap();
}

pub fn remove_child_at(scope: &mut HandleScope, args: Local<Array>, _: Option<Local<Function>>) {
    let node_id = get_from_v8_array!(scope, args, 0);
    let index = get_from_v8_array!(scope, args, 1);

    check_exist!(scope, node_id);
    check_exist!(scope, index);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);
    let index = try_from_value_or_throw_exception!(scope, Number, index);

    let state = get_shared_state!(scope, State);
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));

    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    let mut node = node.unwrap().lock().unwrap();
    let index = index.value() as usize;

    node.remove_child_at(index).unwrap();
}

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

    let node_map = {
        let state = get_shared_state!(scope, State);
        let state = state.lock().unwrap();
        state.node_map.clone()
    };
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));
    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    let mut node = node.unwrap().lock().unwrap();
    node.move_to(x.value() as i32, y.value() as i32);
}

pub fn get_translate(
    scope: &mut HandleScope,
    args: Local<Array>,
    callback: Option<Local<Function>>,
) {
    let node_id = get_from_v8_array!(scope, args, 0);

    check_exist!(scope, node_id);

    let node_id = try_from_value_or_throw_exception!(scope, Number, node_id);

    let state = get_shared_state!(scope, State);
    let state = state.lock().unwrap();
    let node_map = state.node_map.clone();
    let node_map = node_map.lock().unwrap();

    let node = node_map.get(&(node_id.value() as u32));

    if node.is_none() {
        throw_exception!(scope, format!("Cannot find node by id {}", node_id.value()));
        return;
    }

    let node = node.unwrap().lock().unwrap();

    let &Point { x, y } = node.translate();

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
