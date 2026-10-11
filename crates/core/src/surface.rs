//! WGPU surface initialization.
//!
//! The platform object behind the surface comes from the window system; everything
//! else here (backend selection, adapter validation, device creation, surface format
//! and present mode) is graphics work.

use anyhow::{Result, anyhow};
use log::info;
#[cfg(android)]
use log::warn;
use moyu_pal::config::{RenderingBackend, get_engine_config};
use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};

use crate::core::WindowSystem;

pub(crate) async fn create_wgpu_surface(
    window_system: &Arc<dyn WindowSystem>,
    size: (u32, u32),
) -> (
    Instance,
    Surface<'static>,
    Device,
    Queue,
    SurfaceConfiguration,
) {
    let backends = match get_engine_config().backend {
        #[cfg(android)]
        RenderingBackend::Auto => {
            match create_wgpu_surface_with_backends(window_system, size, wgpu::Backends::VULKAN)
                .await
            {
                Ok(surface) => return surface,
                Err(err) => {
                    warn!("Vulkan is unavailable on this device, falling back to GLES: {err}");
                    wgpu::Backends::GL
                }
            }
        }
        #[cfg(not(android))]
        RenderingBackend::Auto => wgpu::Backends::all(),
        RenderingBackend::Vulkan => wgpu::Backends::VULKAN,
        RenderingBackend::Metal => wgpu::Backends::METAL,
        RenderingBackend::DX12 => wgpu::Backends::DX12,
        RenderingBackend::WebGPU => wgpu::Backends::BROWSER_WEBGPU,
        RenderingBackend::GLES => wgpu::Backends::GL,
    };

    create_wgpu_surface_with_backends(window_system, size, backends)
        .await
        .unwrap_or_else(|err| panic!("Failed to initialize graphics: {err}"))
}

async fn create_wgpu_surface_with_backends(
    window_system: &Arc<dyn WindowSystem>,
    size: (u32, u32),
    backends: wgpu::Backends,
) -> Result<(
    Instance,
    Surface<'static>,
    Device,
    Queue,
    SurfaceConfiguration,
)> {
    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_descriptor.backends = backends;
    instance_descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
    let instance = wgpu::Instance::new(instance_descriptor);
    let surface = window_system
        .create_surface(&instance)
        .map_err(|error| anyhow!("failed to create surface: {error}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })
        .await
        .map_err(|err| anyhow!("no suitable GPU adapter found: {err}"))?;

    let adapter_info = adapter.get_info();
    info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let adapter_limits = adapter.limits();

    // Some drivers report bogus limits. Android emulators, for instance, pass the host's Vulkan
    // extension list through but never fill in the `VkPhysicalDeviceProperties2` pNext chain, so
    // the buffer limits stay at 0. Reject such adapters here so the caller can fall back, instead
    // of failing later on the first buffer allocation.
    let mut unmet_limit = None;
    wgpu::Limits::downlevel_webgl2_defaults().check_limits_with_fail_fn(
        &adapter_limits,
        true,
        |name, required, reported| {
            unmet_limit = Some(format!("{name}: needs {required}, reported {reported}"));
        },
    );
    if let Some(unmet_limit) = unmet_limit {
        return Err(anyhow!(
            "adapter does not meet the engine's minimum limits ({unmet_limit})"
        ));
    }

    let required_limits = if adapter_info.backend == wgpu::Backend::Gl {
        // downgrade to webgl2 limits for web
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    } else {
        // use the adapter's limits directly for native and webgpu
        adapter_limits
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
        .map_err(|err| anyhow!("unable to create device: {err}"))?;

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
        color_space: wgpu::SurfaceColorSpace::Auto,
        // width or height should not be 0 or it will cause crash
        width: size.0,
        height: size.1,
        // determines how to sync the surface with the display
        present_mode,
        alpha_mode,
        view_formats: vec![],
        desired_maximum_frame_latency: get_engine_config().desired_maximum_frame_latency,
    };
    surface.configure(&device, &config);

    Ok((instance, surface, device, queue, config))
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
