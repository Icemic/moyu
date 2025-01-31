use serde::Serialize;

use crate::events::CustomEvent;

use super::Node;
use super::Plugin;

pub trait Event: Serialize + Send + 'static {
    fn name(&self) -> &'static str;
}

pub trait NodeEventSource: Node {
    type Event: Event;
    fn send_event(&self, key: &str, event: Self::Event) {
        #[cfg(any(all(native, feature = "js_runtime"), web))]
        crate::utils::dispatch_event::dispatch_event(CustomEvent {
            target_id: *self.base().id(),
            name: key.to_string(),
            body: Some(event),
        });
    }
}

pub trait PluginEventSource: Plugin {
    type Event: Event;
    fn send_event(&self, key: &str, event: Self::Event) {
        #[cfg(any(all(native, feature = "js_runtime"), web))]
        crate::utils::dispatch_event::dispatch_event(CustomEvent {
            target_id: 0,
            name: key.to_string(),
            body: Some(event),
        });
    }
}
