pub mod outcome;
pub mod registry;
pub mod roots;
pub mod scan;
pub mod spec;

pub use crate::source::{EntryKind, EntryMetadata, PayloadSource, SourceError};
pub use outcome::{
    DuplicateStatus, LocationState, ScanCandidate, ScanLocation, ScanOutcome, ScanProgress,
    ScanWarning,
};
pub use registry::{AgentAdapter, Registry, RegistryError, RootTemplate};
pub use roots::{ScanRoot, project_scan_inputs};
pub use scan::{LibraryCatalog, scan_global_roots, scan_roots};
pub use spec::{
    Containment, ExclusionReason, ExclusionRule, ResolvedRoot, ScanExclusion, ScanInput,
    ScanLimits, ScanPolicy,
};
