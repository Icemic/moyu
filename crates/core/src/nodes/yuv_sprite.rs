use arc_swap::ArcSwapOption;
use serde::{Deserialize, Serialize};
use std::any::Any;
use wgpu::Buffer;

use crate::resource::TextureId;
use crate::traits::{Focusable, GetNodeBase, Node, NodeType, UpdateProps};
use crate::types::Vertex;
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::{from_js, JSValue};

use super::{NodeBase, Texture};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum YUVSpriteFormat {
    #[default]
    I420 = 0,
    NV12 = 1,
}

/// Sprite for rendering YUV format images, ex. a ffmpeg AVFrame.
/// textures should always be set manully, once `textures` is set, rendering will start.
#[derive(Debug)]
pub struct YUVSprite {
    pub texture_id: ArcSwapOption<TextureId>,
    /// (Y, U, V) or (Y, UV, _)
    pub textures: ArcSwapOption<(Texture, Texture, Texture)>,
    /// clip area
    pub area: [f64; 4],
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,
    pub vertex_buffer: Option<Buffer>,
    pub mode: YUVSpriteFormat,

    node_base: NodeBase,
}

impl YUVSprite {
    pub fn new(label: String) -> Self {
        YUVSprite {
            texture_id: ArcSwapOption::default(),
            textures: ArcSwapOption::default(),
            area: [0., 0., 1., 1.],
            vertices: None,
            vertex_buffer: None,
            mode: YUVSpriteFormat::default(),
            node_base: NodeBase::new(label),
        }
    }
}

impl NodeType for YUVSprite {
    fn node_type(&self) -> &'static str {
        "yuv_sprite"
    }
}

impl Focusable for YUVSprite {
    fn contains(&self, x: f64, y: f64) -> bool {
        if let Some(textures) = self.textures.load().as_ref() {
            let (texture, _, _) = &**textures;

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
pub struct YUVSpriteProps {
    pub area: Option<[f64; 4]>,
}

impl UpdateProps for YUVSprite {
    #[cfg(all(not(feature = "web"), feature = "js_runtime"))]
    fn update_properties(&mut self, props: &mut JSValue) {
        let props: YUVSpriteProps = from_js(props).unwrap();

        if let Some(area) = props.area {
            self.area = area;
        }

        // force update vertices
        self.base_mut().pend_update();
    }
}

impl GetNodeBase for YUVSprite {
    #[inline]
    fn base(&self) -> &NodeBase {
        &self.node_base
    }

    #[inline]
    fn base_mut(&mut self) -> &mut NodeBase {
        &mut self.node_base
    }
}

impl Node for YUVSprite {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
