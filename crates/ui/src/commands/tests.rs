use super::needs_setup;
use skillbinder_proto::command_line::{Change, Consent, Find, Query, Request};

#[test]
fn only_status_answers_before_onboarding_finishes() {
    assert!(!needs_setup(&Request::Status));
    assert!(!needs_setup(&Request::Open));
    assert!(!needs_setup(&Request::Hold));
    assert!(needs_setup(&Request::Read {
        query: Query::Presets
    }));
    assert!(needs_setup(&Request::Find { find: Find::Scan }));
    assert!(needs_setup(&Request::Change {
        change: Change::RetryCopies,
        consent: Consent::default(),
    }));
}
