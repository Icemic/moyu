use std::any::Any;

use super::Command;

/// Base trait that provides any casting functionality for plugins
pub trait PluginBaseTrait: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A Plugin means an extra function module that different from a [Node](super::Node).
pub trait Plugin: PluginBaseTrait + Send + Sync {
    /// plugin type identifier
    fn plugin_name(&self) -> &'static str;

    /// Called on the main thread inside the event loop. This is where you can do anything polling related.
    /// When `vsync` is true, it runs as part of a renderable frame (`WindowSystemEvent::Frame`).
    /// When `vsync` is false, it runs on an idle event loop turn (`WindowSystemEvent::Idle`).
    #[allow(unused_variables)]
    fn update(&mut self, vsync: bool) {
        // defaults to do nothing
    }

    /// return Some(self) manually if you've implemented Command for the plugin
    fn as_command(&mut self) -> Option<&mut dyn Command> {
        None
    }
}
