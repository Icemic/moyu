use anyhow::Result;
use moyu_core::base::Rect;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};

use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, FocusablePayload, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use ts_rs::TS;

#[derive(Debug, Default, Node)]
pub struct Clip {
    pub rect: Option<Rect>,

    #[base]
    node_base: NodeBase,
}

impl Clip {
    pub fn new(label: String) -> Self {
        Self {
            rect: None,
            node_base: NodeBase::new(label),
        }
    }

    fn contains_bounds(&self, x: f32, y: f32) -> bool {
        let width = *self.base().width() as f32;
        let height = *self.base().height() as f32;

        width > 0.0 && height > 0.0 && x >= 0.0 && x <= width && y >= 0.0 && y <= height
    }
}

impl Focusable for Clip {
    fn contains(&self, x: f32, y: f32, _: &FocusablePayload) -> bool {
        self.contains_bounds(x, y)
    }

    fn contains_children(&self, x: f32, y: f32, _: &FocusablePayload) -> bool {
        self.contains_bounds(x, y)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct ClipProps {
    pub width: Patch<u32>,
    pub height: Patch<u32>,
}

impl Node for Clip {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "clip"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: ClipProps = from_js(props).unwrap();
        apply_patch!(props.width => |v| self.base_mut().set_intrinsic_width(v as f32), 0);
        apply_patch!(props.height => |v| self.base_mut().set_intrinsic_height(v as f32), 0);
        self.base_mut().pend_update();
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
