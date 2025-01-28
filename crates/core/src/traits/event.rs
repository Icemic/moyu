use serde::Serialize;

use crate::events::{NodeEvent, NodeEventKind};
#[cfg(any(all(native, feature = "js_runtime"), web))]
use crate::utils::dispatch_event::dispatch_event;

#[cfg(any(all(native, feature = "js_runtime"), web))]
use super::Node;

pub trait Event: Serialize + Send + 'static {
    fn name(&self) -> &'static str;
}

pub trait BindEvent: Node {
    type Event: Event;
    fn send_event(&self, key: &str, event: Self::Event) {
        #[cfg(any(all(native, feature = "js_runtime"), web))]
        dispatch_event(NodeEvent {
            kind: NodeEventKind::Custom,
            target_id: *self.base().id(),
            custom_kind: Some(key.to_string()),
            custom_body: Some(event),
        });
    }
}
