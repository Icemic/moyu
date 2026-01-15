use anyhow::Result;
use moyu_core::base::Rect;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};

use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
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
}

impl Focusable for Clip {}

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
        apply_patch!(props.width => |v| self.base_mut().set_width(v), 0);
        apply_patch!(props.height => |v| self.base_mut().set_height(v), 0);
        self.base_mut().pend_update();
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
