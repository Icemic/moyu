//! The `live2d` node: properties, commands and events.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use moyu_core::apply_patch;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{
    Command, Focusable, Node, NodeBaseTrait, NodeEventSource, PointerEventKind,
};
use moyu_core::utils::convert::{JSValue, from_js};
#[cfg(native)]
use moyu_core::utils::convert::to_js;
use moyu_core::utils::patch::Patch;
use moyu_macros::Node;
use moyu_live2d_abi::command::options;
#[cfg(native)]
use moyu_live2d_abi::command::priority;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::event::Live2dEvent;
use crate::job::{self, NodeJob};
use crate::renderer::Live2dGpu;

/// Default look target: straight ahead.
pub const LOOK_CENTER: (f32, f32) = (0.0, 0.0);

/// Live2D node: renders a `.model3.json` model through the Live2D plugin.
#[derive(Debug, Node)]
pub struct Live2dNode {
    /// Model path, relative to `assets/`.
    pub src: Option<String>,
    pub physics: bool,
    pub pose: bool,
    pub blink: bool,
    pub breath: bool,
    pub look: bool,
    pub idle: bool,

    /// State shared with the plugin driver and the renderer.
    pub job: Arc<NodeJob>,

    /// GPU resources, owned by the renderer.
    pub gpu: Option<Box<Live2dGpu>>,

    #[base]
    node_base: NodeBase,
}

impl Live2dNode {
    pub fn new(label: String) -> Self {
        let node_base = NodeBase::new(label);
        let job = job::register(*node_base.id());
        Self {
            src: None,
            physics: true,
            pose: true,
            blink: true,
            breath: true,
            look: false,
            idle: true,
            job,
            gpu: None,
            node_base,
        }
    }

    /// Packs the option props into the plugin's option bits.
    fn option_bits(&self) -> u32 {
        let mut bits = 0;
        if self.physics {
            bits |= options::PHYSICS;
        }
        if self.pose {
            bits |= options::POSE;
        }
        if self.blink {
            bits |= options::BLINK;
        }
        if self.breath {
            bits |= options::BREATH;
        }
        if self.look {
            bits |= options::LOOK;
        }
        if self.idle {
            bits |= options::IDLE;
        }
        bits
    }

    /// Publishes the wanted state to the driver.
    fn sync_state(&self) {
        let mut state = self.job.state.lock();
        state.src = self.src.clone();
        state.options = self.option_bits();
        if !self.look {
            state.look_target = LOOK_CENTER;
        }
    }

    /// The loaded model, or a message explaining why there is none.
    fn loaded_model(&self) -> Result<u32> {
        let model = self.job.model();
        if model == 0 {
            return Err(anyhow!("the live2d node has no loaded model"));
        }
        Ok(model)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct Live2dProps {
    #[ts(optional = false)]
    pub src: Patch<String>,
    pub physics: Patch<bool>,
    pub pose: Patch<bool>,
    pub blink: Patch<bool>,
    pub breath: Patch<bool>,
    pub look: Patch<bool>,
    pub idle: Patch<bool>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
#[ts(export, optional_fields)]
pub enum Live2dCommand {
    /// Plays a motion; `priority` defaults to Normal, negative fades keep the
    /// model3.json value.
    #[serde(rename_all = "camelCase")]
    StartMotion {
        group: String,
        index: i32,
        #[ts(optional)]
        priority: Option<i32>,
        #[ts(optional)]
        fade_in: Option<f32>,
        #[ts(optional)]
        fade_out: Option<f32>,
        #[serde(rename = "loop")]
        #[ts(optional)]
        loop_motion: Option<bool>,
    },
    StopAllMotions,
    SetExpression {
        name: String,
    },
    StopExpression,
    SetParameter {
        id: String,
        value: f32,
    },
    SetParameters {
        values: HashMap<String, f32>,
    },
    ResetParameters,
    SetPartOpacity {
        id: String,
        value: f32,
    },
    /// Hit test in canvas pixel coordinates; returns whether the point is
    /// inside the named hit area.
    #[serde(rename_all = "camelCase")]
    HitTest { area: String, x: f32, y: f32 },
}

impl Node for Live2dNode {
    fn create_instance(label: Option<String>) -> Result<Box<dyn Node>>
    where
        Self: Sized,
    {
        Ok(Box::new(Self::new(label.unwrap_or_default())))
    }

    #[inline]
    fn node_type(&self) -> &'static str {
        "live2d"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        let props: Live2dProps = from_js(props).unwrap();

        apply_patch!(props.src => |src| {
            // An empty path is how a reset arrives; the node then shows nothing.
            self.src = (!src.is_empty()).then_some(src);
        }, String::new());
        apply_patch!(props.physics => |v| self.physics = v, true);
        apply_patch!(props.pose => |v| self.pose = v, true);
        apply_patch!(props.blink => |v| self.blink = v, true);
        apply_patch!(props.breath => |v| self.breath = v, true);
        apply_patch!(props.look => |v| self.look = v, false);
        apply_patch!(props.idle => |v| self.idle = v, true);

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

impl NodeEventSource for Live2dNode {
    type Event = Live2dEvent;
}

impl Focusable for Live2dNode {
    /// Tracks the pointer over the node as the look target, so the model can
    /// follow it while the `look` option is on.
    fn pointer_event(&self, x: f32, y: f32, kind: PointerEventKind) {
        if !self.look {
            return;
        }
        let (width, height) = self.base().layout_size();
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let target = match kind {
            PointerEventKind::Over => [
                (x / width) * 2.0 - 1.0,
                1.0 - (y / height) * 2.0,
            ],
            PointerEventKind::Leave => [LOOK_CENTER.0, LOOK_CENTER.1],
            _ => return,
        };

        self.job.state.lock().look_target = (target[0], target[1]);
    }
}

impl Command for Live2dNode {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let command: Live2dCommand = from_js(payload)?;
        let model = self.loaded_model()?;

        #[cfg(native)]
        {
            let Some(context) = crate::plugin::context() else {
                return Err(anyhow!("the Live2D plugin is not available"));
            };
            match command {
                Live2dCommand::StartMotion {
                    group,
                    index,
                    priority,
                    fade_in,
                    fade_out,
                    loop_motion,
                } => {
                    context.start_motion(
                        model,
                        &group,
                        index,
                        priority.unwrap_or(priority::NORMAL),
                        fade_in.unwrap_or(-1.0),
                        fade_out.unwrap_or(-1.0),
                        loop_motion.unwrap_or(false),
                    )?;
                    Ok(None)
                }
                Live2dCommand::StopAllMotions => {
                    context.stop_all_motions(model)?;
                    Ok(None)
                }
                Live2dCommand::SetExpression { name } => {
                    context.set_expression(model, &name)?;
                    Ok(None)
                }
                Live2dCommand::StopExpression => {
                    context.stop_expression(model)?;
                    Ok(None)
                }
                Live2dCommand::SetParameter { id, value } => {
                    context.set_parameters(model, &[parameter_pair(&id, value)])?;
                    Ok(None)
                }
                Live2dCommand::SetParameters { values } => {
                    // Setting nothing is a no-op rather than an error; the plugin
                    // rejects empty batches by contract.
                    if values.is_empty() {
                        Ok(None)
                    } else {
                        let pairs = values
                            .iter()
                            .map(|(id, value)| parameter_pair(id, *value))
                            .collect::<Vec<_>>();
                        context.set_parameters(model, &pairs)?;
                        Ok(None)
                    }
                }
                Live2dCommand::ResetParameters => {
                    context.reset_parameters(model)?;
                    Ok(None)
                }
                Live2dCommand::SetPartOpacity { id, value } => {
                    context.set_part_opacity(model, &id, value)?;
                    Ok(None)
                }
                Live2dCommand::HitTest { area, x, y } => {
                    Ok(Some(to_js(&context.hit_test(model, &area, x, y)?)?))
                }
            }
        }

        #[cfg(web)]
        {
            let _ = model;
            let _ = command;
            Err(anyhow!(
                "the live2d node needs the native engine; the web runtime has no Live2D plugin"
            ))
        }
    }
}

/// Wraps one parameter override as the contract type.
#[cfg(native)]
fn parameter_pair(id: &str, value: f32) -> moyu_live2d_abi::command::ParameterPair {
    moyu_live2d_abi::command::ParameterPair {
        id: moyu_live2d_abi::types::Span::from_bytes(id.as_bytes()),
        value,
    }
}
