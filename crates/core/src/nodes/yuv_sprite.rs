use arc_swap::ArcSwapOption;
use hai_macros::node;
use hai_pal::sync::RwLock;
use log::warn;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use wgpu::Buffer;

use crate::resource::TextureId;
use crate::traits::{Focusable, Node, NodeType, UpdateProps, NODE_ID};
use crate::types::{Point, SurfaceSize, Transform, Vertex};
#[cfg(all(not(feature = "web"), feature = "js_runtime"))]
use crate::utils::convert::{from_js, JSValue};

use super::Texture;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum YUVSpriteFormat {
    #[default]
    I420 = 0,
    NV12 = 1,
}

/// Sprite for rendering YUV format images, ex. a ffmpeg AVFrame.
/// textures should always be set manully, once `textures` is set, rendering will start.
#[node]
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
}

impl YUVSprite {
    pub fn new(label: String) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };

        YUVSprite {
            id,
            label,
            anchor: Point::default(),
            pivot: Point::default(),
            translate: Point::default(),
            scale: Point::one(),
            rotation: 0.,
            skew: Point::default(),

            _update_id: 0,
            _current_update_id: 0,

            transform: Transform::default(),
            global_transform: Transform::default(),
            children: vec![],

            texture_id: ArcSwapOption::default(),
            textures: ArcSwapOption::default(),
            area: [0., 0., 1., 1.],
            vertices: None,
            vertex_buffer: None,
            mode: YUVSpriteFormat::default(),
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

            let translate = self.translate();
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
        self._update_id += 1;
    }
}
