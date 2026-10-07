//! The file and folder name index: what is stored per volume and how it is built.

use crate::error::AppError;

pub mod mft;
pub mod service;
pub mod snapshot;
pub mod store;
pub mod walk;
pub mod watch;

/// The error a build or watcher stops with when a newer start replaced its work.
pub fn superseded() -> AppError {
    AppError::Invalid("indexing was restarted".into())
}
