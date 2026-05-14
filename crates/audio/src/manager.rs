use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use anyhow::Result;
use kira::sound::static_sound::StaticSoundSettings;
use kira::sound::{EndPosition, PlaybackPosition, Region};
use kira::{AudioManagerSettings, DefaultBackend, Panning, StartTime};
use log::{debug, warn};
use serde::{Deserialize, Serialize};

use moyu_core::traits::Command;
use moyu_core::traits::Plugin;
use moyu_core::traits::PluginBaseTrait;
use moyu_core::utils::convert::{JSValue, create_promise, from_js};
use moyu_macros::Plugin;
use moyu_pal::dir::assets_dir;
use moyu_pal::sync::Mutex;
use ts_rs::TS;

use crate::audio::{Audio, AudioLoadingState};
use crate::kira_static_data::from_boxed_media_source;
use crate::utils::linear_volume;
use crate::wildcard::{is_wildcard, wildcard_match};

// The audio plugin is instantiated once for the app lifetime, so module-level
// global volumes remain a single source of truth for async load/play paths.
enum GlobalVolumeRule {
    Exact(String, f64),
    Wildcard(String, f64),
}

static GLOBAL_VOLUMES: LazyLock<StdMutex<Vec<GlobalVolumeRule>>> =
    LazyLock::new(|| StdMutex::new(Vec::new()));

fn get_global_volume(name: &str) -> f64 {
    let rules = GLOBAL_VOLUMES.lock().unwrap();
    let mut matched_wildcard = None;

    for rule in rules.iter().rev() {
        match rule {
            GlobalVolumeRule::Exact(rule_name, volume) if rule_name == name => return *volume,
            GlobalVolumeRule::Wildcard(pattern, volume)
                if matched_wildcard.is_none() && wildcard_match(pattern, name) =>
            {
                matched_wildcard = Some(*volume);
            }
            _ => {}
        }
    }

    matched_wildcard.unwrap_or(1.0)
}

fn set_global_volume(name: &str, volume: f64) {
    let mut rules = GLOBAL_VOLUMES.lock().unwrap();

    if is_wildcard(name) {
        rules.retain(
            |rule| !matches!(rule, GlobalVolumeRule::Wildcard(pattern, _) if pattern == name),
        );
        rules.push(GlobalVolumeRule::Wildcard(name.to_string(), volume));
    } else {
        rules.retain(
            |rule| !matches!(rule, GlobalVolumeRule::Exact(rule_name, _) if rule_name == name),
        );
        rules.push(GlobalVolumeRule::Exact(name.to_string(), volume));
    }
}

/// Settings for audio playback, including delay, start position, volume, etc.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[derive(TS)]
#[ts(export, optional_fields)]
pub struct AudioSettings {
    /// Optional delay time in seconds before the audio starts playing.
    pub delay_time: Option<f64>,
    /// The position in seconds where the audio should start playing.
    #[ts(as = "Option<f64>")]
    pub start_position: f64,
    /// Whether the sound should be played in reverse.
    #[ts(as = "Option<bool>")]
    pub reverse: bool,
    /// Loop region defined by a start and end time in seconds. `-1` for end means loop to the end of the audio.
    pub loop_region: Option<(f64, f64)>,
    /// Volume of the audio, ranges from 0.0 (silence) to 1.0 (normal volume),
    /// values greater than 1.0 can be used for amplification.
    #[ts(as = "Option<f64>")]
    pub volume: f64,
    /// Playback rate of the audio, where 1.0 is normal speed.
    #[ts(as = "Option<f64>")]
    pub playback_rate: f64,
    /// Panning value from -1.0 (left) to 1.0 (right), with 0.0 being center.
    #[ts(as = "Option<f64>")]
    pub panning: f64,
    /// Whether the audio should start playing automatically after loading.
    #[ts(as = "Option<bool>")]
    pub auto_play: bool,
    /// Optional fade time in milliseconds for audio transitions.
    pub fade_time: Option<u32>,
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
            fade_time: None,
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

    pub fn create_audio(&mut self, name: &str) -> Result<Arc<Mutex<Audio>>> {
        if is_wildcard(name) {
            return Err(anyhow::anyhow!(
                "Audio name {} cannot be a wildcard pattern",
                name
            ));
        }

        if let Some(old) = self.audios.remove(name) {
            warn!("Audio {} already exists, stopping it", name);
            let mut old = old.lock();
            old.stale = true;
            old.stop(None).ok();
        }

        let audio = Arc::new(Mutex::new(Audio::new()));
        self.audios.insert(name.to_string(), audio.clone());

        Ok(audio)
    }

    pub fn remove_audio(
        &mut self,
        name: &str,
        fade_time: Option<u32>,
        silent_fail: bool,
    ) -> Result<()> {
        let audio_names = self
            .get_audios(name)
            .into_iter()
            .map(|(audio_name, _)| audio_name)
            .collect::<Vec<_>>();

        for audio_name in audio_names {
            if let Some(audio) = self.audios.remove(&audio_name) {
                let mut audio = audio.lock();
                audio.stale = true;
                if audio.played()
                    && let Err(err) = audio.stop(fade_time)
                {
                    warn!("Failed to stop audio {}: {}", audio_name, err);
                }
            } else if !silent_fail {
                return Err(anyhow::anyhow!("Audio {} not found", audio_name));
            }
        }

        Ok(())
    }

    pub fn load_audio(
        &mut self,
        name: &str,
        src: &str,
        settings: AudioSettings,
    ) -> Result<impl Future<Output = Result<()>> + 'static> {
        debug!("audio will load from {}", src);

        let audio = self.create_audio(name)?;
        let audio_name = name.to_string();
        let manager = self.manager.clone();
        let asset_full_path = assets_dir().join(src)?;

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
                volume: linear_volume(0.0).into(),
                playback_rate: settings.playback_rate.into(),
                panning: Panning::from(settings.panning as f32).into(),
                ..Default::default()
            });

            let mut audio = audio.lock();
            if audio.stale {
                return Ok(());
            }

            audio.sound = Some(sound_data);
            audio.loading_state = AudioLoadingState::Loaded;
            audio.volume = settings.volume;

            // Auto-play after loading when requested.
            if settings.auto_play {
                let global_volume = get_global_volume(&audio_name);
                let mut manager = manager.lock();
                audio.play(&mut manager, settings.fade_time, global_volume, None)?;
            }

            Ok(())
        });
    }

    pub fn get_audios(&self, name: &str) -> Vec<(String, Arc<Mutex<Audio>>)> {
        if !is_wildcard(name) {
            return self
                .audios
                .get(name)
                .cloned()
                .map(|audio| vec![(name.to_string(), audio)])
                .unwrap_or(vec![]);
        }

        self.audios
            .iter()
            .filter(|(audio_name, _)| wildcard_match(name, audio_name))
            .map(|(audio_name, audio)| (audio_name.clone(), audio.clone()))
            .collect::<Vec<_>>()
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
#[derive(TS)]
#[ts(export, optional_fields)]
pub enum AudioCommand {
    Load {
        name: String,
        src: String,
        settings: Option<AudioSettings>,
    },
    /// Load if not loaded then play, or play if already loaded.
    LoadAndPlay {
        name: String,
        src: String,
        settings: Option<AudioSettings>,
    },
    Release {
        name: String,
        fade_time: Option<u32>,
        silent_fail: Option<bool>,
    },
    Play {
        name: String,
        fade_time: Option<u32>,
        wait_for_end: Option<bool>,
    },
    Stop {
        name: String,
        fade_time: Option<u32>,
    },
    Pause {
        name: String,
        fade_time: Option<u32>,
    },
    Resume {
        name: String,
        fade_time: Option<u32>,
    },
    SetVolume {
        name: String,
        volume: f64,
        fade_time: Option<u32>,
    },
    SetGlobalVolume {
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
        let payload: AudioCommand = from_js(payload)?;
        log::debug!("audio manager received: {:?}", payload);

        match payload {
            AudioCommand::Load {
                name,
                src,
                settings,
            } => {
                let fut = self.load_audio(&name, &src, settings.unwrap_or_default())?;
                let promise = create_promise(fut)?;
                return Ok(Some(promise));
            }
            AudioCommand::LoadAndPlay {
                name,
                src,
                settings,
            } => {
                if is_wildcard(&name) {
                    return Err(anyhow::anyhow!(
                        "Audio name {} cannot be a wildcard pattern",
                        name
                    ));
                }

                let mut settings = settings.unwrap_or_default();
                settings.auto_play = true;

                if !self.audios.contains_key(&name) {
                    let fut = self.load_audio(&name, &src, settings)?;
                    let promise = create_promise(fut)?;
                    return Ok(Some(promise));
                }

                let audios = self.get_audios(&name);
                let Some((audio_name, audio)) = audios.first() else {
                    return Ok(None);
                };

                let mut manager = self.manager.lock();
                let global_volume = get_global_volume(&audio_name);
                let mut audio = audio.lock();
                let _ = audio.stop(settings.fade_time);
                audio.play(&mut manager, settings.fade_time, global_volume, None)?;

                return Ok(None);
            }
            AudioCommand::Release {
                name,
                fade_time,
                silent_fail,
            } => {
                self.remove_audio(&name, fade_time, silent_fail.unwrap_or(false))?;
            }
            AudioCommand::Play {
                name,
                fade_time,
                wait_for_end,
            } => {
                let matched_audios = self.get_audios(&name);
                let mut manager = self.manager.lock();

                if wait_for_end.unwrap_or(false) {
                    let mut receivers = Vec::with_capacity(matched_audios.len());
                    for (audio_name, audio) in matched_audios {
                        let global_volume = get_global_volume(&audio_name);
                        let mut audio = audio.lock();
                        let _ = audio.stop(fade_time);

                        let (sender, receiver) = moyu_pal::sync::oneshot::channel();
                        audio.play(
                            &mut manager,
                            fade_time,
                            global_volume,
                            Some(Box::new(move || {
                                let _ = sender.send(());
                            })),
                        )?;
                        receivers.push(receiver);
                    }

                    let promise = create_promise(async move {
                        for receiver in receivers {
                            receiver.await?;
                        }
                        Ok(())
                    })?;
                    return Ok(Some(promise));
                } else {
                    for (audio_name, audio) in matched_audios {
                        let global_volume = get_global_volume(&audio_name);
                        let mut audio = audio.lock();
                        let _ = audio.stop(fade_time);
                        audio.play(&mut manager, fade_time, global_volume, None)?;
                    }
                }
            }
            AudioCommand::Stop { name, fade_time } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().stop(fade_time)?;
                }
            }
            AudioCommand::Pause { name, fade_time } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().pause(fade_time)?;
                }
            }
            AudioCommand::Resume { name, fade_time } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().resume(fade_time)?;
                }
            }
            AudioCommand::SetVolume {
                name,
                volume,
                fade_time,
            } => {
                for (audio_name, audio) in self.get_audios(&name) {
                    let global_volume = get_global_volume(&audio_name);
                    audio.lock().set_volume(volume, global_volume, fade_time)?;
                }
            }
            AudioCommand::SetGlobalVolume { name, volume } => {
                let clamped = volume.clamp(0.0, 1.0);
                set_global_volume(&name, clamped);

                if is_wildcard(&name) {
                    let matched_audios = self
                        .audios
                        .iter()
                        .filter(|(audio_name, _)| wildcard_match(&name, audio_name))
                        .map(|(audio_name, audio)| (audio_name.clone(), audio.clone()))
                        .collect::<Vec<_>>();

                    for (audio_name, audio) in matched_audios {
                        let mut audio = audio.lock();
                        if audio.played() {
                            let own_volume = audio.volume;
                            let global_volume = get_global_volume(&audio_name);
                            audio.set_volume(own_volume, global_volume, None)?;
                        }
                    }
                } else if let Some(audio) = self.audios.get(&name) {
                    let mut audio = audio.lock();
                    if audio.played() {
                        let own_volume = audio.volume;
                        let global_volume = get_global_volume(&name);
                        audio.set_volume(own_volume, global_volume, None)?;
                    }
                }
            }
            AudioCommand::SeekBy { name, time } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().seek_by(time)?;
                }
            }
            AudioCommand::SeekTo { name, time } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().seek_to(time)?;
                }
            }
            AudioCommand::SetPlaybackRate { name, rate } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().set_playback_rate(rate)?;
                }
            }
            AudioCommand::SetLoopRegion { name, start, end } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().set_loop_region(start, end)?;
                }
            }
            AudioCommand::SetPanning { name, panning } => {
                for (_, audio) in self.get_audios(&name) {
                    audio.lock().set_panning(panning)?;
                }
            }
        }

        Ok(None)
    }
}
