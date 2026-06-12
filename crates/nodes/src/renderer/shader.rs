use moyu_core::base::{Bound, MVPMatrix, Rect};
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{
    Node, NodeBaseTrait, RenderCommandSender, Renderer, RendererUpdatePayload,
};
use moyu_core::utils::coordinates::calculate_surface_physical_coordinates;
use wgpu::*;

use super::pass::{ShaderPass, ShaderPassBuiltins};
use crate::nodes::{
    RetainMode, Shader, ShaderSlot, ShaderTimeControl, TransitionFromSource, TransitionPhase,
};

#[derive(Clone, Copy)]
struct ShaderSlotDescriptor {
    channel: usize,
    empty: bool,
    is_static: bool,
    width: u32,
    height: u32,
    bounds: Option<Bound>,
}

pub struct ShaderRenderer {
    pass: ShaderPass,
    present_bind_group_layout: BindGroupLayout,
    present_pipeline: RenderPipeline,
}

pub struct ShaderSlotRenderer;

impl ShaderRenderer {
    const IDLE_TEXTURE_GRACE_SECONDS: f64 = 5.0;
    const BOOTSTRAP_RECT_SIZE: f32 = 100.0;

    fn ensure_channel_texture(
        &self,
        shader: &mut Shader,
        device: &Device,
        channel: usize,
        width: u32,
        height: u32,
        empty: bool,
    ) -> bool {
        if width == 0 || height == 0 {
            shader.channel_views[channel] = None;
            shader.channel_texture_widths[channel] = 0;
            shader.channel_texture_heights[channel] = 0;
            shader.channel_empty[channel] = empty;
            shader.bind_group = None;
            shader.present_bind_group = None;
            return false;
        }

        let needs_recreation = shader.channel_views[channel].is_none()
            || shader.channel_texture_widths[channel] != width
            || shader.channel_texture_heights[channel] != height
            || shader.channel_empty[channel] != empty;

        if needs_recreation {
            let label = format!("Shader Channel {channel}");
            shader.channel_views[channel] =
                Some(self.pass.create_texture_view(device, width, height, &label));
            shader.channel_texture_widths[channel] = width;
            shader.channel_texture_heights[channel] = height;
            shader.channel_empty[channel] = empty;
            shader.bind_group = None;
            shader.present_bind_group = None;
        }

        needs_recreation
    }

    fn ensure_display_texture(
        &self,
        shader: &mut Shader,
        device: &Device,
        width: u32,
        height: u32,
    ) -> bool {
        if width == 0 || height == 0 {
            shader.display_view = None;
            shader.display_texture_width = 0;
            shader.display_texture_height = 0;
            shader.present_bind_group = None;
            return false;
        }

        let needs_recreation = shader.display_view.is_none()
            || shader.display_texture_width != width
            || shader.display_texture_height != height;

        if needs_recreation {
            shader.display_view =
                Some(
                    self.pass
                        .create_texture_view(device, width, height, "Shader Display"),
                );
            shader.display_texture_width = width;
            shader.display_texture_height = height;
            shader.present_bind_group = None;
        }

        needs_recreation
    }

    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let pass = ShaderPass::new(device, config);

        let present_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shader Present Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let present_vertex_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader Present Vertex Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader_vertex.wgsl").into()),
        });
        let present_fragment_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader Present Fragment Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader_present.wgsl").into()),
        });
        let present_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Shader Present Pipeline Layout"),
            bind_group_layouts: &[
                &MVPMatrix::bind_group_layout(device),
                &present_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let present_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Shader Present Pipeline"),
            layout: Some(&present_pipeline_layout),
            vertex: VertexState {
                module: &present_vertex_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &present_fragment_module,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pass,
            present_bind_group_layout,
            present_pipeline,
        }
    }

    fn slot_children_bounds(slot: &ShaderSlot) -> Option<Bound> {
        let mut bounds: Option<Bound> = None;

        for child in slot.base().children() {
            let child = child.read();
            let child_bounds = *child.base().global_content_bounds();

            if child_bounds.is_empty() {
                continue;
            }

            bounds = Some(match bounds {
                Some(current) => current.union(&child_bounds),
                None => child_bounds,
            });
        }

        bounds
    }

    fn resolve_shader_rect(stage_rect: Rect, slots: &[ShaderSlotDescriptor]) -> Option<Rect> {
        let mut bounds: Option<Bound> = None;

        for slot in slots {
            let Some(slot_bounds) = slot.bounds else {
                continue;
            };

            bounds = Some(match bounds {
                Some(current) => current.union(&slot_bounds),
                None => slot_bounds,
            });
        }

        let bounds = bounds?;
        let clamped = bounds.clamp(
            stage_rect.x(),
            stage_rect.y(),
            stage_rect.x() + stage_rect.width(),
            stage_rect.y() + stage_rect.height(),
        );

        if clamped.is_empty() {
            None
        } else {
            Some(clamped.into_rect())
        }
    }

    fn resolve_transition_rect(
        stage_rect: Rect,
        from_bounds: Option<Bound>,
        to_bounds: Option<Bound>,
    ) -> Option<Rect> {
        let union = match (from_bounds, to_bounds) {
            (Some(from_bounds), Some(to_bounds)) => Some(from_bounds.union(&to_bounds)),
            (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
            (None, None) => None,
        }?;

        let clamped = union.clamp(
            stage_rect.x(),
            stage_rect.y(),
            stage_rect.x() + stage_rect.width(),
            stage_rect.y() + stage_rect.height(),
        );

        if clamped.is_empty() {
            None
        } else {
            Some(clamped.into_rect())
        }
    }

    fn bootstrap_transition_rect(stage_rect: Rect) -> Rect {
        let width = Self::BOOTSTRAP_RECT_SIZE.min(stage_rect.width());
        let height = Self::BOOTSTRAP_RECT_SIZE.min(stage_rect.height());
        Rect::new(stage_rect.x(), stage_rect.y(), width, height)
    }

    fn create_present_bind_group(
        &self,
        device: &Device,
        uniform_buffer: &Buffer,
        source_view: &TextureView,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shader Present Bind Group"),
            layout: &self.present_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(self.pass.sampler()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(source_view),
                },
            ],
        })
    }
}

impl ShaderSlotRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Renderer for ShaderRenderer {
    fn name(&self) -> &'static str {
        "shader"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!("ShaderRenderer keeps the pipeline on the node")
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        self.pass.bind_group_layout()
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        let shader = node.as_any_mut().downcast_mut::<Shader>().unwrap();

        let stage_rect = Rect::new(
            0.0,
            0.0,
            payload.stage_logical_size.0,
            payload.stage_logical_size.1,
        );
        let children = shader.base().children().clone();
        let mut slots = Vec::new();
        let mut slot_key_parts = Vec::new();
        let mut duplicate_channel = None;

        for child in &children {
            let child = child.read();
            let Some(slot) = child.as_any().downcast_ref::<ShaderSlot>() else {
                continue;
            };

            if slot.channel > 3 {
                shader.mark_error(format!(
                    "channel {} is out of range, expected 0..3",
                    slot.channel
                ));
                return;
            }

            let channel = slot.channel as usize;
            if slots
                .iter()
                .any(|item: &ShaderSlotDescriptor| item.channel == channel)
            {
                duplicate_channel = Some(slot.channel);
            }

            slot_key_parts.push(format!(
                "{}:{}:{}:{}:{}",
                slot.channel, slot.empty, slot.is_static, slot.width, slot.height
            ));
            slots.push(ShaderSlotDescriptor {
                channel,
                empty: slot.empty,
                is_static: slot.is_static,
                width: slot.width,
                height: slot.height,
                bounds: if slot.empty {
                    None
                } else {
                    Self::slot_children_bounds(slot)
                },
            });
        }

        let slot_key = slot_key_parts.join("|");
        shader.update_slot_layout_key(slot_key);

        if let Some(channel) = duplicate_channel {
            shader.mark_error(format!("channel {} is declared more than once", channel));
            return;
        }

        if let Some(display_channel) = shader.display_channel {
            if display_channel > 3 {
                shader.mark_error(format!(
                    "displayChannel {} is out of range, expected 0..3",
                    display_channel
                ));
                return;
            }
        }

        if shader.error_state && !shader.needs_retry {
            return;
        }

        let mut declared_channels = [false; 4];
        for slot in &slots {
            declared_channels[slot.channel] = true;
        }

        for (index, declared) in declared_channels.iter().copied().enumerate() {
            if !declared {
                shader.channel_declared[index] = false;
                shader.channel_empty[index] = false;
                shader.channel_static[index] = false;
                shader.channel_texture_widths[index] = 0;
                shader.channel_texture_heights[index] = 0;
                shader.channel_views[index] = None;
                shader.channel_needs_redraw[index] = true;
                shader.present_bind_group = None;
            }
        }

        let effect_id = shader.shader.builtin_effect_id();

        match shader.time_control {
            ShaderTimeControl::Auto | ShaderTimeControl::Manual => {
                if let Some(last_active_at) = shader.last_active_at {
                    if !shader.base().visible()
                        && payload.timestamp - last_active_at >= Self::IDLE_TEXTURE_GRACE_SECONDS
                    {
                        shader.clear_idle_runtime_state();
                        shader.last_active_at = None;
                    }
                }

                let resolved_rect = Self::resolve_shader_rect(stage_rect, &slots).or_else(|| {
                    if slots.iter().any(|slot| !slot.empty) {
                        if shader.shader_rect.width() > 0.0 && shader.shader_rect.height() > 0.0 {
                            Some(shader.shader_rect)
                        } else {
                            Some(stage_rect)
                        }
                    } else {
                        None
                    }
                });

                if let Some(rect) = resolved_rect {
                    let (_, _, render_width, render_height) =
                        calculate_surface_physical_coordinates(
                            &rect,
                            payload.stage_logical_size,
                            payload.surface_logical_size,
                            payload.scale_factor,
                        );

                    shader.shader_rect = rect;
                    shader.render_width = render_width;
                    shader.render_height = render_height;
                    shader.last_active_at = Some(payload.timestamp);
                } else {
                    shader.shader_rect = Rect::default();
                    shader.render_width = 0;
                    shader.render_height = 0;
                }

                if shader.render_width > 0 && shader.render_height > 0 {
                    for slot in &slots {
                        shader.channel_declared[slot.channel] = true;

                        if shader.channel_static[slot.channel] != slot.is_static {
                            shader.channel_static[slot.channel] = slot.is_static;
                            shader.channel_needs_redraw[slot.channel] = true;
                        }

                        let (width, height) = if slot.empty {
                            (slot.width, slot.height)
                        } else {
                            (shader.render_width, shader.render_height)
                        };

                        if slot.empty && (width == 0 || height == 0) {
                            shader.mark_error(format!(
                                "empty channel {} requires a non-zero width and height",
                                slot.channel
                            ));
                            return;
                        }

                        let recreated = self.ensure_channel_texture(
                            shader,
                            device,
                            slot.channel,
                            width,
                            height,
                            slot.empty,
                        );

                        if recreated {
                            shader.channel_needs_redraw[slot.channel] = true;
                        }
                    }
                }

                let mut static_rendered = [false; 4];
                for child in &children {
                    let mut child = child.write();
                    let Some(slot) = child.as_any_mut().downcast_mut::<ShaderSlot>() else {
                        continue;
                    };

                    slot.render_rect = shader.shader_rect;

                    if shader.render_width == 0 || shader.render_height == 0 || slot.empty {
                        slot.render_target = None;
                        slot.render_children = false;
                        continue;
                    }

                    let channel = slot.channel as usize;
                    let Some(target_view) = shader.channel_views[channel].clone() else {
                        slot.render_target = None;
                        slot.render_children = false;
                        continue;
                    };

                    if slot.is_static {
                        let should_render = shader.channel_needs_redraw[channel];
                        slot.render_children = should_render;
                        slot.render_target = if should_render {
                            Some(target_view)
                        } else {
                            None
                        };

                        if should_render {
                            static_rendered[channel] = true;
                        }
                    } else {
                        slot.render_children = true;
                        slot.render_target = Some(target_view);
                    }
                }

                for (channel, rendered) in static_rendered.iter().copied().enumerate() {
                    if rendered {
                        shader.channel_needs_redraw[channel] = false;
                    }
                }

                if shader.render_width == 0 || shader.render_height == 0 {
                    shader.bind_group = None;
                    shader.present_bind_group = None;
                    shader.finish_update();
                    return;
                }

                if shader.shader_dirty || shader.pipeline.is_none() || shader.needs_retry {
                    match self.pass.compile_pipeline(device, &shader.shader) {
                        Ok(pipeline) => {
                            shader.pipeline = Some(pipeline);
                            shader.bind_group = None;
                        }
                        Err(err) => {
                            shader.mark_error(err);
                            return;
                        }
                    }
                }

                self.pass.ensure_uniform_buffers(
                    device,
                    &mut shader.render_uniform_buffer,
                    &mut shader.builtins_uniform_buffer,
                    &mut shader.params_uniform_buffer,
                );

                if shader.bind_group.is_none() || shader.params_dirty || shader.slots_dirty {
                    let (
                        Some(render_uniform_buffer),
                        Some(builtins_uniform_buffer),
                        Some(params_uniform_buffer),
                    ) = (
                        shader.render_uniform_buffer.as_ref(),
                        shader.builtins_uniform_buffer.as_ref(),
                        shader.params_uniform_buffer.as_ref(),
                    )
                    else {
                        shader.mark_error("shader uniform buffers are not initialized");
                        return;
                    };

                    shader.bind_group = Some(self.pass.create_bind_group(
                        device,
                        render_uniform_buffer,
                        builtins_uniform_buffer,
                        params_uniform_buffer,
                        &shader.channel_views,
                    ));
                }

                shader.present_bind_group = None;

                let (time, time_delta, frame) = shader.advance_generic_timeline(payload.timestamp);

                if let Some(buffer) = shader.render_uniform_buffer.as_ref() {
                    self.pass
                        .write_render_uniform(render_queue, buffer, shader.shader_rect);
                }

                if let Some(buffer) = shader.builtins_uniform_buffer.as_ref() {
                    self.pass.write_builtins_uniform(
                        render_queue,
                        buffer,
                        ShaderPassBuiltins {
                            time,
                            time_delta,
                            progress: 0.0,
                            effect_id,
                            frame,
                            channel_count: declared_channels
                                .iter()
                                .filter(|declared| **declared)
                                .count() as u32,
                        },
                    );
                }

                if let Some(buffer) = shader.params_uniform_buffer.as_ref() {
                    let params = match shader.shader.pack_params_uniform_bytes() {
                        Ok(params) => params,
                        Err(err) => {
                            shader.mark_error(err);
                            return;
                        }
                    };

                    if let Err(err) = self
                        .pass
                        .write_params_uniform(render_queue, buffer, &params)
                    {
                        shader.mark_error(err);
                        return;
                    }
                }
            }
            ShaderTimeControl::Transition => {
                shader.finish_transition_if_ready();

                if let Some(request) = shader.pending_prepare.take() {
                    let has_from_slot = slots
                        .iter()
                        .any(|slot| slot.channel == request.from_channel as usize && !slot.empty);
                    let has_to_slot = slots
                        .iter()
                        .any(|slot| slot.channel == request.to_channel as usize && !slot.empty);

                    if !has_from_slot || !has_to_slot {
                        log::warn!(
                            "shader node {}: prepare requires declared non-empty fromChannel/toChannel slots",
                            shader.base().id()
                        );
                    } else {
                        let capture_display = matches!(
                            shader.transition_phase,
                            TransitionPhase::Running | TransitionPhase::Finishing
                        ) && shader.display_view.is_some();

                        shader.apply_prepare_request(request, capture_display);
                    }
                }

                if let Some(request) = shader.pending_perform.take() {
                    let Some(from_channel) = shader.transition_from_channel else {
                        log::warn!(
                            "shader node {}: perform requires a prior prepare in transition mode",
                            shader.base().id()
                        );
                        shader.present_bind_group = None;
                        shader.bind_group = None;
                        shader.finish_update();
                        return;
                    };
                    let Some(to_channel) = shader.transition_to_channel else {
                        log::warn!(
                            "shader node {}: perform requires a prior prepare in transition mode",
                            shader.base().id()
                        );
                        shader.present_bind_group = None;
                        shader.bind_group = None;
                        shader.finish_update();
                        return;
                    };

                    let has_from_slot = slots
                        .iter()
                        .any(|slot| slot.channel == from_channel as usize && !slot.empty);
                    let has_to_slot = slots
                        .iter()
                        .any(|slot| slot.channel == to_channel as usize && !slot.empty);

                    if !has_from_slot || !has_to_slot {
                        log::warn!(
                            "shader node {}: perform requires existing from/to slots",
                            shader.base().id()
                        );
                    } else {
                        shader.apply_perform_request(request.duration);
                    }
                }

                let from_channel = shader
                    .transition_from_channel
                    .map(|channel| channel as usize);
                let to_channel = shader.transition_to_channel.map(|channel| channel as usize);

                let from_bounds = from_channel.and_then(|channel| {
                    slots
                        .iter()
                        .find(|slot| slot.channel == channel && !slot.empty)
                        .and_then(|slot| slot.bounds)
                });
                let to_bounds = to_channel.and_then(|channel| {
                    slots
                        .iter()
                        .find(|slot| slot.channel == channel && !slot.empty)
                        .and_then(|slot| slot.bounds)
                });
                let has_to_slot = to_channel.is_some_and(|channel| {
                    slots
                        .iter()
                        .any(|slot| slot.channel == channel && !slot.empty)
                });

                if shader.is_active() {
                    if let Some(rect) =
                        Self::resolve_transition_rect(stage_rect, from_bounds, to_bounds)
                    {
                        let (_, _, render_width, render_height) =
                            calculate_surface_physical_coordinates(
                                &rect,
                                payload.stage_logical_size,
                                payload.surface_logical_size,
                                payload.scale_factor,
                            );

                        shader.shader_rect = rect;
                        shader.render_width = render_width;
                        shader.render_height = render_height;
                    } else if has_to_slot {
                        let rect = Self::bootstrap_transition_rect(stage_rect);
                        let (_, _, render_width, render_height) =
                            calculate_surface_physical_coordinates(
                                &rect,
                                payload.stage_logical_size,
                                payload.surface_logical_size,
                                payload.scale_factor,
                            );

                        shader.shader_rect = rect;
                        shader.render_width = render_width;
                        shader.render_height = render_height;
                    } else {
                        shader.shader_rect = Rect::default();
                        shader.render_width = 0;
                        shader.render_height = 0;
                    }

                    shader.last_active_at = Some(payload.timestamp);
                } else {
                    shader.shader_rect = Rect::default();
                    shader.render_width = 0;
                    shader.render_height = 0;

                    if let Some(last_active_at) = shader.last_active_at {
                        if payload.timestamp - last_active_at >= Self::IDLE_TEXTURE_GRACE_SECONDS {
                            shader.clear_idle_runtime_state();
                            shader.last_active_at = None;
                        }
                    }
                }

                if shader.is_active() && shader.render_width > 0 && shader.render_height > 0 {
                    for slot in &slots {
                        shader.channel_declared[slot.channel] = true;

                        if shader.channel_static[slot.channel] != slot.is_static {
                            shader.channel_static[slot.channel] = slot.is_static;
                            shader.channel_needs_redraw[slot.channel] = true;
                        }

                        let (width, height) = if slot.empty {
                            (slot.width, slot.height)
                        } else {
                            (shader.render_width, shader.render_height)
                        };

                        if slot.empty && (width == 0 || height == 0) {
                            shader.mark_error(format!(
                                "empty channel {} requires a non-zero width and height",
                                slot.channel
                            ));
                            return;
                        }

                        let recreated = self.ensure_channel_texture(
                            shader,
                            device,
                            slot.channel,
                            width,
                            height,
                            slot.empty,
                        );

                        if recreated {
                            shader.channel_needs_redraw[slot.channel] = true;

                            if Some(slot.channel) == from_channel
                                && (shader.retain == RetainMode::Static
                                    || shader.transition_from_source
                                        == TransitionFromSource::Display)
                            {
                                shader.from_needs_redraw = true;
                            }
                        }
                    }

                    self.ensure_display_texture(
                        shader,
                        device,
                        shader.render_width,
                        shader.render_height,
                    );
                    shader.display_rect = shader.shader_rect;
                }

                let mut snapshot_from_rendered = false;
                let mut static_rendered = [false; 4];
                let mut awaiting_prepare_captured = false;
                let display_channel = shader.display_channel.map(|channel| channel as usize);

                for child in &children {
                    let mut child = child.write();
                    let Some(slot) = child.as_any_mut().downcast_mut::<ShaderSlot>() else {
                        continue;
                    };

                    slot.render_rect = shader.shader_rect;

                    if slot.empty {
                        slot.render_target = None;
                        slot.render_children = false;
                        continue;
                    }

                    if matches!(shader.transition_phase, TransitionPhase::Stable) {
                        let should_display = display_channel == Some(slot.channel as usize);
                        slot.render_target = None;
                        slot.render_children = should_display;
                        continue;
                    }

                    if shader.render_width == 0 || shader.render_height == 0 {
                        slot.render_target = None;
                        slot.render_children = false;
                        continue;
                    }

                    let channel = slot.channel as usize;
                    let Some(target_view) = shader.channel_views[channel].clone() else {
                        slot.render_target = None;
                        slot.render_children = false;
                        continue;
                    };

                    if Some(channel) == from_channel {
                        if shader.transition_from_source == TransitionFromSource::Display {
                            slot.render_children = false;
                            slot.render_target = None;
                            continue;
                        }

                        match shader.transition_phase {
                            TransitionPhase::AwaitingPrepare => {
                                slot.render_children = true;
                                slot.render_target = Some(target_view);
                                awaiting_prepare_captured = true;
                            }
                            TransitionPhase::Prepared
                            | TransitionPhase::Running
                            | TransitionPhase::Finishing => match shader.retain {
                                RetainMode::Static => {
                                    let should_render = shader.from_needs_redraw;
                                    slot.render_children = should_render;
                                    slot.render_target = if should_render {
                                        Some(target_view)
                                    } else {
                                        None
                                    };

                                    if should_render {
                                        snapshot_from_rendered = true;
                                    }
                                }
                                RetainMode::Live => {
                                    slot.render_children = true;
                                    slot.render_target = Some(target_view);
                                }
                            },
                            TransitionPhase::Stable => {
                                slot.render_children = false;
                                slot.render_target = None;
                            }
                        }

                        continue;
                    }

                    if matches!(shader.transition_phase, TransitionPhase::AwaitingPrepare) {
                        slot.render_children = false;
                        slot.render_target = None;
                        continue;
                    }

                    let should_render = if slot.is_static {
                        shader.channel_needs_redraw[channel]
                    } else {
                        true
                    };

                    slot.render_children = should_render;
                    slot.render_target = if should_render {
                        Some(target_view)
                    } else {
                        None
                    };

                    if should_render && slot.is_static {
                        static_rendered[channel] = true;
                    }
                }

                if awaiting_prepare_captured {
                    shader.mark_prepare_captured();
                }

                if snapshot_from_rendered {
                    shader.from_needs_redraw = false;
                }

                for (channel, rendered) in static_rendered.iter().copied().enumerate() {
                    if rendered {
                        shader.channel_needs_redraw[channel] = false;
                    }
                }

                if !shader.is_active() || shader.render_width == 0 || shader.render_height == 0 {
                    shader.bind_group = None;
                    shader.present_bind_group = None;
                    shader.finish_update();
                    return;
                }

                self.pass.ensure_uniform_buffers(
                    device,
                    &mut shader.render_uniform_buffer,
                    &mut shader.builtins_uniform_buffer,
                    &mut shader.params_uniform_buffer,
                );

                if let Some(buffer) = shader.render_uniform_buffer.as_ref() {
                    self.pass
                        .write_render_uniform(render_queue, buffer, shader.shader_rect);
                }

                let Some(render_uniform_buffer) = shader.render_uniform_buffer.as_ref() else {
                    shader.mark_error("shader render uniform buffer is not initialized");
                    return;
                };

                match shader.transition_phase {
                    TransitionPhase::AwaitingPrepare | TransitionPhase::Prepared => {
                        shader.bind_group = None;
                        shader.snapshot_bind_group = None;

                        let source_view = match shader.transition_from_source {
                            TransitionFromSource::Display => shader.snapshot_display_view.as_ref(),
                            TransitionFromSource::Slot => from_channel
                                .and_then(|channel| shader.channel_views[channel].as_ref()),
                        };

                        shader.present_bind_group = source_view.map(|source_view| {
                            self.create_present_bind_group(
                                device,
                                render_uniform_buffer,
                                source_view,
                            )
                        });
                    }
                    TransitionPhase::Running | TransitionPhase::Finishing => {
                        if shader.shader_dirty || shader.pipeline.is_none() || shader.needs_retry {
                            match self.pass.compile_pipeline(device, &shader.shader) {
                                Ok(pipeline) => {
                                    shader.pipeline = Some(pipeline);
                                    shader.bind_group = None;
                                }
                                Err(err) => {
                                    shader.mark_error(err);
                                    return;
                                }
                            }
                        }

                        if shader.bind_group.is_none() || shader.params_dirty || shader.slots_dirty
                        {
                            let (
                                Some(render_uniform_buffer),
                                Some(builtins_uniform_buffer),
                                Some(params_uniform_buffer),
                            ) = (
                                shader.render_uniform_buffer.as_ref(),
                                shader.builtins_uniform_buffer.as_ref(),
                                shader.params_uniform_buffer.as_ref(),
                            )
                            else {
                                shader.mark_error("shader uniform buffers are not initialized");
                                return;
                            };

                            shader.bind_group = Some(self.pass.create_bind_group(
                                device,
                                render_uniform_buffer,
                                builtins_uniform_buffer,
                                params_uniform_buffer,
                                &shader.channel_views,
                            ));
                        }

                        shader.present_bind_group =
                            shader.display_view.as_ref().map(|display_view| {
                                self.create_present_bind_group(
                                    device,
                                    render_uniform_buffer,
                                    display_view,
                                )
                            });

                        if shader.transition_from_source == TransitionFromSource::Display
                            && shader.from_needs_redraw
                        {
                            let Some(snapshot_display_view) = shader.snapshot_display_view.as_ref()
                            else {
                                shader.mark_error(
                                    "display-backed transition snapshot is not available",
                                );
                                return;
                            };

                            let snapshot_uniform_buffer =
                                shader.snapshot_uniform_buffer.get_or_insert_with(|| {
                                    device.create_buffer(&BufferDescriptor {
                                        label: Some("Shader Snapshot Uniform Buffer"),
                                        size: 16,
                                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                                        mapped_at_creation: false,
                                    })
                                });

                            self.pass.write_render_uniform(
                                render_queue,
                                snapshot_uniform_buffer,
                                shader.snapshot_display_rect,
                            );

                            shader.snapshot_bind_group = Some(self.create_present_bind_group(
                                device,
                                snapshot_uniform_buffer,
                                snapshot_display_view,
                            ));
                            shader.from_needs_redraw = false;
                        } else {
                            shader.snapshot_bind_group = None;
                        }

                        let (time, time_delta, frame, progress) =
                            shader.advance_transition_timeline(payload.timestamp);

                        if let Some(buffer) = shader.builtins_uniform_buffer.as_ref() {
                            self.pass.write_builtins_uniform(
                                render_queue,
                                buffer,
                                ShaderPassBuiltins {
                                    time,
                                    time_delta,
                                    progress,
                                    effect_id,
                                    frame,
                                    channel_count: declared_channels
                                        .iter()
                                        .filter(|declared| **declared)
                                        .count()
                                        as u32,
                                },
                            );
                        }

                        if let Some(buffer) = shader.params_uniform_buffer.as_ref() {
                            let params = match shader.shader.pack_params_uniform_bytes() {
                                Ok(params) => params,
                                Err(err) => {
                                    shader.mark_error(err);
                                    return;
                                }
                            };

                            if let Err(err) =
                                self.pass
                                    .write_params_uniform(render_queue, buffer, &params)
                            {
                                shader.mark_error(err);
                                return;
                            }
                        }
                    }
                    TransitionPhase::Stable => {
                        shader.bind_group = None;
                        shader.present_bind_group = None;
                        shader.snapshot_bind_group = None;
                    }
                }
            }
        }

        shader.finish_update();
    }

    fn should_collect_commands(&self, node: &dyn Node, stage_bound: &Bound) -> bool {
        let shader = node.as_any().downcast_ref::<Shader>().unwrap();

        let ready_to_draw = match shader.time_control {
            ShaderTimeControl::Auto | ShaderTimeControl::Manual => {
                shader.pipeline.is_some() && shader.bind_group.is_some()
            }
            ShaderTimeControl::Transition => match shader.transition_phase {
                TransitionPhase::AwaitingPrepare | TransitionPhase::Prepared => {
                    shader.present_bind_group.is_some()
                }
                TransitionPhase::Running | TransitionPhase::Finishing => {
                    shader.pipeline.is_some()
                        && shader.bind_group.is_some()
                        && shader.present_bind_group.is_some()
                }
                TransitionPhase::Stable => false,
            },
        };

        shader.base().visible()
            && !shader.error_state
            && ready_to_draw
            && shader.render_width > 0
            && shader.render_height > 0
            && Bound::from(shader.shader_rect).intersects(stage_bound)
    }

    fn collect_commands(&self, _node: &dyn Node, _render_queue: &RenderCommandSender) {}

    fn collect_post_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let shader = node.as_any().downcast_ref::<Shader>().unwrap();

        match shader.time_control {
            ShaderTimeControl::Auto | ShaderTimeControl::Manual => {
                let (Some(pipeline), Some(bind_group)) =
                    (shader.pipeline.as_ref(), shader.bind_group.as_ref())
                else {
                    return;
                };

                render_queue
                    .send(RenderCommand::Draw {
                        pipeline: pipeline.clone(),
                        bind_group: bind_group.clone(),
                        extra_bind_groups: vec![],
                        vertex_buffer: None,
                        index_buffer: None,
                        instance_buffer: None,
                        count: 6,
                        instance_count: 1,
                    })
                    .unwrap();
            }
            ShaderTimeControl::Transition => match shader.transition_phase {
                TransitionPhase::AwaitingPrepare | TransitionPhase::Prepared => {
                    let Some(bind_group) = shader.present_bind_group.as_ref() else {
                        return;
                    };

                    render_queue
                        .send(RenderCommand::Draw {
                            pipeline: self.present_pipeline.clone(),
                            bind_group: bind_group.clone(),
                            extra_bind_groups: vec![],
                            vertex_buffer: None,
                            index_buffer: None,
                            instance_buffer: None,
                            count: 6,
                            instance_count: 1,
                        })
                        .unwrap();
                }
                TransitionPhase::Running | TransitionPhase::Finishing => {
                    let (
                        Some(display_view),
                        Some(pipeline),
                        Some(bind_group),
                        Some(present_bind_group),
                    ) = (
                        shader.display_view.as_ref(),
                        shader.pipeline.as_ref(),
                        shader.bind_group.as_ref(),
                        shader.present_bind_group.as_ref(),
                    )
                    else {
                        return;
                    };

                    if shader.transition_from_source == TransitionFromSource::Display {
                        let from_view = shader
                            .transition_from_channel
                            .and_then(|channel| shader.channel_views[channel as usize].as_ref());
                        let snapshot_bind_group = shader.snapshot_bind_group.as_ref();

                        if let (Some(from_view), Some(snapshot_bind_group)) =
                            (from_view, snapshot_bind_group)
                        {
                            render_queue
                                .send(RenderCommand::BeginRenderTargetPass {
                                    target_view: from_view.clone(),
                                    rect: shader.shader_rect,
                                })
                                .unwrap();

                            render_queue
                                .send(RenderCommand::Draw {
                                    pipeline: self.present_pipeline.clone(),
                                    bind_group: snapshot_bind_group.clone(),
                                    extra_bind_groups: vec![],
                                    vertex_buffer: None,
                                    index_buffer: None,
                                    instance_buffer: None,
                                    count: 6,
                                    instance_count: 1,
                                })
                                .unwrap();

                            render_queue
                                .send(RenderCommand::EndRenderTargetPass)
                                .unwrap();
                        }
                    }

                    render_queue
                        .send(RenderCommand::BeginRenderTargetPass {
                            target_view: display_view.clone(),
                            rect: shader.shader_rect,
                        })
                        .unwrap();

                    render_queue
                        .send(RenderCommand::Draw {
                            pipeline: pipeline.clone(),
                            bind_group: bind_group.clone(),
                            extra_bind_groups: vec![],
                            vertex_buffer: None,
                            index_buffer: None,
                            instance_buffer: None,
                            count: 6,
                            instance_count: 1,
                        })
                        .unwrap();

                    render_queue
                        .send(RenderCommand::EndRenderTargetPass)
                        .unwrap();

                    render_queue
                        .send(RenderCommand::Draw {
                            pipeline: self.present_pipeline.clone(),
                            bind_group: present_bind_group.clone(),
                            extra_bind_groups: vec![],
                            vertex_buffer: None,
                            index_buffer: None,
                            instance_buffer: None,
                            count: 6,
                            instance_count: 1,
                        })
                        .unwrap();
                }
                TransitionPhase::Stable => {}
            },
        }
    }
}

impl Renderer for ShaderSlotRenderer {
    fn name(&self) -> &'static str {
        "shader-slot"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!("ShaderSlotRenderer does not draw directly")
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        unreachable!("ShaderSlotRenderer does not allocate bind groups")
    }

    fn update(
        &mut self,
        _node: &mut dyn Node,
        _device: &Device,
        _queue: &Queue,
        _render_queue: &RenderCommandSender,
        _payload: &RendererUpdatePayload,
    ) {
    }

    fn should_collect_commands(&self, node: &dyn Node, _stage_bound: &Bound) -> bool {
        let slot = node.as_any().downcast_ref::<ShaderSlot>().unwrap();

        slot.render_children
            && slot.render_target.is_some()
            && slot.render_rect.width() > 0.0
            && slot.render_rect.height() > 0.0
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let slot = node.as_any().downcast_ref::<ShaderSlot>().unwrap();

        if let Some(target_view) = slot.render_target.as_ref() {
            render_queue
                .send(RenderCommand::BeginRenderTargetPass {
                    target_view: target_view.clone(),
                    rect: slot.render_rect,
                })
                .unwrap();
        }
    }

    fn collect_post_commands(&self, _node: &dyn Node, render_queue: &RenderCommandSender) {
        render_queue
            .send(RenderCommand::EndRenderTargetPass)
            .unwrap();
    }
}
