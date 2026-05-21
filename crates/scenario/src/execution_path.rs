use sixu::runtime::RuntimeContext;
use sixu::Fingerprint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicFrameMeta {
    pub story: String,
    pub paragraph: String,
    pub block_fingerprint: Fingerprint,
    pub index: usize,
    pub is_loop_body: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicExecutionPath {
    pub current_marker_id: Option<String>,
    pub frames: Vec<DynamicFrameMeta>,
}

impl DynamicExecutionPath {
    pub fn is_simple_root_path(&self) -> bool {
        self.frames.len() == 1 && !self.frames[0].is_loop_body
    }
}

pub fn capture_execution_path(
    context: &RuntimeContext,
    current_marker_id: Option<String>,
) -> DynamicExecutionPath {
    let frames = context
        .stack()
        .iter()
        .map(|frame| DynamicFrameMeta {
            story: frame.story.clone(),
            paragraph: frame.paragraph.clone(),
            block_fingerprint: frame.block.fingerprint(),
            index: frame.index,
            is_loop_body: frame.is_loop_body,
        })
        .collect();

    DynamicExecutionPath {
        current_marker_id,
        frames,
    }
}
