//! The frame output boundary: where the engine's finished frames go.
//!
//! An output owns one output object and hands out a render target per frame. The
//! window swapchain is one kind of output; an engine-owned texture — rendered for a
//! host to sample instead of being shown on a display — is another. Both obey the
//! same contract: acquire the target for the frame, render into it, present it.

use std::sync::Arc;

use log::{error, info, warn};
use moyu_pal::config::get_engine_config;
use moyu_pal::sync::Mutex;
use wgpu::{
    Adapter, CurrentSurfaceTexture, Device, Extent3d, Instance, Surface, SurfaceConfiguration,
    SurfaceTexture, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

use super::WindowSystem;

/// Format of an engine-owned output texture.
///
/// UNORM matches the swapchain formats the engine already draws to, and the sRGB view
/// format is declared so a host can sample the same image as an sRGB texture.
const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
const TEXTURE_VIEW_FORMATS: &[TextureFormat] = &[TextureFormat::Rgba8UnormSrgb];

/// One frame's render target.
pub struct RenderTarget {
    pub texture: Texture,
    pub view: TextureView,
    /// Size in physical pixels.
    pub size: (u32, u32),
    pub format: TextureFormat,
    delivery: Delivery,
}

/// What happens to a frame when it is presented.
enum Delivery {
    /// The target is a swapchain image; presenting shows the frame on screen.
    Swapchain(SurfaceTexture),
    /// The target is an engine-owned texture; the frame stays in it for the host.
    Texture,
}

impl RenderTarget {
    /// Hand the finished frame back to the output.
    pub fn present(self, queue: &wgpu::Queue) {
        if let Delivery::Swapchain(frame) = self.delivery {
            queue.present(frame);
        }
    }
}

/// The frame output boundary the renderer draws into.
pub trait FrameOutput: Send + Sync {
    /// Current output size in physical pixels.
    fn size(&self) -> (u32, u32);
    /// Texture format of the frame target.
    fn format(&self) -> TextureFormat;
    /// Whether the frame target can be copied from (screenshots).
    fn supports_copy_src(&self) -> bool;

    /// Record the size the output should have from now on. The change is applied by
    /// the next [`FrameOutput::refresh`], which runs on the render thread.
    fn set_size(&self, width: u32, height: u32);

    /// Acquire the render target for the next frame, or `None` when the frame has to
    /// be skipped.
    fn acquire(&self, device: &Device) -> Option<RenderTarget>;

    /// Re-apply the output configuration. Runs on the render thread.
    fn refresh(&self, device: &Device);

    /// Release the platform output object while keeping the device alive.
    fn release(&self);
    /// Rebuild the platform output object after a release. Runs on the render thread.
    fn recreate(&self, device: &Device);
}

/// The window swapchain output: frames are presented to a display.
pub(crate) struct SwapchainOutput {
    window_system: Arc<dyn WindowSystem>,
    instance: Instance,
    surface: Mutex<Option<Surface<'static>>>,
    config: Mutex<SurfaceConfiguration>,
}

impl SwapchainOutput {
    /// Configure a swapchain output from a surface the window system provided.
    pub(crate) fn new(
        window_system: Arc<dyn WindowSystem>,
        instance: Instance,
        surface: Surface<'static>,
        adapter: &Adapter,
        device: &Device,
        size: (u32, u32),
    ) -> Self {
        let caps = surface.get_capabilities(adapter);
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

        let mut usage = TextureUsages::RENDER_ATTACHMENT;

        if caps.usages.contains(TextureUsages::COPY_SRC) {
            usage |= TextureUsages::COPY_SRC;
        }

        if adapter.get_info().backend == wgpu::Backend::BrowserWebGpu {
            // WebGPU on Chrome and Firefox (at least) do support COPY_SRC, but wgpu hard-coded its usages to RENDER_ATTACHMENT only
            // on WebGPU, we have to force add COPY_SRC.
            usage |= TextureUsages::COPY_SRC;
        }

        // define how the surface creates its underlying SurfaceTextures
        let config = SurfaceConfiguration {
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
        surface.configure(device, &config);

        Self {
            window_system,
            instance,
            surface: Mutex::new(Some(surface)),
            config: Mutex::new(config),
        }
    }

    fn configure(&self, device: &Device) {
        let config = self.config.lock().clone();
        if let Some(surface) = self.surface.lock().as_ref() {
            surface.configure(device, &config);
        }
    }
}

impl FrameOutput for SwapchainOutput {
    fn size(&self) -> (u32, u32) {
        let config = self.config.lock();
        (config.width, config.height)
    }

    fn format(&self) -> TextureFormat {
        self.config.lock().format
    }

    fn supports_copy_src(&self) -> bool {
        self.config.lock().usage.contains(TextureUsages::COPY_SRC)
    }

    fn set_size(&self, width: u32, height: u32) {
        let mut config = self.config.lock();
        config.width = width;
        config.height = height;
    }

    fn acquire(&self, device: &Device) -> Option<RenderTarget> {
        let surface = self.surface.lock();
        let surface = surface.as_ref()?;
        let frame = match surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => frame,
            CurrentSurfaceTexture::Lost
            | CurrentSurfaceTexture::Outdated
            | CurrentSurfaceTexture::Suboptimal(_) => {
                warn!("surface needs reconfiguration.");
                self.configure(device);
                return None;
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return None,
            CurrentSurfaceTexture::Validation => {
                error!("surface validation error, frame skipped.");
                return None;
            }
        };

        let texture = frame.texture.clone();
        let size = (texture.width(), texture.height());
        let view = texture.create_view(&TextureViewDescriptor::default());
        let format = self.config.lock().format;

        Some(RenderTarget {
            texture,
            view,
            size,
            format,
            delivery: Delivery::Swapchain(frame),
        })
    }

    fn refresh(&self, device: &Device) {
        self.configure(device);
    }

    fn release(&self) {
        let surface = self.surface.lock().take();
        drop(surface);
        self.instance.poll_all(true);
    }

    fn recreate(&self, device: &Device) {
        let surface = match self.window_system.create_surface(&self.instance) {
            Ok(surface) => surface,
            Err(error) => {
                error!("Failed to recreate surface: {}", error);
                return;
            }
        };

        let config = self.config.lock().clone();
        surface.configure(device, &config);

        self.surface.lock().replace(surface);
        info!("Surface recreated on render thread.");
    }
}

/// The texture output: frames stay in an engine-owned texture for a host to consume.
pub(crate) struct TextureOutput {
    state: Mutex<TextureOutputState>,
}

struct TextureOutputState {
    /// Size the texture should have; applied on the next refresh.
    size: (u32, u32),
    texture: Texture,
    view: TextureView,
}

impl TextureOutput {
    pub(crate) fn new(device: &Device, size: (u32, u32)) -> Self {
        let (texture, view) = create_output_texture(device, size);
        Self {
            state: Mutex::new(TextureOutputState {
                size,
                texture,
                view,
            }),
        }
    }
}

impl FrameOutput for TextureOutput {
    fn size(&self) -> (u32, u32) {
        self.state.lock().size
    }

    fn format(&self) -> TextureFormat {
        TEXTURE_FORMAT
    }

    fn supports_copy_src(&self) -> bool {
        true
    }

    fn set_size(&self, width: u32, height: u32) {
        self.state.lock().size = (width, height);
    }

    fn acquire(&self, _device: &Device) -> Option<RenderTarget> {
        let state = self.state.lock();
        Some(RenderTarget {
            texture: state.texture.clone(),
            view: state.view.clone(),
            // The target reports the texture's real size, not the size that was
            // requested: a frame drawn before a pending resize is applied has to be
            // skipped the same way a mismatched swapchain image is.
            size: (state.texture.width(), state.texture.height()),
            format: TEXTURE_FORMAT,
            delivery: Delivery::Texture,
        })
    }

    fn refresh(&self, device: &Device) {
        let mut state = self.state.lock();
        if (state.texture.width(), state.texture.height()) == state.size {
            return;
        }
        let (texture, view) = create_output_texture(device, state.size);
        state.texture = texture;
        state.view = view;
    }

    fn release(&self) {
        // An engine-owned texture is not something the platform can take away, so
        // there is nothing to release or rebuild.
    }

    fn recreate(&self, _device: &Device) {}
}

/// Create the engine-owned texture a texture output renders into.
///
/// `TEXTURE_BINDING` is what lets a host sample the finished frame; `COPY_SRC` keeps
/// screenshots working the same way they do on the swapchain output.
fn create_output_texture(device: &Device, size: (u32, u32)) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("Frame Output Texture"),
        size: Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TEXTURE_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: TEXTURE_VIEW_FORMATS,
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

/// Whether the window system can provide a surface at all decides which output the
/// engine renders into. `Unsupported` means the platform has no surface to give
/// (headless runs, hosts that sample a texture), so the engine owns the texture.
pub(crate) fn create_render_output(
    window_system: &Arc<dyn WindowSystem>,
    instance: &Instance,
    adapter: &Adapter,
    device: &Device,
    surface: Option<Surface<'static>>,
    size: (u32, u32),
) -> Box<dyn FrameOutput> {
    match surface {
        Some(surface) => Box::new(SwapchainOutput::new(
            window_system.clone(),
            instance.clone(),
            surface,
            adapter,
            device,
            size,
        )),
        None => Box::new(TextureOutput::new(device, size)),
    }
}

#[inline(always)]
fn choose_surface_format(formats: &[TextureFormat]) -> TextureFormat {
    if formats.contains(&TextureFormat::Bgra8Unorm) {
        return TextureFormat::Bgra8Unorm;
    }

    if formats.contains(&TextureFormat::Rgba8Unorm) {
        return TextureFormat::Rgba8Unorm;
    }

    panic!(
        "Cannot find a proper surface format. Expected Bgra8Unorm or Rgba8Unorm, available: {:?}",
        formats
    );
}
