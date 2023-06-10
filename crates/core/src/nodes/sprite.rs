use arc_swap::ArcSwapOption;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use wgpu::Buffer;

use crate::core::get_core;
use crate::resource::TextureId;
use crate::traits::{Focusable, GetNodeBase, Node, NodeType, UpdateProps};
use crate::types::Vertex;
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::{from_js, JSValue};

use super::{NodeBase, Texture};

// #[node]
#[derive(Debug, Default)]
pub struct Sprite {
    /// loaded texture
    pub texture_id: ArcSwapOption<TextureId>,
    pub texture: ArcSwapOption<Texture>,
    /// clip area
    pub area: [f64; 4],
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,

    pub src: Option<String>,

    pub vertex_buffer: Option<Buffer>,

    pub node_base: NodeBase,
}

impl Sprite {
    pub fn new(label: String) -> Self {
        // let id = unsafe {
        //     NODE_ID += 1;
        //     NODE_ID
        // };

        Sprite {
            // id,
            // label,
            texture_id: ArcSwapOption::default(),
            texture: ArcSwapOption::default(),
            area: [0., 0., 1., 1.],
            vertices: None,
            src: None,
            vertex_buffer: None,

            node_base: NodeBase::new(label),

            ..Default::default()
        }
    }
}

impl NodeType for Sprite {
    fn node_type(&self) -> &'static str {
        "sprite"
    }
}

impl Focusable for Sprite {
    fn contains(&self, x: f64, y: f64) -> bool {
        if let Some(texture) = self.texture.load().as_ref() {
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

        false
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteProps {
    pub src: Option<String>,
    pub area: Option<[f64; 4]>,
}

impl UpdateProps for Sprite {
    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    fn update_properties(&mut self, props: &mut JSValue) {
        let props: SpriteProps = from_js(props).unwrap();

        if let Some(src) = props.src {
            let core = get_core();
            let mut resource_manager = core.resource_manager.lock();
            let texture_id = Arc::new(TextureId::Path(src.clone()));
            let texture = resource_manager.get_texture(&texture_id);
            self.texture_id.store(Some(texture_id));
            self.texture.store(Some(texture));
            self.src = Some(src);
        }

        if let Some(area) = props.area {
            self.area = area;
        }

        // force update vertices
        self.base_mut().pend_update();
    }
}

impl GetNodeBase for Sprite {
    #[inline]
    fn base(&self) -> &NodeBase {
        &self.node_base
    }

    #[inline]
    fn base_mut(&mut self) -> &mut NodeBase {
        &mut self.node_base
    }
}

impl Node for Sprite {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
