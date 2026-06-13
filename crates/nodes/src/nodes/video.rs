use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, Focusable, Node, NodeBaseTrait, NodeEventSource};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use moyu_video::{PlaybackState, VideoPlayer};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::events::VideoEvent;

use moyu_pal::sync::Mutex;

/// Video node: renders a video file using the Moyu video pipeline.
#[derive(Debug, Node)]
pub struct Video {
    /// Video source path
    pub src: Option<String>,
    /// Pending source change
    pub(crate) next_src: Option<String>,
    /// Whether to loop
    pub looping: bool,
    /// Whether to auto-play when loaded
    pub auto_play: bool,
    /// Volume (0.0 - 1.0)
    pub volume: f64,
    /// Whether muted
    pub muted: bool,

    /// Previous playback state (for change detection)
    pub(crate) prev_state: PlaybackState,

    /// The video player instance. We wrap it in Mutex because the `dyn VideoDecoder` inside
    /// is not Sync (web decoder), but we only ever access it on the render thread.
    pub(crate) player: Arc<Mutex<VideoPlayer>>,

    /// Loaded file data waiting for player initialization
    pub(crate) next_data: Arc<ArcSwapOption<Vec<u8>>>,

    // Per-node GPU resources (like Animation)
    /// Y plane texture bind group (or RGBA bind group for simple path)
    pub(crate) bind_group: Option<wgpu::BindGroup>,
    /// Y texture view
    pub(crate) view_y: Option<wgpu::TextureView>,
    /// U texture view
    pub(crate) view_u: Option<wgpu::TextureView>,
    /// V texture view
    pub(crate) view_v: Option<wgpu::TextureView>,
    /// Quad vertex buffer
    pub(crate) vertex_buffer: Option<wgpu::Buffer>,
    /// Current frame dimensions (for texture re-creation detection)
    pub(crate) current_dimensions: Option<(u32, u32)>,
    /// Current pixel format (for texture re-creation when format changes)
    pub(crate) current_format: Option<moyu_video::PixelFormat>,

    #[base]
    node_base: NodeBase,
}

impl Video {
    pub fn new(label: String) -> Self {
        Self {
            src: None,
            next_src: None,
            looping: false,
            auto_play: true,
            volume: 1.0,
            muted: false,
            prev_state: PlaybackState::Idle,
            player: Arc::new(Mutex::new(VideoPlayer::new())),
            next_data: Arc::new(ArcSwapOption::default()),
            bind_group: None,
            view_y: None,
            view_u: None,
            view_v: None,
            vertex_buffer: None,
            current_dimensions: None,
            current_format: None,
            node_base: NodeBase::new(label),
        }
    }
}

impl Focusable for Video {}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct VideoProps {
    #[ts(optional = false)]
    pub src: Patch<String>,
    #[serde(rename = "loop")]
    pub looping: Patch<bool>,
    pub auto_play: Patch<bool>,
    pub volume: Patch<f64>,
    pub muted: Patch<bool>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[ts(export, optional_fields)]
pub enum VideoCommand {
    Play,
    Pause,
    Resume,
    Stop,
    Seek { time: f64 },
    SetVolume { volume: f64 },
    SetMuted { muted: bool },
    SetLoop { enabled: bool },
}

impl Node for Video {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "video"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: VideoProps = from_js(props).unwrap();

        apply_patch!(props.src => |src| {
            self.src = Some(src);
            self.next_src = self.src.clone();
        }, String::new());

        apply_patch!(props.looping => |v| {
            self.looping = v;
            self.player.lock().set_loop(v);
        }, false);

        apply_patch!(props.auto_play => |v| {
            self.auto_play = v;
        }, true);

        apply_patch!(props.volume => |v| {
            self.volume = v;
            self.player.lock().set_volume(v);
        }, 1.0);

        apply_patch!(props.muted => |v| {
            self.muted = v;
            self.player.lock().set_muted(v);
        }, false);

        self.base_mut().pend_update();
    }

    fn ready(&self) -> bool {
        self.bind_group.is_some()
            && self.next_src.is_none()
            && self.next_data.load().is_none()
            && self.children_ready()
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

impl NodeEventSource for Video {
    type Event = VideoEvent;
}

impl Command for Video {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let cmd: VideoCommand = from_js(payload)?;
        let mut player = self.player.lock();
        match cmd {
            VideoCommand::Play => {
                player.play()?;
            }
            VideoCommand::Pause => {
                player.pause()?;
            }
            VideoCommand::Resume => {
                player.resume()?;
            }
            VideoCommand::Stop => {
                player.stop();
            }
            VideoCommand::Seek { time } => {
                player.seek(time)?;
            }
            VideoCommand::SetVolume { volume } => {
                player.set_volume(volume);
            }
            VideoCommand::SetMuted { muted } => {
                player.set_muted(muted);
            }
            VideoCommand::SetLoop { enabled } => {
                player.set_loop(enabled);
            }
        }
        Ok(None)
    }
}
