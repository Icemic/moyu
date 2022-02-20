use std::sync::{Arc, Mutex};

use crate::state::State;

pub fn render<'a>(state: &Arc<Mutex<State<'a>>>) -> Result<(), wgpu::SurfaceError> {
    let state = state.lock().unwrap();
    let pending_renderable = state.pending_renderable.clone();
    let mut pending_renderable = pending_renderable.lock().unwrap();
    let surface = state.surface.clone();
    let surface = surface.lock().unwrap();
    let device = state.device.clone();
    let device = device.lock().unwrap();
    let queue = state.queue.clone();
    let queue = queue.lock().unwrap();
    let render_pipeline = state.render_pipeline.clone();
    let render_pipeline = render_pipeline.lock().unwrap();

    drop(state);

    let output = surface.get_current_texture()?;
    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&render_pipeline);

        for (bind_group, vertex_buffer, index_buffer, _, num_indices) in pending_renderable.iter() {
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..*num_indices, 0, 0..1);
        }
    }

    // clear queue
    pending_renderable.clear();

    // submit will accept anything that implements IntoIter
    queue.submit(std::iter::once(encoder.finish()));
    output.present();

    Ok(())
}
