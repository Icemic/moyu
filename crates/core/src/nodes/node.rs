use csscolorparser::Color;
use hai_pal::sync::RwLock;
use log::warn;
use std::sync::Arc;

use crate::base::*;
use crate::traits::Node;
use crate::utils::constants::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH};
use crate::utils::convert::{from_js, JSValue};
use crate::utils::dispatch_event::{dispatch_event, HaiEvent, HaiEventKind};

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
    /// visible to render
    visible: bool,
    /// tint color
    tint: Color,
    /// opcaity, aka alpha. Ranges from 0.0 to 1.0.
    opacity: f32,
    /// opacity that has been multiplied with parent
    global_opacity: f32,
    /// if this node will response to user input, will affect itself and all children
    interactive: bool,
    /// cursor style
    cursor: HaiCursor,
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
            visible: true,
            tint: Color::new(1.0, 1.0, 1.0, 1.0),
            opacity: 1.0,
            global_opacity: 1.0,
            interactive: true,
            cursor: HaiCursor::default(),

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
    pub fn visible(&self) -> bool {
        self.visible
    }
    #[inline]
    pub fn tint(&self) -> &Color {
        &self.tint
    }
    #[inline]
    pub fn opacity(&self) -> &f32 {
        &self.opacity
    }
    #[inline]
    pub fn global_opacity(&self) -> &f32 {
        &self.global_opacity
    }
    #[inline]
    pub fn interactive(&self) -> bool {
        self.interactive
    }
    #[inline]
    pub fn cursor(&self) -> &HaiCursor {
        &self.cursor
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
    #[inline]
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_tint(&mut self, color: Color) {
        self.tint = color;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_interactive(&mut self, interactive: bool) {
        self.interactive = interactive;
    }
    #[inline]
    pub fn set_cursor(&mut self, cursor: HaiCursor) {
        self.cursor = cursor;
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

    #[inline]
    pub fn update_properties(&mut self, props: &mut JSValue) {
        let props: NodeProps = match from_js(props) {
            Ok(v) => v,
            Err(err) => {
                warn!("Failed to convert JSValue to NodeProps: {:?}", err);
                return;
            }
        };

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

        if let Some(visible) = props.visible {
            self.set_visible(visible);
        }

        if let Some(tint) = props.tint {
            self.set_tint(tint);
        }

        if let Some(opacity) = props.opacity {
            self.set_opacity(opacity);
        }

        if let Some(interactive) = props.interactive {
            self.set_interactive(interactive);
        }

        if let Some(cursor) = props.cursor {
            self.set_cursor(cursor);
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
    pub fn update(&mut self, parent: &Self, _: &SurfaceSize, force: bool) {
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

            // use logical size to calculate transform matrix, so that the transform matrix will not be affected by scale ratio
            let tx = tx / VIEWPORT_WIDTH;
            let ty = ty / VIEWPORT_HEIGHT;

            self.transform.x_axis.x = a;
            self.transform.x_axis.y = b;
            self.transform.y_axis.x = c;
            self.transform.y_axis.y = d;
            self.transform.z_axis.x = tx;
            self.transform.z_axis.y = ty;

            // refresh global transform matrix
            let mut global_transform = *parent.global_transform();
            global_transform.multiply(self.transform);
            self.global_transform = global_transform;

            // refresh global opacity
            self.global_opacity = self.opacity * parent.global_opacity;

            self._current_update_id = self._update_id;
            self._need_update_vertices = true;

            // mark all children pend update
            for child in self.children.iter() {
                let mut child = child.write();
                child.base_mut().pend_update();
            }
        }
    }
}

use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
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
    pub visible: Option<bool>,
    pub tint: Option<Color>,
    pub opacity: Option<f32>,
    pub interactive: Option<bool>,
    pub cursor: Option<HaiCursor>,
}

impl Drop for NodeBase {
    fn drop(&mut self) {
        dispatch_event(HaiEvent {
            kind: HaiEventKind::NodeDestroyed,
            target_id: self.id,
            bubble_target_ids: vec![],
            ..Default::default()
        });
    }
}
