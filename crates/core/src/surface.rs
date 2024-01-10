use hai_pal::env::{get_hai_env, RenderingBackend, RenderingPresentMode};
use log::info;
use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::dpi::{LogicalSize, Size};
use winit::event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget};
use winit::window::WindowBuilder;
use winit::{dpi::PhysicalSize, window::Window};

use crate::user_event::UserEvent;

pub fn create_eventloop() -> EventLoop<UserEvent> {
    // create main thread infinity loop
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build().unwrap();
    event_loop
}

pub fn create_window(event_loop: &EventLoopWindowTarget<UserEvent>) -> Window {
    // create window
    let window = WindowBuilder::new()
        .with_inner_size(Size::Logical(LogicalSize::new(1280., 720.)))
        .with_resizable(false)
        .with_visible(false)
        .build(event_loop)
        .unwrap();

    // web target only
    // add a canvas element to dom as 'window'
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }

    window
}

pub fn create_wgpu_surface(
    window: &Window,
) -> (
    Arc<Instance>,
    Arc<Surface>,
    Arc<Device>,
    Arc<Queue>,
    SurfaceConfiguration,
) {
    // create wgpu surface
    #[cfg(not(feature = "web"))]
    let (instance, surface, device, queue, config) =
        futures::executor::block_on(create_surface_inner(window, &window.inner_size()));
    #[cfg(feature = "web")]
    let (surface, device, queue, config) =
        { pollster::block_on(create_surface_inner(&window, &window.inner_size())) };
    let instance = Arc::new(instance);
    let surface = Arc::new(surface);
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    (instance, surface, device, queue, config)
}

pub(self) async fn create_surface_inner(
    window: &Window,
    size: &PhysicalSize<u32>,
) -> (Instance, Surface, Device, Queue, SurfaceConfiguration) {
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
    let surface = unsafe {
        instance
            .create_surface(window)
            .expect("Failed to create surface.")
    };
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

    let limits = if !cfg!(feature = "web") {
        wgpu::Limits::default()
    } else {
        wgpu::Limits::downlevel_webgl2_defaults()
    };

    let features = if adapter
        .features()
        .contains(wgpu::Features::BGRA8UNORM_STORAGE)
    {
        wgpu::Features::default()
            | wgpu::Features::BGRA8UNORM_STORAGE
            | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
    } else {
        wgpu::Features::default()
    };

    // graphic card with specific backend
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features,
                limits,
                label: None,
            },
            None, // Trace path
        )
        .await
        .expect("Unable to find a suitable GPU adapter.");

    let caps = surface.get_capabilities(&adapter);

    let format = *caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .expect("Cannot find a proper surface format.");

    info!("Surface format: {:?}", format);

    let alpha_mode = *caps
        .alpha_modes
        .get(0)
        .expect("Cannot find a proper surface alpha mode.");

    info!("Alpha mode: {:?}", alpha_mode);

    #[cfg(not(feature = "web"))]
    let present_mode = match get_hai_env().present_mode {
        RenderingPresentMode::Recommended => {
            if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else if caps.present_modes.contains(&wgpu::PresentMode::FifoRelaxed) {
                wgpu::PresentMode::FifoRelaxed
            } else {
                wgpu::PresentMode::Fifo
            }
        }
        RenderingPresentMode::AutoVsync => wgpu::PresentMode::AutoVsync,
        RenderingPresentMode::AutoNoVsync => wgpu::PresentMode::AutoNoVsync,
    };

    #[cfg(feature = "web")]
    let present_mode = wgpu::PresentMode::AutoVsync;

    info!("Present mode: {:?}", present_mode);

    // define how the surface creates its underlying SurfaceTextures
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        format,
        // width or height should not be 0 or it will cause crash
        width: size.width,
        height: size.height,
        // determines how to sync the surface with the display
        present_mode,
        alpha_mode,
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    (instance, surface, device, queue, config)
}
