//! Graphics context initialization.
//!
//! The platform object behind the frame output comes from the window system; everything
//! else here (backend selection, adapter validation, device creation, output construction)
//! is graphics work. A window system that cannot provide a surface gets a texture output
//! instead, which is how the engine renders without a display of its own.

use anyhow::{Result, anyhow};
use log::info;
#[cfg(android)]
use log::warn;
use moyu_pal::config::{RenderingBackend, get_engine_config};
use std::sync::Arc;
use wgpu::{Device, Instance, Queue};

use crate::core::render_output::{FrameOutput, create_render_output};
use crate::core::{WindowSystem, WindowSystemError};

pub(crate) async fn create_render_context(
    window_system: &Arc<dyn WindowSystem>,
    size: (u32, u32),
) -> (Instance, Device, Queue, Box<dyn FrameOutput>) {
    let backends = match get_engine_config().backend {
        #[cfg(android)]
        RenderingBackend::Auto => {
            match create_render_context_with_backends(window_system, size, wgpu::Backends::VULKAN)
                .await
            {
                Ok(context) => return context,
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

    create_render_context_with_backends(window_system, size, backends)
        .await
        .unwrap_or_else(|err| panic!("Failed to initialize graphics: {err}"))
}

async fn create_render_context_with_backends(
    window_system: &Arc<dyn WindowSystem>,
    size: (u32, u32),
    backends: wgpu::Backends,
) -> Result<(Instance, Device, Queue, Box<dyn FrameOutput>)> {
    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    instance_descriptor.backends = backends;
    instance_descriptor.backend_options.dx12.shader_compiler = wgpu::Dx12Compiler::Fxc;
    let instance = wgpu::Instance::new(instance_descriptor);
    let surface = match window_system.create_surface(&instance) {
        Ok(surface) => Some(surface),
        // The window system has no surface to give: headless runs, and hosts that
        // present the frame themselves sample a texture output instead.
        Err(WindowSystemError::Unsupported) => None,
        Err(error) => return Err(anyhow!("failed to create surface: {error}")),
    };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
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

    let output = create_render_output(&window_system, &instance, &adapter, &device, surface, size);

    Ok((instance, device, queue, output))
}
