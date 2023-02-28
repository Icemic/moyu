use log::info;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

pub fn create_wgpu_surface(
    window: &Window,
) -> (Arc<Surface>, Arc<Device>, Arc<Queue>, SurfaceConfiguration) {
    // create wgpu surface
    #[cfg(not(target_arch = "wasm32"))]
    let (surface, device, queue, config) =
        futures::executor::block_on(create_surface_inner(&window, &window.inner_size()));
    #[cfg(target_arch = "wasm32")]
    let (surface, device, queue, config) =
        { pollster::block_on(create_surface_inner(&window, &window.inner_size())) };
    let surface = Arc::new(surface);
    let device = Arc::new(device);
    let queue = Arc::new(queue);

    (surface, device, queue, config)
}

pub(self) async fn create_surface_inner(
    window: &Window,
    size: &PhysicalSize<u32>,
) -> (Surface, Device, Queue, SurfaceConfiguration) {
    // The instance is a handle to our GPU
    // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let adapter_info = adapter.get_info();
        info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    let limits = if !cfg!(target_arch = "wasm32") {
        wgpu::Limits::default()
    } else {
        wgpu::Limits::downlevel_webgl2_defaults()
    };

    // graphic card with specific backend
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
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
        .find(|f| f.describe().srgb)
        .expect("Cannot find a proper surface format.");

    info!("Surface format: {:?}", format);

    let alpha_mode = *caps
        .alpha_modes
        .get(0)
        .expect("Cannot find a proper surface alpha mode.");

    info!("Alpha mode: {:?}", alpha_mode);

    // define how the surface creates its underlying SurfaceTextures
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        // width or height should not be 0 or it will cause crash
        width: size.width,
        height: size.height,
        // determines how to sync the surface with the display
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode,
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    (surface, device, queue, config)
}
