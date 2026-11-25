use gpui_kit::{App, Entity, Window};
mod arguments;
mod commands;
mod gallery;
mod holds;
mod workspaces;
pub(crate) use arguments::{NativeMode, parse_launch};
pub(crate) use commands::register_commands;
use skillbinder_engine::{Bridge, Engine};
use skillbinder_ui::NativeView;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

static RENDER_ERROR: AtomicBool = AtomicBool::new(false);

/// The log's bridge, which in a smoke run also echoes `INFO` and above to stderr and fails the
/// run on an error from anything but the host.
struct SmokeLogger(Bridge);
impl log::Log for SmokeLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info || self.0.enabled(metadata)
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.0.enabled(record.metadata()) {
            self.0.log(record);
        }
        if record.level() <= log::Level::Info {
            if record.level() == log::Level::Error && !is_host_environment(record.target()) {
                RENDER_ERROR.store(true, Ordering::Relaxed);
            }
            eprintln!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }
    fn flush(&self) {
        self.0.flush();
    }
}

/// Platform backends and wgpu's EGL adapter probe report host display, input,
/// and driver conditions, such as Xvfb exposing no pointer or nested Weston
/// lacking a GL driver while Vulkan renders, rather than rendering failures.
fn is_host_environment(target: &str) -> bool {
    let krate = target.split("::").next().unwrap_or_default();
    matches!(krate, "gpui_linux" | "gpui_macos" | "gpui_windows")
        || target.starts_with("wgpu_hal::gles::egl")
}

#[cfg(test)]
mod tests {
    use super::is_host_environment;

    #[test]
    fn ignores_only_host_environment_errors() {
        assert!(is_host_environment("gpui_linux::linux::x11::client"));
        assert!(is_host_environment("gpui_windows"));
        assert!(is_host_environment("wgpu_hal::gles::egl"));
        assert!(!is_host_environment("wgpu_hal::vulkan::instance"));
        assert!(!is_host_environment("gpui::elements::svg"));
        assert!(!is_host_environment("gpui_wgpu::wgpu_context"));
        assert!(!is_host_environment(""));
    }
}

/// Installs the log's bridge, wrapped in a smoke run so the run sees what it logs.
pub(crate) fn initialize_logging(bridge: Bridge, smoke: bool) -> anyhow::Result<()> {
    if !smoke {
        return crate::install_logging(bridge);
    }
    let level = bridge.max_level().max(log::LevelFilter::Info);
    log::set_boxed_logger(Box::new(SmokeLogger(bridge)))?;
    log::set_max_level(level);
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

/// What a native-test window adds: the MCP bridge on every window it opens, stopped when the
/// window closes, and the smoke run on the first.
#[derive(Clone)]
pub(crate) struct WindowHooks {
    mode: NativeMode,
    smoke: Rc<RefCell<Option<Smoke>>>,
    bridge: Rc<RefCell<Option<gpui_mcp::BridgeHandle>>>,
}

impl WindowHooks {
    pub(crate) fn new(mode: NativeMode, smoke: Option<Smoke>) -> Self {
        Self {
            mode,
            smoke: Rc::new(RefCell::new(smoke)),
            bridge: Rc::default(),
        }
    }

    /// Installs the bridge and starts the smoke run; returns whether the window can be driven.
    pub(crate) fn attach(&self, view: Entity<NativeView>, window: &mut Window, cx: &App) -> bool {
        if self.mode == NativeMode::Mcp {
            let Ok(app_id) = gpui_mcp::AppId::new("skillbinder") else {
                unreachable!("static SkillBinder app ID is valid");
            };
            let config = gpui_mcp::BridgeConfig::new(app_id, "SkillBinder");
            match gpui_mcp::BridgeHandle::install(window, cx, config) {
                Ok(handle) => {
                    if let Err(error) = register_commands(&handle, view.clone()) {
                        eprintln!("Could not register native-test commands: {error}");
                    }
                    *self.bridge.borrow_mut() = Some(handle);
                }
                Err(error) => {
                    eprintln!("Could not start native MCP bridge: {error}");
                    return false;
                }
            }
        }
        if let Some(smoke) = self.smoke.borrow_mut().take() {
            eprintln!("Native smoke: creating view");
            smoke.start(view, window);
        }
        true
    }

    /// Stops the bridge when its window closes, and at quit, since AppKit terminates the process
    /// before it would drop.
    pub(crate) fn shutdown(&self) {
        if let Some(bridge) = self.bridge.borrow_mut().take() {
            bridge.shutdown();
        }
    }
}

/// Finishes onboarding with every detected agent chosen and nothing imported.
pub(crate) async fn prepare(engine: &Engine) -> anyhow::Result<()> {
    if !engine.bootstrap().await?.onboarding.finished {
        let found: BTreeSet<String> = engine
            .snapshot()
            .await?
            .agents
            .into_iter()
            .filter(|agent| agent.detected)
            .filter_map(|agent| agent.global_skills_dir)
            .collect();
        engine.choose_agents(found.clone(), found).await?;
        engine.finish_onboarding(None).await?;
    }
    Ok(())
}
