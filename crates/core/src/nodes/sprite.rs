use arc_swap::ArcSwapOption;
use hai_macros::Node;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::Buffer;

use crate::core::get_core;
use crate::resource::TextureId;
use crate::traits::{Focusable, Node, NodeBaseTrait};
use crate::types::Vertex;
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::{from_js, JSValue};

use super::NodeBase;

// #[node]
#[derive(Debug, Default, Node)]
pub struct Sprite {
    /// loaded texture
    pub texture_id: ArcSwapOption<TextureId>,
    /// clip area
    pub area: [f64; 4],
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,

    pub src: Option<String>,

    pub vertex_buffer: Option<Buffer>,

    #[base]
    node_base: NodeBase,
}

impl Sprite {
    pub fn new(label: String) -> Self {
        Sprite {
            texture_id: ArcSwapOption::default(),
            area: [0., 0., 1., 1.],
            vertices: None,
            src: None,
            vertex_buffer: None,
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Sprite {
    fn contains(&self, x: f64, y: f64) -> bool {
        if let Some(texture_id) = self.texture_id.load().as_ref() {
            let core = get_core();
            if let Some(texture) = core.resource_manager.try_get_texture(&texture_id) {
                let translate = self.base().translate();

                let (width, height) = texture.size();

                if x > translate.x
                    && x < width as f64 + translate.x
                    && y > translate.y
                    && y < height as f64 + translate.y
                {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteProps {
    pub src: Option<String>,
    pub area: Option<[f64; 4]>,
}

impl Node for Sprite {
    #[inline]
    fn node_type(&self) -> &'static str {
        "sprite"
    }

    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    fn update_properties(&mut self, props: &mut JSValue) {
        let props: SpriteProps = from_js(props).unwrap();

        if let Some(src) = props.src {
            let texture_id = Arc::new(TextureId::Path(src.clone()));
            self.texture_id.store(Some(texture_id));
            self.src = Some(src);
        }

        if let Some(area) = props.area {
            self.area = area;
        }

        // force update vertices
        self.base_mut().pend_update();
    }
}
