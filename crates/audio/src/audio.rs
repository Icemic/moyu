use anyhow::{Result, anyhow};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::*;

pub struct Audio {
    pub(crate) sound: Option<StaticSoundData>,
    pub(crate) handle: Option<StaticSoundHandle>,
    pub(crate) loading_state: AudioLoadingState,
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

    pub fn play(&mut self, manager: &mut AudioManager) -> Result<()> {
        if let Some(ref sound_data) = self.sound {
            let handle = match manager.play(sound_data.clone()) {
                Ok(handle) => handle,
                Err(e) => {
                    log::error!("Failed to play sound: {}", e);
                    self.loading_state = AudioLoadingState::Failed;
                    return Err(e.into());
                }
            };
            self.handle = Some(handle);
            Ok(())
        } else {
            Err(anyhow!("No sound data available to play"))
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        self.handle().map(|handle| {
            handle.stop(Tween::default());
        })?;
        self.handle = None;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        self.handle().map(|handle| {
            handle.pause(Tween::default());
        })
    }

    pub fn resume(&mut self) -> Result<()> {
        let _ = self.handle().map(|handle| {
            if handle.state() != kira::sound::PlaybackState::Paused {
                return Err(anyhow!("Sound is not paused"));
            }
            handle.resume(Tween::default());
            Ok(())
        })?;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<()> {
        self.handle().map(|handle| {
            handle.set_volume(
                Decibels::interpolate(Decibels::SILENCE, Decibels::IDENTITY, volume),
                Tween::default(),
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
