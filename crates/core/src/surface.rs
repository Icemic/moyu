use log::{info, warn};
use moyu_pal::config::{get_engine_config, RenderingBackend};
use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::dpi::Size;
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget};
#[cfg(android)]
use winit::platform::android::activity::AndroidApp;
use winit::window::WindowBuilder;
use winit::{dpi::PhysicalSize, window::Window};

pub fn create_eventloop(#[cfg(android)] app: AndroidApp) -> EventLoop<()> {
    // create main thread infinity loop
    #[cfg(not(android))]
    {
        EventLoopBuilder::with_user_event().build().unwrap()
    }

    #[cfg(android)]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        EventLoopBuilder::with_user_event()
            .with_android_app(app)
            .build()
            .unwrap()
    }
}

pub fn create_window<T>(
    event_loop: &EventLoopWindowTarget<T>,
    #[cfg(web)] element_id: &str,
) -> Arc<Window> {
    let env = get_engine_config();
    // create window
    let mut builder = WindowBuilder::new()
        .with_inner_size(Size::Logical(env.surface_size.as_tuple().into()))
        .with_resizable(env.window_resizable)
        .with_visible(false)
        .with_active(true)
        .with_title(&env.window_title)
        .with_min_inner_size(PhysicalSize::new(400, 300));

    match env.window_state {
        moyu_pal::config::WindowState::Maximized => {
            builder = builder.with_maximized(true);
        }
        moyu_pal::config::WindowState::Minimized => {
            warn!("You should not start with a minimized window.");
        }
        moyu_pal::config::WindowState::Fullscreen => {
            builder = builder.with_fullscreen(Some(winit::window::Fullscreen::Borderless(
                event_loop.primary_monitor(),
            )));
        }
        _ => {
            // idle
        }
    };

    let window = builder.build(event_loop).unwrap();

    // web target only
    // add a canvas element to dom as 'window'
    #[cfg(web)]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|document| document.get_element_by_id(element_id))
            .and_then(|el| {
                el.append_child(&web_sys::Element::from(
                    window.canvas().expect("canvas not found"),
                ))
                .ok()
            })
            .expect(format!("couldn't append canvas to {}", element_id).as_str());
    }

    Arc::new(window)
}

pub async fn create_wgpu_surface(
    window: &Arc<Window>,
) -> (
    Arc<Instance>,
    Arc<Surface<'static>>,
    Arc<Device>,
    Arc<Queue>,
    SurfaceConfiguration,
) {
    // create wgpu surface
    #[cfg(native)]
    let (instance, surface, device, queue, config) =
        create_surface_inner(window, &window.inner_size()).await;
    #[cfg(web)]
    let (instance, surface, device, queue, config) =
        create_surface_inner(window, &PhysicalSize::new(1280, 720)).await;
    let instance = Arc::new(instance);
    let surface = Arc::new(surface);
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    (instance, surface, device, queue, config)
}

async fn create_surface_inner(
    window: &Arc<Window>,
    size: &PhysicalSize<u32>,
) -> (
    Instance,
    Surface<'static>,
    Device,
    Queue,
    SurfaceConfiguration,
) {
    // The instance is a handle to our GPU
    // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
    let backends = match get_engine_config().backend {
        RenderingBackend::Auto => wgpu::Backends::all(),
        RenderingBackend::Vulkan => wgpu::Backends::VULKAN,
        RenderingBackend::Metal => wgpu::Backends::METAL,
        RenderingBackend::DX12 => wgpu::Backends::DX12,
        RenderingBackend::WebGPU => wgpu::Backends::BROWSER_WEBGPU,
        RenderingBackend::GLES => wgpu::Backends::GL,
    };

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        backend_options: wgpu::BackendOptions {
            dx12: wgpu::Dx12BackendOptions {
                shader_compiler: wgpu::Dx12Compiler::Fxc,
            },
            ..Default::default()
        },
        ..Default::default()
    });
    let surface = instance
        .create_surface(window.clone())
        .expect("Failed to create surface.");
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("No suitable GPU adapters found on the system.");

    #[cfg(native)]
    {
        let adapter_info = adapter.get_info();
        info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let required_limits = if cfg!(native) {
        adapter.limits()
    } else {
        // downgrade to webgl2 limits for web
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    };

    // graphic card with specific backend
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: adapter.features(),
            required_limits,
            label: None,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Unable to find a suitable GPU adapter.");

    let caps = surface.get_capabilities(&adapter);

    let format = *caps
        .formats
        .iter()
        .find(|f| !f.is_srgb())
        .expect("Cannot find a proper surface format.");

    info!("Available surface format: {:?}", caps.formats);
    info!("Selected surface format: {:?}", format);

    let alpha_mode = *caps
        .alpha_modes
        .first()
        .expect("Cannot find a proper surface alpha mode.");

    info!("Available alpha mode: {:?}", caps.alpha_modes);
    info!("Selected alpha mode: {:?}", alpha_mode);

    #[cfg(native)]
    let present_mode = match get_engine_config().present_mode {
        moyu_pal::config::RenderingPresentMode::Recommended => {
            if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else if caps.present_modes.contains(&wgpu::PresentMode::FifoRelaxed) {
                wgpu::PresentMode::FifoRelaxed
            } else {
                wgpu::PresentMode::Fifo
            }
        }
        moyu_pal::config::RenderingPresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
        moyu_pal::config::RenderingPresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
    };

    #[cfg(web)]
    let present_mode = wgpu::PresentMode::AutoVsync;

    info!("Present mode: {:?}", present_mode);

    // opengl backend does not support surface as COPY_SRC
    let usage = if adapter.get_info().backend == wgpu::Backend::Gl {
        wgpu::TextureUsages::RENDER_ATTACHMENT
    } else {
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
    };

    // define how the surface creates its underlying SurfaceTextures
    let config = wgpu::SurfaceConfiguration {
        usage,
        format,
        // width or height should not be 0 or it will cause crash
        width: size.width,
        height: size.height,
        // determines how to sync the surface with the display
        present_mode,
        alpha_mode,
        view_formats: vec![],
        desired_maximum_frame_latency: get_engine_config().desired_maximum_frame_latency,
    };
    surface.configure(&device, &config);

    (instance, surface, device, queue, config)
}
