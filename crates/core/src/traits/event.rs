use serde::Serialize;

use crate::events::CustomEvent;
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
        dispatch_event(CustomEvent {
            target_id: *self.base().id(),
            name: key.to_string(),
            body: Some(event),
        });
    }
}
