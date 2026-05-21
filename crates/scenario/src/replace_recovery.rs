use std::collections::HashMap;

use anyhow::{Result, anyhow};
use sixu::format::{Block, Story};
use sixu::Fingerprint;

use crate::execution_path::DynamicExecutionPath;
use crate::story_graph::{MarkerNode, StoryGraphCache};
use crate::types::{
    ExecutableRestartBoundary, RuntimeCheckpoint, StoryReplaceExecutionPlan, StoryReplaceMode,
    StoryReplaceOutcome,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryDiffResult {
    pub common_prefix_len: usize,
    pub first_affected_marker_id: Option<String>,
    pub changed_control_flow: bool,
    pub has_executable_changes: bool,
}

#[derive(Debug)]
pub struct ReplaceRecoveryPlan {
    pub outcome: StoryReplaceOutcome,
    pub checkpoints: HashMap<String, RuntimeCheckpoint>,
    pub checkpoint_blocks: HashMap<Fingerprint, Block>,
}

#[derive(Debug, Default)]
struct PatchedStoryCheckpoints {
    checkpoints: HashMap<String, RuntimeCheckpoint>,
    blocks: HashMap<Fingerprint, Block>,
    boundary_checkpoint_key: Option<String>,
}

fn marker_ids(graph: &StoryGraphCache) -> impl Iterator<Item = &str> {
    graph.markers.iter().map(|marker| marker.marker_id.as_str())
}

fn find_marker<'a>(graph: &'a StoryGraphCache, marker_id: &str) -> Option<&'a MarkerNode> {
    graph.markers.iter().find(|marker| marker.marker_id == marker_id)
}

fn marker_prefix_matches(left: &MarkerNode, right: &MarkerNode) -> bool {
    left.marker_id == right.marker_id
        && left.paragraph == right.paragraph
        && left.semantic_fingerprint == right.semantic_fingerprint
}

fn paragraph_signatures_equal(old_graph: &StoryGraphCache, new_graph: &StoryGraphCache) -> bool {
    if old_graph.paragraphs.len() != new_graph.paragraphs.len() {
        return false;
    }

    old_graph.paragraphs.iter().all(|(paragraph, old_info)| {
        new_graph
            .paragraphs
            .get(paragraph)
            .map(|new_info| old_info.signature == new_info.signature)
            .unwrap_or(false)
    })
}

pub fn diff_story_graphs(old_graph: &StoryGraphCache, new_graph: &StoryGraphCache) -> StoryDiffResult {
    // The longest stable marker prefix is our conservative approximation of the
    // unaffected CFG prefix. Any divergence in marker semantics/order, plus any
    // paragraph signature mismatch, marks the remainder as needing recovery.
    let common_prefix_len = old_graph
        .markers
        .iter()
        .zip(new_graph.markers.iter())
        .take_while(|(left, right)| marker_prefix_matches(left, right))
        .count();

    let first_affected_marker_id = new_graph
        .markers
        .get(common_prefix_len)
        .map(|marker| marker.marker_id.clone());
    let changed_control_flow = marker_ids(old_graph).ne(marker_ids(new_graph));
    let has_executable_changes = common_prefix_len != old_graph.markers.len()
        || common_prefix_len != new_graph.markers.len()
        || !paragraph_signatures_equal(old_graph, new_graph);

    StoryDiffResult {
        common_prefix_len,
        first_affected_marker_id,
        changed_control_flow,
        has_executable_changes,
    }
}

fn can_patch_checkpoint(
    story_name: &str,
    old_graph: &StoryGraphCache,
    new_graph: &StoryGraphCache,
    old_node: &MarkerNode,
    new_node: &MarkerNode,
    checkpoint: &RuntimeCheckpoint,
) -> bool {
    // Prefix checkpoint patching is intentionally limited to the simplest
    // execution shape: a single root frame positioned immediately after a
    // top-level marker. Nested blocks, loop bodies, or multi-frame stacks would
    // require occurrence-aware CFG/SCC mapping, so those cases fall back to a
    // broader restart boundary instead of trying to be clever.
    if old_node.paragraph != new_node.paragraph || old_node.path != new_node.path {
        return false;
    }

    if old_node.path.len() != 1 {
        return false;
    }

    let Some(cursor) = checkpoint.cursor.as_ref() else {
        return false;
    };

    if cursor.story != story_name || cursor.paragraph != old_node.paragraph {
        return false;
    }

    if cursor.marker_id.as_deref() != Some(old_node.marker_id.as_str()) {
        return false;
    }

    if checkpoint.snapshot.stack.len() != 1 {
        return false;
    }

    let frame = &checkpoint.snapshot.stack[0];
    if frame.story != story_name || frame.paragraph != old_node.paragraph || frame.is_loop_body {
        return false;
    }

    if frame.index != old_node.path[0] + 1 {
        return false;
    }

    let Some(old_paragraph) = old_graph.paragraphs.get(&old_node.paragraph) else {
        return false;
    };
    let Some(new_paragraph) = new_graph.paragraphs.get(&new_node.paragraph) else {
        return false;
    };

    old_paragraph.signature == new_paragraph.signature
}

fn patch_prefix_story_checkpoints(
    story_name: &str,
    old_story: &Story,
    old_graph: &StoryGraphCache,
    new_story: &Story,
    new_graph: &StoryGraphCache,
    diff: &StoryDiffResult,
    checkpoints: &HashMap<String, RuntimeCheckpoint>,
) -> PatchedStoryCheckpoints {
    let mut patched = PatchedStoryCheckpoints::default();

    for index in 0..diff.common_prefix_len {
        let old_node = &old_graph.markers[index];
        let new_node = &new_graph.markers[index];
        let Some(checkpoint) = checkpoints.get(&old_node.marker_id) else {
            continue;
        };

        if !can_patch_checkpoint(story_name, old_graph, new_graph, old_node, new_node, checkpoint) {
            continue;
        }

        let Some(paragraph) = new_story
            .paragraphs
            .iter()
            .find(|paragraph| paragraph.name == new_node.paragraph)
        else {
            continue;
        };

        let mut checkpoint = checkpoint.clone();
        checkpoint.snapshot.stack[0].block_fingerprint = paragraph.block.fingerprint();

        patched
            .blocks
            .entry(paragraph.block.fingerprint())
            .or_insert_with(|| paragraph.block.clone());
        patched.boundary_checkpoint_key = Some(old_node.marker_id.clone());
        patched
            .checkpoints
            .insert(old_node.marker_id.clone(), checkpoint);
    }

    // Keep the compiler honest about both graphs being intentional inputs.
    let _ = old_story;

    patched
}

fn rebuild_checkpoint_blocks(
    checkpoints: &HashMap<String, RuntimeCheckpoint>,
    old_blocks: &HashMap<Fingerprint, Block>,
    patched_blocks: &HashMap<Fingerprint, Block>,
) -> Result<HashMap<Fingerprint, Block>> {
    let mut blocks = HashMap::new();

    for checkpoint in checkpoints.values() {
        for frame in &checkpoint.snapshot.stack {
            if blocks.contains_key(&frame.block_fingerprint) {
                continue;
            }

            if let Some(block) = patched_blocks.get(&frame.block_fingerprint) {
                blocks.insert(frame.block_fingerprint, block.clone());
                continue;
            }

            let Some(block) = old_blocks.get(&frame.block_fingerprint) else {
                return Err(anyhow!(
                    "Checkpoint block fingerprint {} not found while rebuilding checkpoint pool",
                    frame.block_fingerprint.to_hex()
                ));
            };

            blocks.insert(frame.block_fingerprint, block.clone());
        }
    }

    Ok(blocks)
}

fn retain_unaffected_checkpoints(
    story_name: &str,
    checkpoints: &HashMap<String, RuntimeCheckpoint>,
) -> HashMap<String, RuntimeCheckpoint> {
    checkpoints
        .iter()
        .filter(|(_, checkpoint)| {
            checkpoint
                .cursor
                .as_ref()
                .map(|cursor| cursor.story != story_name)
                .unwrap_or(true)
        })
        .map(|(key, checkpoint)| (key.clone(), checkpoint.clone()))
        .collect()
}

fn paragraph_is_restartable(story: &Story, paragraph_name: &str) -> bool {
    story
        .paragraphs
        .iter()
        .find(|paragraph| paragraph.name == paragraph_name)
        .map(|paragraph| {
            paragraph
                .parameters
                .iter()
                .all(|parameter| parameter.default_value.is_some())
        })
        .unwrap_or(false)
}

fn select_fallback_boundary(
    story_name: &str,
    new_story: &Story,
    new_graph: &StoryGraphCache,
    diff: &StoryDiffResult,
    execution_path: &DynamicExecutionPath,
) -> ExecutableRestartBoundary {
    if execution_path.is_simple_root_path() {
        let current_frame = &execution_path.frames[0];

        if let Some(target_marker_id) = diff.first_affected_marker_id.as_deref() {
            if let Some(target_marker) = find_marker(new_graph, target_marker_id) {
                if target_marker.paragraph == current_frame.paragraph
                    && paragraph_is_restartable(new_story, &current_frame.paragraph)
                {
                    if current_frame.paragraph == "entry" {
                        return ExecutableRestartBoundary::RestartStory {
                            story: story_name.to_string(),
                            entry: "entry".to_string(),
                        };
                    }

                    return ExecutableRestartBoundary::RestartParagraph {
                        story: story_name.to_string(),
                        paragraph: current_frame.paragraph.clone(),
                    };
                }
            }
        }
    }

    ExecutableRestartBoundary::RestartStory {
        story: story_name.to_string(),
        entry: "entry".to_string(),
    }
}

pub fn plan_story_replace(
    story_name: &str,
    current_story_affected: bool,
    old_story: &Story,
    new_story: &Story,
    old_graph: &StoryGraphCache,
    new_graph: &StoryGraphCache,
    execution_path: &DynamicExecutionPath,
    checkpoints: &HashMap<String, RuntimeCheckpoint>,
    checkpoint_blocks: &HashMap<Fingerprint, Block>,
) -> Result<ReplaceRecoveryPlan> {
    let diff = diff_story_graphs(old_graph, new_graph);

    if !diff.has_executable_changes {
        return Ok(ReplaceRecoveryPlan {
            outcome: StoryReplaceOutcome {
                story: story_name.to_string(),
                current_story_affected,
                plan: StoryReplaceExecutionPlan {
                    mode: StoryReplaceMode::Noop,
                    boundary: None,
                    target_marker_id: None,
                    changed_control_flow: false,
                },
            },
            checkpoints: checkpoints.clone(),
            checkpoint_blocks: checkpoint_blocks.clone(),
        });
    }

    let mut surviving_checkpoints = retain_unaffected_checkpoints(story_name, checkpoints);

    if !current_story_affected {
        return Ok(ReplaceRecoveryPlan {
            outcome: StoryReplaceOutcome {
                story: story_name.to_string(),
                current_story_affected: false,
                plan: StoryReplaceExecutionPlan {
                    mode: StoryReplaceMode::InvalidateOnly,
                    boundary: None,
                    target_marker_id: None,
                    changed_control_flow: diff.changed_control_flow,
                },
            },
            checkpoint_blocks: rebuild_checkpoint_blocks(
                &surviving_checkpoints,
                checkpoint_blocks,
                &HashMap::new(),
            )?,
            checkpoints: surviving_checkpoints,
        });
    }

    let patched = if execution_path.is_simple_root_path() {
        patch_prefix_story_checkpoints(
            story_name,
            old_story,
            old_graph,
            new_story,
            new_graph,
            &diff,
            checkpoints,
        )
    } else {
        PatchedStoryCheckpoints::default()
    };

    surviving_checkpoints.extend(patched.checkpoints.clone());

    let boundary = patched.boundary_checkpoint_key.as_ref().map(|checkpoint_key| {
        ExecutableRestartBoundary::Checkpoint {
            checkpoint_key: checkpoint_key.clone(),
        }
    });

    let plan = if boundary.is_some() {
        // Reusing the last proven-stable prefix checkpoint is the cheapest safe
        // boundary: restore there first, then seek to the first affected marker.
        StoryReplaceExecutionPlan {
            mode: StoryReplaceMode::Reposition,
            boundary,
            target_marker_id: diff.first_affected_marker_id.clone(),
            changed_control_flow: diff.changed_control_flow,
        }
    } else {
        // Without a reusable checkpoint we only keep a paragraph-local restart
        // when execution is a single root frame and the affected suffix stays in
        // that paragraph. Everything else falls back to story head to avoid
        // crossing unresolved loop/call occurrences.
        StoryReplaceExecutionPlan {
            mode: StoryReplaceMode::Reposition,
            boundary: Some(select_fallback_boundary(
                story_name,
                new_story,
                new_graph,
                &diff,
                execution_path,
            )),
            target_marker_id: diff.first_affected_marker_id,
            changed_control_flow: diff.changed_control_flow,
        }
    };

    let checkpoint_blocks = rebuild_checkpoint_blocks(
        &surviving_checkpoints,
        checkpoint_blocks,
        &patched.blocks,
    )?;

    Ok(ReplaceRecoveryPlan {
        outcome: StoryReplaceOutcome {
            story: story_name.to_string(),
            current_story_affected: true,
            plan,
        },
        checkpoints: surviving_checkpoints,
        checkpoint_blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_path::{DynamicExecutionPath, DynamicFrameMeta};
    use crate::types::{ExecutionCursor, RuntimeSnapshot, SavedExecutionState};
    use crate::story_graph::build_story_graph;
    use sixu::parser;

    fn parse_story(name: &str, source: &str) -> Story {
        parser::parse(name, source).unwrap().1
    }

    #[test]
    fn diff_is_noop_for_comment_only_changes() {
        let old_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"hello\"\n}\n",
        );
        let new_story = parse_story(
            "test",
            "::entry {\n    // comment\n    //#marker id=L1\n    \"hello\"\n}\n",
        );

        let diff = diff_story_graphs(&build_story_graph(&old_story), &build_story_graph(&new_story));
        assert!(!diff.has_executable_changes);
        assert!(!diff.changed_control_flow);
    }

    #[test]
    fn linear_prefix_checkpoint_is_patched_and_selected_as_boundary() {
        let old_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"hello\"\n    //#marker id=L2\n    \"world\"\n}\n",
        );
        let new_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"hello\"\n    //#marker id=L2\n    \"world updated\"\n}\n",
        );

        let old_graph = build_story_graph(&old_story);
        let new_graph = build_story_graph(&new_story);
        let boundary_marker = old_graph
            .markers
            .iter()
            .find(|marker| marker.marker_id == "L1")
            .unwrap();
        let old_root = old_story.paragraphs[0].block.fingerprint();
        let new_root = new_story.paragraphs[0].block.fingerprint();

        let checkpoints = HashMap::from([(
            "L1".to_string(),
            RuntimeCheckpoint {
                cursor: Some(ExecutionCursor {
                    story: "test".to_string(),
                    paragraph: "entry".to_string(),
                    marker_id: Some("L1".to_string()),
                }),
                snapshot: RuntimeSnapshot {
                    stack: vec![SavedExecutionState {
                        story: "test".to_string(),
                        paragraph: "entry".to_string(),
                        block_fingerprint: old_root,
                        index: boundary_marker.path[0] + 1,
                        is_loop_body: false,
                        locals: None,
                    }],
                    variables: serde_json::json!({}),
                },
            },
        )]);
        let checkpoint_blocks = HashMap::from([(old_root, old_story.paragraphs[0].block.clone())]);
        let execution_path = DynamicExecutionPath {
            current_marker_id: Some("L2".to_string()),
            frames: vec![DynamicFrameMeta {
                story: "test".to_string(),
                paragraph: "entry".to_string(),
                block_fingerprint: old_root,
                index: boundary_marker.path[0] + 1,
                is_loop_body: false,
            }],
        };

        let plan = plan_story_replace(
            "test",
            true,
            &old_story,
            &new_story,
            &old_graph,
            &new_graph,
            &execution_path,
            &checkpoints,
            &checkpoint_blocks,
        )
        .unwrap();

        assert_eq!(plan.outcome.plan.mode, StoryReplaceMode::Reposition);
        assert!(matches!(
            plan.outcome.plan.boundary,
            Some(ExecutableRestartBoundary::Checkpoint { ref checkpoint_key }) if checkpoint_key == "L1"
        ));
        assert_eq!(plan.outcome.plan.target_marker_id.as_deref(), Some("L2"));
        assert_ne!(old_root, new_root);

        let checkpoint = plan.checkpoints.get("L1").unwrap();
        assert_eq!(checkpoint.snapshot.stack[0].block_fingerprint, new_root);
        assert!(plan.checkpoint_blocks.contains_key(&new_root));
    }

    #[test]
    fn complex_execution_path_falls_back_to_restart_story() {
        let old_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"hello\"\n    //#marker id=L2\n    \"world\"\n}\n",
        );
        let new_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"hello\"\n    //#marker id=L2\n    \"world updated\"\n}\n",
        );

        let old_graph = build_story_graph(&old_story);
        let new_graph = build_story_graph(&new_story);
        let boundary_marker = old_graph
            .markers
            .iter()
            .find(|marker| marker.marker_id == "L1")
            .unwrap();
        let old_root = old_story.paragraphs[0].block.fingerprint();

        let checkpoints = HashMap::from([(
            "L1".to_string(),
            RuntimeCheckpoint {
                cursor: Some(ExecutionCursor {
                    story: "test".to_string(),
                    paragraph: "entry".to_string(),
                    marker_id: Some("L1".to_string()),
                }),
                snapshot: RuntimeSnapshot {
                    stack: vec![SavedExecutionState {
                        story: "test".to_string(),
                        paragraph: "entry".to_string(),
                        block_fingerprint: old_root,
                        index: boundary_marker.path[0] + 1,
                        is_loop_body: false,
                        locals: None,
                    }],
                    variables: serde_json::json!({}),
                },
            },
        )]);
        let checkpoint_blocks = HashMap::from([(old_root, old_story.paragraphs[0].block.clone())]);
        let execution_path = DynamicExecutionPath {
            current_marker_id: Some("L2".to_string()),
            frames: vec![
                DynamicFrameMeta {
                    story: "test".to_string(),
                    paragraph: "entry".to_string(),
                    block_fingerprint: old_root,
                    index: boundary_marker.path[0] + 1,
                    is_loop_body: false,
                },
                DynamicFrameMeta {
                    story: "test".to_string(),
                    paragraph: "entry".to_string(),
                    block_fingerprint: old_root,
                    index: boundary_marker.path[0] + 1,
                    is_loop_body: false,
                },
            ],
        };

        let plan = plan_story_replace(
            "test",
            true,
            &old_story,
            &new_story,
            &old_graph,
            &new_graph,
            &execution_path,
            &checkpoints,
            &checkpoint_blocks,
        )
        .unwrap();

        assert_eq!(plan.outcome.plan.mode, StoryReplaceMode::Reposition);
        assert!(matches!(
            plan.outcome.plan.boundary,
            Some(ExecutableRestartBoundary::RestartStory {
                ref story,
                ref entry,
            }) if story == "test" && entry == "entry"
        ));
        assert!(!plan.checkpoints.contains_key("L1"));
    }

    #[test]
    fn falls_back_to_restart_paragraph_when_current_paragraph_is_restartable() {
        let old_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"intro\"\n}\n\n::branch {\n    //#marker id=B1\n    \"choice\"\n    //#marker id=B2\n    \"tail\"\n}\n",
        );
        let new_story = parse_story(
            "test",
            "::entry {\n    //#marker id=L1\n    \"intro\"\n}\n\n::branch {\n    //#marker id=B1\n    \"choice\"\n    //#marker id=B2\n    \"tail updated\"\n}\n",
        );

        let old_graph = build_story_graph(&old_story);
        let new_graph = build_story_graph(&new_story);
        let branch_marker = old_graph
            .markers
            .iter()
            .find(|marker| marker.marker_id == "B1")
            .unwrap();
        let branch_root = old_story.paragraphs[1].block.fingerprint();

        let plan = plan_story_replace(
            "test",
            true,
            &old_story,
            &new_story,
            &old_graph,
            &new_graph,
            &DynamicExecutionPath {
                current_marker_id: Some("B2".to_string()),
                frames: vec![DynamicFrameMeta {
                    story: "test".to_string(),
                    paragraph: "branch".to_string(),
                    block_fingerprint: branch_root,
                    index: branch_marker.path[0] + 1,
                    is_loop_body: false,
                }],
            },
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(plan.outcome.plan.mode, StoryReplaceMode::Reposition);
        assert!(matches!(
            plan.outcome.plan.boundary,
            Some(ExecutableRestartBoundary::RestartParagraph {
                ref story,
                ref paragraph,
            }) if story == "test" && paragraph == "branch"
        ));
        assert_eq!(plan.outcome.plan.target_marker_id.as_deref(), Some("B2"));
    }
}
