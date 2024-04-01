use anyhow::{anyhow, Result};
use kira::sound::static_sound::StaticSoundHandle;
use kira::tween::Tween;
use kira::Volume;

pub struct Audio {
    sound: Option<StaticSoundHandle>,
}

impl Audio {
    pub fn new() -> Self {
        Self { sound: None }
    }

    pub fn load(&mut self, path: &str) {}

    pub fn play(&mut self) -> Result<()> {
        if let Some(ref mut handle) = self.sound {
            handle.seek_to(0.0)?;
            handle.resume(Tween::default())?;
            Ok(())
        } else {
            Err(anyhow!("Sound not loaded"))
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(ref mut handle) = self.sound {
            handle.pause(Tween::default())?;
            handle.seek_to(0.0)?;
            Ok(())
        } else {
            Err(anyhow!("Sound not loaded"))
        }
    }

    pub fn pause(&mut self) -> Result<()> {
        if let Some(ref mut handle) = self.sound {
            handle.pause(Tween::default())?;
            Ok(())
        } else {
            Err(anyhow!("Sound not loaded"))
        }
    }

    pub fn resume(&mut self) -> Result<()> {
        if let Some(ref mut handle) = self.sound {
            handle.resume(Tween::default())?;
            Ok(())
        } else {
            Err(anyhow!("Sound not loaded"))
        }
    }

    pub fn set_volume(&mut self, volume: f64) -> Result<()> {
        if let Some(ref mut handle) = self.sound {
            handle.set_volume(Volume::Amplitude(volume), Tween::default())?;
            Ok(())
        } else {
            Err(anyhow!("Sound not loaded"))
        }
    }
}
