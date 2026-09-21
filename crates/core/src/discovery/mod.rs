pub mod registry;
pub mod roots;
pub mod scan;

pub use registry::{AgentAdapter, Registry, RegistryError, RootTemplate};
pub use roots::ScanRoot;
pub use scan::{
    Containment, EntryKind, EntryMetadata, ExclusionReason, ExclusionRule, LibraryCatalog,
    LocationState, PayloadSource, ResolvedRoot, ScanCandidate, ScanExclusion, ScanInput,
    ScanLimits, ScanLocation, ScanOutcome, ScanPolicy, ScanProgress, ScanWarning,
    scan_global_roots, scan_roots,
};
