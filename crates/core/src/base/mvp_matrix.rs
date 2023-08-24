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
    pub fn from_logical_size((width, height): (f32, f32)) -> Self {
        let mut matrix = Mat4::IDENTITY;

        matrix.x_axis.x = 2.0 / width;
        matrix.y_axis.y = -2.0 / height;
        matrix.z_axis.z = 1.0;
        matrix.w_axis.x = -1.0;
        matrix.w_axis.y = 1.0;
        matrix.w_axis.z = 0.0;
        matrix.w_axis.w = 1.0;

        Self(matrix)
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
