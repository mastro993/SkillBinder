//! Embedded operations. A single worker owns SQLite and serializes durable mutations.

mod discovery;
mod git;
mod grants;
mod imports;
mod index;
mod library;
mod lifecycle;
mod logging;
mod payload;
mod persistence;
mod targets;
mod worker;

pub use lifecycle::EngineConfig;
pub use worker::Engine;
