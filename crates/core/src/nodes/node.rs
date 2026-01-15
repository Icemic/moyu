use std::sync::Arc;

use csscolorparser::Color;
use log::warn;
use ts_rs::TS;

use crate::apply_patch;
use crate::base::*;
use crate::core::NodeLock;
use crate::events::NodeEvent;
use crate::utils::convert::{JSValue, from_js};
use crate::utils::dispatch_event::dispatch_event;
use crate::utils::patch::Patch;

pub static mut NODE_ID: u32 = 0;

#[derive(Debug, Default)]
pub struct NodeBase {
    /// Debug label
    label: String,
    /// id
    id: u32,
    /// calculated width of the node
    width: u32,
    /// calculated height of the node
    height: u32,
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
    cursor: MoyuCursor,
    /// AABB bounds of the node, relative to itself
    bounds: Bound,
    /// AABB bounds of the node, relative to global(stage)
    global_bounds: Bound,
    /// for update transform dirty check
    _update_id: u32,
    _current_update_id: u32,
    _need_update_vertices: bool,
    /// transform matrix relative to parent
    transform: Transform,
    /// transform matrix relative to global
    global_transform: Transform,
    /// children
    children: Vec<NodeLock>,
}

impl NodeBase {
    pub fn new(label: String) -> Self {
        let id = unsafe {
            let id = NODE_ID;
            NODE_ID += 1;
            id
        };
        Self {
            label,
            id,
            width: 0,
            height: 0,
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
            cursor: MoyuCursor::default(),
            bounds: Bound::default(),
            global_bounds: Bound::default(),

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

    #[inline]
    pub fn cancel_update(&mut self) {
        self._update_id = self._current_update_id;
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
    pub fn width(&self) -> &u32 {
        &self.width
    }
    #[inline]
    pub fn height(&self) -> &u32 {
        &self.height
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
    pub fn cursor(&self) -> &MoyuCursor {
        &self.cursor
    }

    #[inline]
    pub fn bounds(&self) -> &Bound {
        &self.bounds
    }

    #[inline]
    pub fn global_bounds(&self) -> &Bound {
        &self.global_bounds
    }

    #[inline]
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_width(&mut self, width: u32) {
        self.width = width;
        self._update_id += 1;
    }
    #[inline]
    pub fn set_height(&mut self, height: u32) {
        self.height = height;
        self._update_id += 1;
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
    pub fn set_cursor(&mut self, cursor: MoyuCursor) {
        self.cursor = cursor;
    }

    pub fn transform(&self) -> &Transform {
        &self.transform
    }
    pub fn global_transform(&self) -> &Transform {
        &self.global_transform
    }
    pub fn children(&self) -> &Vec<NodeLock> {
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

        apply_patch!(props.label => self.label, String::new());
        apply_patch!(props.x => |v| self.set_x(v), 0.0);
        apply_patch!(props.y => |v| self.set_y(v), 0.0);
        apply_patch!(props.scale => |v| self.set_scale(v, v), 1.0);
        apply_patch!(props.scale_x => |v| self.set_scale_x(v), 1.0);
        apply_patch!(props.scale_y => |v| self.set_scale_y(v), 1.0);
        apply_patch!(props.rotation => |v| self.set_rotation(v), 0.0);
        apply_patch!(props.skew => |v| self.set_skew(v, v), 0.0);
        apply_patch!(props.skew_x => |v| self.set_skew_x(v), 0.0);
        apply_patch!(props.skew_y => |v| self.set_skew_y(v), 0.0);
        apply_patch!(props.anchor => |point| self.set_anchor(point[0], point[1]), [0.0, 0.0]);
        apply_patch!(props.pivot => |point| self.set_pivot(point[0], point[1]), [0.0, 0.0]);
        apply_patch!(props.visible => |v| self.set_visible(v), true);
        apply_patch!(props.tint => |v| self.set_tint(v), Color::new(1.0, 1.0, 1.0, 1.0));
        apply_patch!(props.opacity => |v| self.set_opacity(v), 1.0);
        apply_patch!(props.interactive => |v| self.set_interactive(v), true);
        apply_patch!(props.cursor => |v| self.set_cursor(v), MoyuCursor::default());
    }

    #[inline]
    pub fn get_child(&self, index: usize) -> Option<NodeLock> {
        if let Some(child) = self.children.get(index) {
            return Some(child.clone());
        }
        None
    }

    #[inline]
    pub fn add_child(&mut self, child: NodeLock) {
        self.children.push(child);
    }

    #[inline]
    pub fn insert_child(&mut self, index: usize, child: NodeLock) {
        self.children.insert(index, child);
    }

    #[inline]
    pub fn insert_child_before(&mut self, before_child: NodeLock, child: NodeLock) {
        let index = self
            .children
            .iter()
            .position(|item| Arc::ptr_eq(item, &before_child));
        if index.is_none() {
            warn!(
                "Cannot insert child before another one because the another child does not present in current children."
            );
        }
        self.children.insert(index.unwrap_or(0), child);
    }

    #[inline]
    pub fn remove_child(&mut self, child: NodeLock) -> Option<NodeLock> {
        if let Some(index) = self
            .children
            .iter()
            .position(|item| Arc::ptr_eq(item, &child))
        {
            return Some(self.children.remove(index));
        }
        None
    }

    #[inline]
    pub fn remove_child_at(&mut self, index: usize) -> Option<NodeLock> {
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
    pub fn calculate_bounds(&mut self) {
        let mut bounds = Bound::new(0.0, 0.0, self.width as f32, self.height as f32);
        for child in &self.children {
            let child_read = child.read();
            let child_base = child_read.base();
            let child_bounds = child_base.bounds().transform(child_base.transform());
            bounds = bounds.union(&child_bounds);
        }
        self.bounds = bounds;
        self.global_bounds = bounds.transform(&self.global_transform);
    }

    #[inline]
    pub fn update(&mut self, parent: &Self, force: bool) {
        if force || self._update_id != self._current_update_id {
            let x = self.translate.x;
            let y = self.translate.y;
            let rotation = self.rotation;
            let scale_x = self.scale.x;
            let scale_y = self.scale.y;
            let skew_x = self.skew.x;
            let skew_y = self.skew.y;

            let bounds = &self.bounds;

            let pivot_x = bounds.min_x() + self.pivot.x * bounds.width();
            let pivot_y = bounds.min_y() + self.pivot.y * bounds.height();
            // FIXME:
            // Cannot use parent_bounds to calculate anchor, because anchor will in turn affect
            // the calculation of parent_bounds, eventually forming a non-converging loop.
            //
            // However, using self width/height is not perfect either, because children may
            // overflow the parent bounds, causing anchor misplacement.
            //
            let anchor_x = self.anchor.x * parent.width as f32;
            let anchor_y = self.anchor.y * parent.height as f32;

            let a = (rotation + skew_y).cos() * scale_x;
            let b = (rotation + skew_y).sin() * scale_x;
            let c = -(rotation - skew_x).sin() * scale_y;
            let d = (rotation - skew_x).cos() * scale_y;
            let tx = x - ((pivot_x * a) + (pivot_y * c)) + anchor_x;
            let ty = y - ((pivot_x * b) + (pivot_y * d)) + anchor_y;

            self.transform.x_axis.x = a;
            self.transform.x_axis.y = b;
            self.transform.y_axis.x = c;
            self.transform.y_axis.y = d;
            self.transform.translation.x = tx;
            self.transform.translation.y = ty;

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
#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct NodeProps {
    pub label: Patch<String>,
    pub anchor: Patch<[f32; 2]>,
    pub pivot: Patch<[f32; 2]>,
    pub x: Patch<f32>,
    pub y: Patch<f32>,
    pub scale: Patch<f32>,
    pub scale_x: Patch<f32>,
    pub scale_y: Patch<f32>,
    pub rotation: Patch<f32>,
    pub skew: Patch<f32>,
    pub skew_x: Patch<f32>,
    pub skew_y: Patch<f32>,
    pub visible: Patch<bool>,
    #[ts(type = "string", optional)]
    pub tint: Patch<Color>,
    pub opacity: Patch<f32>,
    pub interactive: Patch<bool>,
    pub cursor: Patch<MoyuCursor>,
}

impl Drop for NodeBase {
    fn drop(&mut self) {
        dispatch_event(NodeEvent::Destory { target_id: self.id });
    }
}
