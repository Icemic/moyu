use anyhow::Result;
use ffmpeg_rs::decoder::Check;
use ffmpeg_rs::format::{input, Pixel};
use ffmpeg_rs::media::Type;
use ffmpeg_rs::software::scaling::{context::Context, flag::Flags};
use ffmpeg_rs::util::frame::video::Video as FFmpegVideo;
use ffmpeg_rs::Packet;
use hai_macros::node;
use hai_pal::sync::RwLock;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::Debug;
use std::ops::{Deref, Mul};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::{DeviceExt, StagingBelt};
use wgpu::{BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Queue};

use crate::traits::{
    Focusable, Node, NodeType, Renderable, RendererUpdatePayload, UpdateProps, NODE_ID,
};
use crate::types::{Point, SurfaceSize, Transform, Vertex};
use crate::utils::calculate::calculate_rect_vertices;
use crate::utils::convert::{from_js, JSValue};

use super::{Texture, TextureStatus};

struct VideoFrame(FFmpegVideo);

impl Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VideoFrame")
            .field(&self.0.format())
            .field(&self.0.width())
            .field(&self.0.height())
            .finish()
    }
}

impl Deref for VideoFrame {
    type Target = FFmpegVideo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[node(renderable)]
#[derive(Debug)]
pub struct Video {
    /// loaded texture
    pub texture: Arc<RwLock<Texture>>,
    /// clip area
    pub area: [f64; 4],
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,

    pub bind_group: Option<BindGroup>,
    pub vertex_buffer: Option<Buffer>,

    pending_frame: Arc<RwLock<Option<VideoFrame>>>,
    src: String,
    mode: VideoPlayingMode,
}

impl Video {
    pub fn new(label: String) -> Self {
        let id = unsafe {
            NODE_ID += 1;
            NODE_ID
        };

        let texture = Arc::new(RwLock::new(Texture::new()));

        Video {
            id,
            label,
            anchor: Point::default(),
            pivot: Point::default(),
            translate: Point::default(),
            scale: Point::one(),
            rotation: 0.,
            skew: Point::default(),

            _update_id: 0,
            _current_update_id: 1,

            transform: Transform::default(),
            global_transform: Transform::default(),
            children: vec![],

            texture,
            area: [0., 0., 1., 1.],
            vertices: None,
            bind_group: None,
            vertex_buffer: None,

            pending_frame: Arc::new(RwLock::new(None)),
            src: String::new(),
            mode: VideoPlayingMode::File,
        }
    }

    fn calculate_vertices(&mut self, surface_size: &SurfaceSize) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let (logical_width, logical_height) = surface_size.logical_size();
        let scale_factor = surface_size.scale_factor();
        let texture = self.texture.read();
        let width = (texture.width as f64 * scale_factor) / (logical_width * scale_factor) * 2.;
        let height =
            (texture.height as f64 * scale_factor) / (logical_height * scale_factor) as f64 * 2.;

        drop(texture);

        let vertices = calculate_rect_vertices(self, width, height, &self.area);

        self.vertices = Some(vertices);
    }

    fn play(&mut self) -> Result<()> {
        let mut ictx = input(&PathBuf::from_str(&self.src)?)?;
        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg_rs::Error::StreamNotFound)?;
        let video_stream_index = input.index();

        let mut packet = Packet::new(0);

        let codec = ffmpeg_rs::codec::decoder::find_by_name("h264").unwrap();
        let mut context_decoder = ffmpeg_rs::codec::context::Context::new_with_codec(&codec);
        let mut params = input.parameters();
        params.set_codec_id(codec.id());
        context_decoder.set_parameters(params)?;

        let decoder = context_decoder.decoder();
        let mut decoder = decoder.open_as(codec).and_then(|o| o.video()).unwrap();

        decoder.check(Check::IGNORE_ERROR);

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            Flags::LANCZOS,
        )?;

        let pending_frame = self.pending_frame.clone();

        let time_base = input.time_base();
        let time_base = std::time::Duration::from_nanos(
            1_000_000_000 * time_base.0 as u64 / time_base.1 as u64,
        );

        let start_pts = input.start_time();
        let start_time = Instant::now();

        let mode = self.mode.clone();

        // use dedicated thread instead of tokio thread pool (including spawn_blocking)
        // ref: https://stackoverflow.com/questions/74547541/when-should-you-use-tokios-spawn-blocking
        std::thread::spawn(move || -> Result<()> {
            loop {
                if let Ok(()) = packet.read(&mut ictx) {
                    if packet.stream() == video_stream_index {
                        decoder.send_packet(&packet).unwrap_or_else(|err| {
                            warn!("decoder error: {}", err);
                        });
                        let mut decoded = FFmpegVideo::empty();
                        while decoder.receive_frame(&mut decoded).is_ok() {
                            let mut rgb_frame = FFmpegVideo::empty();
                            scaler.run(&decoded, &mut rgb_frame)?;
                            if unsafe { decoded.is_empty() } {
                                println!("aaa");
                            }

                            if let Some(current_pts) = decoded.pts() {
                                //  a streaming file will have a negative start_pts, so do not control time
                                if start_pts >= 0 {
                                    while start_time.elapsed()
                                        < time_base.mul((current_pts - start_pts) as u32)
                                    {
                                        std::thread::yield_now();
                                    }
                                }
                            }

                            *pending_frame.write() = Some(VideoFrame(rgb_frame));
                        }
                    }
                } else if mode == VideoPlayingMode::File {
                    break;
                }
            }

            debug!("file ended.");

            Ok(())
        });

        Ok(())
    }
}

impl NodeType for Video {
    fn node_type(&self) -> &'static str {
        "video"
    }
}

impl Renderable for Video {
    fn update(
        &mut self,
        device: &Arc<Device>,
        queue: &Arc<Queue>,
        encoder: &mut CommandEncoder,
        staging_belt: &mut StagingBelt,
        bind_group_layout: &BindGroupLayout,
        payload: &RendererUpdatePayload,
    ) {
        if let Some(frame) = self.pending_frame.read().as_ref() {
            {
                let mut texture = self.texture.write();

                // texture is not created
                if texture.status() == TextureStatus::Reading {
                    let size = wgpu::Extent3d {
                        width: frame.width(),
                        height: frame.height(),
                        depth_or_array_layers: 1,
                    };

                    let texture_gpu = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some(""),
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        view_formats: &vec![],
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    });

                    let view = texture_gpu.create_view(&wgpu::TextureViewDescriptor::default());
                    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    });

                    texture.set_texture(texture_gpu, view, sampler);
                    texture.set_size(frame.width(), frame.height());
                    texture.set_status(TextureStatus::Ready);
                }
            }

            let texture = self.texture.read();

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: texture.texture_unwrap(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                frame.data(0),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(4 * frame.width()),
                    rows_per_image: std::num::NonZeroU32::new(frame.height()),
                },
                texture.texture_unwrap().size(),
            );
        }

        self.calculate_vertices(&payload.surface_size);

        let vertices = self.vertices.as_ref().unwrap();

        /*
         * bind group and vertex buffer should be created at the same time.
         * if bind_group (as well as vertex_buffer) is none, try to create it.
         */
        if self.bind_group.is_none() {
            let texture = self.texture.read();
            if let TextureStatus::Ready = texture.status() {
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                texture.view.as_ref().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                texture.sampler.as_ref().unwrap(),
                            ),
                        },
                    ],
                    label: Some("bind_group"),
                });

                // release texture lock for better performance
                drop(texture);

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

                self.bind_group = Some(bind_group);
                self.vertex_buffer = Some(vertex_buffer);
            };
        } else {
            let buf = bytemuck::cast_slice(self.vertices.as_ref().unwrap());
            staging_belt
                .write_buffer(
                    encoder,
                    self.vertex_buffer.as_ref().unwrap(),
                    0,
                    (buf.len() as u64).try_into().unwrap(),
                    &device,
                )
                .copy_from_slice(buf);
        }
    }

    fn get_renderable(&self) -> Option<(&BindGroup, &wgpu::Buffer)> {
        if self.bind_group.is_some() {
            Some((
                self.bind_group.as_ref().unwrap(),
                self.vertex_buffer.as_ref().unwrap(),
            ))
        } else {
            None
        }
    }
}

impl Focusable for Video {
    fn contains(&self, x: f64, y: f64) -> bool {
        let texture = self.texture.read();

        let translate = self.translate();

        if x > translate.x
            && x < texture.width as f64 + translate.x
            && y > translate.y
            && y < texture.height as f64 + translate.y
        {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoPlayingMode {
    File,
    Stream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProps {
    pub src: Option<String>,
    pub area: Option<[f64; 4]>,
    pub autoplay: Option<bool>,
    pub mode: Option<VideoPlayingMode>,
}

impl UpdateProps for Video {
    fn update_properties(&mut self, props: &mut JSValue) {
        let props: VideoProps = from_js(props).unwrap();

        if let Some(area) = props.area {
            self.area = area;
        }

        if let Some(mode) = props.mode {
            self.mode = mode;
        }

        if let Some(src) = props.src {
            // FIXME: path should be relative to assets/
            self.src = src;

            if let Err(err) = self.play() {
                error!("{}", err);
            }
        }

        // force update vertices
        self._update_id += 1;
    }
}
