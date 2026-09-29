//! Properties handed to nodes, kept in a side table.
//!
//! A node ends up holding typed fields rather than the object it was given, so what JS
//! asked for is only observable while it passes through the ops layer. The bridge takes
//! a copy there and keys it by node id, which keeps the engine free of debug-only state.

use std::collections::HashMap;
use std::sync::Mutex;

use moyu_core::utils::convert::{JSValue, from_js};
use serde_json::Value;

static PROPS: Mutex<Option<HashMap<u32, Value>>> = Mutex::new(None);

/// Take copies of every property object nodes are given from now on.
pub(super) fn start() {
    *PROPS.lock().unwrap() = Some(HashMap::new());

    moyu_ops::node::set_props_hook(record);
}

/// What JS has asked of a node so far, if anything was recorded.
pub(super) fn get(node_id: u32) -> Option<Value> {
    PROPS.lock().unwrap().as_ref()?.get(&node_id).cloned()
}

/// Drop entries whose node no longer exists.
///
/// Nodes are destroyed without telling anyone, so records are pruned whenever a client
/// asks about nodes, which is also when the map would be read.
pub(super) fn prune(exists: impl Fn(u32) -> bool) {
    let mut guard = PROPS.lock().unwrap();

    if let Some(props) = guard.as_mut() {
        props.retain(|node_id, _| exists(*node_id));
    }
}

/// Collect one property object into the side table. Installed as the ops hook.
fn record(node_id: u32, props: &JSValue) {
    let Ok(incoming) = from_js::<Value>(props) else {
        return;
    };

    let mut guard = PROPS.lock().unwrap();
    let Some(store) = guard.as_mut() else {
        return;
    };

    // Properties reach the engine as patches, so what JS has asked for by now is the
    // merge of everything it handed over, not only the most recent object.
    match (store.entry(node_id).or_insert(Value::Null), incoming) {
        (Value::Object(recorded), Value::Object(incoming)) => recorded.extend(incoming),
        (recorded, incoming) => *recorded = incoming,
    }
}
