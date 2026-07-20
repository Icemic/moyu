use anyhow::Result;
use log::warn;
use moyu_core::core::NodeLock;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Node, NodeBaseTrait, NodeEventSource};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::events::LayoutEvent;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AlignItems {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct VBoxProps {
    pub width: Patch<f32>,
    pub height: Patch<f32>,
    pub gap: Patch<f32>,
    pub padding: Patch<f32>,
    pub padding_x: Patch<f32>,
    pub padding_y: Patch<f32>,
    pub justify_content: Patch<JustifyContent>,
    pub align_items: Patch<AlignItems>,
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct HBoxProps {
    pub width: Patch<f32>,
    pub height: Patch<f32>,
    pub gap: Patch<f32>,
    pub padding: Patch<f32>,
    pub padding_x: Patch<f32>,
    pub padding_y: Patch<f32>,
    pub justify_content: Patch<JustifyContent>,
    pub align_items: Patch<AlignItems>,
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Default)]
struct LinearLayout {
    width: Option<f32>,
    height: Option<f32>,
    gap: f32,
    padding: f32,
    padding_x: Option<f32>,
    padding_y: Option<f32>,
    justify_content: JustifyContent,
    align_items: AlignItems,
    warned_unsupported_child: bool,
}

impl LinearLayout {
    fn apply_vbox_props(&mut self, props: VBoxProps) {
        self.apply_props(
            props.width,
            props.height,
            props.gap,
            props.padding,
            props.padding_x,
            props.padding_y,
            props.justify_content,
            props.align_items,
        );
    }

    fn apply_hbox_props(&mut self, props: HBoxProps) {
        self.apply_props(
            props.width,
            props.height,
            props.gap,
            props.padding,
            props.padding_x,
            props.padding_y,
            props.justify_content,
            props.align_items,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_props(
        &mut self,
        width: Patch<f32>,
        height: Patch<f32>,
        gap: Patch<f32>,
        padding: Patch<f32>,
        padding_x: Patch<f32>,
        padding_y: Patch<f32>,
        justify_content: Patch<JustifyContent>,
        align_items: Patch<AlignItems>,
    ) {
        apply_optional_size(width, &mut self.width, "width");
        apply_optional_size(height, &mut self.height, "height");
        apply_spacing(gap, &mut self.gap, "gap");
        apply_spacing(padding, &mut self.padding, "padding");
        apply_optional_spacing(padding_x, &mut self.padding_x, "paddingX");
        apply_optional_spacing(padding_y, &mut self.padding_y, "paddingY");
        justify_content.apply_to(&mut self.justify_content, JustifyContent::Start);
        align_items.apply_to(&mut self.align_items, AlignItems::Start);
    }

    fn padding_x(&self) -> f32 {
        self.padding_x.unwrap_or(self.padding)
    }

    fn padding_y(&self) -> f32 {
        self.padding_y.unwrap_or(self.padding)
    }

    fn measured_size(&mut self, children: &[NodeLock], axis: Axis) -> (f32, f32) {
        let mut main_size = 0.0_f32;
        let mut cross_size = 0.0_f32;
        let mut count = 0_usize;
        let mut has_unsupported_child = false;

        for child in children {
            let child = child.read();
            if !child.participates_in_parent_measure() {
                has_unsupported_child = true;
                continue;
            }

            let (width, height) = child.base().layout_size();
            let (main, cross) = match axis {
                Axis::Horizontal => (width, height),
                Axis::Vertical => (height, width),
            };
            main_size += main;
            cross_size = cross_size.max(cross);
            count += 1;
        }

        if count > 1 {
            main_size += self.gap * (count - 1) as f32;
        }

        if has_unsupported_child && !self.warned_unsupported_child {
            warn!(
                "Shader and ShaderSlot nodes cannot be direct VBox/HBox children; they use zero layout space"
            );
        }
        self.warned_unsupported_child = has_unsupported_child;

        let auto_width;
        let auto_height;
        match axis {
            Axis::Horizontal => {
                auto_width = self.padding_x() * 2.0 + main_size;
                auto_height = self.padding_y() * 2.0 + cross_size;
            }
            Axis::Vertical => {
                auto_width = self.padding_x() * 2.0 + cross_size;
                auto_height = self.padding_y() * 2.0 + main_size;
            }
        }

        (
            self.width.unwrap_or(auto_width),
            self.height.unwrap_or(auto_height),
        )
    }

    fn arrange(&self, children: &[NodeLock], layout_size: (f32, f32), axis: Axis) {
        let padding_x = self.padding_x();
        let padding_y = self.padding_y();
        let (container_main, container_cross, main_padding, cross_padding) = match axis {
            Axis::Horizontal => (layout_size.0, layout_size.1, padding_x, padding_y),
            Axis::Vertical => (layout_size.1, layout_size.0, padding_y, padding_x),
        };

        let mut items = Vec::with_capacity(children.len());
        let mut content_main = 0.0_f32;
        for child in children {
            let child_read = child.read();
            if !child_read.participates_in_parent_measure() {
                continue;
            }
            let (width, height) = child_read.base().layout_size();
            let (main, cross) = match axis {
                Axis::Horizontal => (width, height),
                Axis::Vertical => (height, width),
            };
            content_main += main;
            items.push((child.clone(), main, cross));
        }

        if items.len() > 1 {
            content_main += self.gap * (items.len() - 1) as f32;
        }

        let available_main = (container_main - main_padding * 2.0).max(0.0);
        let remaining = (available_main - content_main).max(0.0);
        let (main_offset, extra_gap) = match self.justify_content {
            JustifyContent::Start => (0.0, 0.0),
            JustifyContent::Center => (remaining / 2.0, 0.0),
            JustifyContent::End => (remaining, 0.0),
            JustifyContent::SpaceBetween if items.len() >= 2 => {
                (0.0, remaining / (items.len() - 1) as f32)
            }
            JustifyContent::SpaceBetween => (0.0, 0.0),
        };
        let available_cross = (container_cross - cross_padding * 2.0).max(0.0);
        let mut cursor = main_padding + main_offset;

        for (child, main, cross) in items {
            let cross_offset = if cross >= available_cross {
                0.0
            } else {
                match self.align_items {
                    AlignItems::Start => 0.0,
                    AlignItems::Center => (available_cross - cross) / 2.0,
                    AlignItems::End => available_cross - cross,
                }
            };
            let (x, y) = match axis {
                Axis::Horizontal => (cursor, cross_padding + cross_offset),
                Axis::Vertical => (cross_padding + cross_offset, cursor),
            };
            child.write().base_mut().set_layout_position(x, y);
            cursor += main + self.gap + extra_gap;
        }

        for child in children {
            let mut child = child.write();
            if !child.participates_in_parent_measure() {
                child.base_mut().set_layout_position(padding_x, padding_y);
            }
        }
    }
}

fn safe_non_negative(value: f32, name: &str) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        warn!("VBox/HBox {name} must be a finite non-negative number; using 0");
        0.0
    }
}

fn apply_optional_size(patch: Patch<f32>, target: &mut Option<f32>, name: &str) {
    match patch {
        Patch::Set(value) => *target = Some(safe_non_negative(value, name)),
        Patch::Reset => *target = None,
        Patch::Missing => {}
    }
}

fn apply_spacing(patch: Patch<f32>, target: &mut f32, name: &str) {
    match patch {
        Patch::Set(value) => *target = safe_non_negative(value, name),
        Patch::Reset => *target = 0.0,
        Patch::Missing => {}
    }
}

fn apply_optional_spacing(patch: Patch<f32>, target: &mut Option<f32>, name: &str) {
    match patch {
        Patch::Set(value) => *target = Some(safe_non_negative(value, name)),
        Patch::Reset => *target = None,
        Patch::Missing => {}
    }
}

#[derive(Debug, Default, Node)]
pub struct VBox {
    layout: LinearLayout,
    #[base]
    node_base: NodeBase,
}

impl VBox {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
            ..Default::default()
        }
    }
}

impl Node for VBox {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>> {
        Ok(Box::new(Self::new(label.unwrap_or_default())))
    }

    fn node_type(&self) -> &'static str {
        "vbox"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        match from_js::<VBoxProps>(props) {
            Ok(props) => self.layout.apply_vbox_props(props),
            Err(error) => {
                warn!("Failed to convert JSValue to VBoxProps: {error:?}");
                return;
            }
        }
        self.base_mut().pend_update();
    }

    fn measure(&mut self) {
        let children = self.base().children().clone();
        let size = self.layout.measured_size(&children, Axis::Vertical);
        if self.base().layout_size() != size {
            self.base_mut().set_layout_size(size.0, size.1);
            self.send_event(LayoutEvent {
                width: size.0,
                height: size.1,
            });
        }
    }

    fn arrange(&mut self) {
        let children = self.base().children().clone();
        self.layout
            .arrange(&children, self.base().layout_size(), Axis::Vertical);
    }
}

impl NodeEventSource for VBox {
    type Event = LayoutEvent;
}

#[derive(Debug, Default, Node)]
pub struct HBox {
    layout: LinearLayout,
    #[base]
    node_base: NodeBase,
}

impl HBox {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
            ..Default::default()
        }
    }
}

impl Node for HBox {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>> {
        Ok(Box::new(Self::new(label.unwrap_or_default())))
    }

    fn node_type(&self) -> &'static str {
        "hbox"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        match from_js::<HBoxProps>(props) {
            Ok(props) => self.layout.apply_hbox_props(props),
            Err(error) => {
                warn!("Failed to convert JSValue to HBoxProps: {error:?}");
                return;
            }
        }
        self.base_mut().pend_update();
    }

    fn measure(&mut self) {
        let children = self.base().children().clone();
        let size = self.layout.measured_size(&children, Axis::Horizontal);
        if self.base().layout_size() != size {
            self.base_mut().set_layout_size(size.0, size.1);
            self.send_event(LayoutEvent {
                width: size.0,
                height: size.1,
            });
        }
    }

    fn arrange(&mut self) {
        let children = self.base().children().clone();
        self.layout
            .arrange(&children, self.base().layout_size(), Axis::Horizontal);
    }
}

impl NodeEventSource for HBox {
    type Event = LayoutEvent;
}
