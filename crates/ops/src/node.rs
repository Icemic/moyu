use std::sync::Arc;

use anyhow::Result;
use log::{debug, warn};
#[cfg(web)]
use wasm_bindgen::prelude::wasm_bindgen;

use moyu_core::core::{NodeLock, NodeMap, NodeRef, get_core};
use moyu_core::nodes::Container;
use moyu_core::traits::NodeBaseTrait;
use moyu_core::utils::convert::JSValue;
#[cfg(native)]
use moyu_core::utils::convert::{from_js, to_js};
#[cfg(native)]
use moyu_macros::moyu_bindgen;
#[cfg(feature = "text")]
use moyu_nodes::nodes::Text;
#[cfg(feature = "video")]
use moyu_nodes::nodes::Video;
use moyu_nodes::nodes::{Animation, Backdrop, Clip, Filter, Sprite};
use moyu_pal::sync::RwLock;
#[cfg(native)]
use moyu_runtime::quickjs_rusty::{JSContext, RawJSValue};

#[inline]
pub(super) fn get_node<'a>(
    node_map: &'a NodeMap,
    node_id: u32,
) -> Result<NodeRef<'a>, std::string::String> {
    let node = node_map.get(&node_id);

    if let Some(node) = node {
        Ok(node)
    } else {
        Err(format!("Cannot find node by id {}", node_id))
    }
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn create_instance(
    node_type: std::string::String,
    label: Option<std::string::String>,
    mut props: JSValue,
) -> Result<u32, std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    let label = label.unwrap_or_default();

    let node_id;
    let node: NodeLock;
    match node_type.as_str() {
        "container" => {
            let n = Container::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        "sprite" => {
            let n = Sprite::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        "clip" => {
            let n = Clip::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        "filter" => {
            let n = Filter::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        "backdrop" => {
            let n = Backdrop::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        "animation" => {
            let n = Animation::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        #[cfg(feature = "video")]
        "video" => {
            let n = Video::new(label);
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        #[cfg(feature = "text")]
        "text" => {
            let n = Text::new(label, "");
            node_id = *n.base().id();
            node = Arc::new(RwLock::new(n));
        }
        _ => {
            return Err(format!("Unknown nodeType '{}'", node_type));
        }
    };

    node_map.insert(node_id, node.clone());

    let mut node = node.write();

    node.base_mut().update_properties(&mut props);
    node.update_properties(&mut props);

    Ok(node_id)
}

/**
 * This function will remove the node from the node_map and destroy it with all of its children
 * whose reference count is only 2 (1 for the node_map and 1 for the node itself).
 * Otherwise, the node who has more reference count will not be destroyed and rema
 */
#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn destroy_instance(node_id: u32) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    destroy_instance_recursive(&node_map, node_id)?;

    Ok(())
}

fn destroy_instance_recursive(node_map: &NodeMap, node_id: u32) -> Result<(), std::string::String> {
    if let Some(node) = node_map.get(&node_id) {
        let ref_count = Arc::strong_count(&node);
        if ref_count > 2 {
            debug!(
                "Node {} has {} references, cannot destroy it",
                node_id, ref_count
            );

            return Ok(());
        }
    } else {
        warn!("Node {} not found", node_id);
        return Ok(());
    }

    let (_, node) = node_map.remove(&node_id).unwrap();

    for child in node.read().base().children().iter() {
        let child_id = *child.read().base().id();
        destroy_instance_recursive(node_map, child_id)?;
    }

    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn add_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.base_mut().add_child(child_node);

    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn insert_child(
    node_id: u32,
    index: usize,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.base_mut().insert_child(index, child_node);

    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn insert_child_before(
    node_id: u32,
    before_node_id: u32,
    child_node_id: u32,
) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    let node = get_node(&node_map, node_id)?;
    let before_node = get_node(&node_map, before_node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let before_node = before_node.clone();
    let child_node = child_node.clone();

    node.base_mut().insert_child_before(before_node, child_node);

    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn remove_child(node_id: u32, child_node_id: u32) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    let node = get_node(&node_map, node_id)?;
    let child_node = get_node(&node_map, child_node_id)?;

    let mut node = node.write();
    let child_node = child_node.clone();

    node.base_mut().remove_child(child_node).unwrap();

    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn remove_child_at(node_id: u32, index: usize) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();

    let node = get_node(&node_map, node_id)?;

    let mut node = node.write();

    node.base_mut().remove_child_at(index).unwrap();

    Ok(())
}

#[cfg_attr(web, wasm_bindgen)]
#[cfg_attr(native, moyu_bindgen)]
pub fn update_props(node_id: u32, mut props: JSValue) -> Result<(), std::string::String> {
    let core = get_core();
    let node_map = core.node_map();
    let node = get_node(&node_map, node_id)?;
    let mut node = node.write();

    // set node props
    node.base_mut().update_properties(&mut props);

    // set props
    node.update_properties(&mut props);

    Ok(())
}
