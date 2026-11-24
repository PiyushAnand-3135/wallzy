use crate::app::WallzyApp;
use crate::models::{GridDensity, NavSection};
use crate::theme::ThemePalette;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::f32::consts::TAU;
use std::time::Duration;

pub fn render_topbar(
    app: &WallzyApp,
    visible_count: usize,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let show_search = !matches!(app.active_nav, NavSection::Settings | NavSection::Downloads);

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        // Left: Editorial Display Header
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2p5()
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::BOLD)
                                .text_color(palette.text_primary)
                                .child(app.active_nav.label()),
                        )
                        .when(show_search, |this| {
                            this.child(
                                div()
                                    .font_family(app.typography.mono_font.clone())
                                    .px_2()
                                    .py_0p5()
                                    .rounded_full()
                                    .bg(palette.bg_elevated)
                                    .text_xs()
                                    .text_color(palette.text_muted)
                                    .child(format!("{}", visible_count)),
                            )
                        }),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(palette.text_muted)
                        .child(app.active_nav.subtitle()),
                ),
        )
        // Right: Unified Search & Studio Controls
        .when(show_search, |this| {
            let density = app.settings.grid_density;
            this.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().w(px(280.0)).child(
                            Input::new(&app.search_input)
                                .small()
                                .cleanable(true)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .size(px(14.0))
                                        .text_color(palette.text_subtle),
                                ),
                        ),
                    )
                    .when(app.settings.has_any_online_provider(), |row| {
                        row.child(
                            Button::new("online-search-btn")
                                .small()
                                .primary()
                                .rounded(ButtonRounded::Large)
                                .loading(app.is_fetching)
                                .icon(Icon::new(IconName::Sparkles))
                                .label("Explore Online")
                                .tooltip("Fetch wallpapers from all enabled online providers")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.fetch_online_providers(Some(window), cx);
                                })),
                        )
                    })
                    .child(
                        Button::new("cycle-density")
                            .small()
                            .ghost()
                            .rounded(ButtonRounded::Large)
                            .icon(Icon::new(IconName::LayoutGrid))
                            .label(density.label())
                            .tooltip("Cycle card density")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let next = match this.settings.grid_density {
                                    GridDensity::Compact => GridDensity::Comfortable,
                                    GridDensity::Comfortable => GridDensity::Spacious,
                                    GridDensity::Spacious => GridDensity::Compact,
                                };
                                this.set_grid_density(next, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("refresh-catalog")
                            .size(px(28.0))
                            .rounded_lg()
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|s| s.bg(palette.bg_hover))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.refresh_catalog(Some(window), cx);
                            }))
                            .child(if app.is_fetching {
                                Icon::new(IconName::RefreshCw)
                                    .size(px(14.0))
                                    .text_color(palette.accent)
                                    .with_animation(
                                        "sync-spin",
                                        Animation::new(Duration::from_millis(850)).repeat(),
                                        |icon, delta| icon.rotate(radians(delta * TAU)),
                                    )
                                    .into_any_element()
                            } else {
                                Icon::new(IconName::RefreshCw)
                                    .size(px(14.0))
                                    .text_color(palette.text_muted)
                                    .into_any_element()
                            }),
                    ),
            )
        })
}
