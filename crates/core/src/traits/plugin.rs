use super::Command;

/// A Plugin means an extra function module that different from a [Node](super::Node).
pub trait Plugin: Send + Sync {
    /// plugin type identifier
    fn plugin_name(&self) -> &'static str;

    /// return Some(self) manually if you've implemented Command for the plugin
    fn as_command(&mut self) -> Option<&mut dyn Command> {
        None
    }
}
