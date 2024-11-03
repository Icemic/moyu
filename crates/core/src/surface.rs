use hai_pal::env::{get_hai_env, RenderingBackend};
use log::{info, warn};
use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::dpi::Size;
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget};
use winit::window::WindowBuilder;
use winit::{dpi::PhysicalSize, window::Window};

use crate::user_event::UserEvent;

pub fn create_eventloop() -> EventLoop<UserEvent> {
    // create main thread infinity loop
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build().unwrap();
    event_loop
}

pub fn create_window(event_loop: &EventLoopWindowTarget<UserEvent>) -> Arc<Window> {
    let env = get_hai_env();
    // create window
    let mut builder = WindowBuilder::new()
        .with_inner_size(Size::Logical(env.surface_size.as_tuple().into()))
        .with_resizable(env.window_resizable)
        .with_visible(false)
        .with_active(true)
        .with_title(&env.window_title)
        .with_min_inner_size(PhysicalSize::new(400, 300));

    match env.window_state {
        hai_pal::env::WindowState::Maximized => {
            builder = builder.with_maximized(true);
        }
        hai_pal::env::WindowState::Minimized => {
            warn!("You should not start with a minimized window.");
        }
        hai_pal::env::WindowState::Fullscreen => {
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
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(
                    window.canvas().expect("canvas not found"),
                ))
                .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    Arc::new(window)
}

pub fn create_wgpu_surface(
    window: &Arc<Window>,
) -> (
    Arc<Instance>,
    Arc<Surface<'static>>,
    Arc<Device>,
    Arc<Queue>,
    SurfaceConfiguration,
) {
    // create wgpu surface
    #[cfg(not(feature = "web"))]
    let (instance, surface, device, queue, config) =
        futures::executor::block_on(create_surface_inner(window, &window.inner_size()));
    #[cfg(feature = "web")]
    let (instance, surface, device, queue, config) =
        { pollster::block_on(create_surface_inner(window, &PhysicalSize::new(1280, 720))) };
    let instance = Arc::new(instance);
    let surface = Arc::new(surface);
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    (instance, surface, device, queue, config)
}

pub(self) async fn create_surface_inner(
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
    let backends = match get_hai_env().backend {
        RenderingBackend::Auto => wgpu::Backends::all(),
        RenderingBackend::Vulkan => wgpu::Backends::VULKAN,
        RenderingBackend::Metal => wgpu::Backends::METAL,
        RenderingBackend::DX12 => wgpu::Backends::DX12,
        RenderingBackend::WebGPU => wgpu::Backends::BROWSER_WEBGPU,
        RenderingBackend::GLES => wgpu::Backends::GL,
    };

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
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

    #[cfg(not(feature = "web"))]
    {
        let adapter_info = adapter.get_info();
        info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let required_limits = if !cfg!(feature = "web") {
        adapter.limits()
    } else {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    };

    // graphic card with specific backend
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: adapter.features(),
                required_limits,
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None, // Trace path
        )
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
        .get(0)
        .expect("Cannot find a proper surface alpha mode.");

    info!("Available alpha mode: {:?}", caps.alpha_modes);
    info!("Selected alpha mode: {:?}", alpha_mode);

    #[cfg(not(feature = "web"))]
    let present_mode = match get_hai_env().present_mode {
        hai_pal::env::RenderingPresentMode::Recommended => {
            if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else if caps.present_modes.contains(&wgpu::PresentMode::FifoRelaxed) {
                wgpu::PresentMode::FifoRelaxed
            } else {
                wgpu::PresentMode::Fifo
            }
        }
        hai_pal::env::RenderingPresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
        hai_pal::env::RenderingPresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
    };

    #[cfg(feature = "web")]
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
        desired_maximum_frame_latency: get_hai_env().desired_maximum_frame_latency,
    };
    surface.configure(&device, &config);

    (instance, surface, device, queue, config)
}
