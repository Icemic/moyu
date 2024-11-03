use anyhow::{anyhow, Result};
use kira::sound::static_sound::StaticSoundHandle;
use kira::tween::Tween;
use kira::Volume;

pub struct Audio {
    pub(crate) sound: Option<StaticSoundHandle>,
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
            loading_state: AudioLoadingState::Unloaded,
        }
    }

    fn find_and(&mut self, f: impl FnOnce(&mut StaticSoundHandle) -> Result<()>) -> Result<()> {
        if let Some(ref mut handle) = self.sound {
            f(handle)
        } else {
            Err(anyhow!("Sound not loaded"))
        }
    }

    pub fn play(&mut self) -> Result<()> {
        self.find_and(|handle| {
            handle.seek_to(0.0);
            handle.resume(Tween::default());
            Ok(())
        })
    }

    pub fn stop(&mut self) -> Result<()> {
        self.find_and(|handle| {
            handle.pause(Tween::default());
            handle.seek_to(0.0);
            Ok(())
        })
    }

    pub fn pause(&mut self) -> Result<()> {
        self.find_and(|handle| {
            handle.pause(Tween::default());
            Ok(())
        })
    }

    pub fn resume(&mut self) -> Result<()> {
        self.find_and(|handle| {
            handle.resume(Tween::default());
            Ok(())
        })
    }

    pub fn stop_and_release(&mut self) -> Result<()> {
        self.find_and(|handle| {
            handle.stop(Tween::default());
            Ok(())
        })?;
        self.sound = None;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<()> {
        self.find_and(|handle| {
            handle.set_volume(Volume::Amplitude(volume), Tween::default());
            Ok(())
        })
    }

    pub fn seek_by(&mut self, seconds: f64) -> Result<()> {
        self.find_and(|handle| {
            handle.seek_by(seconds);
            Ok(())
        })
    }

    pub fn seek_to(&mut self, seconds: f64) -> Result<()> {
        self.find_and(|handle| {
            handle.seek_to(seconds);
            Ok(())
        })
    }

    pub fn set_playback_rate(&mut self, rate: f64) -> Result<()> {
        self.find_and(|handle| {
            handle.set_playback_rate(rate, Tween::default());
            Ok(())
        })
    }

    pub fn set_loop_region(&mut self, start: f64, end: f64) -> Result<()> {
        self.find_and(|handle| {
            handle.set_loop_region(start..end);
            Ok(())
        })
    }

    pub fn set_panning(&mut self, panning: f64) -> Result<()> {
        self.find_and(|handle| {
            handle.set_panning(panning, Tween::default());
            Ok(())
        })
    }
}
