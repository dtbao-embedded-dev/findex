//! Business rules: plain functions over plain data, unit-tested without a window.

pub mod db;
pub mod greeting;
// Reached from commands once the index service lands; until then only tests use them.
#[allow(dead_code)]
pub mod index;
#[allow(dead_code)]
pub mod search;
