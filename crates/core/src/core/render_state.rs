use wgpu::{TextureFormat, TextureView};

#[derive(Debug, Default)]
pub struct RenderState {
    scissor_stack: Vec<[u32; 4]>,
    offscreen_stack: Vec<(
        wgpu::TextureView,
        Option<wgpu::TextureView>,
        (u32, u32),
        [f32; 4],
    )>,
    current_offscreen_offset: (u32, u32),
    current_viewport: [f32; 4],
    current_view: Option<TextureView>,
    current_resolve_view: Option<TextureView>,
    current_format: Option<TextureFormat>,
}

impl RenderState {
    pub fn clear_frame_resources(&mut self) {
        self.scissor_stack.clear();
        self.offscreen_stack.clear();
        self.current_offscreen_offset = (0, 0);
        self.current_viewport = [0.0, 0.0, 0.0, 0.0];
        self.current_view = None;
        self.current_resolve_view = None;
        self.current_format = None;
    }

    pub fn get_current_view(&self) -> Option<&TextureView> {
        self.current_view.as_ref()
    }

    pub fn get_current_resolve_view(&self) -> Option<&TextureView> {
        self.current_resolve_view.as_ref()
    }

    pub fn get_current_format(&self) -> Option<TextureFormat> {
        self.current_format
    }

    pub fn get_current_viewport(&self) -> [f32; 4] {
        self.current_viewport
    }

    pub fn get_current_scissor_rect(&self) -> Option<&[u32; 4]> {
        self.scissor_stack.last()
    }

    pub fn reset(
        &mut self,
        scissor_rect: [u32; 4],
        viewport_width: f32,
        viewport_height: f32,
        view: TextureView,
        resolve_view: Option<TextureView>,
    ) {
        self.clear_frame_resources();

        self.scissor_stack.push(scissor_rect);
        self.current_viewport = [0.0, 0.0, viewport_width, viewport_height];
        self.current_format = Some(view.texture().format());
        self.current_view = Some(view);
        self.current_resolve_view = resolve_view;
    }

    pub fn push_scissor_rect(&mut self, x: i32, y: i32, w: i32, h: i32) -> (u32, u32, u32, u32) {
        // 转换为相对于当前视图的坐标
        let x = (x as i32 - self.current_offscreen_offset.0 as i32) as i32;
        let y = (y as i32 - self.current_offscreen_offset.1 as i32) as i32;

        let current = self.scissor_stack.last().unwrap();
        let new_x = x.max(current[0] as i32);
        let new_y = y.max(current[1] as i32);
        let new_right = (x + w as i32).min((current[0] + current[2]) as i32);
        let new_bottom = (y + h as i32).min((current[1] + current[3]) as i32);

        let new_w = (new_right - new_x).max(0) as u32;
        let new_h = (new_bottom - new_y).max(0) as u32;
        let new_x = new_x.max(0) as u32;
        let new_y = new_y.max(0) as u32;

        if new_w > 0 && new_h > 0 {
            self.scissor_stack.push([new_x, new_y, new_w, new_h]);
            (new_x, new_y, new_w, new_h)
        } else {
            self.scissor_stack.push([new_x, new_y, 0, 0]);
            (0, 0, 1, 1)
        }
    }

    pub fn pop_scissor_rect(&mut self) {
        self.scissor_stack.pop();
    }

    pub fn push_offscreen_state(
        &mut self,
        scissor_rect: [u32; 4],
        offset: (u32, u32),
        viewport: [f32; 4],
        view: TextureView,
        resolve_view: Option<TextureView>,
    ) {
        self.offscreen_stack.push((
            self.current_view.as_ref().cloned().unwrap().clone(),
            self.current_resolve_view.clone(),
            self.current_offscreen_offset,
            self.current_viewport,
        ));

        // 将离屏纹理的尺寸压入 scissor_stack
        self.scissor_stack.push(scissor_rect);

        self.current_offscreen_offset = offset;
        self.current_viewport = viewport;
        self.current_view = Some(view);
        self.current_resolve_view = resolve_view;
    }

    pub fn pop_offscreen_state(&mut self) {
        if let Some((view, resolve_view, offset, viewport)) = self.offscreen_stack.pop() {
            self.current_view = Some(view);
            self.current_resolve_view = resolve_view;
            self.current_offscreen_offset = offset;
            self.current_viewport = viewport;
        } else {
            log::error!("EndOffscreenPass: stack underflow");
            return;
        }

        // 弹出离屏纹理的 scissor_stack
        self.scissor_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_empty_scissor_restores_parent_rect() {
        let mut state = RenderState::default();
        state.scissor_stack.push([0, 0, 1920, 1080]);

        assert_eq!(
            state.push_scissor_rect(100, 80, 300, 200),
            (100, 80, 300, 200)
        );
        assert_eq!(state.push_scissor_rect(500, 400, 100, 100), (0, 0, 1, 1));
        assert_eq!(state.get_current_scissor_rect(), Some(&[500, 400, 0, 0]));

        state.pop_scissor_rect();
        assert_eq!(state.get_current_scissor_rect(), Some(&[100, 80, 300, 200]));
    }
}
