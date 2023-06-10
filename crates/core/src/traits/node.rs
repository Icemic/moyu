use serde::{Deserialize, Serialize};
use std::{any::Any, fmt::Debug};

use super::{Renderable, UpdateProps};
use crate::nodes::NodeBase;
use crate::types::{SurfaceSize, Transform};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeProps {
    pub anchor: Option<[f64; 2]>,
    pub pivot: Option<[f64; 2]>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub scale: Option<f64>,
    pub scale_x: Option<f64>,
    pub scale_y: Option<f64>,
    pub rotation: Option<f64>,
    pub skew: Option<f64>,
    pub skew_x: Option<f64>,
    pub skew_y: Option<f64>,
}

pub trait Node: NodeType + UpdateProps + GetNodeBase + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn try_as_renderable(&self) -> Option<&dyn Renderable> {
        None
    }
    fn try_as_renderable_mut(&mut self) -> Option<&mut dyn Renderable> {
        None
    }

    fn update_transform(
        &mut self,
        parent_transform: &Transform,
        surface_size: &SurfaceSize,
        force: bool,
    ) {
        self.base_mut()
            .update_transform(parent_transform, surface_size, force)
    }
}

impl PartialEq for dyn Node {
    fn eq(&self, other: &Self) -> bool {
        self.base().id() == other.base().id()
    }
}

pub trait NodeType {
    fn node_type(&self) -> &'static str;
}

pub trait GetNodeBase {
    fn base(&self) -> &NodeBase;
    fn base_mut(&mut self) -> &mut NodeBase;
}
