//! Typed contracts shared by the embedded engine, client, and native views.

mod discovery;
mod error;
mod library;
mod lifecycle;
mod sync;

pub use discovery::*;
pub use error::*;
pub use library::*;
pub use lifecycle::*;
pub use sync::*;

use serde::{Deserialize, Serialize};

macro_rules! identifier {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl $name {
            /// Creates an opaque identifier.
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4().to_string())
            }
            /// Returns the serialized identifier.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
identifier!(
    SkillId,
    "A portable skill identity, independent of its slug."
);
identifier!(FolderId, "A flat folder identity.");
identifier!(ScanId, "A discovery run identity.");
identifier!(CandidateId, "An opaque discovery candidate identity.");
identifier!(PlanId, "An immutable import plan and replay key.");
identifier!(RootId, "A registered project root identity.");
identifier!(GrantId, "A short-lived, single-use native directory grant.");
