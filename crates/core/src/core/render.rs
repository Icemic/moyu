use std::sync::Arc;

use doufu_pal::config::get_engine_config;
use doufu_pal::time::Instant;
use winit::window::Window;

use crate::utils::walk::walk_nodes_top_bottom;

use super::{Core, RendererUpdatePayload};

impl Core {
    #[inline(always)]
    pub fn render(&self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        // fps
        if doufu_pal::config::get_engine_config().show_fps {
            if self.fps_meter.tick() {
                let fps = self.fps_meter.get_fps();
                self.event_proxy
                    .send_event(crate::user_event::UserEvent::SetTitle(format!(
                        "fps: {:.1}",
                        fps
                    )))
                    .unwrap();
            }
        }

        let surface = self.surface.clone();
        let device = self.device.clone();
        let queue = self.queue.clone();
        let root_node = self.root_node.clone();

        let mut staging_belt = self.staging_belt.lock();

        let surface_size = self.surface_size.read();
        let stage_size = self.stage_size.read();

        let output = surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Command Encoder"),
            })
        };

        let mut belt_encoder = {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Belt Command Encoder"),
            })
        };

        {
            let root_node = root_node.read();

            let timestamp = self.instant.elapsed().as_secs_f64();
            let instant_last = self.instant_last.swap(Arc::new(Instant::now()));

            let upload_payload = RendererUpdatePayload {
                timestamp,
                delta: instant_last.elapsed().as_micros() as u32,
                surface_size: *surface_size,
                stage_size: *stage_size,
                resource_manager: self.resource_manager.clone(),
            };

            let color = &get_engine_config().background_color;
            let color = wgpu::Color {
                r: color.r,
                g: color.g,
                b: color.b,
                a: color.a,
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            render_pass.set_bind_group(0, &self.mvp_bind_group, &[]);

            walk_nodes_top_bottom(&*root_node, &mut |child, parent| {
                let mut _child = child.write();
                _child.base_mut().update(parent.base(), &stage_size, false);

                let renderer_type = _child.renderer_type();

                if let Some(current_renderer) = self.renderers.lock().get_mut(renderer_type) {
                    current_renderer.update(
                        &mut *_child,
                        &device,
                        &queue,
                        &mut belt_encoder,
                        &mut staging_belt,
                        &upload_payload,
                    );

                    current_renderer.render(&device, &queue, &mut render_pass, &*_child);
                }

                false
            });
        }

        // call after render callback if registered
        if let Some(after_render_callback) = self.after_render_handler.lock().as_ref() {
            after_render_callback(
                &device,
                &queue,
                &mut encoder,
                &output,
                &view,
                &mut staging_belt,
            );
        }

        staging_belt.finish();

        // TODO: in winit, it is an empty function now, keep an eye on it.
        window.pre_present_notify();

        queue.submit(
            std::iter::once(belt_encoder.finish()).chain(std::iter::once(encoder.finish())),
        );
        output.present();

        staging_belt.recall();

        Ok(())
    }
}
