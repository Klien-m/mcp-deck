//! Agent-specific paths and dialects. The engine only calls these conversion/patch functions.
mod codec;
mod document;
mod registry;

pub use codec::{decode, encode};
pub use document::{parse, patch};
pub use registry::{default_targets, get, registry, Adapter, Dialect};
