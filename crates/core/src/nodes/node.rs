use hai_pal::sync::RwLock;
use log::warn;
use std::sync::Arc;

use crate::base::*;
use crate::traits::Node;
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::{from_js, JSValue};

pub static mut NODE_ID: u32 = 0;

#[derive(Debug, Default)]
pub struct NodeBase {
    /// Debug label
    label: String,
    /// id
    id: u32,
    /// anchor point
    anchor: Point,
    /// pivot point
    pivot: Point,
    /// translate relative to parent
    translate: Point,
    /// scale relative to parent
    scale: Point,
    /// rotation relative to parent
    rotation: f32,
    /// skew relative to parent
    skew: Point,
    /// for update transform dirty check
    _update_id: u32,
    _current_update_id: u32,
    _need_update_vertices: bool,
    /// transform matrix relative to parent
    transform: Transform,
    /// transform matrix relative to global
    global_transform: Transform,
    /// children
    children: Vec<Arc<RwLock<dyn Node>>>,
}

impl NodeBase {
    pub fn new(label: String) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };
        Self {
            label,
            id,
            anchor: Point::default(),
            pivot: Point::default(),
            translate: Point::default(),
            scale: Point::one(),
            rotation: 0.,
            skew: Point::default(),

            _update_id: 0,
            _current_update_id: 0,
            _need_update_vertices: true,

            transform: Transform::default(),
            global_transform: Transform::default(),
            children: vec![],
        }
    }
}

impl NodeBase {
    pub fn node_type(&self) -> &'static str {
        unreachable!("Should not call Node::node_type, use NodeType::node_type(&node) instead.");
    }

    #[inline]
    pub fn pend_update(&mut self) {
        self._update_id += 1;
    }

    /// pop vertices update flag, returns the current flag value, and set it to false
    #[inline]
    pub fn pop_update_vertices(&mut self) -> bool {
        let flag = self._need_update_vertices;
        self._need_update_vertices = false;
        flag
    }

    #[inline]
    pub fn id(&self) -> &u32 {
        &self.id
    }

    #[inline]
    pub fn label(&self) -> &String {
        &self.label
    }

    #[inline]
    pub fn anchor(&self) -> &Point {
        &self.anchor
    }
    #[inline]
    pub fn pivot(&self) -> &Point {
        &self.pivot
    }
    #[inline]
    pub fn translate(&self) -> &Point {
        &self.translate
    }
    #[inline]
    pub fn scale(&self) -> &Point {
        &self.scale
    }
    #[inline]
    pub fn rotation(&self) -> &f32 {
        &self.rotation
    }
    #[inline]
    pub fn skew(&self) -> &Point {
        &self.skew
    }

    #[inline]
    pub fn set_anchor(&mut self, x: f32, y: f32) {
        self.anchor.x = x;
        self.anchor.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_pivot(&mut self, x: f32, y: f32) {
        self.pivot.x = x;
        self.pivot.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_translate(&mut self, x: f32, y: f32) {
        self.translate.x = x;
        self.translate.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_x(&mut self, x: f32) {
        self.translate.x = x;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_y(&mut self, y: f32) {
        self.translate.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_scale(&mut self, x: f32, y: f32) {
        self.scale.x = x;
        self.scale.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_scale_x(&mut self, x: f32) {
        self.scale.x = x;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_scale_y(&mut self, y: f32) {
        self.scale.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_rotation(&mut self, radian: f32) {
        self.rotation = radian;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_skew(&mut self, x: f32, y: f32) {
        self.skew.x = x;
        self.skew.y = y;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_skew_x(&mut self, x: f32) {
        self.skew.x = x;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_skew_y(&mut self, y: f32) {
        self.skew.y = y;
        self._update_id += 1;
    }

    pub fn transform(&self) -> &Transform {
        &self.transform
    }
    pub fn global_transform(&self) -> &Transform {
        &self.global_transform
    }
    pub fn children(&self) -> &Vec<Arc<RwLock<dyn Node>>> {
        &self.children
    }

    // pub fn as_any(&self) -> &dyn Any {
    //     self
    // }

    // pub fn as_any_mut(&mut self) -> &mut dyn Any {
    //     self
    // }

    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    #[inline]
    pub fn update_properties(&mut self, props: &mut JSValue) {
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

    #[inline]
    pub fn get_child(&self, index: usize) -> Option<Arc<RwLock<dyn Node>>> {
        if let Some(child) = self.children.get(index) {
            return Some(child.clone());
        }
        None
    }

    #[inline]
    pub fn add_child(&mut self, child: Arc<RwLock<dyn Node>>) {
        self.children.push(child);
    }

    #[inline]
    pub fn insert_child(&mut self, index: usize, child: Arc<RwLock<dyn Node>>) {
        self.children.insert(index, child);
    }

    #[inline]
    pub fn insert_child_before(
        &mut self,
        before_child: Arc<RwLock<dyn Node>>,
        child: Arc<RwLock<dyn Node>>,
    ) {
        let index = self.children.iter().position(|item| {
            let l = item.read();
            let r = before_child.read();
            *l == *r
        });
        if index.is_none() {
            warn!("Cannot insert child before another one because the another child does not present in current children.");
        }
        self.children.insert(index.unwrap_or(0), child);
    }

    #[inline]
    pub fn remove_child(&mut self, child: Arc<RwLock<dyn Node>>) -> Option<Arc<RwLock<dyn Node>>> {
        if let Some(index) = self.children.iter().position(|item| {
            let l = item.read();
            let r = child.read();
            *l == *r
        }) {
            return Some(self.children.remove(index));
        }
        None
    }

    #[inline]
    pub fn remove_child_at(&mut self, index: usize) -> Option<Arc<RwLock<dyn Node>>> {
        if index < self.children.len() {
            return Some(self.children.remove(index));
        }
        None
    }

    #[inline]
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.set_translate(x, y);
    }

    #[inline]
    pub fn update_transform(
        &mut self,
        parent_transform: &Transform,
        surface_size: &SurfaceSize,
        force: bool,
    ) {
        if force || self._update_id != self._current_update_id {
            let x = self.translate.x;
            let y = self.translate.y;
            let rotation = self.rotation;
            let scale_x = self.scale.x;
            let scale_y = self.scale.y;
            let skew_x = self.skew.x;
            let skew_y = self.skew.y;
            let pivot_x = self.pivot.x;
            let pivot_y = self.pivot.y;

            let a = (rotation + skew_y).cos() * scale_x;
            let b = (rotation + skew_y).sin() * scale_x;
            let c = -(rotation - skew_x).sin() * scale_y;
            let d = (rotation - skew_x).cos() * scale_y;
            let tx = x - ((pivot_x * a) + (pivot_y * c));
            let ty = y - ((pivot_x * b) + (pivot_y * d));

            let (logical_width, logical_height) = surface_size.logical_size_f32();

            // use logical size to calculate transform matrix, so that the transform matrix will not be affected by scale ratio
            let tx = tx / logical_width * 2.;
            let ty = ty / logical_height * 2.;

            self.transform.matrix2.x_axis.x = a;
            self.transform.matrix2.x_axis.y = b;
            self.transform.matrix2.y_axis.x = c;
            self.transform.matrix2.y_axis.y = d;
            self.transform.translation.x = tx;
            self.transform.translation.y = ty;

            // refresh global transform matrix
            let mut global_transform = parent_transform.clone();
            global_transform.multiply(self.transform);
            self.global_transform = global_transform;

            self._current_update_id = self._update_id;
            self._need_update_vertices = true;
        }
    }
}

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeProps {
    pub anchor: Option<[f32; 2]>,
    pub pivot: Option<[f32; 2]>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub scale: Option<f32>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub rotation: Option<f32>,
    pub skew: Option<f32>,
    pub skew_x: Option<f32>,
    pub skew_y: Option<f32>,
}
