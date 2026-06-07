use anyhow::Result;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Command, Node, NodeBaseTrait, NodeEventSource};
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::events::TransitionContainerEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum TransitionEffect {
    #[default]
    Crossfade,
    Wipe,
}

impl From<TransitionEffect> for i32 {
    fn from(effect: TransitionEffect) -> Self {
        match effect {
            TransitionEffect::Crossfade => 0,
            TransitionEffect::Wipe => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "kebab-case")]
#[ts(export)]
pub enum RetainMode {
    #[default]
    Snapshot,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TransitionPhase {
    #[default]
    Stable,
    AwaitingPrepare,
    Prepared,
    Running,
    Finishing,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPerform {
    // Queued perform request. It is applied during renderer update so command
    // execution stays side-effect free from the render loop state machine. The
    // renderer consumes it once the current slot pair is ready to run.
    pub effect: TransitionEffect,
    pub duration: f64,
}

#[derive(Debug, Node)]
pub struct TransitionContainer {
    // Effect parameters of the current or next run.
    // `progress` stays in [0, 1]. `transition_start_at` is only meaningful
    // while the phase is `Running`.
    pub(crate) effect: TransitionEffect,
    pub(crate) retain: RetainMode,
    pub(crate) duration: f64,
    pub(crate) progress: f32,
    pub(crate) transition_start_at: Option<f64>,

    // Pending commands coming from JS. They are stored here first and then
    // folded into the renderer state machine on the next update tick.
    pub(crate) pending_prepare: bool,
    pub(crate) pending_perform: Option<PendingPerform>,

    // Transition state machine:
    // - `Stable`: transparent wrapper, no retained output is presented.
    // - `AwaitingPrepare`: both slots exist and the container is ready to
    //   accept a prepare, but no retained state has been armed yet.
    // - `Prepared`: old side is retained and the new side can render into its
    //   hidden target, but progress has not started advancing.
    // - `Running`: time-based progress is advancing.
    // - `Finishing`: progress already reached 1.0, but one more update is kept
    //   so the final frame can be presented before cleanup and event dispatch.
    // - `slots_armed`: the current from/to slot pair has been recognized as a
    //   transition candidate. This avoids re-entering `awaiting_prepare`
    //   repeatedly while the same pair stays mounted.
    // - `from_needs_redraw`: snapshot mode uses this as a one-shot latch to
    //   decide whether the old subtree must redraw into `from_view`.
    pub(crate) phase: TransitionPhase,
    pub(crate) slots_armed: bool,
    pub(crate) from_needs_redraw: bool,

    // Shared texture allocation for the current transition.
    // `texture_*` is the backing allocation size in physical pixels. It only
    // grows while active so existing retained textures stay valid.
    // `render_*` is the physical size of the current logical `transition_rect`
    // inside that allocation, and may therefore be smaller than `texture_*`.
    // `last_active_at` records when the container last needed retained
    // textures, so idle resources can be released later.
    // `transition_rect` is the current logical union rect of the from/to
    // subtrees, or a temporary bootstrap rect before real bounds exist.
    pub(crate) texture_width: u32,
    pub(crate) texture_height: u32,
    pub(crate) render_width: u32,
    pub(crate) render_height: u32,
    pub(crate) last_active_at: Option<f64>,
    pub(crate) transition_rect: moyu_core::base::Rect,

    // Three retained textures:
    // - `from_view`: retained old subtree.
    // - `to_view`: hidden new subtree.
    // - `display_view`: composited result used while `Running` or `Finishing`.
    pub(crate) from_view: Option<wgpu::TextureView>,
    pub(crate) to_view: Option<wgpu::TextureView>,
    pub(crate) display_view: Option<wgpu::TextureView>,

    // GPU resources for the two fullscreen-style passes:
    // - composite: from + to -> display
    // - present: retained source -> parent target
    pub(crate) composite_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) present_uniform_buffer: Option<wgpu::Buffer>,
    pub(crate) present_bind_group: Option<wgpu::BindGroup>,
    pub(crate) composite_bind_group: Option<wgpu::BindGroup>,

    #[base]
    node_base: NodeBase,
}

impl Default for TransitionContainer {
    fn default() -> Self {
        Self {
            effect: TransitionEffect::Crossfade,
            retain: RetainMode::Snapshot,
            duration: 0.0,
            progress: 1.0,
            transition_start_at: None,
            pending_prepare: false,
            pending_perform: None,
            phase: TransitionPhase::Stable,
            slots_armed: false,
            from_needs_redraw: false,
            texture_width: 0,
            texture_height: 0,
            render_width: 0,
            render_height: 0,
            last_active_at: None,
            transition_rect: moyu_core::base::Rect::default(),
            from_view: None,
            to_view: None,
            display_view: None,
            composite_uniform_buffer: None,
            present_uniform_buffer: None,
            present_bind_group: None,
            composite_bind_group: None,
            node_base: NodeBase::default(),
        }
    }
}

impl TransitionContainer {
    pub fn new(label: String) -> Self {
        Self {
            node_base: NodeBase::new(label),
            ..Default::default()
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        // "Active" means the container must keep offscreen resources alive and
        // route its slots through retained targets instead of acting as a
        // transparent wrapper.
        !matches!(self.phase, TransitionPhase::Stable)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct TransitionContainerProps {}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[ts(export, optional_fields)]
pub enum TransitionContainerCommand {
    Prepare {
        retain: Option<RetainMode>,
    },
    Perform {
        effect: TransitionEffect,
        duration: f64,
    },
}

impl Node for TransitionContainer {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        let label = label.unwrap_or_default();
        Ok(Box::new(Self::new(label)))
    }

    fn node_type(&self) -> &'static str {
        "transition_container"
    }

    fn pre_update(&mut self, parent: &NodeBase) {
        let width = *parent.width();
        let height = *parent.height();

        if *self.base().width() != width || *self.base().height() != height {
            self.base_mut().set_size(width, height);
        }
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

impl NodeEventSource for TransitionContainer {
    type Event = TransitionContainerEvent;
}

impl Command for TransitionContainer {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let command: TransitionContainerCommand = from_js(payload)?;

        match command {
            TransitionContainerCommand::Prepare { retain } => {
                if let Some(retain) = retain {
                    self.retain = retain;
                }
                self.pending_prepare = true;
            }
            TransitionContainerCommand::Perform { effect, duration } => {
                self.pending_perform = Some(PendingPerform { effect, duration });
            }
        }

        self.base_mut().pend_update();

        Ok(None)
    }
}
