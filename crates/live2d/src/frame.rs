//! Frame snapshot copied out of the plugin.
//!
//! The plugin owns the frame data and keeps it valid only until the next call, so
//! everything the renderer needs is copied here on the script thread. Geometry is
//! copied straight into its upload layout, so the renderer hands these bytes to the
//! GPU without touching them again.
//!
//! The matrices stay in the contract's row-major layout all the way to the GPU:
//! the shaders apply them as `vector * matrix`, which is the same product as
//! transposing them and writing `matrix * vector`, and leaves the uniform without
//! a transpose step. See `live2d.wgsl`.

use moyu_live2d_abi::Matrix4;
use moyu_live2d_abi::frame::FrameDesc;

/// One drawable of a frame, ready for GPU upload.
pub struct DrawableSnapshot {
    /// `moyu_live2d_abi::frame::FLAG_*` bitfield.
    pub flags: u32,
    /// Effective blend mode, one of `moyu_live2d_abi::frame::BLEND_*`.
    pub blend_mode: i32,
    /// Mode the model declared, for diagnostics only.
    pub source_blend_mode: i32,
    /// Index into the texture list of the model.
    pub texture_index: i32,
    /// Drawable opacity with the model opacity folded in.
    pub opacity: f32,
    pub multiply: [f32; 3],
    pub screen: [f32; 3],
    /// Triangle indices to draw.
    pub index_count: usize,
    /// Vertex data, interleaved `[x, y, u, v]` f32 per vertex: canvas pixel
    /// position and normalized UV, already in the layout the GPU takes.
    pub vertex_data: Vec<u8>,
    /// Index data, `u16` each, padded to a multiple of four bytes.
    pub index_data: Vec<u8>,
    /// Mask buffer this drawable samples; -1 when unmasked.
    pub mask_buffer_index: i32,
    /// Mask channel inside the buffer, 0-3.
    pub mask_channel: u32,
    /// Transform from canvas pixel space to mask sampling UV.
    pub clip_matrix: Matrix4,
}

/// One mask draw command of a frame.
pub struct MaskCommandSnapshot {
    pub buffer_index: u32,
    pub channel: u32,
    /// Index into [`FrameSnapshot::drawables`].
    pub drawable_index: usize,
    /// Transform from canvas pixel space to mask clip space.
    pub matrix: Matrix4,
    /// Rectangle in the same clip space; fragments outside do not write.
    pub clip_rect: [f32; 4],
}

/// One frame of a model.
///
/// Canvas geometry is not part of the frame: it is model metadata, delivered
/// once through the `ready` event.
pub struct FrameSnapshot {
    pub drawables: Vec<DrawableSnapshot>,
    /// Mask buffer size; empty when the model needs no masks.
    pub mask_buffers: Vec<(u32, u32)>,
    pub mask_commands: Vec<MaskCommandSnapshot>,
}

impl FrameSnapshot {
    /// Copies a frame description into owned data.
    ///
    /// # Safety
    ///
    /// `desc` must come from `..._get_frame` for a live model on the current
    /// plugin thread; its pointers must still be valid.
    pub unsafe fn copy(desc: &FrameDesc) -> Self {
        let drawables = if desc.drawables.is_null() {
            Vec::new()
        } else {
            let source = unsafe { std::slice::from_raw_parts(desc.drawables, desc.drawable_count) };
            source.iter().map(|drawable| unsafe { copy_drawable(drawable) }).collect()
        };

        let mask_buffers = if desc.mask_buffers.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(desc.mask_buffers, desc.mask_buffer_count) }
                .iter()
                .map(|buffer| (buffer.width, buffer.height))
                .collect()
        };

        let mask_commands = if desc.mask_commands.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(desc.mask_commands, desc.mask_command_count) }
                .iter()
                .map(|command| MaskCommandSnapshot {
                    buffer_index: command.buffer_index,
                    channel: command.channel,
                    drawable_index: command.drawable_index,
                    matrix: command.matrix,
                    clip_rect: command.clip_rect,
                })
                .collect()
        };

        Self {
            drawables,
            mask_buffers,
            mask_commands,
        }
    }
}

/// Interleaves one drawable's vertices and UVs into a single upload buffer.
///
/// # Safety
///
/// The pointers of `drawable` must be valid for their element counts.
unsafe fn copy_drawable(drawable: &moyu_live2d_abi::frame::DrawableDesc) -> DrawableSnapshot {
    // Positions and UVs arrive in separate arrays and are interleaved here, in the
    // bytes the vertex buffer takes.
    let mut vertex_data = Vec::with_capacity(drawable.vertex_count * 16);
    if !drawable.vertices.is_null() && drawable.vertex_count > 0 {
        let positions =
            unsafe { std::slice::from_raw_parts(drawable.vertices, drawable.vertex_count) };
        let uvs = if drawable.uv.is_null() {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(drawable.uv, drawable.vertex_count) }
        };
        for (index, position) in positions.iter().enumerate() {
            let uv = uvs.get(index).copied().unwrap_or([0.0, 0.0]);
            vertex_data
                .extend_from_slice(bytemuck::bytes_of(&[position[0], position[1], uv[0], uv[1]]));
        }
    }

    // The staging belt writes in four-byte units, so an odd index count carries
    // two bytes of padding.
    let mut index_data = Vec::with_capacity(drawable.index_count * 2 + 2);
    if !drawable.indices.is_null() && drawable.index_count > 0 {
        let source = unsafe { std::slice::from_raw_parts(drawable.indices, drawable.index_count) };
        index_data.extend_from_slice(bytemuck::cast_slice(source));
    }
    while !index_data.len().is_multiple_of(4) {
        index_data.push(0);
    }

    DrawableSnapshot {
        flags: drawable.flags,
        blend_mode: drawable.blend_mode,
        source_blend_mode: drawable.source_blend_mode,
        texture_index: drawable.texture_index,
        opacity: drawable.opacity,
        multiply: drawable.multiply,
        screen: drawable.screen,
        index_count: drawable.index_count,
        vertex_data,
        index_data,
        mask_buffer_index: drawable.mask_buffer_index,
        mask_channel: drawable.mask_channel,
        clip_matrix: drawable.clip_matrix,
    }
}
