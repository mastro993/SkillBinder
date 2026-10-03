//! Native UI fixture with an isolated home. No fixture switch exists in the production binary.
use gpui::{prelude::*, *};
use skillbinder_app::AppState;
use skillbinder_core::onboarding::OnboardingStep;
use skillbinder_platform::paths::AppPaths;
use std::{fs, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let home = fixture.path().join("home");
    let paths = AppPaths::for_home(&home);
    fs::create_dir_all(&home)?;
    for index in 0..105 {
        let slug = format!("sample-{index:03}");
        let skill = home.join(".claude/skills").join(&slug);
        fs::create_dir_all(skill.join("assets"))?;
        fs::write(
            skill.join("SKILL.md"),
            format!(
                "---\nname: {slug}\ndescription: A native UI test skill\n---\n# {slug}\nFixture contents.\n"
            ),
        )?;
        fs::write(
            skill.join("assets/notes.txt"),
            "A second file for preview and copy.",
        )?;
    }
    let invalid = home.join(".claude/skills/needs-repair");
    fs::create_dir_all(&invalid)?;
    fs::write(invalid.join("SKILL.md"), "This skill has no frontmatter.")?;
    for (agent, body) in [(".claude", "First copy"), (".codex", "Second copy")] {
        let skill = home.join(agent).join("skills/shared-slug");
        fs::create_dir_all(&skill)?;
        fs::write(
            skill.join("SKILL.md"),
            format!("---\nname: shared-slug\ndescription: {body}\n---\n{body}\n"),
        )?;
    }
    let service = Arc::new(AppState::open(paths, home)?);
    if std::env::args().any(|arg| arg == "--ready") {
        service.bootstrap.snapshot()?;
        for step in [
            OnboardingStep::Boundaries,
            OnboardingStep::SyncChoice,
            OnboardingStep::Ready,
        ] {
            service.bootstrap.save_step(step)?;
        }
        service.bootstrap.complete_local()?;
    }
    println!("Isolated UI fixture: {}", fixture.path().display());
    let _logging = skillbinder_app::logging::install(&service.paths, &service.home);
    let app = Application::new().with_assets(skillbinder_ui::Assets);
    println!("Native platform initialized");
    app.run(move |cx| {
        skillbinder_ui::init(cx);
        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1180.), px(760.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("SkillBinder · isolated fixture".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                skillbinder_ui::configure_window(window, cx);
                let view = cx.new(|cx| skillbinder_ui::Workspace::new(service, window, cx));
                cx.new(|cx| skillbinder_ui::Root::new(view, window, cx))
            },
        )
        .expect("open fixture window");
        println!("Native fixture window opened");
        cx.activate(true);
    });
    drop(fixture);
    Ok(())
}
