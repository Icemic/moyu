//! The `spine` node: properties, commands and events.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{
    Command, Focusable, Node, NodeBaseTrait, NodeEventSource, PointerEventKind,
};
#[cfg(native)]
use moyu_core::utils::convert::to_js;
use moyu_core::utils::convert::{JSValue, from_js};
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::event::{EntryIdentity, SpineEvent};
use crate::job::{self, NodeJob};
use crate::renderer::SpineGpu;

/// Spine node: renders a Spine skeleton through the Spine plugin.
#[derive(Debug, Node)]
pub struct SpineNode {
    /// Skeleton path, relative to `assets/`.
    pub src: Option<String>,
    /// Atlas path; empty means "next to the skeleton".
    pub atlas: Option<String>,
    /// Skin name; empty means the model's default skin.
    pub skin: String,
    /// Speed multiplier.
    pub time_scale: f32,
    /// Whether time stops.
    pub paused: bool,
    /// Physics mode: `none` / `pose` / `update`.
    pub physics: PhysicsMode,
    /// Runs a hit test on every pointer press and reports `hit` events.
    pub hit_area: bool,

    /// State shared with the plugin driver and the renderer.
    pub job: Arc<NodeJob>,

    /// GPU resources, owned by the renderer.
    pub gpu: Option<Box<SpineGpu>>,

    #[base]
    node_base: NodeBase,
}

impl SpineNode {
    pub fn new(label: String) -> Self {
        let node_base = NodeBase::new(label);
        let job = job::register(*node_base.id());
        Self {
            src: None,
            atlas: None,
            skin: String::new(),
            time_scale: 1.0,
            paused: false,
            physics: PhysicsMode::Update,
            hit_area: false,
            job,
            gpu: None,
            node_base,
        }
    }

    /// Publishes the wanted state to the driver.
    fn sync_state(&self) {
        let mut state = self.job.state.lock();
        state.src = self.src.clone();
        state.atlas = self.atlas.clone();
        state.skin = self.skin.clone();
        state.time_scale = self.time_scale;
        state.paused = self.paused;
        state.physics = self.physics as i32;
    }

    /// The loaded model, or a message explaining why there is none.
    fn loaded_model(&self) -> Result<u32> {
        let model = self.job.model();
        if model == 0 {
            return Err(anyhow!("the spine node has no loaded model"));
        }
        Ok(model)
    }

    /// Runs the plugin call a command asked for, keeping errors intact.
    #[cfg(native)]
    fn with_context<T>(&self, call: impl FnOnce(&crate::plugin::PluginContext, u32) -> T) -> Result<T> {
        let model = self.loaded_model()?;
        let Some(context) = crate::plugin::context() else {
            return Err(anyhow!("the Spine plugin is not available"));
        };
        Ok(call(context, model))
    }
}

/// How the runtime advances physics constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PhysicsMode {
    /// Constraints are neither updated nor applied; the current pose is kept.
    None,
    /// Constraints are applied without advancing their state.
    Pose,
    /// Constraints are updated from the elapsed time.
    Update,
}

impl Default for PhysicsMode {
    fn default() -> Self {
        Self::Update
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct SpineProps {
    #[ts(optional = false)]
    pub src: Patch<String>,
    pub atlas: Patch<String>,
    pub skin: Patch<String>,
    pub time_scale: Patch<f32>,
    pub paused: Patch<bool>,
    pub physics: Patch<PhysicsMode>,
    pub hit_area: Patch<bool>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[ts(export, optional_fields)]
pub enum SpineCommand {
    /// Replaces whatever runs on `track`.
    #[serde(rename_all = "camelCase")]
    SetAnimation {
        track: i32,
        animation: String,
        #[serde(rename = "loop")]
        #[ts(optional)]
        loop_animation: Option<bool>,
        /// Transition duration; negative keeps the runtime's default.
        #[ts(optional)]
        mix: Option<f32>,
        #[ts(optional)]
        delay: Option<f32>,
        /// Start offset in seconds.
        #[ts(optional)]
        time: Option<f32>,
    },
    /// Queues an animation behind whatever runs on `track`.
    #[serde(rename_all = "camelCase")]
    AddAnimation {
        track: i32,
        animation: String,
        #[serde(rename = "loop")]
        #[ts(optional)]
        loop_animation: Option<bool>,
        #[ts(optional)]
        mix: Option<f32>,
        #[ts(optional)]
        delay: Option<f32>,
        #[ts(optional)]
        time: Option<f32>,
    },
    /// Clears one track; a positive `mix` fades out instead of stopping.
    ClearTrack {
        track: i32,
        #[ts(optional)]
        mix: Option<f32>,
    },
    ClearTracks,
    /// Sets the mix duration used when no specific pair is configured.
    SetDefaultMix {
        duration: f32,
    },
    /// Sets the transition duration between two named animations.
    SetMix {
        from: String,
        to: String,
        duration: f32,
    },
    /// Sets or removes one slot's attachment; an empty name removes it.
    SetAttachment {
        slot: String,
        name: String,
    },
    /// Restores the setup pose and stops every track.
    ResetToSetupPose,
    /// Resets physics constraints once.
    ResetPhysics,
    /// Hit test in world units; returns the bounding box name or `null`.
    HitTest {
        x: f32,
        y: f32,
    },
}

impl Node for SpineNode {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        Ok(Box::new(Self::new(label.unwrap_or_default())))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "spine"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: SpineProps = from_js(props).unwrap();

        apply_patch!(props.src => |src| {
            // An empty path is how a reset arrives; the node then shows nothing.
            self.src = (!src.is_empty()).then_some(src);
        }, String::new());
        apply_patch!(props.atlas => |atlas| {
            self.atlas = (!atlas.is_empty()).then_some(atlas);
        }, String::new());
        apply_patch!(props.skin => |skin| self.skin = skin, String::new());
        apply_patch!(props.time_scale => |time_scale| self.time_scale = time_scale, 1.0);
        apply_patch!(props.paused => |paused| self.paused = paused, false);
        apply_patch!(props.physics => |physics| self.physics = physics, PhysicsMode::Update);
        apply_patch!(props.hit_area => |hit_area| self.hit_area = hit_area, false);

        self.sync_state();
        // The model change takes effect now, so a command in the same script tick
        // already talks to the new model.
        job::sync_now(&self.job);
        self.base_mut().pend_prepare();
    }

    fn ready(&self) -> bool {
        self.gpu.as_ref().is_some_and(|gpu| gpu.ready) && self.children_ready()
    }

    fn as_focusable(&self) -> Option<&dyn Focusable> {
        Some(self)
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

impl NodeEventSource for SpineNode {
    type Event = SpineEvent;
}

impl Focusable for SpineNode {
    /// Runs the built-in hit test on a press when `hitArea` is on.
    fn pointer_event(&self, x: f32, y: f32, kind: PointerEventKind) {
        if !self.hit_area || kind != PointerEventKind::Down {
            return;
        }

        #[cfg(web)]
        {
            let _ = (x, y);
        }

        #[cfg(native)]
        {
            let Some(context) = crate::plugin::context() else {
                return;
            };
            let model = self.job.model();
            if model == 0 {
                return;
            }

            // The pointer arrives in node space, which is the model's world space
            // shifted by the reference bounds; undoing that shift gives the world
            // units the plugin's hit test expects.
            let (origin_x, origin_y) = self.bounds_origin();
            let world_x = x + origin_x;
            let world_y = y + origin_y;

            match context.hit_test(model, world_x, world_y) {
                Ok(Some(name)) => {
                    let epoch = self.job.state.lock().epoch();
                    job::send_event(
                        *self.base().id(),
                        SpineEvent::Hit {
                            epoch,
                            name,
                            x,
                            y,
                        },
                    );
                }
                Ok(None) => {}
                Err(_) => {}
            }
        }
    }
}

impl SpineNode {
    /// Origin of the loaded model's reference bounds, which is the shift the
    /// renderer applies to put the model inside the node.
    fn bounds_origin(&self) -> (f32, f32) {
        match self.job.state.lock().model_info.as_ref() {
            Some(info) => (info.bounds[0], info.bounds[1]),
            None => (0.0, 0.0),
        }
    }
}

impl Command for SpineNode {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let command: SpineCommand = from_js(payload)?;

        #[cfg(native)]
        {
            match command {
                SpineCommand::SetAnimation {
                    track,
                    animation,
                    loop_animation,
                    mix,
                    delay,
                    time,
                } => {
                    let epoch = self.job.state.lock().epoch();
                    let entry = self.with_context(|context, model| {
                        context.set_animation(
                            model,
                            track,
                            &animation,
                            loop_animation.unwrap_or(false),
                            mix.unwrap_or(-1.0),
                            delay.unwrap_or(0.0),
                            time.unwrap_or(0.0),
                        )
                    })??;
                    Ok(Some(to_js(&EntryIdentity { entry, epoch })?))
                }
                SpineCommand::AddAnimation {
                    track,
                    animation,
                    loop_animation,
                    mix,
                    delay,
                    time,
                } => {
                    let epoch = self.job.state.lock().epoch();
                    let entry = self.with_context(|context, model| {
                        context.add_animation(
                            model,
                            track,
                            &animation,
                            loop_animation.unwrap_or(false),
                            mix.unwrap_or(-1.0),
                            delay.unwrap_or(0.0),
                            time.unwrap_or(0.0),
                        )
                    })??;
                    Ok(Some(to_js(&EntryIdentity { entry, epoch })?))
                }
                SpineCommand::ClearTrack { track, mix } => {
                    let result = self.with_context(|context, model| {
                        context.clear_track(model, track, mix.unwrap_or(0.0))
                    })?;
                    result?;
                    Ok(None)
                }
                SpineCommand::ClearTracks => {
                    let result = self.with_context(|context, model| context.clear_tracks(model))?;
                    result?;
                    Ok(None)
                }
                SpineCommand::SetDefaultMix { duration } => {
                    let result = self
                        .with_context(|context, model| context.set_default_mix(model, duration))?;
                    result?;
                    Ok(None)
                }
                SpineCommand::SetMix { from, to, duration } => {
                    let result = self.with_context(|context, model| {
                        context.set_mix(model, &from, &to, duration)
                    })?;
                    result?;
                    Ok(None)
                }
                SpineCommand::SetAttachment { slot, name } => {
                    let result = self.with_context(|context, model| {
                        context.set_attachment(model, &slot, &name)
                    })?;
                    result?;
                    Ok(None)
                }
                SpineCommand::ResetToSetupPose => {
                    let result =
                        self.with_context(|context, model| context.reset_to_setup_pose(model))?;
                    result?;
                    Ok(None)
                }
                SpineCommand::ResetPhysics => {
                    let result = self.with_context(|context, model| context.reset_physics(model))?;
                    result?;
                    Ok(None)
                }
                SpineCommand::HitTest { x, y } => {
                    let (origin_x, origin_y) = self.bounds_origin();
                    let world_x = x + origin_x;
                    let world_y = y + origin_y;
                    let name = self
                        .with_context(|context, model| context.hit_test(model, world_x, world_y))??;
                    Ok(Some(to_js(&name)?))
                }
            }
        }

        #[cfg(web)]
        {
            let _ = command;
            Err(anyhow!(
                "the spine node needs the native engine; the web runtime has no Spine plugin"
            ))
        }
    }
}
