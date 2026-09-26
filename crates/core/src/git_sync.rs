#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    NotConfigured,
    Synced,
    NeedsPull,
    NeedsPush,
    NeedsSync,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFacts {
    pub local_revision: Option<String>,
    pub remote_revision: Option<String>,
    pub has_local_changes: bool,
    pub ahead: u32,
    pub behind: u32,
}

pub fn classify(facts: &SyncFacts, configured: bool) -> SyncState {
    if !configured {
        return SyncState::NotConfigured;
    }
    if facts.behind > 0 && (facts.ahead > 0 || facts.has_local_changes) {
        return SyncState::NeedsSync;
    }
    if facts.behind > 0 {
        return SyncState::NeedsPull;
    }
    if facts.ahead > 0 || facts.has_local_changes || facts.remote_revision.is_none() {
        return SyncState::NeedsPush;
    }
    SyncState::Synced
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(ahead: u32, behind: u32, dirty: bool, remote: bool) -> SyncFacts {
        SyncFacts {
            local_revision: Some("local".into()),
            remote_revision: remote.then(|| "remote".into()),
            has_local_changes: dirty,
            ahead,
            behind,
        }
    }

    #[test]
    fn classifies_every_user_visible_state() {
        assert_eq!(
            classify(&facts(0, 0, false, false), false),
            SyncState::NotConfigured
        );
        assert_eq!(classify(&facts(0, 0, false, true), true), SyncState::Synced);
        assert_eq!(
            classify(&facts(0, 1, false, true), true),
            SyncState::NeedsPull
        );
        assert_eq!(
            classify(&facts(1, 0, false, true), true),
            SyncState::NeedsPush
        );
        assert_eq!(
            classify(&facts(0, 0, true, true), true),
            SyncState::NeedsPush
        );
        assert_eq!(
            classify(&facts(1, 1, false, true), true),
            SyncState::NeedsSync
        );
        assert_eq!(
            classify(&facts(0, 1, true, true), true),
            SyncState::NeedsSync
        );
    }
}
