//! SkillBinder's domain: the agent registry, the discovery engine and its policies, payload
//! validation, import planning and apply, and the onboarding state machine.
//!
//! The crate performs no I/O. Every filesystem, Git, database, clock, and identifier effect arrives
//! through a port trait, which `crates/platform` and `crates/db` implement against the real world
//! and the unit tests implement in memory.

pub mod bindings;
pub mod bootstrap;
pub mod discovery;
pub mod import;
pub mod library;
pub mod onboarding;
pub mod source;
