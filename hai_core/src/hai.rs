use hai_pal::sync::Mutex;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::renderer::SpriteRenderer;
use crate::state::{set_shared_state, State};
use crate::user_event::UserEvent;

pub fn create_hai_state(
    surface: Arc<Surface>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    window: &Window,
    event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
) -> Arc<State> {
    let sprite_renderer = SpriteRenderer::new(&device, &config);

    // create multithread shared state
    let state = State::new(surface, device, queue, config, event_proxy);

    // state.register_renderer("null".to_string(), null_renderer);
    state.register_renderer("sprite".to_string(), Box::new(sprite_renderer));

    // set screen size
    let size = window.inner_size();
    let scale_factor = window.scale_factor();
    state.set_screen_size((size.width, size.height), scale_factor);

    // make state sharable among threads
    let state = Arc::new(state);

    set_shared_state(state.clone());

    state
}
