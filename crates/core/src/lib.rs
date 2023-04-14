pub mod core;
pub mod nodes;
pub mod ops;
pub mod presets;
pub mod renderer;
pub mod resource;
pub mod surface;
pub mod traits;
pub mod types;
pub mod user_event;
pub mod utils;

use futures::Future;
use hai_pal::sync::Mutex;
use renderer::YUVSpriteRenderer;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::core::{set_core, Core};
use crate::renderer::SpriteRenderer;
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
    surface: Arc<Surface>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    window: &Window,
    event_proxy: Arc<Mutex<EventLoopProxy<UserEvent>>>,
) -> Arc<Core> {
    let sprite_renderer = SpriteRenderer::new(&device, &config);
    let yuv_sprite_renderer = YUVSpriteRenderer::new(&device, &config);
    // use sprite renderer on video node
    #[cfg(feature = "video")]
    let video_renderer = SpriteRenderer::new(&device, &config);

    // create multithread shared core
    let core = Core::new(surface, device, queue, config, event_proxy);

    // core.register_renderer("null".to_string(), null_renderer);
    core.register_renderer("sprite".to_string(), Box::new(sprite_renderer));
    core.register_renderer("yuv_sprite".to_string(), Box::new(yuv_sprite_renderer));
    #[cfg(feature = "video")]
    core.register_renderer("video".to_string(), Box::new(video_renderer));

    // set screen size
    let size = window.inner_size();
    let scale_factor = window.scale_factor();
    core.set_screen_size((size.width, size.height), scale_factor);

    // make core sharable among threads
    let core = Arc::new(core);

    set_core(core.clone());

    core
}

use std::pin::Pin;

pub type SpawnRuntimeCallback =
    Box<dyn (FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
pub fn spawn_runtime_with_core(core: &Arc<Core>, spawn_callback: Option<SpawnRuntimeCallback>) {
    // desktop targets only
    // spawn a v8 thread
    #[cfg(not(feature = "web"))]
    {
        use log::error;
        use std::process::exit;

        use hai_js_runtime::JSRuntime;

        let core = core.clone();

        std::thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();

            if let Some(spawn_callback) = spawn_callback {
                let async_callback = spawn_callback();
                runtime.spawn(async_callback);
            }

            let resource_manager = core.resource_manager.clone();
            runtime.block_on(async {
                let mut vm = JSRuntime::new(core);

                vm.with_global(|scope, global| {
                    ops::init(scope, global);
                });

                if let Err(err) = vm.prepare_entry().await {
                    error!("{}", err.to_string());
                    exit(-1);
                };

                vm.run_event_loop(|cx| {
                    let mut resource_manager = resource_manager.lock();
                    resource_manager.poll(cx)
                })
                .await;
            });
        });
    }

    #[cfg(feature = "web")]
    {
        use log::debug;

        if let Some(spawn_callback) = spawn_callback {
            let async_callback = spawn_callback();
            wasm_bindgen_futures::spawn_local(spawn_callback);
        }

        wasm_bindgen_futures::spawn_local(async move {
            debug!("Injecting entry script.");
            let window = web_sys::window().expect("Cannot get global `window` object.");
            let document = window.document().expect("No document found.");
            let body = document.body().expect("No body found.");

            let root_script = document
                .create_element("script")
                .expect("Cannot create script element.");
            root_script
                .set_attribute("src", env::entry_dir().as_str())
                .unwrap();
            root_script.set_attribute("type", "module").unwrap();

            body.append_child(&root_script).unwrap();
        });
    }
}
