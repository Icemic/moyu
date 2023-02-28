use hai_pal::sync::Mutex;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::core::{set_core, Core};
use crate::renderer::SpriteRenderer;
use crate::user_event::UserEvent;

pub fn create_hai_core(
    surface: Arc<Surface>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    window: &Window,
    event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
) -> Arc<Core> {
    let sprite_renderer = SpriteRenderer::new(&device, &config);

    // create multithread shared core
    let core = Core::new(surface, device, queue, config, event_proxy);

    // core.register_renderer("null".to_string(), null_renderer);
    core.register_renderer("sprite".to_string(), Box::new(sprite_renderer));

    // set screen size
    let size = window.inner_size();
    let scale_factor = window.scale_factor();
    core.set_screen_size((size.width, size.height), scale_factor);

    // make core sharable among threads
    let core = Arc::new(core);

    set_core(core.clone());

    core
}
