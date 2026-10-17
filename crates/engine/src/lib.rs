//! Embedded operations. A single worker owns SQLite and serializes durable mutations.

mod database;
mod discovery;
mod folders;
mod git;
mod grants;
mod imports;
mod index;
mod library;
mod lifecycle;
mod logging;
mod payload;
mod persistence;
mod schema;
mod worker;
mod workspaces;

pub use lifecycle::EngineConfig;
pub use worker::Engine;
