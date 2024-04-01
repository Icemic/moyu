use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use hai_core::traits::{Command, Plugin};
use hai_core::utils::convert::{from_js, JSValue};

use crate::audio::Audio;

pub struct AudioManager {
    audios: HashMap<String, Audio>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            audios: HashMap::new(),
        }
    }

    pub fn create_audio(&mut self, name: &str) {
        self.audios.insert(name.to_string(), Audio::new());
    }

    pub fn get_audio(&self, name: &str) -> Option<&Audio> {
        self.audios.get(name)
    }
}

impl Plugin for AudioManager {
    fn plugin_name(&self) -> &'static str {
        "audio"
    }
    fn as_command(&mut self) -> Option<&mut dyn Command> {
        Some(self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "subCommand")]
pub enum AudioCommmad {
    Play,
}

impl Command for AudioManager {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        log::info!("dsfsdfsdfsdf");
        let payload: AudioCommmad = from_js(payload)?;
        log::info!("Text received: {:?}", payload);

        Ok(None)
    }
}
