use std::time::Duration;

use anyhow::{Result, anyhow};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::*;

pub struct Audio {
    pub(crate) sound: Option<StaticSoundData>,
    pub(crate) handle: Option<StaticSoundHandle>,
    pub(crate) loading_state: AudioLoadingState,
    volume: f64,
}

#[derive(Debug)]
pub enum AudioLoadingState {
    Unloaded,
    Loading,
    Loaded,
    Failed,
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Audio {
    pub fn new() -> Self {
        Self {
            sound: None,
            handle: None,
            loading_state: AudioLoadingState::Unloaded,
            volume: 1.0,
        }
    }

    fn handle(&mut self) -> Result<&mut StaticSoundHandle> {
        if let Some(ref mut handle) = self.handle {
            Ok(handle)
        } else {
            Err(anyhow!("Sound not playing or not loaded"))
        }
    }

    /// Returns true if the sound is currently or has been played.
    pub fn played(&self) -> bool {
        self.handle.is_some()
    }

    pub fn play(&mut self, manager: &mut AudioManager, fade_time: Option<u32>) -> Result<()> {
        if let Some(ref sound_data) = self.sound {
            let mut handle = match manager.play(sound_data.clone()) {
                Ok(handle) => handle,
                Err(e) => {
                    log::error!("Failed to play sound: {}", e);
                    self.loading_state = AudioLoadingState::Failed;
                    return Err(e.into());
                }
            };
            handle.set_volume(
                Decibels::interpolate(Decibels::SILENCE, Decibels::IDENTITY, 0.0),
                tween(Some(0)),
            );
            handle.set_volume(
                Decibels::interpolate(Decibels::SILENCE, Decibels::IDENTITY, self.volume),
                tween(fade_time),
            );
            self.handle = Some(handle);
            Ok(())
        } else {
            Err(anyhow!("No sound data available to play"))
        }
    }

    pub fn stop(&mut self, fade_time: Option<u32>) -> Result<()> {
        self.handle().map(|handle| {
            handle.stop(tween(fade_time));
        })?;
        self.handle = None;
        Ok(())
    }

    pub fn pause(&mut self, fade_time: Option<u32>) -> Result<()> {
        self.handle().map(|handle| {
            handle.pause(tween(fade_time));
        })
    }

    pub fn resume(&mut self, fade_time: Option<u32>) -> Result<()> {
        let _ = self.handle().map(|handle| {
            if handle.state() != kira::sound::PlaybackState::Paused {
                return Err(anyhow!("Sound is not paused"));
            }
            handle.resume(tween(fade_time));
            Ok(())
        })?;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f64, fade_time: Option<u32>) -> Result<()> {
        self.volume = volume;
        self.handle().map(|handle| {
            handle.set_volume(
                Decibels::interpolate(Decibels::SILENCE, Decibels::IDENTITY, volume),
                tween(fade_time),
            );
        })
    }

    pub fn seek_by(&mut self, seconds: f64) -> Result<()> {
        self.handle().map(|handle| {
            handle.seek_by(seconds);
        })
    }

    pub fn seek_to(&mut self, seconds: f64) -> Result<()> {
        self.handle().map(|handle| {
            handle.seek_to(seconds);
        })
    }

    pub fn set_playback_rate(&mut self, rate: f64) -> Result<()> {
        self.handle().map(|handle| {
            handle.set_playback_rate(rate, Tween::default());
        })
    }

    pub fn set_loop_region(&mut self, start: f64, end: f64) -> Result<()> {
        self.handle().map(|handle| {
            handle.set_loop_region(start..end);
        })
    }

    pub fn set_panning(&mut self, panning: f64) -> Result<()> {
        self.handle().map(|handle| {
            handle.set_panning(Panning::from(panning as f32), Tween::default());
        })
    }
}

fn tween(fade_time: Option<u32>) -> Tween {
    if let Some(time) = fade_time {
        Tween {
            start_time: StartTime::Immediate,
            duration: Duration::from_millis(time as u64),
            easing: Easing::InOutPowf(2.),
        }
    } else {
        Tween::default()
    }
}
