use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingBackend {
    #[default]
    Auto,
    Vulkan,
    Metal,
    DX12,
    WebGPU,
    GLES,
}
