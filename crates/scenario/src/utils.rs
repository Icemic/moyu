use std::collections::{HashMap, HashSet};

use anyhow::Result;
use sixu::BlockFingerprint;
use sixu::format::{Block, Literal};
use sixu::runtime::RuntimeContext;

use crate::types::{BacklogState, RuntimeSnapshot, SavedExecutionState};

pub(crate) fn convert_to_literal(value: serde_json::Value) -> Literal {
    match value {
        serde_json::Value::Null => Literal::Null,
        serde_json::Value::Bool(v) => Literal::Boolean(v),
        serde_json::Value::Number(number) => {
            if let Some(v) = number.as_i64() {
                Literal::Integer(v)
            } else if let Some(v) = number.as_f64() {
                Literal::Float(v)
            } else {
                Literal::String(number.to_string())
            }
        }
        serde_json::Value::String(v) => Literal::String(v),
        serde_json::Value::Array(values) => {
            let converted_values = values.into_iter().map(convert_to_literal).collect();
            Literal::Array(converted_values)
        }
        serde_json::Value::Object(map) => {
            let converted_map = map
                .into_iter()
                .map(|(k, v)| (k, convert_to_literal(v)))
                .collect();
            Literal::Object(converted_map)
        }
    }
}

pub(crate) fn timestamp_millis() -> Result<u64> {
    let millis = moyu_pal::time::SystemTime::now()
        .duration_since(moyu_pal::time::SystemTime::UNIX_EPOCH)?
        .as_millis();

    Ok(u64::try_from(millis).unwrap_or(u64::MAX))
}
pub(crate) fn next_record_id(backlog: &mut BacklogState) -> Result<String> {
    let id = format!(
        "record-{}-{}",
        timestamp_millis()?,
        backlog.next_record_serial
    );
    backlog.next_record_serial += 1;
    Ok(id)
}
pub(crate) fn snapshot_blocks(snapshot: &RuntimeSnapshot) -> HashSet<BlockFingerprint> {
    snapshot
        .stack
        .iter()
        .map(|state| state.block_fingerprint)
        .collect()
}

pub(crate) fn prune_backlog_blocks(backlog: &mut BacklogState) {
    let referenced = backlog
        .records
        .iter()
        .flat_map(|record| snapshot_blocks(&record.snapshot))
        .collect::<HashSet<_>>();

    backlog
        .blocks
        .retain(|fingerprint, _| referenced.contains(fingerprint));
}

pub(crate) fn create_runtime_snapshot_from_context(
    context: &RuntimeContext,
    blocks: &mut HashMap<BlockFingerprint, Block>,
) -> Result<RuntimeSnapshot> {
    let mut stack = Vec::with_capacity(context.stack().len());

    for state in context.stack().iter() {
        let fingerprint = state.block.fingerprint();

        if let std::collections::hash_map::Entry::Vacant(entry) = blocks.entry(fingerprint) {
            entry.insert(state.block.clone());
        }

        stack.push(SavedExecutionState {
            story: state.story.clone(),
            paragraph: state.paragraph.clone(),
            block_fingerprint: fingerprint,
            index: state.index,
            is_loop_body: state.is_loop_body,
            locals: state
                .locals
                .clone()
                .map(Literal::Object)
                .map(serde_json::Value::from),
        });
    }
    let variables = serde_json::Value::from(context.archive_variables().clone());

    Ok(RuntimeSnapshot { stack, variables })
}
