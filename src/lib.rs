#![doc = include_str!("../docs/CRATE_DOCS.md")]

pub mod app;
pub mod core;
pub mod export;
pub mod profile;
pub mod state;
pub mod ui;
// Vendored verbatim from esp-csi-rs, a no_std crate whose fixed-capacity frames are meant to
// live inline; lints on it are allowed here rather than by editing the copy.
#[allow(clippy::large_enum_variant)]
pub mod wire;

/// Tests of the vendored `wire` module, kept outside it so the vendored files stay verbatim.
#[cfg(test)]
mod wire_tests;
