use gpui_kit::{Entity, Window};
mod arguments;
pub(crate) use arguments::{NativeMode, parse_launch};
use skillbinder_engine::Engine;
use skillbinder_proto::OnboardingStep;
use skillbinder_ui::NativeView;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

static RENDER_ERROR: AtomicBool = AtomicBool::new(false);

struct SmokeLogger;
impl log::Log for SmokeLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            if record.level() == log::Level::Error && !is_platform_backend(record.target()) {
                RENDER_ERROR.store(true, Ordering::Relaxed);
            }
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }
    fn flush(&self) {}
}

/// Platform backends report host display and input conditions, such as Xvfb
/// exposing no pointer, rather than rendering failures.
fn is_platform_backend(target: &str) -> bool {
    let krate = target.split("::").next().unwrap_or_default();
    matches!(krate, "gpui_linux" | "gpui_macos" | "gpui_windows")
}

#[cfg(test)]
mod tests {
    use super::is_platform_backend;

    #[test]
    fn ignores_only_platform_backend_errors() {
        assert!(is_platform_backend("gpui_linux::linux::x11::client"));
        assert!(is_platform_backend("gpui_windows"));
        assert!(!is_platform_backend("gpui::elements::svg"));
        assert!(!is_platform_backend("gpui_wgpu::wgpu_context"));
        assert!(!is_platform_backend(""));
    }
}

pub(crate) fn initialize_logging() -> anyhow::Result<()> {
    log::set_logger(&SmokeLogger)
        .map_err(|_| anyhow::anyhow!("Could not initialize native smoke logging"))?;
    log::set_max_level(log::LevelFilter::Info);
    Ok(())
}

#[derive(Clone, Default)]
pub(crate) struct Smoke(Arc<Mutex<Option<Result<(), String>>>>);

impl Smoke {
    pub(crate) fn start(self, view: Entity<NativeView>, window: &Window) {
        self.frame(view, window, 0);
    }
    fn frame(self, view: Entity<NativeView>, window: &Window, step: usize) {
        window.on_next_frame(move |window, cx| {
            eprintln!("Native smoke: rendered frame {step}");
            let result = view.update(cx, |view, cx| view.smoke_step(step, window, cx));
            match result {
                Ok(false) => self.frame(view, window, step + 1),
                result => {
                    if let Ok(mut outcome) = self.0.lock() {
                        *outcome = Some(result.map(|_| ()));
                    }
                    cx.quit();
                }
            }
        });
    }
    pub(crate) fn verify(self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !RENDER_ERROR.load(Ordering::Relaxed),
            "Native renderer reported an error"
        );
        let outcome = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Native smoke result lock failed"))?;
        match outcome.as_ref() {
            Some(Ok(())) => {
                println!("NATIVE_SMOKE_OK: rendered screens, text, focus, overlay, and shutdown")
            }
            Some(Err(message)) => anyhow::bail!("Native smoke failed: {message}"),
            None => anyhow::bail!("Native smoke did not finish"),
        }
        Ok(())
    }
}

pub(crate) async fn prepare(engine: &Engine) -> anyhow::Result<()> {
    if engine.bootstrap().await?.step == OnboardingStep::Complete {
        return Ok(());
    }
    for step in [
        OnboardingStep::Boundaries,
        OnboardingStep::SyncChoice,
        OnboardingStep::Ready,
    ] {
        engine.set_onboarding(step).await?;
    }
    engine.complete_onboarding().await?;
    Ok(())
}
