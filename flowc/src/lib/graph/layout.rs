//! Topological graph layout for flow diagrams.
//!
//! Thin wrapper around `flowcore::graph::layout` that converts flowcore model
//! types into the generic inputs expected by the shared layout algorithm.

#![allow(clippy::cast_precision_loss, clippy::implicit_hasher)]

use std::collections::HashMap;

use flowcore::graph::layout as shared;
pub use flowcore::graph::layout::PositionedNode;

use flowcore::model::connection::Connection;
use flowcore::model::process_reference::ProcessReference;

/// Get the alias for a process reference.
pub(crate) fn process_alias(p: &ProcessReference) -> String {
    if p.alias.is_empty() {
        shared::derive_short_name(&p.source)
    } else {
        p.alias.clone()
    }
}

/// Compute topological layout for a set of processes and connections.
///
/// Converts flowcore model types into the generic format expected by
/// `flowcore::graph::layout::compute_layout` and returns positioned nodes.
#[must_use]
pub fn compute_layout(
    process_refs: &[ProcessReference],
    connections: &[Connection],
    node_info: &HashMap<String, (Vec<String>, Vec<String>)>,
) -> HashMap<String, PositionedNode> {
    let node_specs: Vec<(String, Vec<String>, Vec<String>)> = process_refs
        .iter()
        .map(|p| {
            let alias = process_alias(p);
            let (inputs, outputs) = node_info.get(&alias).cloned().unwrap_or_default();
            (alias, inputs, outputs)
        })
        .collect();

    let conn_pairs: Vec<(String, String)> = connections
        .iter()
        .flat_map(|conn| {
            let from = conn.from().to_string();
            conn.to()
                .iter()
                .map(move |to| (from.clone(), to.to_string()))
        })
        .collect();

    shared::compute_layout(&node_specs, &conn_pairs)
}
