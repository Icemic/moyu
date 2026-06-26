use std::collections::HashMap;

use sixu::Fingerprint;
use sixu::fingerprint_child_semantics;
use sixu::fingerprint_paragraph_signature;
use sixu::format::{Block, ChildContent, Story};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerNode {
    pub marker_id: String,
    pub paragraph: String,
    pub semantic_fingerprint: Fingerprint,
    pub path: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphInfo {
    pub signature: Fingerprint,
    pub root_block_fingerprint: Fingerprint,
}

/// Lightweight static summary used by hot-reload planning.
///
/// The current planner does not need an explicit CFG/SCC object graph yet.
/// A deterministic marker stream plus per-paragraph fingerprints is enough to
/// detect the last proven-stable executable prefix conservatively.
#[derive(Debug, Clone)]
pub struct StoryGraphCache {
    pub markers: Vec<MarkerNode>,
    pub paragraphs: HashMap<String, ParagraphInfo>,
}

fn collect_marker_nodes(
    paragraph: &str,
    block: &Block,
    path: &mut Vec<usize>,
    markers: &mut Vec<MarkerNode>,
) {
    // Markers are collected in deterministic DFS order. Hot-reload diffing uses
    // this order as the linearized execution frontier: once the stream diverges,
    // the remaining suffix is treated as affected.
    for (child_index, child) in block.children().iter().enumerate() {
        path.push(child_index);

        if let Some(marker) = child.marker.as_ref() {
            if !child.content.is_comment() {
                markers.push(MarkerNode {
                    marker_id: marker.id.clone(),
                    paragraph: paragraph.to_string(),
                    semantic_fingerprint: fingerprint_child_semantics(child),
                    path: path.clone(),
                });
            }
        }

        if let ChildContent::Block(inner_block) = &child.content {
            collect_marker_nodes(paragraph, inner_block, path, markers);
        }

        path.pop();
    }
}

pub fn build_story_graph(story: &Story) -> StoryGraphCache {
    let mut markers = Vec::new();
    let mut paragraphs = HashMap::new();

    for paragraph in &story.paragraphs {
        // The paragraph fingerprint catches structural edits that do not change
        // marker count or ordering, while the marker list captures localized
        // control-flow changes inside the paragraph body.
        paragraphs.insert(
            paragraph.name.clone(),
            ParagraphInfo {
                signature: fingerprint_paragraph_signature(paragraph),
                root_block_fingerprint: paragraph.block.fingerprint(),
            },
        );

        let mut path = Vec::new();
        collect_marker_nodes(&paragraph.name, &paragraph.block, &mut path, &mut markers);
    }

    StoryGraphCache {
        markers,
        paragraphs,
    }
}
