pub mod base;
pub mod core;
pub mod events;
pub mod nodes;
pub mod plugins;
pub mod resource;
pub mod state;
pub mod surface;
pub mod traits;
pub mod user_event;
pub mod utils;

use std::sync::Arc;
use winit::event_loop::{EventLoop, EventLoopProxy};

pub use winit;

use crate::core::Core;
use crate::user_event::UserEvent;

/// setup hai core
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

/// create hai core instance
pub fn create_doufu_core<T>(
    event_loop: &EventLoop<T>,
    #[cfg(web)] element_id: &str,
    event_proxy: Arc<EventLoopProxy<UserEvent>>,
) -> Arc<Core> {
    // create multithread shared core
    let core = Core::new(
        event_loop,
        #[cfg(web)]
        element_id,
        event_proxy,
    );

    Arc::new(core)
}
