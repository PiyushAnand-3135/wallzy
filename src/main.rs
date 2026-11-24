mod app;
mod models;
mod providers;
mod services;
mod theme;
mod ui;

use app::WallzyApp;
use gpui_kit::component::TitleBar;
use gpui_kit::*;

fn main() {
    application().with_assets(assets::Assets).run(|cx| {
        init(cx);

        let bounds = Bounds::centered(None, size(px(1340.0), px(860.0)), cx);

        open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(960.0), px(640.0))),
                ..TitleBar::window_options()
            },
            cx,
            |window, cx| cx.new(|cx| WallzyApp::new(window, cx)),
        )
        .expect("Failed to open Wallzy window");
    });
}
