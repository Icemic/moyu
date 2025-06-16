use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use kira::manager::backend::DefaultBackend;
use kira::manager::AudioManagerSettings;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::sound::{EndPosition, PlaybackPosition, Region};
use kira::StartTime;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use moyu_core::traits::Command;
use moyu_core::traits::Plugin;
use moyu_core::utils::convert::{create_promise, from_js, JSValue};
use moyu_pal::config::entry_dir;
use moyu_pal::sync::Mutex;

use crate::audio::{Audio, AudioLoadingState};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
    pub delay_time: Option<f64>,
    pub start_position: f64,
    pub reverse: bool,
    pub loop_region: Option<(f64, f64)>,
    pub volume: f64,
    pub playback_rate: f64,
    pub panning: f64,
    pub auto_play: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            delay_time: None,
            start_position: 0.0,
            reverse: false,
            loop_region: None,
            volume: 1.0,
            playback_rate: 1.0,
            panning: 0.5,
            auto_play: false,
        }
    }
}

pub struct AudioManager {
    manager: Arc<Mutex<kira::manager::AudioManager<DefaultBackend>>>,
    audios: HashMap<String, Arc<Mutex<Audio>>>,
}

impl AudioManager {
    pub fn new() -> Result<Self> {
        let manager =
            kira::manager::AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
        let manager = Arc::new(Mutex::new(manager));

        Ok(Self {
            manager,
            audios: HashMap::new(),
        })
    }

    pub fn create_audio(&mut self, name: &str) {
        if self.audios.contains_key(name) {
            warn!("Audio {} already exists", name);
            return;
        }
        self.audios
            .insert(name.to_string(), Arc::new(Mutex::new(Audio::new())));
    }

    pub fn remove_audio(&mut self, name: &str) {
        if let Some(audio) = self.audios.remove(name) {
            audio.lock().stop().unwrap();
        } else {
            warn!("Audio {} not found", name);
        }
    }

    pub fn load_audio(
        &mut self,
        name: &str,
        src: &str,
        settings: AudioSettings,
    ) -> impl Future<Output = Result<()>> + 'static {
        debug!("audio will load from {}", src);

        let audio = self.get_audio(name).unwrap();
        let manager = self.manager.clone();
        let asset_full_path = entry_dir().join("assets/").unwrap().join(src).unwrap();

        return async move {
            audio.lock().loading_state = AudioLoadingState::Loading;
            let file = match moyu_pal::fs::open(&asset_full_path).await {
                Ok(file) => file,
                Err(e) => {
                    log::error!("Failed to open file: {}", e);
                    audio.lock().loading_state = AudioLoadingState::Failed;
                    return Err(e);
                }
            };

            let sound_data = match StaticSoundData::from_cursor(file) {
                Ok(data) => data,
                Err(e) => {
                    log::error!("Failed to create sound data: {}", e);
                    audio.lock().loading_state = AudioLoadingState::Failed;
                    return Err(e.into());
                }
            };

            let sound_data = sound_data.with_settings(StaticSoundSettings {
                start_time: settings
                    .delay_time
                    .map(|v| StartTime::Delayed(Duration::from_secs_f64(v)))
                    .unwrap_or_default(),
                start_position: PlaybackPosition::Seconds(settings.start_position),
                reverse: settings.reverse,
                loop_region: settings.loop_region.map(|(start, end)| Region {
                    start: PlaybackPosition::Seconds(start),
                    end: if end == -1.0 {
                        EndPosition::EndOfAudio
                    } else {
                        EndPosition::Custom(PlaybackPosition::Seconds(end))
                    },
                }),
                volume: settings.volume.into(),
                playback_rate: settings.playback_rate.into(),
                panning: settings.panning.into(),
                ..Default::default()
            });

            let mut audio = audio.lock();
            audio.sound = Some(sound_data);
            audio.loading_state = AudioLoadingState::Loaded;

            // audio will play automatically by default, so if auto_play is false, stop it
            if settings.auto_play {
                let mut manager = manager.lock();
                audio.play(&mut manager).unwrap();
            }

            Ok(())
        };
    }

    pub fn get_audio(&self, name: &str) -> Option<Arc<Mutex<Audio>>> {
        self.audios.get(name).cloned()
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
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "subCommand"
)]
pub enum AudioCommmad {
    Load {
        name: String,
        src: String,
        settings: Option<AudioSettings>,
    },
    Release {
        name: String,
    },
    Play {
        name: String,
    },
    Stop {
        name: String,
    },
    Pause {
        name: String,
    },
    Resume {
        name: String,
    },
    SetVolume {
        name: String,
        volume: f64,
    },
    SeekBy {
        name: String,
        time: f64,
    },
    SeekTo {
        name: String,
        time: f64,
    },
    SetPlaybackRate {
        name: String,
        rate: f64,
    },
    SetLoopRegion {
        name: String,
        start: f64,
        end: f64,
    },
    SetPanning {
        name: String,
        panning: f64,
    },
}

impl Command for AudioManager {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>> {
        let payload: AudioCommmad = from_js(payload)?;
        log::info!("audio manager received: {:?}", payload);

        match payload {
            AudioCommmad::Load {
                name,
                src,
                settings,
            } => {
                self.create_audio(&name);
                let fut = self.load_audio(&name, &src, settings.unwrap_or_default());
                let promise = create_promise(fut)?;
                return Ok(Some(promise));
            }
            AudioCommmad::Release { name } => {
                self.remove_audio(&name);
            }
            AudioCommmad::Play { name } => {
                let audio = self.get_audio(&name).unwrap();
                let mut audio = audio.lock();
                // ignore the error if audio is not playing or not loaded
                let _ = audio.stop();
                let mut manager = self.manager.lock();
                audio.play(&mut manager)?;
            }
            AudioCommmad::Stop { name } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().stop()?;
            }
            AudioCommmad::Pause { name } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().pause()?;
            }
            AudioCommmad::Resume { name } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().resume()?;
            }
            AudioCommmad::SetVolume { name, volume } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().set_volume(volume)?;
            }
            AudioCommmad::SeekBy { name, time } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().seek_by(time)?;
            }
            AudioCommmad::SeekTo { name, time } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().seek_to(time)?;
            }
            AudioCommmad::SetPlaybackRate { name, rate } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().set_playback_rate(rate)?;
            }
            AudioCommmad::SetLoopRegion { name, start, end } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().set_loop_region(start, end)?;
            }
            AudioCommmad::SetPanning { name, panning } => {
                let audio = self.get_audio(&name).unwrap();
                audio.lock().set_panning(panning)?;
            }
        }

        Ok(None)
    }
}
