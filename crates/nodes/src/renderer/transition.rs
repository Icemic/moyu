use bytemuck::{Pod, Zeroable};
use moyu_core::base::{Bound, MVPMatrix, Rect};
use moyu_core::core::render_command::RenderCommand;
use moyu_core::traits::{
    Node, NodeBaseTrait, NodeEventSource, RenderCommandSender, Renderer, RendererUpdatePayload,
};
use moyu_core::utils::coordinates::calculate_surface_physical_coordinates;
use wgpu::util::DeviceExt;
use wgpu::*;

use crate::nodes::{
    RetainMode, TransitionContainer, TransitionPhase, TransitionSlot, TransitionSlotPhase,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TransitionParams {
    // Logical destination rect of the current blit/composite pass.
    position: [f32; 2],
    size: [f32; 2],
    // The allocated texture can be larger than the active rect because
    // transition allocations only grow while active.
    uv_scale: [f32; 2],
    progress: f32,
    // Negative means "plain blit"; non-negative values select an effect.
    effect_id: i32,
}

pub struct TransitionContainerRenderer {
    // The same pipeline is used for both effect composition and final present.
    // The only difference between the two passes is which textures and params
    // are bound at draw time.
    format: TextureFormat,
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    sampler: Sampler,
}

pub struct TransitionSlotRenderer;

impl TransitionContainerRenderer {
    // Seed rect used to bootstrap "to" rendering before the subtree has
    // produced meaningful bounds.
    const BOOTSTRAP_RECT_SIZE: f32 = 100.0;

    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Transition Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/transition.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Transition Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
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
                BindGroupLayoutEntry {
                    binding: 3,
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

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Transition Pipeline Layout"),
            bind_group_layouts: &[&MVPMatrix::bind_group_layout(device), &bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Transition Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
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

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Transition Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            format: config.format,
            pipeline,
            bind_group_layout,
            sampler,
        }
    }

    fn create_texture_view(
        &self,
        device: &Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> TextureView {
        device
            .create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&TextureViewDescriptor::default())
    }

    fn create_bind_group(
        &self,
        device: &Device,
        uniform_buffer: &Buffer,
        from_view: &TextureView,
        to_view: &TextureView,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("Transition Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(from_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(to_view),
                },
            ],
        })
    }

    fn ensure_textures(
        &self,
        container: &mut TransitionContainer,
        device: &Device,
        width: u32,
        height: u32,
    ) -> bool {
        if width == 0 || height == 0 {
            return false;
        }

        let needs_recreation = container.from_view.is_none()
            || container.to_view.is_none()
            || container.display_view.is_none()
            || container.texture_width != width
            || container.texture_height != height;

        if !needs_recreation {
            return false;
        }

        // The container keeps exactly three shared textures for one transition:
        // retained "from", hidden "to", and composited "display".
        // All three share the same backing allocation in the current design.
        container.from_view =
            Some(self.create_texture_view(device, width, height, "Transition From"));
        container.to_view = Some(self.create_texture_view(device, width, height, "Transition To"));
        container.display_view =
            Some(self.create_texture_view(device, width, height, "Transition Display"));
        container.texture_width = width;
        container.texture_height = height;
        container.present_bind_group = None;
        container.composite_bind_group = None;

        true
    }

    fn release_textures(container: &mut TransitionContainer) {
        // Idle containers fall back to transparent-wrapper mode and release all
        // offscreen resources after a grace period.
        container.from_view = None;
        container.to_view = None;
        container.display_view = None;
        container.composite_uniform_buffer = None;
        container.present_uniform_buffer = None;
        container.present_bind_group = None;
        container.composite_bind_group = None;
        container.texture_width = 0;
        container.texture_height = 0;
        container.render_width = 0;
        container.render_height = 0;
        container.transition_rect = Rect::default();
    }

    fn ensure_uniform_buffers(&self, container: &mut TransitionContainer, device: &Device) {
        if container.composite_uniform_buffer.is_none() {
            container.composite_uniform_buffer =
                Some(device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("Transition Composite Params Buffer"),
                    contents: bytemuck::bytes_of(&TransitionParams::zeroed()),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }));
        }

        if container.present_uniform_buffer.is_none() {
            container.present_uniform_buffer =
                Some(device.create_buffer_init(&util::BufferInitDescriptor {
                    label: Some("Transition Present Params Buffer"),
                    contents: bytemuck::bytes_of(&TransitionParams::zeroed()),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                }));
        }
    }

    fn update_bind_groups(&self, container: &mut TransitionContainer, device: &Device) {
        container.composite_bind_group = match (
            matches!(
                container.phase,
                TransitionPhase::Running | TransitionPhase::Finishing
            ),
            container.composite_uniform_buffer.as_ref(),
            container.from_view.as_ref(),
            container.to_view.as_ref(),
        ) {
            (true, Some(uniform_buffer), Some(from_view), Some(to_view)) => {
                Some(self.create_bind_group(device, uniform_buffer, from_view, to_view))
            }
            _ => None,
        };

        // Prepared uses `from_view` as the visible image. Running uses the
        // composited `display_view`. Stable mode presents nothing here and lets
        // children draw directly into the parent target.
        let present_source = if matches!(
            container.phase,
            TransitionPhase::Running | TransitionPhase::Finishing
        ) {
            container.display_view.as_ref()
        } else if matches!(
            container.phase,
            TransitionPhase::AwaitingPrepare | TransitionPhase::Prepared
        ) {
            container.from_view.as_ref()
        } else {
            None
        };

        container.present_bind_group =
            match (container.present_uniform_buffer.as_ref(), present_source) {
                (Some(uniform_buffer), Some(source_view)) => {
                    Some(self.create_bind_group(device, uniform_buffer, source_view, source_view))
                }
                _ => None,
            };
    }

    fn texture_uv_scale(container: &TransitionContainer) -> [f32; 2] {
        if container.texture_width == 0 || container.texture_height == 0 {
            return [1.0, 1.0];
        }

        // The active logical rect can be smaller than the backing allocation,
        // so the shader must ignore the unused tail of the texture.
        [
            container.render_width as f32 / container.texture_width as f32,
            container.render_height as f32 / container.texture_height as f32,
        ]
    }

    fn composite_params(container: &TransitionContainer) -> TransitionParams {
        TransitionParams {
            position: [container.transition_rect.x(), container.transition_rect.y()],
            size: [
                container.transition_rect.width(),
                container.transition_rect.height(),
            ],
            uv_scale: Self::texture_uv_scale(container),
            progress: container.progress,
            effect_id: container.effect.into(),
        }
    }

    fn present_params(container: &TransitionContainer) -> TransitionParams {
        TransitionParams {
            position: [container.transition_rect.x(), container.transition_rect.y()],
            size: [
                container.transition_rect.width(),
                container.transition_rect.height(),
            ],
            uv_scale: Self::texture_uv_scale(container),
            // Present is a plain rect-aware blit from the retained texture back
            // into the parent target.
            progress: 0.0,
            effect_id: -1,
        }
    }

    fn slot_children_bounds(slot: &TransitionSlot) -> Option<Bound> {
        // Transition rects are derived from actual child content, not from the
        // wrapper slot node itself.
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

    fn transition_rect(
        stage_rect: Rect,
        from_bounds: Option<Bound>,
        to_bounds: Option<Bound>,
    ) -> Option<Rect> {
        // A transition always covers the union of both subtrees, but is
        // currently clamped to the stage rect.
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
        // The seed only exists to let the "to" subtree render one frame and
        // produce real bounds. It is intentionally small and cheap.
        let width = Self::BOOTSTRAP_RECT_SIZE.min(stage_rect.width());
        let height = Self::BOOTSTRAP_RECT_SIZE.min(stage_rect.height());
        Rect::new(stage_rect.x(), stage_rect.y(), width, height)
    }

    fn assign_slot_state(
        &self,
        container: &TransitionContainer,
        slot: &mut TransitionSlot,
        from_target: &Option<TextureView>,
        to_target: &Option<TextureView>,
    ) -> bool {
        // Both slots use the same logical transition rect in the current
        // design, even though they may contain different subtree content.
        slot.render_rect = container.transition_rect;

        match slot.phase {
            TransitionSlotPhase::From => {
                if matches!(container.phase, TransitionPhase::AwaitingPrepare) {
                    // Before prepare, the old subtree still provides the stable
                    // frame shown on screen.
                    slot.render_children = from_target.is_some();
                    slot.render_target = from_target.clone();
                    return slot.render_children;
                }

                if matches!(
                    container.phase,
                    TransitionPhase::Prepared
                        | TransitionPhase::Running
                        | TransitionPhase::Finishing
                ) {
                    match container.retain {
                        RetainMode::Snapshot => {
                            // Snapshot redraws the old subtree only when the
                            // retained "from" texture needs a refresh.
                            let should_render =
                                from_target.is_some() && container.from_needs_redraw;
                            slot.render_children = should_render;
                            slot.render_target = if should_render {
                                from_target.clone()
                            } else {
                                None
                            };
                            return should_render;
                        }
                        RetainMode::Live => {
                            // Live keeps the old subtree rendering into the
                            // retained target for the whole transition.
                            slot.render_children = from_target.is_some();
                            slot.render_target = from_target.clone();
                            return slot.render_children;
                        }
                    }
                }

                // Outside the active transition window, the old slot is inert.
                slot.render_children = false;
                slot.render_target = None;
                false
            }
            TransitionSlotPhase::To => {
                if matches!(
                    container.phase,
                    TransitionPhase::Prepared
                        | TransitionPhase::Running
                        | TransitionPhase::Finishing
                ) {
                    // The new subtree renders into a hidden retained target as
                    // soon as prepare completes, but it is not presented until
                    // the container decides to composite or reveal it.
                    slot.render_children = to_target.is_some();
                    slot.render_target = to_target.clone();
                } else if matches!(container.phase, TransitionPhase::AwaitingPrepare) {
                    slot.render_children = false;
                    slot.render_target = None;
                } else {
                    // Stable mode: the slot becomes a transparent wrapper and
                    // lets children render directly into the parent target.
                    slot.render_children = true;
                    slot.render_target = None;
                }

                false
            }
        }
    }
}

impl TransitionSlotRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Renderer for TransitionContainerRenderer {
    fn name(&self) -> &'static str {
        "transition_container"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        &self.bind_group_layout
    }

    fn update(
        &mut self,
        node: &mut dyn Node,
        device: &Device,
        _queue: &Queue,
        render_queue: &RenderCommandSender,
        payload: &RendererUpdatePayload,
    ) {
        // Update flow, from high level to low level:
        // 1. Resolve the slot pair and advance the container state machine
        //    (`Stable -> AwaitingPrepare -> Prepared -> Running -> Finishing`).
        // 2. Measure the current from/to subtree bounds and derive one shared
        //    logical `transition_rect` for this transition.
        // 3. Ensure retained textures exist for that rect, then push per-slot
        //    runtime state so each slot knows whether to render children into
        //    an offscreen target or stay shadowed.
        // 4. Upload composite/present params and emit only the commands needed
        //    for the current state. Stable mode emits no container commands.
        let container = node
            .as_any_mut()
            .downcast_mut::<TransitionContainer>()
            .unwrap();

        // Transition rects are always resolved in stage logical coordinates.
        let stage_rect = Rect::new(
            0.0,
            0.0,
            payload.stage_logical_size.0,
            payload.stage_logical_size.1,
        );

        let mut has_from_slot = false;
        let mut has_to_slot = false;
        let children = container.base().children().clone();

        for child in &children {
            let child = child.read();
            if let Some(slot) = child.as_any().downcast_ref::<TransitionSlot>() {
                match slot.phase {
                    TransitionSlotPhase::From => has_from_slot = true,
                    TransitionSlotPhase::To => has_to_slot = true,
                }
            }
        }

        // Finish is delayed by one update so the last composited frame can be
        // presented before the container collapses back to stable mode.
        if matches!(container.phase, TransitionPhase::Finishing)
            && !container.pending_prepare
            && container.pending_perform.is_none()
        {
            container.phase = TransitionPhase::Stable;
            container.transition_start_at = None;
            container.send_event(crate::events::TransitionContainerEvent::Finished);
        }

        // Slots arm themselves once both sides exist. Prepare can then freeze
        // that pair into an explicit transition state.
        // `slots_armed` is only about pair discovery. It does not mean prepare
        // or perform has already happened.
        if !has_from_slot
            && matches!(
                container.phase,
                TransitionPhase::Stable | TransitionPhase::AwaitingPrepare
            )
        {
            container.phase = TransitionPhase::Stable;
            container.slots_armed = false;
        } else if has_from_slot
            && has_to_slot
            && matches!(
                container.phase,
                TransitionPhase::Stable | TransitionPhase::AwaitingPrepare
            )
            && !container.slots_armed
            && !container.pending_prepare
        {
            // The pair is now stable enough that an external prepare may choose
            // to retain the current "from" subtree and start building "to".
            container.phase = TransitionPhase::AwaitingPrepare;
        }

        if container.pending_prepare {
            // Prepare freezes the current slot pair into retained mode but does
            // not start time-based progress yet.
            container.pending_prepare = false;
            container.phase = TransitionPhase::Prepared;
            container.slots_armed = true;
            container.progress = 0.0;
            container.transition_start_at = None;
            container.from_needs_redraw = true;
        }

        if let Some(request) = container.pending_perform.take() {
            if !matches!(container.phase, TransitionPhase::Prepared) {
                // Perform without a prior prepare is allowed. In that case the
                // renderer tries to arm the current slot pair immediately.
                container.slots_armed = has_from_slot && has_to_slot;
                container.from_needs_redraw = true;
            }

            container.effect = request.effect;
            container.duration = request.duration.max(0.0001);
            container.progress = 0.0;

            if has_from_slot && has_to_slot {
                if request.duration <= 0.0 {
                    // Zero-duration perform still goes through one final frame
                    // so the result is composited consistently.
                    container.progress = 1.0;
                    container.phase = TransitionPhase::Finishing;
                    container.transition_start_at = None;
                } else {
                    container.phase = TransitionPhase::Running;
                    container.transition_start_at = Some(payload.timestamp);
                }
            } else {
                container.transition_start_at = None;
                container.phase = TransitionPhase::Stable;
                container.progress = 1.0;
            }
        }

        // Once the transition enters `Running`, progress is driven only by wall
        // clock time until the renderer promotes it to `Finishing`.
        if matches!(container.phase, TransitionPhase::Running) {
            let start_at = container.transition_start_at.unwrap_or(payload.timestamp);
            let duration = container.duration.max(0.0001);
            let progress = ((payload.timestamp - start_at) / duration).clamp(0.0, 1.0) as f32;
            container.progress = progress;

            if progress >= 1.0 {
                container.progress = 1.0;
                container.phase = TransitionPhase::Finishing;
            }
        }

        let is_active = container.is_active();
        let mut from_bounds = None;
        let mut to_bounds = None;

        for child in &children {
            let child = child.read();
            if let Some(slot) = child.as_any().downcast_ref::<TransitionSlot>() {
                match slot.phase {
                    TransitionSlotPhase::From => {
                        from_bounds = Self::slot_children_bounds(slot);
                    }
                    TransitionSlotPhase::To => {
                        to_bounds = Self::slot_children_bounds(slot);
                    }
                }
            }
        }

        // One shared transition rect drives all three retained textures in the
        // current implementation.
        if is_active {
            if let Some(rect) = Self::transition_rect(stage_rect, from_bounds, to_bounds) {
                let (_, _, render_width, render_height) = calculate_surface_physical_coordinates(
                    &rect,
                    payload.stage_logical_size,
                    payload.surface_logical_size,
                    payload.scale_factor,
                );

                let alloc_width = container.texture_width.max(render_width);
                let alloc_height = container.texture_height.max(render_height);
                let recreated = self.ensure_textures(container, device, alloc_width, alloc_height);

                // `render_*` describes the current active rect. `texture_*`
                // describes the actual allocation size backing all retained
                // textures, which may already be larger because of a previous
                // frame in the same transition.
                container.transition_rect = rect;
                container.render_width = render_width;
                container.render_height = render_height;

                // Snapshot must refresh the old texture after a resize because
                // the retained image no longer matches the new allocation.
                if recreated && container.retain == RetainMode::Snapshot && has_from_slot {
                    container.from_needs_redraw = true;
                }
            } else if has_to_slot {
                // Bootstrap lets the "to" subtree render at least one frame so
                // it can produce real bounds for the next update.
                let rect = Self::bootstrap_transition_rect(stage_rect);
                let (_, _, render_width, render_height) = calculate_surface_physical_coordinates(
                    &rect,
                    payload.stage_logical_size,
                    payload.surface_logical_size,
                    payload.scale_factor,
                );

                self.ensure_textures(container, device, render_width, render_height);
                container.transition_rect = rect;
                container.render_width = render_width;
                container.render_height = render_height;
            } else {
                container.transition_rect = Rect::default();
                container.render_width = 0;
                container.render_height = 0;
            }

            container.last_active_at = Some(payload.timestamp);
        } else {
            // Stable mode clears the active rect immediately, but keeps the
            // textures around for a short idle window to avoid reallocating on
            // every nearby transition.
            container.transition_rect = Rect::default();
            container.render_width = 0;
            container.render_height = 0;
            container.present_bind_group = None;
            container.composite_bind_group = None;

            if let Some(last_active_at) = container.last_active_at {
                if payload.timestamp - last_active_at >= 5.0 {
                    Self::release_textures(container);
                    container.last_active_at = None;
                }
            }
        }

        // Slots only receive retained targets while the container has a valid
        // physical rect for the current transition.
        let from_target = if container.render_width > 0 && container.render_height > 0 {
            container.from_view.clone()
        } else {
            None
        };
        let to_target = if container.render_width > 0 && container.render_height > 0 {
            container.to_view.clone()
        } else {
            None
        };
        let mut snapshot_from_rendered = false;

        for child in &children {
            let mut child = child.write();
            if let Some(slot) = child.as_any_mut().downcast_mut::<TransitionSlot>() {
                snapshot_from_rendered |=
                    self.assign_slot_state(container, slot, &from_target, &to_target);
            }
        }

        if container.retain == RetainMode::Snapshot
            && snapshot_from_rendered
            && !matches!(container.phase, TransitionPhase::AwaitingPrepare)
        {
            // Once the snapshot is captured, the old subtree can stop drawing
            // until an explicit refresh becomes necessary again.
            container.from_needs_redraw = false;
        }

        if is_active && container.render_width > 0 && container.render_height > 0 {
            self.ensure_uniform_buffers(container, device);

            if let Some(uniform_buffer) = container.composite_uniform_buffer.as_ref() {
                let params = Self::composite_params(container);

                render_queue
                    .send(RenderCommand::WriteBuffer {
                        buffer: uniform_buffer.clone(),
                        offset: 0,
                        data: bytemuck::bytes_of(&params).to_vec(),
                        use_staging_belt: true,
                    })
                    .unwrap();
            }

            if let Some(uniform_buffer) = container.present_uniform_buffer.as_ref() {
                let params = Self::present_params(container);

                render_queue
                    .send(RenderCommand::WriteBuffer {
                        buffer: uniform_buffer.clone(),
                        offset: 0,
                        data: bytemuck::bytes_of(&params).to_vec(),
                        use_staging_belt: true,
                    })
                    .unwrap();
            }

            self.update_bind_groups(container, device);
        } else {
            // Stable mode should not emit any container-level draw command.
            container.present_bind_group = None;
            container.composite_bind_group = None;
        }
    }

    fn should_collect_commands(&self, node: &dyn Node, _stage_bound: &Bound) -> bool {
        let container = node.as_any().downcast_ref::<TransitionContainer>().unwrap();
        // The container only contributes render commands while it owns a valid
        // retained source to present back into the parent target.
        container.present_bind_group.is_some()
            && container.render_width > 0
            && container.render_height > 0
    }

    fn collect_commands(&self, _node: &dyn Node, _render_queue: &RenderCommandSender) {}

    fn collect_post_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let container = node.as_any().downcast_ref::<TransitionContainer>().unwrap();

        if matches!(
            container.phase,
            TransitionPhase::Running | TransitionPhase::Finishing
        ) {
            if let (Some(target_view), Some(bind_group)) = (
                container.display_view.clone(),
                container.composite_bind_group.as_ref(),
            ) {
                // Composite "from" and "to" into the intermediate display
                // texture first. The parent target only receives the final
                // result of this pass.
                render_queue
                    .send(RenderCommand::BeginRenderTargetPass {
                        target_view,
                        rect: container.transition_rect,
                    })
                    .unwrap();

                render_queue
                    .send(RenderCommand::Draw {
                        pipeline: self.pipeline.clone(),
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
            }
        }

        if let Some(bind_group) = container.present_bind_group.as_ref() {
            // Present either the prepared/running retained output or the final
            // composited frame back into the parent target.
            render_queue
                .send(RenderCommand::Draw {
                    pipeline: self.pipeline.clone(),
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
    }
}

impl Renderer for TransitionSlotRenderer {
    fn name(&self) -> &'static str {
        "transition_slot"
    }

    fn render_pipeline(&self) -> &RenderPipeline {
        unreachable!("TransitionSlotRenderer does not draw directly")
    }

    fn bind_group_layout(&self) -> &BindGroupLayout {
        unreachable!("TransitionSlotRenderer does not allocate bind groups")
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
        let slot = node.as_any().downcast_ref::<TransitionSlot>().unwrap();
        // A slot only opens an offscreen pass when the container has assigned a
        // retained target for this frame.
        slot.render_children
            && slot.render_target.is_some()
            && slot.render_rect.width() > 0.0
            && slot.render_rect.height() > 0.0
    }

    fn collect_commands(&self, node: &dyn Node, render_queue: &RenderCommandSender) {
        let slot = node.as_any().downcast_ref::<TransitionSlot>().unwrap();

        if let Some(target_view) = slot.render_target.as_ref() {
            // Slot renderers only wrap their subtree with an offscreen pass.
            // The actual transition effect is owned by the container renderer.
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
