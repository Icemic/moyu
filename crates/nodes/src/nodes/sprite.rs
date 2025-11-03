use arc_swap::ArcSwapOption;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use wgpu::Buffer;

use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_resource::types::AssetId;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpriteMode {
    #[default]
    Normal,
    Nineslice,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NineSliceMode {
    /// Stretch edge and center areas to fill the bounds.
    #[default]
    Stretch,
    /// Repeat edge and center areas to fill the bounds.
    Repeat,
    /// Repeat edge and center areas to fill the bounds, but mirror the texture on each repeat.
    Mirror,
    /// Leave edge and center areas blank (do not draw center area).
    Blank,
}

// #[node]
#[derive(Debug, Default, Node)]
pub struct Sprite {
    /// loaded texture
    pub texture_id: ArcSwapOption<AssetId>,
    /// next texture id to load, it will replace `texture_id` after loaded and reset to None
    pub next_texture_id: ArcSwapOption<AssetId>,
    /// texture source path
    pub src: Option<String>,
    /// next texture source path
    pub next_src: Option<String>,

    /// sprite mode, `normal` (default) or `nineslice`
    pub mode: SpriteMode,
    /// (for sprite mode) clip area
    pub area: [f32; 4],

    /// (for nineslice mode) bounds, [left, top, right, bottom]
    pub bounds: [f32; 4],
    /// (for nineslice mode) nine slice mode
    pub nine_slice_mode: NineSliceMode,
    /// (for nineslice mode) target width
    pub target_width: u32,
    /// (for nineslice mode) target height
    pub target_height: u32,

    pub vertex_buffer: Option<Buffer>,

    #[base]
    node_base: NodeBase,
}

impl Sprite {
    pub fn new(label: String) -> Self {
        Sprite {
            texture_id: ArcSwapOption::default(),
            next_texture_id: ArcSwapOption::default(),
            src: None,
            next_src: None,
            mode: SpriteMode::Normal,
            area: [0., 0., 1., 1.],
            bounds: [0., 0., 0., 0.],
            nine_slice_mode: NineSliceMode::Stretch,
            target_width: 0,
            target_height: 0,
            vertex_buffer: None,
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Sprite {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteProps {
    pub src: Option<String>,
    pub mode: Option<SpriteMode>,
    pub area: Option<[f32; 4]>,
    pub bounds: Option<[f32; 4]>,
    pub nine_slice_mode: Option<NineSliceMode>,
    pub target_width: Option<u32>,
    pub target_height: Option<u32>,
}

impl Node for Sprite {
    #[inline]
    fn node_type(&self) -> &'static str {
        "sprite"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: SpriteProps = from_js(props).unwrap();

        // set pending change to next_texture_id, avoid texture loading in render (may cause flash)
        if let Some(src) = props.src {
            self.src = Some(src);
            self.next_src = self.src.clone();
        }

        if let Some(mode) = props.mode {
            self.mode = mode;
            // reset size when mode changed, those values will be recalculated in render
            self.base_mut().set_size(0, 0);
        }

        if let Some(area) = props.area {
            self.area = area;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }

        if let Some(bounds) = props.bounds {
            self.bounds = bounds;
        }

        if let Some(nine_slice_mode) = props.nine_slice_mode {
            self.nine_slice_mode = nine_slice_mode;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }

        if let Some(target_width) = props.target_width {
            self.target_width = target_width;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }

        if let Some(target_height) = props.target_height {
            self.target_height = target_height;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }

        // force update vertices
        self.base_mut().pend_update();
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
