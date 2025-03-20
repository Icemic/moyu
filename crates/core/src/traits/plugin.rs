use super::Command;

/// A Plugin means an extra function module that different from a [Node](super::Node).
pub trait Plugin: Send + Sync {
    /// plugin type identifier
    fn plugin_name(&self) -> &'static str;

    /// Called on the main thread inside the event loop. This is where you can do anything polling related.
    /// When `vsync` is true, it means it is called in [winit::event::WindowEvent::RedrawRequested].
    /// When `vsync` is false, it means it is called in [winit::event::Event::AboutToWait].
    #[allow(unused_variables)]
    fn update(&mut self, vsync: bool) {
        // defaults to do nothing
    }

    /// return Some(self) manually if you've implemented Command for the plugin
    fn as_command(&mut self) -> Option<&mut dyn Command> {
        None
    }
}
