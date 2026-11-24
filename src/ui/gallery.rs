use crate::app::WallzyApp;
use crate::models::{NavSection, Wallpaper};
use crate::theme::ThemePalette;
use crate::ui::card::render_card;
use gpui_kit::assets::IconName;
use gpui_kit::component::animation::{EffectTransition, ease_out_cubic};
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_kit::component::empty::{
    Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{Icon, Sizable as _, v_virtual_list};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::rc::Rc;
use std::time::Duration;

const HERO_ROW_HEIGHT: f32 = 236.0;
const CARD_ROW_GAP: f32 = 16.0;
const GRID_SIDE_CHROME: f32 = 232.0 + 56.0;

pub fn render_gallery(
    app: &WallzyApp,
    visible_wallpapers: Rc<Vec<Wallpaper>>,
    palette: ThemePalette,
    window: &Window,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    if visible_wallpapers.is_empty() {
        let (title, desc) = match app.active_nav {
            NavSection::Favorites => (
                "No favorites yet",
                "Click the heart icon on any wallpaper to save it to your favorites.",
            ),
            NavSection::MyWallpapers => (
                "No local wallpapers found",
                "Download wallpapers or add local folders in Settings to see them here.",
            ),
            _ => (
                "No wallpapers match your filters",
                "Try clearing active filters, changing your search query, or searching online.",
            ),
        };

        return div()
            .id("gallery-empty-container")
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Empty::new()
                    .header(
                        EmptyHeader::new()
                            .media(
                                EmptyMedia::new()
                                    .with_variant(EmptyMediaVariant::Icon)
                                    .child(Icon::new(IconName::Image)),
                            )
                            .title(EmptyTitle::new().child(title))
                            .description(EmptyDescription::new().child(desc)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("empty-reset-filters")
                                    .small()
                                    .secondary()
                                    .rounded(ButtonRounded::Large)
                                    .label("Reset Filters")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.clear_filters(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("empty-go-discover")
                                    .small()
                                    .primary()
                                    .rounded(ButtonRounded::Large)
                                    .label("Browse Discover")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.select_nav(NavSection::Discover, cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element();
    }

    let density = app.settings.grid_density;
    let show_badges = app.settings.show_resolution_badges;
    let mono_font = app.typography.mono_font.clone();

    let card_width = density.card_width();
    let card_height = density.thumb_height() + 26.0;
    let viewport_w: f32 = window.viewport_size().width.into();
    let avail_w = (viewport_w - GRID_SIDE_CHROME).max(card_width);
    let cols = ((avail_w + CARD_ROW_GAP) / (card_width + CARD_ROW_GAP))
        .floor()
        .max(1.0) as usize;

    let show_hero = app.active_nav == NavSection::Discover
        && app.filter.active_filter_count() == 0
        && visible_wallpapers.len() > 1;

    let grid_offset = if show_hero { 1 } else { 0 };
    let grid_count = visible_wallpapers.len().saturating_sub(grid_offset);
    let card_rows = grid_count.div_ceil(cols);
    let total_rows = grid_offset + card_rows;

    let mut row_sizes = Vec::with_capacity(total_rows);
    if show_hero {
        row_sizes.push(size(px(avail_w), px(HERO_ROW_HEIGHT)));
    }
    for row_ix in 0..card_rows {
        let trailing = if row_ix + 1 == card_rows {
            28.0
        } else {
            CARD_ROW_GAP
        };
        row_sizes.push(size(px(avail_w), px(card_height + trailing)));
    }

    let items_rc = visible_wallpapers;
    let list_el = v_virtual_list(
        cx.entity(),
        "wallpaper-vlist",
        Rc::new(row_sizes),
        move |_app, visible_range, _window, cx| {
            let mut rows = Vec::with_capacity(visible_range.len());
            for row_ix in visible_range {
                if show_hero && row_ix == 0 {
                    let hero_wp = items_rc[0].clone();
                    let hero = div().w_full().pb_5().child(render_hero_spotlight(
                        hero_wp,
                        mono_font.clone(),
                        palette,
                        cx,
                    ));
                    rows.push(hero.into_any_element());
                } else {
                    let card_row_ix = row_ix - grid_offset;
                    let start = grid_offset + card_row_ix * cols;
                    let end = (start + cols).min(items_rc.len());
                    let count_in_row = end - start;
                    let mut row_slots: Vec<AnyElement> = Vec::with_capacity(cols);
                    for wp in &items_rc[start..end] {
                        row_slots.push(
                            render_card(
                                wp.clone(),
                                density,
                                show_badges,
                                mono_font.clone(),
                                palette,
                                cx,
                            )
                            .into_any_element(),
                        );
                    }
                    // Pad incomplete final row with flex_1 spacers so all cards keep uniform width
                    for _ in count_in_row..cols {
                        row_slots.push(div().flex_1().min_w_0().into_any_element());
                    }
                    let row_el = div()
                        .w_full()
                        .pb_4()
                        .flex()
                        .items_center()
                        .gap_4()
                        .children(row_slots);
                    rows.push(row_el.into_any_element());
                }
            }
            rows
        },
    )
    .track_scroll(&app.gallery_scroll);

    image_cache(app.gallery_image_cache.clone())
        .flex_1()
        .min_h_0()
        .w_full()
        .child(
            div()
                .id("wallpaper-grid-shell")
                .relative()
                .size_full()
                .on_scroll(cx.listener(|this, event: &ScrollEvent, window, cx| {
                    this.handle_gallery_scroll(event, window, cx);
                }))
                .child(list_el)
                .vertical_scrollbar(&app.gallery_scroll),
        )
        .into_any_element()
}

fn render_hero_spotlight(
    wp: Wallpaper,
    mono_font: SharedString,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement + use<> {
    let id_preview = wp.id.clone();
    let id_apply = wp.id.clone();
    let id_dl = wp.id.clone();
    let image_path = wp.display_image_path().cloned();

    let hero = div()
        .id("hero-spotlight")
        .relative()
        .w_full()
        .h(px(216.0))
        .rounded_2xl()
        .overflow_hidden()
        .bg(rgb(wp.primary_hex))
        .border_1()
        .border_color(palette.hairline_strong)
        .shadow_lg()
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_dropdown = None;
            this.preview_id = Some(id_preview.clone());
            cx.notify();
        }))
        .when_some(image_path, |this, path| {
            this.child(
                div()
                    .absolute()
                    .inset_0()
                    .child(img(path).size_full().object_fit(ObjectFit::Cover)),
            )
        })
        // Editorial Dual-Axis Gradient Scrim
        .child(div().absolute().inset_0().bg(linear_gradient(
            90.,
            linear_color_stop(rgba(0x07080ce8), 0.),
            linear_color_stop(rgba(0x07080c30), 1.),
        )))
        .child(
            div()
                .absolute()
                .inset_0()
                .p_6()
                .flex()
                .items_end()
                .justify_between()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .max_w(px(520.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .px_2p5()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(palette.accent)
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(palette.text_on_accent)
                                        .child("FEATURED PICK"),
                                )
                                .child(
                                    div()
                                        .font_family(mono_font)
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(palette.glass_dark)
                                        .border_1()
                                        .border_color(palette.glass_border)
                                        .text_xs()
                                        .text_color(rgb(0xf8fafc))
                                        .child(format!(
                                            "{} · {}",
                                            wp.resolution_label(),
                                            wp.orientation.label()
                                        )),
                                ),
                        )
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xffffff))
                                .child(wp.name.clone()),
                        )
                        .child(div().text_sm().text_color(rgba(0xe2e8f0cc)).child(format!(
                                    "By {} · {} · {}",
                                    wp.author,
                                    wp.category,
                                    wp.tags
                                        .iter()
                                        .take(3)
                                        .map(|t| format!("#{}", t))
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                ))),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new("hero-apply")
                                .primary()
                                .rounded(ButtonRounded::Large)
                                .icon(Icon::new(IconName::Monitor))
                                .label("Set Wallpaper")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.apply_wallpaper(&id_apply, window, cx);
                                })),
                        )
                        .child(
                            div()
                                .id("hero-download")
                                .px_3p5()
                                .py_2()
                                .rounded_lg()
                                .bg(palette.glass_dark)
                                .border_1()
                                .border_color(palette.glass_border)
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(0xf8fafc))
                                .hover(|s| s.bg(rgba(0x0b0d12f0)))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.download_wallpaper(&id_dl, false, window, cx);
                                }))
                                .child(
                                    Icon::new(if wp.is_downloaded {
                                        IconName::Check
                                    } else {
                                        IconName::Download
                                    })
                                    .size(px(15.0)),
                                )
                                .child(if wp.is_downloaded { "Saved" } else { "Save" }),
                        ),
                ),
        );

    EffectTransition::new(Duration::from_millis(240))
        .ease(ease_out_cubic)
        .fade(0.0, 1.0)
        .slide_y(px(10.0), px(0.0))
        .apply(hero, "hero-spotlight-enter")
}
