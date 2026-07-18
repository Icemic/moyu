use log::{info, warn};
use moyu_pal::config::{RenderingBackend, get_engine_config};
use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::dpi::Size;
use winit::event_loop::{ActiveEventLoop, EventLoop};
#[cfg(android)]
use winit::platform::android::activity::AndroidApp;
use winit::{dpi::PhysicalSize, window::Window};

pub fn create_eventloop<T>(#[cfg(android)] app: AndroidApp) -> EventLoop<T> {
    // create main thread infinity loop
    #[cfg(not(android))]
    {
        EventLoop::with_user_event().build().unwrap()
    }

    #[cfg(android)]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        EventLoop::with_user_event()
            .with_android_app(app)
            .build()
            .unwrap()
    }
}

pub fn create_window(event_loop: &ActiveEventLoop, #[cfg(web)] element_id: &str) -> Arc<Window> {
    let env = get_engine_config();
    // create window
    let mut builder = Window::default_attributes()
        .with_inner_size(Size::Logical(env.initial_surface_size.as_tuple().into()))
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

    let window = event_loop
        .create_window(builder)
        .expect("Failed to create window.");

    // web target only
    // add a canvas element to dom as 'window'
    #[cfg(web)]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|document| document.get_element_by_id(element_id))
            .and_then(|el| {
                let scale_factor = window.scale_factor();

                let size = env.initial_surface_size;

                let canvas_width = (size.width() as f64 * scale_factor) as u32;
                let canvas_height = (size.height() as f64 * scale_factor) as u32;

                let canvas = window
                    .canvas()
                    .expect("Failed to get canvas from winit window.");

                canvas.set_width(canvas_width);
                canvas.set_height(canvas_height);
                canvas
                    .style()
                    .set_property("width", &format!("{}px", size.width()))
                    .ok();
                canvas
                    .style()
                    .set_property("height", &format!("{}px", size.height()))
                    .ok();

                el.append_child(&web_sys::Element::from(canvas)).ok()
            })
            .expect(format!("couldn't append canvas to {}", element_id).as_str());
    }

    Arc::new(window)
}

pub async fn create_wgpu_surface(
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

    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_descriptor.backends = backends;
    instance_descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
    let instance = wgpu::Instance::new(instance_descriptor);
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

    let adapter_info = adapter.get_info();
    info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let required_limits = if adapter_info.backend == wgpu::Backend::Gl {
        // downgrade to webgl2 limits for web
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    } else {
        // use the adapter's limits directly for native and webgpu
        adapter.limits()
    };

    // graphic card with specific backend
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: adapter.features(),
            required_limits,
            experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
            label: None,
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("Unable to find a suitable GPU adapter.");

    let caps = surface.get_capabilities(&adapter);

    let format = choose_surface_format(&caps.formats);

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
        #[cfg(mobile)]
        moyu_pal::config::RenderingPresentMode::Recommended => wgpu::PresentMode::Fifo,
        #[cfg(not(mobile))]
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

    let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;

    if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) {
        usage |= wgpu::TextureUsages::COPY_SRC;
    }

    if adapter_info.backend == wgpu::Backend::BrowserWebGpu {
        // WebGPU on Chrome and Firefox (at least) do support COPY_SRC, but wgpu hard-coded its usages to RENDER_ATTACHMENT only
        // on WebGPU, we have to force add COPY_SRC.
        usage |= wgpu::TextureUsages::COPY_SRC;
    }

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

#[inline(always)]
fn choose_surface_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    if formats.contains(&wgpu::TextureFormat::Bgra8Unorm) {
        return wgpu::TextureFormat::Bgra8Unorm;
    }

    if formats.contains(&wgpu::TextureFormat::Rgba8Unorm) {
        return wgpu::TextureFormat::Rgba8Unorm;
    }

    panic!(
        "Cannot find a proper surface format. Expected Bgra8Unorm or Rgba8Unorm, available: {:?}",
        formats
    );
}
