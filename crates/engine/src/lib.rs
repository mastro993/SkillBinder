//! Embedded operations. A single worker owns SQLite and serializes durable mutations.

mod activation;
mod database;
mod folders;
mod git;
mod grants;
mod imports;
mod library;
mod library_store;
mod lifecycle;
mod logging;
mod onboarding;
mod payload;
mod persistence;
mod reconcile;
mod records;
mod scan;
mod schema;
mod shutdown;
mod sources;
mod worker;
mod workspaces;

pub use lifecycle::{EngineConfig, GitProfile, GitSearch};
pub use worker::Engine;
