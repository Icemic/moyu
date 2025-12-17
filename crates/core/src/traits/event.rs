use serde::Serialize;
use ts_rs::TS;

use crate::events::CustomEvent;

use super::Node;
use super::Plugin;

pub trait Event: Serialize + TS + Send + 'static {
    fn name(&self) -> &'static str;
}

pub trait NodeEventSource: Node {
    type Event: Event;
    fn send_event(&self, event: Self::Event) {
        #[cfg(any(all(native, feature = "js_runtime"), web))]
        crate::utils::dispatch_event::dispatch_event_async(CustomEvent {
            target_id: *self.base().id(),
            name: event.name().to_string(),
            body: Some(event),
        });
    }
}

pub trait PluginEventSource: Plugin {
    type Event: Event;
    fn send_event(&self, event: Self::Event) {
        // Must use `dispatch_event_async` here to avoid deadlock when sending events
        // Since we are already holding the plugin lock when calling this method, if user calls plugin's command
        // which also tries to lock the same plugin, it will cause a deadlock.
        #[cfg(any(all(native, feature = "js_runtime"), web))]
        crate::utils::dispatch_event::dispatch_event_async(CustomEvent {
            target_id: 0,
            name: event.name().to_string(),
            body: Some(event),
        });
    }
}
