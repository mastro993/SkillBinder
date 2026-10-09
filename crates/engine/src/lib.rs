//! Embedded operations. A single worker owns SQLite and serializes durable mutations.

mod discovery;
mod git;
mod imports;
mod index;
mod library;
mod lifecycle;
mod logging;
mod payload;
mod persistence;
mod roots;
mod worker;

pub use lifecycle::EngineConfig;
pub use worker::Engine;
