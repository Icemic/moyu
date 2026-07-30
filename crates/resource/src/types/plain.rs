use std::sync::Arc;

use arc_swap::{ArcSwap, ArcSwapOption};

#[derive(Debug)]
pub struct Plain {
    status: ArcSwap<PlainStatus>,
    content: ArcSwapOption<String>,
}

impl Default for Plain {
    fn default() -> Self {
        Self::new()
    }
}

impl Plain {
    pub fn new() -> Self {
        Self {
            status: ArcSwap::default(),
            content: ArcSwapOption::default(),
        }
    }

    pub fn status(&self) -> PlainStatus {
        *self.status.load().as_ref()
    }

    pub fn set_status(&self, status: PlainStatus) {
        self.status.store(Arc::new(status));
    }

    pub fn content(&self) -> Option<Arc<String>> {
        self.content.load_full()
    }

    pub fn set_content(&self, content: String) {
        self.content.store(Some(Arc::new(content)));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlainStatus {
    #[default]
    Reading,
    Ready,
    Error,
}
