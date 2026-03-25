//! UI rendering modules.
//!
//! These modules follow the dumb-UI rule: they only render widgets,
//! mutate in-memory state, and queue intents. They do not perform side effects.

pub mod config;
pub mod control;
pub mod dashboard;
pub mod stream;
