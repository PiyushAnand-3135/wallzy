use crate::app::WallzyApp;
use crate::models::{Wallpaper, WallpaperFit};
use crate::theme::ThemePalette;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::animation::{EffectTransition, ease_out_cubic};
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::time::Duration;

pub fn render_preview_modal(
    app: &WallzyApp,
    wp: &Wallpaper,
    has_prev: bool,
    has_next: bool,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let id_apply = wp.id.clone();
    let id_dl = wp.id.clone();
    let id_fav = wp.id.clone();
    let id_del = wp.id.clone();
    let local_path = wp.local_path.clone();
    let hd_path = wp
        .local_path
        .as_ref()
        .filter(|p| p.exists())
        .cloned()
        .or_else(|| app.hd_preview_path(&wp.id));
    let thumb_path = wp.display_image_path().cloned();
    let show_thumb_underlay = thumb_path
        .as_ref()
        .is_some_and(|t| hd_path.as_ref() != Some(t));
    let tags = wp.tags.clone();
    let mono_font = app.typography.mono_font.clone();

    let dialog = div()
        .id("modal-content")
        .w(px(980.0))
        .max_w(relative(0.94))
        .h(px(620.0))
        .max_h(relative(0.90))
        .flex_none()
        .rounded_2xl()
        .overflow_hidden()
        .bg(palette.bg_surface)
        .border_1()
        .border_color(palette.hairline_strong)
        .shadow_xl()
        .flex()
        .on_click(|_, _, cx| cx.stop_propagation())
        // Left: Fixed-Bounds High-Contrast Wallpaper Preview Canvas
        .child(
            image_cache(app.preview_image_cache.clone())
                .relative()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .h_full()
                .overflow_hidden()
                .bg(rgb(0x08090c))
                // Instant thumbnail underlay prevents black flashes while HD decodes
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(rgb(wp.primary_hex))
                        .flex()
                        .items_center()
                        .justify_center()
                        .p_4()
                        .when(show_thumb_underlay, |this| {
                            this.when_some(thumb_path, |this, path| {
                                this.child(
                                    img(path)
                                        .size_full()
                                        .max_w_full()
                                        .max_h_full()
                                        .object_fit(ObjectFit::Contain),
                                )
                            })
                        }),
                )
                // Crystal-clear HD layer rendered cleanly over the placeholder once ready
                .when_some(hd_path, |this, path| {
                    this.child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .p_4()
                            .child(
                                img(path)
                                    .size_full()
                                    .max_w_full()
                                    .max_h_full()
                                    .object_fit(ObjectFit::Contain),
                            ),
                    )
                })
                // Previous / Next Carousel Arrows
                .when(has_prev, |this| {
                    this.child(
                        div()
                            .id("modal-prev")
                            .absolute()
                            .left_4()
                            .top(relative(0.5))
                            .mt(px(-19.0))
                            .size(px(38.0))
                            .rounded_full()
                            .bg(palette.glass_dark)
                            .border_1()
                            .border_color(palette.glass_border)
                            .shadow_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgba(0x0b0d12f2)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.step_preview(-1, window, cx);
                            }))
                            .child(
                                Icon::new(IconName::ChevronLeft)
                                    .size(px(18.0))
                                    .text_color(rgb(0xffffff)),
                            ),
                    )
                })
                .when(has_next, |this| {
                    this.child(
                        div()
                            .id("modal-next")
                            .absolute()
                            .right_4()
                            .top(relative(0.5))
                            .mt(px(-19.0))
                            .size(px(38.0))
                            .rounded_full()
                            .bg(palette.glass_dark)
                            .border_1()
                            .border_color(palette.glass_border)
                            .shadow_md()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgba(0x0b0d12f2)))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.step_preview(1, window, cx);
                            }))
                            .child(
                                Icon::new(IconName::ChevronRight)
                                    .size(px(18.0))
                                    .text_color(rgb(0xffffff)),
                            ),
                    )
                }),
        )
        // Right: Fixed-Width Inspector Sidebar
        .child(
            div()
                .w(px(336.0))
                .flex_shrink_0()
                .h_full()
                .min_h_0()
                .border_l_1()
                .border_color(palette.hairline)
                .bg(palette.bg_sidebar)
                .p_5()
                .flex()
                .flex_col()
                .justify_between()
                .gap_4()
                .child(
                    div()
                        .id("modal-inspector-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_4()
                                // Header + Close
                                .child(
                                    div()
                                        .flex()
                                        .items_start()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .gap_0p5()
                                                .child(
                                                    div()
                                                        .text_lg()
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(palette.text_primary)
                                                        .truncate()
                                                        .child(wp.name.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(palette.text_muted)
                                                        .truncate()
                                                        .child(format!(
                                                            "by {} · {}",
                                                            wp.author,
                                                            wp.source.label()
                                                        )),
                                                ),
                                        )
                                        .child(
                                            Button::new("close-modal")
                                                .small()
                                                .ghost()
                                                .rounded(ButtonRounded::Large)
                                                .icon(Icon::new(IconName::X))
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.close_preview(window, cx);
                                                })),
                                        ),
                                )
                                // Metadata Specs Card
                                .child(
                                    div()
                                        .p_3p5()
                                        .rounded_xl()
                                        .bg(palette.bg_surface)
                                        .border_1()
                                        .border_color(palette.hairline)
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(spec_row(
                                            "Resolution",
                                            wp.resolution_label(),
                                            &mono_font,
                                            palette,
                                        ))
                                        .child(spec_row(
                                            "Aspect Ratio",
                                            wp.aspect_ratio_label(),
                                            &mono_font,
                                            palette,
                                        ))
                                        .child(spec_row(
                                            "Orientation",
                                            wp.orientation.label().to_string(),
                                            &mono_font,
                                            palette,
                                        ))
                                        .child(spec_row(
                                            "Category",
                                            wp.category.clone(),
                                            &mono_font,
                                            palette,
                                        ))
                                        .child(spec_row(
                                            "File Size",
                                            wp.formatted_size(),
                                            &mono_font,
                                            palette,
                                        )),
                                )
                                // Color Palette Swatches
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text_subtle)
                                                .child("COLOR PALETTE"),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(color_chip(
                                                    wp.primary_hex,
                                                    &mono_font,
                                                    palette,
                                                ))
                                                .child(color_chip(
                                                    wp.secondary_hex,
                                                    &mono_font,
                                                    palette,
                                                )),
                                        ),
                                )
                                // Clickable Tags
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text_subtle)
                                                .child("TAGS"),
                                        )
                                        .child(div().flex().flex_wrap().gap_1p5().children(
                                            tags.into_iter().map(|tag| {
                                                let tag_query = tag.clone();
                                                div()
                                                    .id(format!("tag-{}", tag))
                                                    .px_2p5()
                                                    .py_1()
                                                    .rounded_full()
                                                    .bg(palette.bg_surface)
                                                    .border_1()
                                                    .border_color(palette.hairline)
                                                    .text_xs()
                                                    .text_color(palette.text_muted)
                                                    .cursor_pointer()
                                                    .hover(|s| {
                                                        s.bg(palette.bg_hover)
                                                            .text_color(palette.text_primary)
                                                    })
                                                    .on_click(cx.listener(
                                                        move |this, _, window, cx| {
                                                            this.close_preview(window, cx);
                                                            this.set_search_query(
                                                                &tag_query, window, cx,
                                                            );
                                                        },
                                                    ))
                                                    .child(format!("#{}", tag))
                                            }),
                                        )),
                                )
                                // Wallpaper Fit Selector
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text_subtle)
                                                .child("DESKTOP FIT"),
                                        )
                                        .child(div().flex().flex_wrap().gap_1().children(
                                            WallpaperFit::all().iter().copied().map(|fit| {
                                                let active = app.settings.wallpaper_fit == fit;
                                                div()
                                                    .id(format!("modal-fit-{}", fit.label()))
                                                    .px_2p5()
                                                    .py_1()
                                                    .rounded_lg()
                                                    .cursor_pointer()
                                                    .text_xs()
                                                    .font_weight(if active {
                                                        FontWeight::SEMIBOLD
                                                    } else {
                                                        FontWeight::NORMAL
                                                    })
                                                    .bg(if active {
                                                        palette.accent
                                                    } else {
                                                        palette.bg_surface
                                                    })
                                                    .border_1()
                                                    .border_color(if active {
                                                        palette.accent
                                                    } else {
                                                        palette.hairline
                                                    })
                                                    .text_color(if active {
                                                        palette.text_on_accent
                                                    } else {
                                                        palette.text_muted
                                                    })
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.set_wallpaper_fit(fit, cx);
                                                    }))
                                                    .child(fit.label())
                                            }),
                                        )),
                                ),
                        ),
                )
                // Footer Actions
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            Button::new("modal-apply")
                                .primary()
                                .rounded(ButtonRounded::Large)
                                .w_full()
                                .icon(Icon::new(IconName::Monitor))
                                .label("Set as Desktop Wallpaper")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.apply_wallpaper(&id_apply, window, cx);
                                })),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new("modal-fav")
                                        .flex_1()
                                        .secondary()
                                        .rounded(ButtonRounded::Large)
                                        .icon(Icon::new(IconName::Heart))
                                        .label(if wp.is_favorite {
                                            "Favorited"
                                        } else {
                                            "Favorite"
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.toggle_favorite(&id_fav, Some(window), cx);
                                        })),
                                )
                                .child(if let Some(path) = local_path {
                                    Button::new("modal-reveal")
                                        .flex_1()
                                        .secondary()
                                        .rounded(ButtonRounded::Large)
                                        .icon(Icon::new(IconName::FolderOpen))
                                        .label("Reveal File")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.reveal_path(&path, window, cx);
                                        }))
                                } else {
                                    Button::new("modal-download")
                                        .flex_1()
                                        .secondary()
                                        .rounded(ButtonRounded::Large)
                                        .icon(Icon::new(IconName::Download))
                                        .label("Download")
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.download_wallpaper(&id_dl, false, window, cx);
                                        }))
                                }),
                        )
                        .when(wp.is_downloaded, |this| {
                            this.child(
                                Button::new("modal-delete")
                                    .small()
                                    .ghost()
                                    .rounded(ButtonRounded::Large)
                                    .icon(Icon::new(IconName::X))
                                    .label("Remove from Saved Files")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.remove_downloaded_wallpaper(&id_del, window, cx);
                                    })),
                            )
                        }),
                ),
        );

    let animated_dialog = EffectTransition::new(Duration::from_millis(220))
        .ease(ease_out_cubic)
        .fade(0.0, 1.0)
        .slide_y(px(16.0), px(0.0))
        .apply(dialog, "preview-modal-dialog");

    let backdrop = div()
        .id("modal-backdrop")
        .absolute()
        .inset_0()
        .bg(rgba(0x040508cc))
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .on_click(cx.listener(|this, _, window, cx| {
            this.close_preview(window, cx);
        }))
        .child(animated_dialog);

    EffectTransition::new(Duration::from_millis(160))
        .ease(ease_out_cubic)
        .fade(0.0, 1.0)
        .apply(backdrop, "preview-modal-backdrop")
}

fn spec_row(
    label: &'static str,
    value: String,
    mono_font: &SharedString,
    palette: ThemePalette,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .text_xs()
        .child(div().text_color(palette.text_muted).child(label))
        .child(
            div()
                .font_family(mono_font.clone())
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.text_primary)
                .child(value),
        )
}

fn color_chip(hex: u32, mono_font: &SharedString, palette: ThemePalette) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2p5()
        .py_1()
        .rounded_lg()
        .bg(palette.bg_surface)
        .border_1()
        .border_color(palette.hairline)
        .child(
            div()
                .size(px(14.0))
                .rounded_full()
                .bg(rgb(hex))
                .border_1()
                .border_color(rgba(0xffffff28)),
        )
        .child(
            div()
                .font_family(mono_font.clone())
                .text_xs()
                .text_color(palette.text_muted)
                .child(format!("#{:06X}", hex)),
        )
}
