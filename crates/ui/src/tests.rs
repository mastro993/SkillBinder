//! Native interaction tests use the production service graph and isolated local files.
use crate::{
    Workspace,
    components::review::Review,
    state::{PAGE_SIZE, Screen},
};
use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, prelude::*};
use gpui_component::{Root, WindowExt};
use skillbinder_app::{AppPaths, AppState, actions, models::*};
use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

fn service(ready: bool, candidates: usize) -> (tempfile::TempDir, Arc<AppState>) {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("home");
    fs::create_dir_all(&home).unwrap();
    for i in 0..candidates {
        let slug = format!("sample-{i:03}");
        let path = home.join(".claude/skills").join(&slug);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("SKILL.md"),
            format!("---\nname: {slug}\ndescription: Test skill\n---\nContents\n"),
        )
        .unwrap();
    }
    let service = Arc::new(AppState::open(AppPaths::for_home(&home), home).unwrap());
    if ready {
        actions::bootstrap::system_bootstrap(&service)
            .into_result()
            .unwrap();
        for step in [
            OnboardingStep::Boundaries,
            OnboardingStep::SyncChoice,
            OnboardingStep::Ready,
        ] {
            actions::onboarding::onboarding_progress_update(
                &service,
                UpdateOnboardingProgressRequest { step },
            )
            .into_result()
            .unwrap();
        }
        actions::onboarding::onboarding_complete_local(&service)
            .into_result()
            .unwrap();
    }
    (directory, service)
}

fn scan(service: &Arc<AppState>) -> DiscoveryResultsResponse {
    let started = actions::discovery::discovery_start(service)
        .into_result()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let data = actions::discovery::discovery_results(
            service,
            DiscoveryResultsRequest {
                scan_id: started.scan_id.clone(),
                offset: 0,
                limit: PAGE_SIZE,
            },
        )
        .into_result()
        .unwrap();
        if data.phase != ScanPhase::Running {
            assert_eq!(data.phase, ScanPhase::Finished);
            return data;
        }
        assert!(Instant::now() < deadline, "fixture scan did not finish");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn mount(
    service: Arc<AppState>,
    cx: &mut TestAppContext,
) -> (Entity<Workspace>, &mut VisualTestContext) {
    cx.update(crate::init);
    let mut workspace = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        crate::configure_window(window, cx);
        let view = cx.new(|cx| Workspace::new(service, window, cx));
        workspace = Some(view.clone());
        Root::new(view, window, cx)
    });
    (workspace.unwrap(), cx)
}

fn click(label: &'static str, cx: &mut VisualTestContext) {
    cx.run_until_parked();
    let bounds = cx
        .debug_bounds(label)
        .unwrap_or_else(|| panic!("missing control: {label}"));
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn onboarding_buttons_persist_progress_and_open_the_workspace(cx: &mut TestAppContext) {
    let (_directory, service) = service(false, 0);
    let (view, cx) = mount(service, cx);
    for step in [
        OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice,
        OnboardingStep::Ready,
    ] {
        click("Continue", cx);
        view.read_with(cx, |view, _| {
            assert_eq!(view.state.bootstrap.as_ref().unwrap().onboarding.step, step)
        });
    }
    click("Create local library", cx);
    view.read_with(cx, |view, _| {
        let bootstrap = view.state.bootstrap.as_ref().unwrap();
        assert!(bootstrap.onboarding.completed);
        assert_eq!(bootstrap.library_state, LibraryState::Ready);
        assert!(view.state.errors.is_empty());
    });
    assert!(cx.debug_bounds("Scan for skills").is_some());
}

#[gpui::test]
fn pagination_replaces_an_inflight_reply_and_navigation_preserves_selection(
    cx: &mut TestAppContext,
) {
    let (_directory, service) = service(true, 101);
    scan(&service);
    let (view, cx) = mount(service, cx);
    view.update_in(cx, |view, window, cx| {
        view.load_scan(window, cx);
        let first = view.state.scan.as_ref().unwrap().candidates[0].clone();
        view.state.selection.toggle(&first);
        view.state.selection.offset = PAGE_SIZE;
        view.load_scan(window, cx);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let data = view.state.scan.as_ref().unwrap();
        assert_eq!(data.offset, PAGE_SIZE);
        assert_eq!(data.candidates.len(), 1);
        assert_eq!(view.state.selection.selected.len(), 1);
        assert!(view.state.pending.is_empty());
    });
    click("Library", cx);
    view.read_with(cx, |view, _| assert_eq!(view.state.screen, Screen::Library));
    click("Discovery · 101", cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.state.screen, Screen::Discovery);
        assert_eq!(view.state.selection.offset, PAGE_SIZE);
        assert_eq!(view.state.selection.selected.len(), 1);
    });
}

#[gpui::test]
fn import_dialog_applies_the_reviewed_plan_and_refreshes_library(cx: &mut TestAppContext) {
    let (_directory, service) = service(true, 1);
    let data = scan(&service);
    let source = service.home.join(".claude/skills/sample-000/SKILL.md");
    let original = fs::read(&source).unwrap();
    let plan = actions::imports::imports_prepare(
        &service,
        ImportPrepareRequest {
            candidate_ids: data
                .candidates
                .into_iter()
                .map(|c| c.candidate_id)
                .collect(),
            allow_invalid_skills: false,
        },
    )
    .into_result()
    .unwrap();
    let (view, cx) = mount(service.clone(), cx);
    cx.update(|window, cx| Review::open_import(plan, service, view.downgrade(), window, cx));
    click("Apply import", cx);
    assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
    view.read_with(cx, |view, _| {
        let library = view.state.library.as_ref().unwrap();
        assert_eq!(library.skills.len(), 1);
        assert_eq!(library.skills[0].slug, "sample-000");
        assert!(library.has_uncommitted_changes);
        assert!(view.state.errors.is_empty());
    });
    assert_eq!(fs::read(source).unwrap(), original);
}
