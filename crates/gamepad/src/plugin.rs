use std::collections::HashMap;
use std::u16;

use anyhow::Result;
use doufu_core::traits::{Command, Plugin, PluginEventSource};
use doufu_core::utils::convert::{from_js, to_js, JSValue};
use doufu_pal::time::Instant;
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Repeat, Replay, Ticks};
use gilrs::{Event, Gilrs};
use serde::{Deserialize, Serialize};

use crate::events::GamepadEvent;
use crate::gamepad::{Gamepad, GamepadHapticActuator};
use crate::utils::{get_w3c_axis, get_w3c_button};

pub struct GamepadPlugin {
    gilrs: Gilrs,
    gamepads: HashMap<u32, Gamepad>,
    instant: Instant,
    effects_cache: HashMap<u32, gilrs::ff::Effect>,
}

unsafe impl Send for GamepadPlugin {}
unsafe impl Sync for GamepadPlugin {}

impl GamepadPlugin {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().unwrap();

        Self {
            gilrs,
            gamepads: Default::default(),
            instant: Instant::now(),
            effects_cache: Default::default(),
        }
    }
}

impl Plugin for GamepadPlugin {
    fn plugin_name(&self) -> &'static str {
        "gamepad"
    }

    fn update(&mut self, _vsync: bool) {
        let mut last_pressed = vec![];
        let mut last_released = vec![];

        while let Some(Event { id, event, .. }) = self.gilrs.next_event() {
            let Ok(index) = id.to_string().parse::<u32>() else {
                log::warn!("failed to parse gamepad id to number: {:?}", id);
                return;
            };

            match event {
                gilrs::EventType::ButtonPressed(button, ..) => {
                    let Some(gamepad) = self.gamepads.get_mut(&index) else {
                        log::warn!("gamepad not found: {:?}", index);
                        return;
                    };

                    let w3c_button = get_w3c_button(button);

                    if w3c_button > 255 {
                        log::warn!("invalid button: {:?}", button);
                        return;
                    }

                    let Some(button) = gamepad.buttons.get_mut(w3c_button as usize) else {
                        log::warn!("button not found: {:?}", button);
                        return;
                    };

                    button.pressed = true;
                    button.touched = true;

                    gamepad.timestamp = self.instant.elapsed().as_secs_f64() * 1000.;

                    // we just push the button to the last_pressed array instead of sending the event
                    // because we have to solve ButtonChanged event for button.value, we'll send the event
                    // after that
                    last_pressed.push(w3c_button);
                }
                gilrs::EventType::ButtonReleased(button, ..) => {
                    let Some(gamepad) = self.gamepads.get_mut(&index) else {
                        log::warn!("gamepad not found: {:?}", index);
                        return;
                    };

                    let w3c_button = get_w3c_button(button);

                    if w3c_button > 255 {
                        log::warn!("invalid button: {:?}", button);
                        return;
                    }

                    let Some(button) = gamepad.buttons.get_mut(w3c_button as usize) else {
                        log::warn!("button not found: {:?}", button);
                        return;
                    };

                    button.pressed = false;
                    button.touched = false;

                    gamepad.timestamp = self.instant.elapsed().as_secs_f64() * 1000.;

                    // we just push the button to the last_released array instead of sending the event
                    // because we have to solve ButtonChanged event for button.value, we'll send the event
                    // after that
                    last_released.push(w3c_button);
                }
                gilrs::EventType::ButtonRepeated(..) => {
                    // do nothing
                }
                gilrs::EventType::ButtonChanged(button, value, ..) => {
                    let Some(gamepad) = self.gamepads.get_mut(&index) else {
                        log::warn!("gamepad not found: {:?}", index);
                        return;
                    };

                    let w3c_button = get_w3c_button(button);

                    if w3c_button > 255 {
                        log::warn!("invalid button: {:?}", button);
                        return;
                    }

                    let Some(button) = gamepad.buttons.get_mut(w3c_button as usize) else {
                        log::warn!("button not found: {:?}", button);
                        return;
                    };

                    button.value = value;

                    gamepad.timestamp = self.instant.elapsed().as_secs_f64() * 1000.;

                    let button = button.clone();
                    let gamepad = gamepad.clone();

                    if last_pressed.contains(&w3c_button) {
                        last_pressed.retain(|&x| x != w3c_button);
                        self.send_event(
                            "gamepadbuttondown",
                            GamepadEvent::ButtonChanged { gamepad, button },
                        );
                    } else if last_released.contains(&w3c_button) {
                        last_released.retain(|&x| x != w3c_button);
                        self.send_event(
                            "gamepadbuttonup",
                            GamepadEvent::ButtonChanged { gamepad, button },
                        );
                    } else {
                        self.send_event(
                            "gamepadbuttonchanged",
                            GamepadEvent::ButtonChanged { gamepad, button },
                        );
                    }
                }
                gilrs::EventType::AxisChanged(axis, value, ..) => {
                    let Some(gamepad) = self.gamepads.get_mut(&index) else {
                        log::warn!("gamepad not found: {:?}", index);
                        return;
                    };

                    let w3c_axis = get_w3c_axis(axis);

                    if w3c_axis > 255 {
                        log::warn!("invalid axis: {:?}", axis);
                        return;
                    }

                    gamepad.axes[axis as usize] = value;

                    gamepad.timestamp = self.instant.elapsed().as_secs_f64() * 1000.;

                    let gamepad = gamepad.clone();

                    self.send_event(
                        "gamepadaxischanged",
                        GamepadEvent::AxisChanged {
                            gamepad,
                            axis: w3c_axis,
                            value,
                        },
                    );
                }
                gilrs::EventType::Connected => {
                    let mut gamepad = Gamepad::default();
                    let gilrs_gamepad = self.gilrs.gamepad(id);
                    gamepad.id = gilrs_gamepad.name().to_owned();
                    gamepad.index = index;
                    gamepad.connected = true;
                    gamepad.mapping = "standard".to_owned();
                    gamepad.timestamp = self.instant.elapsed().as_secs_f64() * 1000.;
                    gamepad.vibration_actuator = if gilrs_gamepad.is_ff_supported() {
                        Some(GamepadHapticActuator::default())
                    } else {
                        None
                    };
                    if let Some(old_gamepad) = self.gamepads.insert(index, gamepad.clone()) {
                        log::warn!("gamepad already exists: {:?}", old_gamepad);
                    };
                    self.send_event("gamepadconnected", GamepadEvent::Connected { gamepad });
                }
                gilrs::EventType::Disconnected => {
                    let gamepad = self.gamepads.remove(&index);
                    if let Some(gamepad) = gamepad {
                        log::info!("gamepad disconnected: {:?}", gamepad);
                        self.send_event(
                            "gamepaddisconnected",
                            GamepadEvent::Disconnected { gamepad },
                        );
                    } else {
                        log::warn!("gamepad not found: {:?}", index);
                    }
                }
                gilrs::EventType::Dropped => {
                    // do nothing
                }
                gilrs::EventType::ForceFeedbackEffectCompleted => {
                    self.effects_cache.remove(&index);
                }
                _ => todo!(),
            }
        }
    }

    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "subCommand"
)]
enum GamepadCommmad {
    GetGamepads,
    /// see https://developer.mozilla.org/en-US/docs/Web/API/GamepadHapticActuator/playEffect
    PlayEffect {
        index: u32,
        effect: String,
        #[serde(default)]
        params: EffectParams,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct EffectParams {
    duration: u32,
    start_delay: u32,
    weak_magnitude: f32,
    strong_magnitude: f32,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            duration: 0,
            start_delay: 0,
            weak_magnitude: 0.0,
            strong_magnitude: 0.0,
        }
    }
}

impl Command for GamepadPlugin {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: GamepadCommmad = from_js(payload)?;

        match payload {
            GamepadCommmad::GetGamepads => {
                let mut gamepads: Vec<Option<Gamepad>> = vec![];

                for (index, gamepad) in self.gamepads.iter() {
                    if gamepads.len() <= *index as usize {
                        gamepads.resize(*index as usize + 1, None);
                    }

                    gamepads[*index as usize] = Some(gamepad.clone());
                }

                return Ok(Some(to_js(&gamepads)?));
            }
            GamepadCommmad::PlayEffect {
                index,
                effect: _,
                params,
            } => {
                if !self.gamepads.contains_key(&index) {
                    log::warn!("gamepad not found: {:?}", index);
                    return Ok(None);
                };

                if let Some((gamepad_id, _)) = self
                    .gilrs
                    .gamepads()
                    .find(|(id, _)| id.to_string() == index.to_string())
                {
                    let strong_effect = BaseEffect {
                        kind: BaseEffectType::Strong {
                            magnitude: (params.strong_magnitude.clamp(0., 1.) * u16::MAX as f32)
                                .floor() as u16,
                        },
                        scheduling: Replay {
                            after: Ticks::from_ms(params.start_delay),
                            play_for: Ticks::from_ms(params.duration),
                            ..Default::default()
                        },
                        envelope: Default::default(),
                    };

                    let weak_effect = BaseEffect {
                        kind: BaseEffectType::Weak {
                            magnitude: (params.weak_magnitude.clamp(0., 1.) * u16::MAX as f32)
                                .floor() as u16,
                        },
                        scheduling: Replay {
                            after: Ticks::from_ms(params.start_delay),
                            play_for: Ticks::from_ms(params.duration),
                            ..Default::default()
                        },
                        envelope: Default::default(),
                    };

                    let effect = EffectBuilder::new()
                        .add_effect(strong_effect)
                        .add_effect(weak_effect)
                        .gamepads(&[gamepad_id])
                        .repeat(Repeat::For(Ticks::from_ms(params.duration)))
                        .finish(&mut self.gilrs)?;

                    effect.play()?;

                    // effect must not be dropped during the play
                    self.effects_cache.insert(index, effect);
                } else {
                    log::warn!("gamepad not found in gilrs: {:?}", index);
                }

                Ok(None)
            }
        }
    }
}

impl PluginEventSource for GamepadPlugin {
    type Event = GamepadEvent;
}
