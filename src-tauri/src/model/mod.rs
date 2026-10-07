//! Business rules: plain functions over plain data, unit-tested without a window.

pub mod about;
pub mod db;
pub mod drives;
pub mod greeting;
// The snapshot (frn, parent) and live-update (node_of, compact_if_wasteful) helpers
// of the index store have no caller until those land.
#[allow(dead_code)]
pub mod index;
pub mod search;
pub mod win32;
