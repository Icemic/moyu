use anyhow::Result;

use crate::utils::convert::JSValue;

/// Command is an optional trait for nodes or plugins who need receive commands.\
/// A command is a message that is sent to a node, and the node can choose to handle it or not.\
/// A typical command payload consists of a "subcommand" followed by any parameter information, which
/// can be serialized and deserialized. In json format as an example, a command could look like:
/// ```json
/// {
///     "subcommand": "move",
///     "x": 100,
///     "y": 100
/// }
/// ```
/// The `received` method is called when a command is sent to the node.\
/// Feel free to define your command payload as a struct or enum, if only it can be serialized and deserialized.
pub trait Command {
    fn execute(&mut self, payload: &mut JSValue) -> Result<Option<JSValue>>;
}
