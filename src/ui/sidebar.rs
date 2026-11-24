use crate::app::WallzyApp;
use crate::models::NavSection;
use crate::theme::ThemePalette;
use gpui_kit::component::Icon;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::sync::{Arc, OnceLock};

const LOGO_BYTES: &[u8] = include_bytes!("../assets/wallzy_logo.png");

fn app_logo_image() -> Arc<Image> {
    static LOGO: OnceLock<Arc<Image>> = OnceLock::new();
    LOGO.get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Png, LOGO_BYTES.to_vec())))
        .clone()
}

pub fn render_sidebar(
    app: &WallzyApp,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let favorites_count = app.wallpapers.iter().filter(|w| w.is_favorite).count();
    let local_count = app
        .wallpapers
        .iter()
        .filter(|w| w.is_downloaded || w.local_path.is_some())
        .count();
    let active_downloads = app
        .downloads
        .iter()
        .filter(|d| d.status.is_active())
        .count();
    let total_downloads = app.downloads.len();

    div()
        .w(px(232.0))
        .h_full()
        .flex()
        .flex_col()
        .justify_between()
        .bg(palette.bg_sidebar)
        .border_r_1()
        .border_color(palette.hairline)
        .px_3p5()
        .py_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_6()
                // Brand Lockup
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_2()
                        .pt_0p5()
                        .child(
                            div()
                                .size(px(36.0))
                                .rounded_xl()
                                .overflow_hidden()
                                .shadow_md()
                                .border_1()
                                .border_color(palette.hairline_strong)
                                .child(
                                    img(app_logo_image())
                                        .size_full()
                                        .object_fit(ObjectFit::Contain),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(palette.text_primary)
                                        .child("Wallzy"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(palette.text_subtle)
                                        .child("Wallpaper Studio"),
                                ),
                        ),
                )
                // Browse Group
                .child(render_nav_group(
                    "DISCOVER",
                    NavSection::browse_sections(),
                    app.active_nav,
                    palette,
                    |_| None,
                    cx,
                ))
                // Library Group
                .child(render_nav_group(
                    "COLLECTION",
                    NavSection::library_sections(),
                    app.active_nav,
                    palette,
                    |section| match section {
                        NavSection::MyWallpapers if local_count > 0 => {
                            Some(local_count.to_string())
                        }
                        NavSection::Favorites if favorites_count > 0 => {
                            Some(favorites_count.to_string())
                        }
                        NavSection::Downloads if active_downloads > 0 => {
                            Some(format!("{} active", active_downloads))
                        }
                        NavSection::Downloads if total_downloads > 0 => {
                            Some(total_downloads.to_string())
                        }
                        _ => None,
                    },
                    cx,
                )),
        )
        // Ambient Footer + Settings
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .px_2p5()
                        .py_1p5()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().size(px(7.0)).rounded_full().bg(if app.is_fetching {
                                    palette.warning
                                } else {
                                    palette.success
                                }))
                                .child(div().text_xs().text_color(palette.text_subtle).child(
                                    if app.is_fetching {
                                        "Syncing catalog..."
                                    } else {
                                        "Library synced"
                                    },
                                )),
                        )
                        .child(
                            div()
                                .font_family(app.typography.mono_font.clone())
                                .text_xs()
                                .text_color(palette.text_subtle)
                                .child(app.wallpapers.len().to_string()),
                        ),
                )
                .child(div().h(px(1.0)).mx_2().bg(palette.hairline))
                .child(render_nav_item(
                    NavSection::Settings,
                    app.active_nav == NavSection::Settings,
                    None,
                    palette,
                    cx,
                )),
        )
}

fn render_nav_group(
    title: &'static str,
    sections: &[NavSection],
    active: NavSection,
    palette: ThemePalette,
    badge_for: impl Fn(NavSection) -> Option<String>,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let mut items = Vec::with_capacity(sections.len());
    for &section in sections {
        let badge = badge_for(section);
        items.push(render_nav_item(
            section,
            active == section,
            badge,
            palette,
            cx,
        ));
    }

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .px_2p5()
                .pb_1()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.text_subtle)
                .child(title),
        )
        .children(items)
}

fn render_nav_item(
    section: NavSection,
    is_active: bool,
    badge: Option<String>,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement + use<> {
    div()
        .id(format!("nav-{}", section.label()))
        .relative()
        .px_3()
        .py_2()
        .rounded_lg()
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .bg(if is_active {
            palette.accent_soft
        } else {
            rgba(0x00000000)
        })
        .when(is_active, |s| s.shadow_sm())
        .hover(|s| if is_active { s } else { s.bg(palette.bg_hover) })
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select_nav(section, cx);
        }))
        .when(is_active, |this| {
            this.child(
                div()
                    .absolute()
                    .left_0()
                    .top_2()
                    .bottom_2()
                    .w(px(3.0))
                    .rounded_full()
                    .bg(palette.accent),
            )
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_2p5()
                .child(
                    Icon::new(section.icon())
                        .size(px(15.0))
                        .text_color(if is_active {
                            palette.accent
                        } else {
                            palette.text_muted
                        }),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(if is_active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if is_active {
                            palette.text_primary
                        } else {
                            palette.text_muted
                        })
                        .child(section.label()),
                ),
        )
        .when_some(badge, |this, text| {
            this.child(
                div()
                    .px_2()
                    .py_0p5()
                    .rounded_full()
                    .bg(if is_active {
                        palette.accent
                    } else {
                        palette.bg_elevated
                    })
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if is_active {
                        palette.text_on_accent
                    } else {
                        palette.text_muted
                    })
                    .child(text),
            )
        })
}
