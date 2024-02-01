pub mod base;
pub mod core;
pub mod nodes;
#[cfg(any(feature = "web", feature = "js_runtime"))]
pub mod ops;
pub mod presets;
pub mod renderer;
pub mod resource;
pub mod surface;
pub mod traits;
pub mod user_event;
pub mod utils;

use futures::Future;
#[cfg(feature = "text")]
use renderer::TextRenderer;
use renderer::YUVSpriteRenderer;
use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

pub use winit;

use crate::core::Core;
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
    instance: Arc<Instance>,
    surface: Arc<Surface<'static>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    window: &Arc<Window>,
    event_proxy: Arc<EventLoopProxy<UserEvent>>,
) -> Arc<Core> {
    let sprite_renderer = SpriteRenderer::new(&device, &config);
    let yuv_sprite_renderer = YUVSpriteRenderer::new(&device, &config);
    // use sprite renderer on video node
    #[cfg(feature = "video")]
    let video_renderer = SpriteRenderer::new(&device, &config);
    #[cfg(feature = "text")]
    let text_renderer = TextRenderer::new(&device, &config);

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

    // core.register_renderer("null".to_string(), null_renderer);
    core.register_renderer("sprite".to_string(), Box::new(sprite_renderer));
    core.register_renderer("yuv_sprite".to_string(), Box::new(yuv_sprite_renderer));
    #[cfg(feature = "video")]
    core.register_renderer("video".to_string(), Box::new(video_renderer));
    #[cfg(feature = "text")]
    core.register_renderer("text".to_string(), Box::new(text_renderer));

    // set screen size
    let size = window.inner_size();
    let scale_factor = window.scale_factor();
    core.set_screen_size((size.width, size.height), scale_factor);

    // make core sharable among threads

    Arc::new(core)
}

use std::pin::Pin;

pub type SpawnRuntimeCallback =
    Box<dyn (FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "v8"))]
pub fn spawn_runtime_with_core(core: &Arc<Core>, spawn_callback: Option<SpawnRuntimeCallback>) {
    // desktop targets only
    // spawn a v8 thread

    use log::error;
    use std::process::exit;

    use hai_js_runtime::JSRuntime;

    let core = core.clone();

    std::thread::spawn(|| {
        let handle = hai_pal::task::get_runtime_handle();

        if let Some(spawn_callback) = spawn_callback {
            let async_callback = spawn_callback();
            handle.spawn(async_callback);
        }

        handle.block_on(async {
            let mut vm = JSRuntime::new(core);

            vm.with_global(|scope, global| {
                ops::init(scope, global);
            });

            if let Err(err) = vm.prepare_entry().await {
                error!("{}", err.to_string());
                exit(-1);
            };

            vm.run_event_loop(|_| std::task::Poll::Pending).await;
        });
    });
}

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(all(not(feature = "web"), feature = "js_runtime", feature = "quickjs"))]
pub fn spawn_runtime_with_core(_core: &Arc<Core>, spawn_callback: Option<SpawnRuntimeCallback>) {
    // desktop targets only
    // spawn a v8 thread

    use hai_runtime::setup_vm;
    use log::error;

    std::thread::Builder::new()
        .name("quickjs".to_string())
        .spawn(|| {
            let handle = hai_pal::task::get_runtime_handle();

            if let Some(spawn_callback) = spawn_callback {
                let async_callback = spawn_callback();
                handle.spawn(async_callback);
            }

            let vm = setup_vm();

            vm.context()
                .eval("console.info('Hello %s!', 'World')")
                .unwrap();

            ops::init(&vm);

            if let Err(err) = vm.prepare_entry() {
                error!("{:?}", err);
            };

            vm.block_on_ticking();
        })
        .ok();
}

/// spawn a thread with javascript runtime and executes scripts
/// use `spawn_callback` to do anything else which should be under a async runtime.
#[cfg(all(feature = "web", not(feature = "js_runtime")))]
pub fn spawn_runtime_with_core(core: &Arc<Core>, spawn_callback: Option<SpawnRuntimeCallback>) {
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
