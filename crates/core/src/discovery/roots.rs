use std::path::PathBuf;

use super::{
    registry::Registry,
    spec::{Containment, ScanInput, ScanPolicy},
};
use crate::source::PayloadSource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    pub id: String,
    pub canonical_path: PathBuf,
    pub display_path: String,
    pub label: String,
    pub enabled: bool,
    pub created_at: u64,
}

/// A registered project root is searched the way a global root is: only inside the project skill
/// directories the registry knows. A known directory that is absent is not a location. A root that
/// is itself gone keeps one input, so the scan still reports it as a missing location.
pub fn project_scan_inputs(
    root: &ScanRoot,
    registry: &Registry,
    source: &dyn PayloadSource,
) -> Vec<ScanInput> {
    let input_for = |path: PathBuf| ScanInput {
        root_id: Some(root.id.clone()),
        path,
        agent_ids: Vec::new(),
        agent_labels: Vec::new(),
        containment: Containment::Grant {
            canonical: root.canonical_path.clone(),
        },
        policy: ScanPolicy::project(),
    };
    let inputs = registry
        .project_skill_dirs()
        .into_iter()
        .map(|directory| root.canonical_path.join(directory))
        .filter(|path| source.exists(path))
        .map(input_for)
        .collect::<Vec<_>>();
    if inputs.is_empty() && !source.exists(&root.canonical_path) {
        return vec![input_for(root.canonical_path.clone())];
    }
    inputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{EntryKind, EntryMetadata, SourceError};
    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
    };

    struct FakeSource {
        directories: HashSet<PathBuf>,
    }
    impl PayloadSource for FakeSource {
        fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError> {
            let _ = path;
            Ok(Vec::new())
        }
        fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError> {
            if self.directories.contains(path) {
                return Ok(EntryMetadata {
                    kind: EntryKind::Directory,
                    executable: false,
                    size: 0,
                });
            }
            Err(SourceError::Missing)
        }
        fn read_file(&self, path: &Path, cap: usize) -> Result<Vec<u8>, SourceError> {
            let _ = (path, cap);
            Err(SourceError::Missing)
        }
        fn resolve_symlink(&self, path: &Path, max_hops: u8) -> Result<PathBuf, SourceError> {
            let _ = (path, max_hops);
            Err(SourceError::Missing)
        }
    }

    const REGISTRY: &str = r#"{
      "registryVersion": 1,
      "agents": [
        { "id": "claude-code", "displayName": "Claude Code", "projectSkillsDir": ".claude/skills", "globalRoots": ["~/.claude/skills"], "status": "path-tested" },
        { "id": "codex", "displayName": "Codex", "projectSkillsDir": ".agents/skills", "globalRoots": [], "status": "path-tested" },
        { "id": "openclaw", "displayName": "OpenClaw", "projectSkillsDir": "skills", "globalRoots": [], "status": "documented" }
      ]
    }"#;

    fn root(path: &str) -> ScanRoot {
        ScanRoot {
            id: "root-1".into(),
            canonical_path: path.into(),
            display_path: path.into(),
            label: "Demo".into(),
            enabled: true,
            created_at: 1,
        }
    }

    #[test]
    fn expansion_keeps_only_the_registry_directories_that_exist() {
        let registry = Registry::parse(REGISTRY).expect("registry parses");
        let source = FakeSource {
            directories: HashSet::from([PathBuf::from("/work/.claude/skills")]),
        };
        let inputs = project_scan_inputs(&root("/work"), &registry, &source);
        assert_eq!(inputs.len(), 1, "{inputs:?}");
        assert_eq!(inputs[0].path, PathBuf::from("/work/.claude/skills"));
        assert_eq!(inputs[0].root_id.as_deref(), Some("root-1"));
        assert_eq!(
            inputs[0].containment,
            Containment::Grant {
                canonical: PathBuf::from("/work")
            }
        );
        assert!(inputs[0].agent_ids.is_empty());
    }

    #[test]
    fn expansion_scans_a_directory_shared_by_several_agents_once() {
        let registry = Registry::parse(REGISTRY).expect("registry parses");
        let shared = registry.project_skill_dirs();
        assert_eq!(shared, vec![".claude/skills", ".agents/skills", "skills"]);
        let source = FakeSource {
            directories: HashSet::from([
                PathBuf::from("/work/.claude/skills"),
                PathBuf::from("/work/.agents/skills"),
                PathBuf::from("/work/skills"),
            ]),
        };
        let inputs = project_scan_inputs(&root("/work"), &registry, &source);
        let paths = inputs
            .iter()
            .map(|input| input.path.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("/work/.claude/skills"),
                PathBuf::from("/work/.agents/skills"),
                PathBuf::from("/work/skills")
            ]
        );
    }

    #[test]
    fn expansion_of_a_root_without_known_directories_yields_no_input() {
        let registry = Registry::parse(REGISTRY).expect("registry parses");
        let source = FakeSource {
            directories: HashSet::from([PathBuf::from("/work"), PathBuf::from("/work/src")]),
        };
        assert!(project_scan_inputs(&root("/work"), &registry, &source).is_empty());
    }

    #[test]
    fn expansion_of_a_vanished_root_keeps_it_as_one_missing_location() {
        let registry = Registry::parse(REGISTRY).expect("registry parses");
        let source = FakeSource {
            directories: HashSet::new(),
        };
        let inputs = project_scan_inputs(&root("/gone"), &registry, &source);
        assert_eq!(inputs.len(), 1, "{inputs:?}");
        assert_eq!(inputs[0].path, PathBuf::from("/gone"));
        assert_eq!(inputs[0].root_id.as_deref(), Some("root-1"));
    }
}
