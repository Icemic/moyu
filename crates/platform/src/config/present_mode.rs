use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingPresentMode {
    /// Chooses Mailbox -> FifoRelaxed -> Fifo based on availability.
    ///
    /// This will ensure that you get a smooth experience as well as low latency if possible.
    ///
    /// Because of the fallback behavior, it is supported everywhere.
    #[default]
    Recommended,
    /// Same as [`wgpu::PresentMode::AutoVsync`]
    AutoVsync,
    /// Same as [`wgpu::PresentMode::AutoNoVsync`]
    AutoNoVsync,
}
