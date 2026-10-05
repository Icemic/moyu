use anyhow::Result;
use arc_swap::ArcSwapOption;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use wgpu::Buffer;

use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Focusable, FocusablePayload, Node, NodeBaseTrait};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_resource::types::{Asset, AssetId};

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
    /// Repeat edge and center areas at their natural size to fill the bounds.
    Repeat,
    /// Like repeat, but mirror the pattern on every other repeat.
    Mirror,
    /// Draw corners and edges as in stretch, and leave the center undrawn.
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

    /// (for nineslice mode) bounds, [left, top, right, bottom] as ratios of `area`, 0..1
    pub bounds: [f32; 4],
    /// (for nineslice mode) nine slice mode
    pub nine_slice_mode: NineSliceMode,
    /// (for nineslice mode) target width
    pub target_width: u32,
    /// (for nineslice mode) target height
    pub target_height: u32,

    /// whether hit testing samples the texture alpha instead of only the rectangle
    pub alpha_hit_test: bool,

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
            alpha_hit_test: false,
            instance_buffer: None,
            node_base: NodeBase::new(label),
        }
    }

    pub(crate) fn update_intrinsic_size(&mut self) {
        let size = match self.mode {
            SpriteMode::Normal => (0.0, 0.0),
            SpriteMode::Nineslice => (self.target_width as f32, self.target_height as f32),
        };
        self.base_mut().set_intrinsic_size(size.0, size.1);
    }
}

impl Focusable for Sprite {
    fn contains(&self, x: f32, y: f32, _: &FocusablePayload) -> bool {
        if !self.base().content_bounds().contains(x, y) {
            return false;
        }

        // Pixel-level testing covers the normal mode quad only; nineslice slices
        // have no single texture mapping and fall back to the rectangle.
        if !self.alpha_hit_test || self.mode != SpriteMode::Normal {
            return true;
        }

        // The mask travels with the texture and is read as plain memory, so hit
        // testing stays synchronous. Fall back to the rectangle while it is missing.
        let texture_id = self.texture_id.load();
        let Some(texture_id) = texture_id.as_ref() else {
            return true;
        };
        let Some(asset) = texture_id.asset() else {
            return true;
        };
        let Asset::Texture(texture) = asset.as_ref() else {
            return true;
        };
        let Some(mask) = texture.alpha_mask() else {
            return true;
        };

        let (tex_width, tex_height) = texture.size();
        let (tex_width, tex_height) = (tex_width as f32, tex_height as f32);
        let ratio = texture.pixel_ratio();
        let [ax0, ay0, ax1, ay1] = self.area;

        // The quad the normal-mode renderer draws, in stage units
        // (see `calculate_sprite_instance`).
        let quad_width = tex_width / ratio * (ax1 - ax0);
        let quad_height = tex_height / ratio * (ay1 - ay0);
        if quad_width <= 0.0 || quad_height <= 0.0 {
            return false;
        }
        if x < 0.0 || y < 0.0 || x > quad_width || y > quad_height {
            return false;
        }

        // Local position -> texel: `u = x / quad_width`, then `texel = (ax0 + u * (ax1 - ax0)) * tex_width`.
        let texel_x = (ax0 * tex_width + x * ratio).clamp(0.0, tex_width - 1.0) as u32;
        let texel_y = (ay0 * tex_height + y * ratio).clamp(0.0, tex_height - 1.0) as u32;

        mask.is_opaque(texel_x, texel_y)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct SpriteProps {
    #[ts(optional = false)]
    pub src: Patch<String>,
    pub mode: Patch<SpriteMode>,
    pub area: Patch<[f32; 4]>,
    pub bounds: Patch<[f32; 4]>,
    pub nine_slice_mode: Patch<NineSliceMode>,
    pub target_width: Patch<f32>,
    pub target_height: Patch<f32>,
    pub alpha_hit_test: Patch<bool>,
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
            self.update_intrinsic_size();
        }, SpriteMode::default());

        apply_patch!(props.area => |area| {
            self.area = area;
            self.update_intrinsic_size();
        }, [0., 0., 1., 1.]);

        apply_patch!(props.bounds => self.bounds, [0., 0., 0., 0.]);

        apply_patch!(props.nine_slice_mode => |nine_slice_mode| {
            self.nine_slice_mode = nine_slice_mode;
            self.update_intrinsic_size();
        }, NineSliceMode::default());

        apply_patch!(props.target_width => |target_width| {
            self.target_width = target_width as u32;
            self.update_intrinsic_size();
        }, 0);

        apply_patch!(props.target_height => |target_height| {
            self.target_height = target_height as u32;
            self.update_intrinsic_size();
        }, 0);

        apply_patch!(props.alpha_hit_test => self.alpha_hit_test, false);

        // force update vertices
        self.base_mut().pend_prepare();
    }

    fn ready(&self) -> bool {
        self.texture_id.load().is_some()
            && self.next_texture_id.load().is_none()
            && self.next_src.is_none()
            && self.children_ready()
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }
}
