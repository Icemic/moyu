use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use image::{Frames, RgbaImage};
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_macros::Node;
use reiterator::Reiterator;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub struct FrameIterator(pub(crate) Reiterator<Frames<'static>>);

/// Safety: Frames is not Send/Sync, but we ensure single-threaded usage
unsafe impl Send for FrameIterator {}
unsafe impl Sync for FrameIterator {}

impl Debug for FrameIterator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FrameIterator")
    }
}

impl Deref for FrameIterator {
    type Target = Reiterator<Frames<'static>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FrameIterator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum AnimationFormat {
    #[default]
    APNG,
    WEBP,
}

#[derive(Debug, Node)]
pub struct Animation {
    /// texture source path
    pub src: Option<String>,
    /// next texture source path
    pub next_src: Option<String>,
    /// (for sprite mode) clip area
    pub area: [f32; 4],
    /// animation format
    pub format: AnimationFormat,

    /// frame iterator
    pub(crate) frames: Option<FrameIterator>,
    /// next frame. (future timestamp, image data)
    pub(crate) next_frame: Option<(f64, RgbaImage)>,

    // Since animation nodes won't be too many, we keep texture view and bind group here
    // to simplify the renderer logic.
    /// texture bind group
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    /// texture view
    pub(crate) view: Option<wgpu::TextureView>,
    /// vertex buffer
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,

    /// next animation data to load, it will replace `frames` after loaded and reset to None
    pub(crate) next_data: Arc<ArcSwapOption<Vec<u8>>>,

    #[base]
    node_base: NodeBase,
}

impl Animation {
    pub fn new(label: String) -> Self {
        Self {
            src: None,
            next_src: None,
            area: [0.0, 0.0, 1.0, 1.0],
            format: AnimationFormat::APNG,
            frames: None,
            next_frame: None,
            bind_group: None,
            view: None,
            vertex_buffer: None,
            next_data: Arc::new(ArcSwapOption::default()),
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Animation {}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct AnimationProps {
    pub src: Option<String>,
    pub area: Option<[f32; 4]>,
    pub format: Option<AnimationFormat>,
}

impl Node for Animation {
    #[inline]
    fn node_type(&self) -> &'static str {
        "animation"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: AnimationProps = from_js(props).unwrap();

        // set pending change to next_texture_id, avoid texture loading in render (may cause flash)
        if let Some(src) = props.src {
            self.src = Some(src);
            self.next_src = self.src.clone();
        }

        if let Some(area) = props.area {
            self.area = area;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }

        if let Some(format) = props.format {
            self.format = format;
        }

        self.base_mut().pend_update();
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
