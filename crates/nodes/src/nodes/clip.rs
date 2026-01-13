use moyu_core::base::Rect;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};

use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
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

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct ClipProps {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Node for Clip {
    #[inline]
    fn node_type(&self) -> &'static str {
        "clip"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: ClipProps = from_js(props).unwrap();

        if let Some(width) = props.width {
            self.base_mut().set_width(width);
        }

        if let Some(height) = props.height {
            self.base_mut().set_height(height);
        }

        self.base_mut().pend_update();
    }
}
