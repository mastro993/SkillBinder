//! Native desktop lifecycle.

use gpui::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

fn main() {
    gpui_platform::application()
        .with_assets(skillbinder_ui::Assets)
        .run(|cx: &mut App| {
            skillbinder_ui::init(cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let mut bounds = Bounds::centered(None, size(px(1180.0), px(728.0)), cx);
            bounds.origin.x = px(f32::from(bounds.origin.x).floor());
            bounds.origin.y = px(f32::from(bounds.origin.y).floor());
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(860.0), px(588.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some("SkillBinder".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            if let Err(error) = cx.open_window(options, |window, cx| {
                skillbinder_theme::follow_window(window, cx);
                cx.new(|cx| skillbinder_ui::NativeView::new(window, cx))
            }) {
                eprintln!("Could not open SkillBinder: {error}");
                cx.quit();
            }
            cx.activate(true);
        });
}
