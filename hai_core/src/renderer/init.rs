use log::info;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

pub async fn create_surface(
    window: &Window,
    size: &PhysicalSize<u32>,
) -> (Surface, Device, Queue, SurfaceConfiguration) {
    // The instance is a handle to our GPU
    // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(window) };
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

    // define how the surface creates its underlying SurfaceTextures
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_supported_formats(&adapter)[0],
        // width or height should not be 0 or it will cause crash
        width: size.width,
        height: size.height,
        // determines how to sync the surface with the display
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &config);

    (surface, device, queue, config)
}
