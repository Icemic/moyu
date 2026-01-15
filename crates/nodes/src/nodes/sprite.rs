use anyhow::Result;
use arc_swap::ArcSwapOption;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use wgpu::Buffer;

use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_resource::types::AssetId;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum SpriteMode {
    #[default]
    Normal,
    Nineslice,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
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

    pub instance_buffer: Option<Buffer>,

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
            instance_buffer: None,
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Sprite {}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct SpriteProps {
    pub src: Patch<String>,
    pub mode: Patch<SpriteMode>,
    pub area: Patch<[f32; 4]>,
    pub bounds: Patch<[f32; 4]>,
    pub nine_slice_mode: Patch<NineSliceMode>,
    pub target_width: Patch<u32>,
    pub target_height: Patch<u32>,
}

impl Node for Sprite {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "sprite"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: SpriteProps = from_js(props).unwrap();

        // set pending change to next_texture_id, avoid texture loading in render (may cause flash)
        apply_patch!(props.src => |src| {
            self.src = Some(src);
            self.next_src = self.src.clone();
        }, String::new());

        apply_patch!(props.mode => |mode| {
            self.mode = mode;
            // reset size when mode changed, those values will be recalculated in render
            self.base_mut().set_size(0, 0);
        }, SpriteMode::default());

        apply_patch!(props.area => |area| {
            self.area = area;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }, [0., 0., 1., 1.]);

        apply_patch!(props.bounds => self.bounds, [0., 0., 0., 0.]);

        apply_patch!(props.nine_slice_mode => |nine_slice_mode| {
            self.nine_slice_mode = nine_slice_mode;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }, NineSliceMode::default());

        apply_patch!(props.target_width => |target_width| {
            self.target_width = target_width;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }, 0);

        apply_patch!(props.target_height => |target_height| {
            self.target_height = target_height;
            // clean base node size, and re-assign it in renderer
            self.base_mut().set_size(0, 0);
        }, 0);

        // force update vertices
        self.base_mut().pend_update();
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
