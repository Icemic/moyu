use glam::Mat4;

/**
 * matrix format:
 *
 * +-------------------------+---------------------------+--------+--------+
 * |         x_axis          |          y_axis           | z_axis | w_axis |
 * +-------------------------+---------------------------+--------+--------+
 * | 1.0 / logical_width * 2 | 0.0                       |    0.0 |   -1.0 |
 * | 0.0                     | -1.0 / logical_height * 2 |    0.0 |    1.0 |
 * | 0.0                     | 0.0                       |    1.0 |    0.0 |
 * | 0.0                     | 0.0                       |    0.0 |    1.0 |
 * +-------------------------+---------------------------+--------+--------+
*/
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct MVPMatrix(Mat4);

impl MVPMatrix {
    pub fn from_logical_size(
        (width, height): (f32, f32),
        (window_width, window_height): (f32, f32),
    ) -> Self {
        let mut matrix = Mat4::IDENTITY;

        matrix.x_axis.x = 2.0 / window_width;
        matrix.y_axis.y = -2.0 / window_height;
        matrix.z_axis.z = 1.0;
        matrix.w_axis.x = -1.0;
        matrix.w_axis.y = 1.0;
        matrix.w_axis.z = 0.0;
        matrix.w_axis.w = 1.0;

        let (scale, translate_x, translate_y) =
            get_scale_and_translate(width, height, window_width, window_height);

        let mut transform = Mat4::IDENTITY;

        // set scale
        transform.x_axis.x = scale;
        transform.y_axis.y = scale;

        // set translate to move the stage to the center of the window
        transform.w_axis.x = translate_x;
        transform.w_axis.y = translate_y;

        Self(matrix.mul_mat4(&transform))
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MVP Matrix Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Self>() as u64),
                },
                count: None,
            }],
        })
    }
}

/// Calculate the scale and translate for the stage to adapt into the surface.
///
/// Surface size an stage size must be in logical pixels.
///
/// Returned translate values are in logical pixels.
///
/// The stage will be scaled to fit the surface and centered.
pub fn get_scale_and_translate(
    stage_width: f32,
    stage_height: f32,
    surface_width: f32,
    surface_height: f32,
) -> (f32, f32, f32) {
    let scale = {
        let scale_x = surface_width / stage_width as f32;
        let scale_y = surface_height / stage_height as f32;
        if scale_x > scale_y {
            scale_y
        } else {
            scale_x
        }
    };

    let translate_x = ((surface_width - stage_width as f32 * scale) / 2.).max(0.);
    let translate_y = ((surface_height - stage_height as f32 * scale) / 2.).max(0.);

    (scale, translate_x, translate_y)
}
