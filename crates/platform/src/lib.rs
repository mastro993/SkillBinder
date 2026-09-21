//! The adapters that touch the machine: platform paths, the single-instance process lock, system
//! Git, the payload filesystem, and the Git-backed library repository.
//!
//! Each module implements a port that `crates/core` declares, so the domain stays testable without
//! a temporary directory or a Git repository.

pub mod git;
pub mod library_repository;
pub mod local_environment;
pub mod paths;
pub mod payload_filesystem;
pub mod process_lock;
