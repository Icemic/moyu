use anyhow::Result;
#[cfg(not(feature = "web"))]
use hai_js_runtime::{prelude::*, *};
use hai_macros::hai_bindgen;
use hai_pal::sync::{RwLock, RwLockReadGuard};
#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(feature = "web")]
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::core::get_core;
use crate::traits::{Node, UpdateProps};
use crate::utils::convert::{from_js, to_js, JSValue};
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

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn create_instance(
    node_type: std::string::String,
    label: Option<std::string::String>,
    mut props: JSValue,
) -> Result<u32, std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();
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

    drop(core);

    let mut node = node.write();

    Node::update_properties(&mut *node, &mut props);
    UpdateProps::update_properties(&mut *node, &mut props);

    Ok(node_id)
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn add_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.add_child(child_node);

    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn insert_child(
    node_id: u32,
    index: usize,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.insert_child(index, child_node);

    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn insert_child_before(
    node_id: u32,
    before_node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();
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

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn remove_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.remove_child(child_node).unwrap();

    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn remove_child_at(node_id: u32, index: usize) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();
    let node_map = node_map.read();

    let node = get_node(&node_map, node_id)?;

    let mut node = node.write();

    node.remove_child_at(index).unwrap();

    Ok(())
}

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn move_to(node_id: u32, x: f64, y: f64) -> Result<(), std::string::String> {
    let node_map = {
        let core = get_core();

        core.node_map.clone()
    };
    let node_map = node_map.read();
    let node = get_node(&node_map, node_id)?;

    let mut node = node.write();
    node.move_to(x, y);

    Ok(())
}

// #[cfg_attr(feature = "web", wasm_bindgen)]
// #[cfg_attr(not(feature = "web"), hai_bindgen)]
// pub fn get_translate(node_id: u32) -> Result<[f64; 2], std::string::String> {
//     let core = get_core();
//     let node_map = core.node_map.clone();
//     let node_map = node_map.read();

//     let node = get_node(&node_map, node_id)?;
//     let node = node.write();

//     let &Point { x, y } = node.translate();

//     Ok([x, y])
// }

#[cfg_attr(feature = "web", wasm_bindgen)]
#[cfg_attr(not(feature = "web"), hai_bindgen)]
pub fn update_props(node_id: u32, mut props: JSValue) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map.clone();

    drop(core);

    let node_map = node_map.read();
    let node = get_node(&node_map, node_id)?;
    let mut node = node.write();

    // set node props
    Node::update_properties(&mut *node, &mut props);

    // set props
    UpdateProps::update_properties(&mut *node, &mut props);

    Ok(())
}
