use anyhow::Result;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
    tween::Tween,
};

fn main() {
    play().unwrap();
}

fn play() -> Result<()> {
    // Create an audio manager. This plays sounds and manages resources.
    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;

    let path = std::env::current_dir()
        .unwrap()
        .join("assets/audio/test.ogg");

    let file = std::fs::File::open(path)?;

    let sound_data = StaticSoundData::from_media_source(file, StaticSoundSettings::new())?;

    let mut handle = manager.play(sound_data)?;

    let mut i = 0;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        i += 1;

        if i == 10 {
            handle.stop(Tween::default())?;
        }
    }
}
