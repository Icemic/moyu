pub mod base;
pub mod core;
pub mod events;
pub mod nodes;
pub mod plugins;
pub mod state;
pub mod surface;
pub mod traits;
pub mod utils;

pub use winit;

/// setup moyu core
pub fn setup() {
    #[cfg(feature = "video")]
    {
        use log::{debug, info};
        ffmpeg_rs::init().unwrap();
        info!(
            "FFmpeg initialized, license: {}",
            ffmpeg_rs::util::license()
        );
        debug!("FFmpeg configuration: {}", ffmpeg_rs::util::configuration());
    }
}
