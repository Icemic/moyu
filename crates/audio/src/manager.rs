use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use kira::sound::static_sound::StaticSoundSettings;
use kira::sound::{EndPosition, PlaybackPosition, Region};
use kira::{AudioManagerSettings, Decibels, DefaultBackend, Panning, StartTime, Tweenable};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use moyu_core::traits::Command;
use moyu_core::traits::Plugin;
use moyu_core::traits::PluginBaseTrait;
use moyu_core::utils::convert::{JSValue, create_promise, from_js};
use moyu_macros::Plugin;
use moyu_pal::dir::entry_dir;
use moyu_pal::sync::Mutex;

use crate::audio::{Audio, AudioLoadingState};
use crate::kira_static_data::from_boxed_media_source;

/// Settings for audio playback, including delay, start position, volume, etc.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
    /// Optional delay time in seconds before the audio starts playing.
    pub delay_time: Option<f64>,
    /// The position in seconds where the audio should start playing.
    pub start_position: f64,
    /// Whether the sound should be played in reverse.
    pub reverse: bool,
    /// Loop region defined by a start and end time in seconds. `-1` for end means loop to the end of the audio.
    pub loop_region: Option<(f64, f64)>,
    /// Volume of the audio, ranges from 0.0 (silence) to 1.0 (normal volume),
    /// values greater than 1.0 can be used for amplification.
    pub volume: f64,
    /// Playback rate of the audio, where 1.0 is normal speed.
    pub playback_rate: f64,
    /// Panning value from -1.0 (left) to 1.0 (right), with 0.0 being center.
    pub panning: f64,
    /// Whether the audio should start playing automatically after loading.
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
            panning: 0.0,
            auto_play: false,
        }
    }
}

#[derive(Plugin)]
pub struct AudioManager {
    manager: Arc<Mutex<kira::AudioManager<DefaultBackend>>>,
    audios: HashMap<String, Arc<Mutex<Audio>>>,
}

impl AudioManager {
    pub fn new() -> Result<Self> {
        let manager = kira::AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
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
            if let Err(err) = audio.lock().stop() {
                warn!("Failed to stop audio {}: {}", name, err);
            }
        } else {
            warn!("Audio {} not found", name);
        }
    }

    pub fn load_audio(
        &mut self,
        name: &str,
        src: &str,
        settings: AudioSettings,
    ) -> Result<impl Future<Output = Result<()>> + 'static> {
        debug!("audio will load from {}", src);

        self.create_audio(&name);
        let audio = self.get_audio(name)?;
        let manager = self.manager.clone();
        let asset_full_path = entry_dir().join("assets/")?.join(src)?;

        return Ok(async move {
            audio.lock().loading_state = AudioLoadingState::Loading;
            let file = match moyu_pal::fs::open(&asset_full_path).await {
                Ok(file) => file,
                Err(e) => {
                    log::error!("Failed to open file: {}", e);
                    audio.lock().loading_state = AudioLoadingState::Failed;
                    return Err(e);
                }
            };

            let sound_data = match from_boxed_media_source(Box::new(file)) {
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
                volume: Decibels::interpolate(
                    Decibels::SILENCE,
                    Decibels::IDENTITY,
                    settings.volume,
                )
                .into(),
                playback_rate: settings.playback_rate.into(),
                panning: Panning::from(settings.panning as f32).into(),
                ..Default::default()
            });

            let mut audio = audio.lock();
            audio.sound = Some(sound_data);
            audio.loading_state = AudioLoadingState::Loaded;

            // audio will play automatically by default, so if auto_play is false, stop it
            if settings.auto_play {
                let mut manager = manager.lock();
                audio.play(&mut manager)?;
            }

            Ok(())
        });
    }

    pub fn get_audio(&self, name: &str) -> Result<Arc<Mutex<Audio>>> {
        self.audios
            .get(name)
            .cloned()
            .ok_or(anyhow::anyhow!("Audio {} not found", name))
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
        log::debug!("audio manager received: {:?}", payload);

        match payload {
            AudioCommmad::Load {
                name,
                src,
                settings,
            } => {
                let fut = self.load_audio(&name, &src, settings.unwrap_or_default())?;
                let promise = create_promise(fut)?;
                return Ok(Some(promise));
            }
            AudioCommmad::Release { name } => {
                self.remove_audio(&name);
            }
            AudioCommmad::Play { name } => {
                let audio = self.get_audio(&name)?;
                let mut audio = audio.lock();
                // ignore the error if audio is not playing or not loaded
                let _ = audio.stop();
                let mut manager = self.manager.lock();
                audio.play(&mut manager)?;
            }
            AudioCommmad::Stop { name } => {
                let audio = self.get_audio(&name)?;
                audio.lock().stop()?;
            }
            AudioCommmad::Pause { name } => {
                let audio = self.get_audio(&name)?;
                audio.lock().pause()?;
            }
            AudioCommmad::Resume { name } => {
                let audio = self.get_audio(&name)?;
                audio.lock().resume()?;
            }
            AudioCommmad::SetVolume { name, volume } => {
                let audio = self.get_audio(&name)?;
                audio.lock().set_volume(volume)?;
            }
            AudioCommmad::SeekBy { name, time } => {
                let audio = self.get_audio(&name)?;
                audio.lock().seek_by(time)?;
            }
            AudioCommmad::SeekTo { name, time } => {
                let audio = self.get_audio(&name)?;
                audio.lock().seek_to(time)?;
            }
            AudioCommmad::SetPlaybackRate { name, rate } => {
                let audio = self.get_audio(&name)?;
                audio.lock().set_playback_rate(rate)?;
            }
            AudioCommmad::SetLoopRegion { name, start, end } => {
                let audio = self.get_audio(&name)?;
                audio.lock().set_loop_region(start, end)?;
            }
            AudioCommmad::SetPanning { name, panning } => {
                let audio = self.get_audio(&name)?;
                audio.lock().set_panning(panning)?;
            }
        }

        Ok(None)
    }
}
