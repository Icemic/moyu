//! Frame snapshot copied out of the plugin.
//!
//! The plugin keeps its frame data valid only until the next call of any entry
//! point, so everything the renderer needs is copied here on the script thread,
//! immediately after the frame is produced. Geometry is copied straight into its
//! upload layout: positions and UVs interleaved, colors as two `u32` per vertex.
//!
//! # Coordinates
//!
//! Positions arrive in world units; the host shifts them onto the node by the
//! reference bounds origin and draws them as they are. UVs arrive already
//! normalized with the origin at the top-left and v growing downwards — the
//! runtime divides by the page height, so the host must not flip them.

use moyu_spine_abi::frame::FrameDesc;

/// Bytes per vertex: position (2), UV (2), light color (1) and dark color (1).
pub const VERTEX_STRIDE: usize = (2 + 2 + 1 + 1) * 4;

/// One batch of a frame, ready for GPU upload.
///
/// Batches are kept in the order the plugin produced them and must be submitted
/// in that order: the runtime groups by texture page and blend mode, and
/// reordering would change how overlapping attachments compose.
pub struct BatchSnapshot {
    /// Index into the model's page list.
    pub page_index: i32,
    /// One of `moyu_spine_abi::frame::BLEND_*`.
    pub blend_mode: i32,
    /// Triangles to draw.
    pub index_count: usize,
    /// Vertices the indices refer to.
    pub vertex_count: usize,
    /// Interleaved vertex data in [`VERTEX_STRIDE`] layout.
    pub vertex_data: Vec<u8>,
    /// Index data, `u16` each, padded to a multiple of four bytes.
    pub index_data: Vec<u8>,
}

/// One frame of a model.
#[derive(Default)]
pub struct FrameSnapshot {
    pub batches: Vec<BatchSnapshot>,
    pub vertex_count_total: usize,
    pub index_count_total: usize,
}

impl FrameSnapshot {
    /// Copies a frame description into owned data.
    ///
    /// # Safety
    ///
    /// `desc` must come from `..._frame` for a live model on the current plugin
    /// thread, and must still be valid, i.e. no other plugin call may have
    /// happened since.
    pub unsafe fn copy(desc: &FrameDesc) -> Self {
        if desc.batches.is_null() || desc.batch_count == 0 {
            return Self::default();
        }

        let source = unsafe { std::slice::from_raw_parts(desc.batches, desc.batch_count) };
        let batches: Vec<BatchSnapshot> =
            source.iter().map(|batch| unsafe { copy_batch(batch) }).collect();

        Self {
            vertex_count_total: desc.vertex_count_total,
            index_count_total: desc.index_count_total,
            batches,
        }
    }
}

/// Copies one batch into its upload layout.
///
/// # Safety
///
/// The pointers of `batch` must be valid for their element counts.
unsafe fn copy_batch(batch: &moyu_spine_abi::frame::BatchDesc) -> BatchSnapshot {
    let vertex_count = batch.vertex_count;
    let index_count = batch.index_count;

    let mut vertex_data = Vec::with_capacity(vertex_count * VERTEX_STRIDE);
    if vertex_count > 0 {
        let positions =
            unsafe { std::slice::from_raw_parts(batch.positions, vertex_count) };
        let uvs = unsafe { std::slice::from_raw_parts(batch.uvs, vertex_count) };
        let colors = unsafe { std::slice::from_raw_parts(batch.colors, vertex_count) };
        let dark_colors = unsafe { std::slice::from_raw_parts(batch.dark_colors, vertex_count) };

        for index in 0..vertex_count {
            let position = positions[index];
            let uv = uvs[index];
            vertex_data.extend_from_slice(&position[0].to_ne_bytes());
            vertex_data.extend_from_slice(&position[1].to_ne_bytes());
            vertex_data.extend_from_slice(&uv[0].to_ne_bytes());
            vertex_data.extend_from_slice(&uv[1].to_ne_bytes());
            vertex_data.extend_from_slice(&colors[index].to_ne_bytes());
            vertex_data.extend_from_slice(&dark_colors[index].to_ne_bytes());
        }
    }

    let mut index_data = Vec::with_capacity(index_count * 2);
    if index_count > 0 {
        let indices = unsafe { std::slice::from_raw_parts(batch.indices, index_count) };
        for index in indices {
            index_data.extend_from_slice(&index.to_ne_bytes());
        }
        // Index buffers are copied in four-byte units, so an odd triangle count
        // has to be padded rather than left to the driver.
        if index_data.len() % 4 != 0 {
            index_data.extend_from_slice(&[0, 0]);
        }
    }

    BatchSnapshot {
        page_index: batch.texture_page_index,
        blend_mode: batch.blend_mode,
        index_count,
        vertex_count,
        vertex_data,
        index_data,
    }
}
