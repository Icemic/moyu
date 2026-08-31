use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_image::AnimationDecoder;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum AnimationFormat {
    #[default]
    APNG,
    WEBP,
    JXL,
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

    /// animation decoder
    pub(crate) decoder: Option<AnimationDecoder>,
    /// next frame timestamp
    pub(crate) next_frame: Option<f64>,
    /// reusable premultiplied RGBA8 GPU upload buffer
    pub(crate) upload_buffer: Vec<u8>,

    // Since animation nodes won't be too many, we keep texture view and bind group here
    // to simplify the renderer logic.
    /// texture bind group
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    /// texture view
    pub(crate) view: Option<wgpu::TextureView>,
    /// vertex buffer
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,

    /// next animation data to load, it will replace `decoder` after loaded and reset to None
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
            decoder: None,
            next_frame: None,
            upload_buffer: Vec::new(),
            bind_group: None,
            view: None,
            vertex_buffer: None,
            next_data: Arc::new(ArcSwapOption::default()),
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Animation {}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct AnimationProps {
    #[ts(optional = false)]
    pub src: Patch<String>,
    pub area: Patch<[f32; 4]>,
    pub format: Patch<AnimationFormat>,
}

impl Node for Animation {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "animation"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: AnimationProps = from_js(props).unwrap();

        // set pending change to next_texture_id, avoid texture loading in render (may cause flash)
        apply_patch!(props.src => |src| {
            self.src = Some(src);
            self.next_src = self.src.clone();
        }, String::new());

        apply_patch!(props.area => |area| {
            self.area = area;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_intrinsic_size(0.0, 0.0);
        }, [0.0, 0.0, 1.0, 1.0]);

        apply_patch!(props.format => self.format, AnimationFormat::default());

        self.base_mut().pend_prepare();
    }

    fn ready(&self) -> bool {
        self.view.is_some()
            && self.bind_group.is_some()
            && self.next_src.is_none()
            && self.next_data.load().is_none()
            && self.children_ready()
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
