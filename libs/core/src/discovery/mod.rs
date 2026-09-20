pub mod registry;
pub mod scan;

pub use registry::{AgentAdapter, Registry, RegistryError, RootTemplate};
pub use scan::{
    EntryKind, EntryMetadata, LibraryCatalog, PayloadSource, ResolvedRoot, ScanCandidate,
    ScanLimits, ScanLocation, ScanOutcome, ScanRoot, ScanWarning, scan_global_roots,
};
