//! Embedded operations. A single worker owns SQLite and serializes durable mutations.

mod database;
mod discovery;
mod folders;
mod git;
mod grants;
mod imports;
mod library;
mod library_store;
mod lifecycle;
mod logging;
mod payload;
mod persistence;
mod reconcile;
mod records;
mod schema;
mod shutdown;
mod sources;
mod worker;
mod workspaces;

pub use lifecycle::EngineConfig;
pub use worker::Engine;
