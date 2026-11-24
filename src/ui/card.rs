use crate::app::WallzyApp;
use crate::models::{GridDensity, Wallpaper, WallpaperSource};
use crate::theme::ThemePalette;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub fn render_card(
    wp: Wallpaper,
    density: GridDensity,
    show_badges: bool,
    mono_font: SharedString,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement + use<> {
    let id = wp.id.clone();
    let id_fav = wp.id.clone();
    let id_dl = wp.id.clone();
    let id_apply = wp.id.clone();

    let card_height = density.thumb_height() + 26.0;
    let quality_badge = wp.quality_badge();
    let resolution_text = wp.resolution_label();
    let image_path = wp.display_image_path().cloned();

    div()
        .id(format!("card-{}", id))
        .group("card")
        .relative()
        .flex_1()
        .min_w_0()
        .h(px(card_height))
        .rounded_xl()
        .overflow_hidden()
        .bg(rgb(wp.primary_hex))
        .border_1()
        .border_color(palette.hairline)
        .shadow_sm()
        .hover(|s| s.border_color(palette.accent).shadow_md())
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_dropdown = None;
            this.preview_id = Some(id.clone());
            cx.notify();
        }))
        // Edge-to-edge Artwork (positioned absolutely so intrinsic dimensions never resize card)
        .when_some(image_path, |this, path| {
            this.child(
                div()
                    .absolute()
                    .inset_0()
                    .child(img(path).size_full().object_fit(ObjectFit::Cover)),
            )
        })
        // Hover-only subtle bottom gradient + editorial metadata
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h(px(88.0))
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(rgba(0x00000000), 0.),
                    linear_color_stop(rgba(0x06070ad8), 1.),
                )),
        )
        // Top-Left Quality & Saved Badges (subtle on rest, crisp on hover)
        .when(show_badges, |this| {
            this.child(
                div()
                    .absolute()
                    .top_2p5()
                    .left_2p5()
                    .flex()
                    .items_center()
                    .gap_1()
                    .opacity(if wp.is_downloaded { 0.92 } else { 0.0 })
                    .group_hover("card", |s| s.opacity(1.0))
                    .when_some(quality_badge, |row, badge| {
                        row.child(
                            div()
                                .font_family(mono_font.clone())
                                .px_2()
                                .py_0p5()
                                .rounded_full()
                                .bg(palette.glass_dark)
                                .border_1()
                                .border_color(palette.glass_border)
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0xf8fafc))
                                .child(badge),
                        )
                    })
                    .when(wp.is_downloaded, |row| {
                        row.child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_full()
                                .bg(rgba(0x16a34ad8))
                                .border_1()
                                .border_color(rgba(0xffffff28))
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0xffffff))
                                .child(if wp.source == WallpaperSource::Local {
                                    "Local"
                                } else {
                                    "Saved"
                                }),
                        )
                    }),
            )
        })
        // Top-Right Frosted Quick Actions
        .child(
            div()
                .absolute()
                .top_2p5()
                .right_2p5()
                .flex()
                .items_center()
                .gap_1p5()
                .opacity(if wp.is_favorite { 1.0 } else { 0.0 })
                .group_hover("card", |s| s.opacity(1.0))
                .child(glass_action_btn(
                    format!("fav-{}", id_fav),
                    IconName::Heart,
                    if wp.is_favorite {
                        rgb(0xf43f5e)
                    } else {
                        rgb(0xf8fafc)
                    },
                    palette,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.toggle_favorite(&id_fav, Some(window), cx);
                    }),
                ))
                .child(glass_action_btn(
                    format!("apply-{}", id_apply),
                    IconName::Monitor,
                    rgb(0xf8fafc),
                    palette,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.apply_wallpaper(&id_apply, window, cx);
                    }),
                ))
                .child(glass_action_btn(
                    format!("dl-{}", id_dl),
                    if wp.is_downloaded {
                        IconName::Check
                    } else {
                        IconName::Download
                    },
                    if wp.is_downloaded {
                        rgb(0x4ade80)
                    } else {
                        rgb(0xf8fafc)
                    },
                    palette,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.download_wallpaper(&id_dl, false, window, cx);
                    }),
                )),
        )
        // Bottom Minimal Overlay Title & Resolution
        .child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .px_3p5()
                .py_3()
                .flex()
                .items_end()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(0xf8fafc))
                                .truncate()
                                .child(wp.name),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgba(0xe2e8f0b8))
                                .truncate()
                                .child(wp.author),
                        ),
                )
                .child(
                    div()
                        .font_family(mono_font)
                        .text_xs()
                        .text_color(rgba(0xe2e8f0cc))
                        .opacity(0.0)
                        .group_hover("card", |s| s.opacity(1.0))
                        .child(resolution_text),
                ),
        )
}

fn glass_action_btn(
    id: impl Into<ElementId>,
    icon: IconName,
    icon_color: Rgba,
    palette: ThemePalette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .size(px(30.0))
        .rounded_full()
        .bg(palette.glass_dark)
        .border_1()
        .border_color(palette.glass_border)
        .flex()
        .items_center()
        .justify_center()
        .hover(|s| s.bg(rgba(0x0b0d12f2)))
        .on_click(on_click)
        .child(Icon::new(icon).size(px(14.0)).text_color(icon_color))
}
