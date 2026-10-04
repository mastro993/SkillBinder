use gpui::{Entity, Window};
use skillbinder_engine::Engine;
use skillbinder_proto::OnboardingStep;
use skillbinder_ui::NativeView;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct Smoke(Arc<Mutex<Option<Result<(), String>>>>);

impl Smoke {
    pub(crate) fn from_arguments() -> Option<Self> {
        std::env::args()
            .any(|argument| argument == "--smoke-test")
            .then(Self::default)
    }
    pub(crate) fn start(self, view: Entity<NativeView>, window: &Window) {
        self.frame(view, window, 0);
    }
    fn frame(self, view: Entity<NativeView>, window: &Window, step: usize) {
        window.on_next_frame(move |window, cx| {
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
