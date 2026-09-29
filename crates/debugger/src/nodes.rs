//! Node views for `engine:tree` and `engine:props`.
//!
//! Both read the node map, which is why they answer on whatever thread the request
//! arrived on instead of the render thread.

use moyu_core::base::{Bound, Point};
use moyu_core::core::{NodeLock, try_get_core};
use moyu_core::traits::Node;
use serde::Serialize;

/// One node in a tree listing.
///
/// Children are listed without their own children unless a depth asks for more, so a
/// client that only needs the next level reads it by asking for a child's id instead
/// of receiving the whole subtree.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSummary {
    pub id: u32,
    /// [`Node::node_type`].
    #[serde(rename = "type")]
    pub node_type: &'static str,
    pub label: String,
    pub visible: bool,
    pub children: Vec<NodeSummary>,
}

/// A node together with the properties it received and the values the engine derived.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDetails {
    pub id: u32,
    #[serde(rename = "type")]
    pub node_type: &'static str,
    /// Last property object received from JS; absent when none was recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<serde_json::Value>,
    pub derived: Derived,
}

/// Values the engine computed for a node, as opposed to the ones JS passed in.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Derived {
    pub visible: bool,
    pub opacity: f32,
    pub global_opacity: f32,
    pub interactive: bool,
    pub z_index: i32,
    pub position: Vector2,
    pub scale: Vector2,
    pub rotation: f32,
    pub skew: Vector2,
    pub size: Size,
    pub intrinsic_size: Size,
    pub bounds: Bounds,
}

#[derive(Serialize)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Serialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Describe a node together with `depth` levels of its children.
pub(super) fn tree(node: &dyn Node, depth: u32) -> NodeSummary {
    let base = node.base();

    NodeSummary {
        children: if depth == 0 {
            Vec::new()
        } else {
            base.children()
                .iter()
                .map(|child| tree(&**child.read(), depth - 1))
                .collect()
        },
        id: *base.id(),
        node_type: node.node_type(),
        label: base.label().clone(),
        visible: base.visible(),
    }
}

/// Describe a node's properties and derived state.
pub(super) fn details(node: &dyn Node, props: Option<serde_json::Value>) -> NodeDetails {
    let base = node.base();

    NodeDetails {
        id: *base.id(),
        node_type: node.node_type(),
        props,
        derived: Derived {
            visible: base.visible(),
            opacity: *base.opacity(),
            global_opacity: *base.global_opacity(),
            interactive: base.interactive(),
            z_index: base.z_index(),
            position: vector(base.translate()),
            scale: vector(base.scale()),
            rotation: *base.rotation(),
            skew: vector(base.skew()),
            size: size(base.layout_size()),
            intrinsic_size: size(base.intrinsic_size()),
            bounds: bounds(base.content_bounds()),
        },
    }
}

/// Find a node in the map. Returns `None` for an id that is not there, which happens
/// when a client reads a node that has just been destroyed.
pub(super) fn find(node_id: u32) -> Option<NodeLock> {
    try_get_core()?.node_map().get(&node_id).map(|node| node.clone())
}

fn vector(point: &Point) -> Vector2 {
    Vector2 {
        x: point.x,
        y: point.y,
    }
}

fn size((width, height): (f32, f32)) -> Size {
    Size { width, height }
}

fn bounds(bound: &Bound) -> Bounds {
    Bounds {
        x: bound.min_x(),
        y: bound.min_y(),
        width: bound.width(),
        height: bound.height(),
    }
}
