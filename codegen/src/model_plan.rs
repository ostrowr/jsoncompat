//! Portable conversion decisions emitted by code generation.
use serde::{Deserialize, Serialize};

/// Only portable decisions are persisted. Python classes and slot offsets are
/// rebound against the installed interpreter when the module is imported.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedPlan {
    pub version: u32,
    pub base_nodes: usize,
    pub guards: Vec<GuardPatch>,
    pub conversion_validates: Vec<bool>,
    pub json_keys: Vec<(usize, Vec<String>)>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GuardPatch {
    pub owner: usize,
    pub field: Option<usize>,
    pub original: usize,
    pub guard: crate::prepared_schema::Node,
}
