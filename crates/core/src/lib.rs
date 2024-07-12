pub mod base;
pub mod core;
pub mod nodes;
pub mod resource;
pub mod surface;
pub mod traits;
pub mod user_event;
pub mod utils;

use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

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
pub fn create_hai_core(
    instance: Arc<Instance>,
    surface: Arc<Surface<'static>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    window: &Arc<Window>,
    event_proxy: Arc<EventLoopProxy<UserEvent>>,
) -> Arc<Core> {
    // create multithread shared core
    let core = Core::new(
        instance,
        surface,
        device,
        queue,
        window.clone(),
        config,
        event_proxy,
    );

    Arc::new(core)
}
