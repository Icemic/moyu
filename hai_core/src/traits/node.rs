use hai_pal::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::{any::Any, fmt::Debug, sync::Arc};

use super::{Renderable, UpdateProps};
use crate::types::{Point, SurfaceSize, Transform};
use crate::utils::convert::{JSValue, from_js};

pub static mut NODE_ID: u32 = 0;

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

pub trait Node: NodeType + UpdateProps + Send + Sync + Debug {
    fn id(&self) -> &u32;
    fn label(&self) -> &String;

    fn anchor(&self) -> &Point;
    fn pivot(&self) -> &Point;
    fn translate(&self) -> &Point;
    fn scale(&self) -> &Point;
    fn rotation(&self) -> &f64;
    fn skew(&self) -> &Point;

    fn set_anchor(&mut self, x: f64, y: f64);
    fn set_pivot(&mut self, x: f64, y: f64);
    fn set_translate(&mut self, x: f64, y: f64);
    fn set_x(&mut self, x: f64);
    fn set_y(&mut self, y: f64);
    fn set_scale(&mut self, x: f64, y: f64);
    fn set_scale_x(&mut self, x: f64);
    fn set_scale_y(&mut self, y: f64);
    fn set_rotation(&mut self, radian: f64);
    fn set_skew(&mut self, x: f64, y: f64);
    fn set_skew_x(&mut self, x: f64);
    fn set_skew_y(&mut self, y: f64);

    fn transform(&self) -> &Transform;
    fn global_transform(&self) -> &Transform;
    fn children(&self) -> &Vec<Arc<RwLock<dyn Node>>>;

    fn node_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn try_as_renderable(&self) -> Option<&dyn Renderable> {
        None
    }
    fn try_as_renderable_mut(&mut self) -> Option<&mut dyn Renderable> {
        None
    }

    fn get_child(&self, index: usize) -> Option<Arc<RwLock<dyn Node>>>;

    fn add_child(&mut self, child: Arc<RwLock<dyn Node>>);

    fn insert_child(&mut self, index: usize, child: Arc<RwLock<dyn Node>>);

    fn insert_child_before(
        &mut self,
        before_child: Arc<RwLock<dyn Node>>,
        child: Arc<RwLock<dyn Node>>,
    );

    fn remove_child(&mut self, child: Arc<RwLock<dyn Node>>) -> Option<Arc<RwLock<dyn Node>>>;

    fn remove_child_at(&mut self, index: usize) -> Option<Arc<RwLock<dyn Node>>>;

    fn move_to(&mut self, x: f64, y: f64);

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: NodeProps = from_js(props).unwrap();

        if let Some(x) = props.x {
            self.set_x(x);
        }

        if let Some(y) = props.y {
            self.set_y(y);
        }

        if let Some(v) = props.scale {
            self.set_scale(v, v);
        }

        if let Some(x) = props.scale_x {
            self.set_scale_x(x);
        }

        if let Some(y) = props.scale_y {
            self.set_scale_y(y);
        }

        if let Some(v) = props.rotation {
            self.set_rotation(v);
        }

        if let Some(v) = props.skew {
            self.set_skew(v, v);
        }

        if let Some(x) = props.skew_x {
            self.set_skew_x(x);
        }

        if let Some(y) = props.skew_y {
            self.set_skew_y(y);
        }

        if let Some(point) = props.anchor {
            self.set_anchor(point[0], point[1]);
        }

        if let Some(point) = props.pivot {
            self.set_pivot(point[0], point[1]);
        }
    }

    fn update_transform(
        &mut self,
        parent_transform: &Transform,
        surface_size: &SurfaceSize,
        force: bool,
    );
}

impl PartialEq for dyn Node {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub trait NodeType {
    fn node_type(&self) -> &'static str;
}
