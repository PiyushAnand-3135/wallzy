use crate::app::WallzyApp;
use crate::models::{Orientation, ResolutionPreset, SortOrder, WallpaperSource};
use crate::theme::ThemePalette;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::animation::{EffectTransition, ease_out_cubic};
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::time::Duration;

const CATEGORIES: &[&str] = &[
    "All",
    "Nature",
    "Architecture",
    "Minimal",
    "Abstract",
    "Space",
    "Local",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterDropdown {
    Resolution,
    Orientation,
    Source,
    Sort,
}

pub fn render_filter_bar(
    app: &WallzyApp,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let active_count = app.filter.active_filter_count();

    let res_label = match app.filter.resolution {
        ResolutionPreset::All => "Resolution",
        other => other.short_label(),
    };
    let orient_label = app
        .filter
        .orientation
        .map(|o| o.label())
        .unwrap_or("Aspect");
    let source_label = app.filter.source.map(|s| s.label()).unwrap_or("Source");
    let sort_label = app.filter.sort.label();

    div()
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .pb_1()
        // Left: Borderless Soft-Pill Category Tabs
        .child(div().flex().items_center().flex_wrap().gap_1().children(
            CATEGORIES.iter().copied().map(|cat| {
                let active = app.filter.category == cat;
                div()
                    .id(format!("cat-{}", cat))
                    .px_3()
                    .py_1p5()
                    .rounded_full()
                    .cursor_pointer()
                    .text_xs()
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .bg(if active {
                        palette.accent
                    } else {
                        rgba(0x00000000)
                    })
                    .when(active, |s| s.shadow_sm())
                    .text_color(if active {
                        palette.text_on_accent
                    } else {
                        palette.text_muted
                    })
                    .hover(|s| {
                        if active {
                            s
                        } else {
                            s.bg(palette.bg_hover).text_color(palette.text_primary)
                        }
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.filter.category = cat.to_string();
                        this.open_dropdown = None;
                        this.reset_gallery_scroll();
                        this.invalidate_visible_cache();
                        cx.notify();
                    }))
                    .child(cat)
            }),
        ))
        // Right: Compact Floating Dropdown Triggers + Clear
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(active_count > 0, |this| {
                    this.child(
                        Button::new("clear-filters")
                            .xsmall()
                            .ghost()
                            .rounded(ButtonRounded::Large)
                            .icon(Icon::new(IconName::X))
                            .label(format!("Reset ({})", active_count))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.clear_filters(window, cx);
                            })),
                    )
                })
                .child(render_dropdown_anchor(
                    app,
                    FilterDropdown::Resolution,
                    res_label,
                    app.filter.resolution != ResolutionPreset::All,
                    palette,
                    cx,
                ))
                .child(render_dropdown_anchor(
                    app,
                    FilterDropdown::Orientation,
                    orient_label,
                    app.filter.orientation.is_some(),
                    palette,
                    cx,
                ))
                .child(render_dropdown_anchor(
                    app,
                    FilterDropdown::Source,
                    source_label,
                    app.filter.source.is_some(),
                    palette,
                    cx,
                ))
                .child(render_dropdown_anchor(
                    app,
                    FilterDropdown::Sort,
                    sort_label,
                    false,
                    palette,
                    cx,
                )),
        )
}

fn render_dropdown_anchor(
    app: &WallzyApp,
    kind: FilterDropdown,
    label: &'static str,
    is_filtered: bool,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement + use<> {
    let is_open = app.open_dropdown == Some(kind);
    let icon = match kind {
        FilterDropdown::Sort => IconName::ArrowUpDown,
        FilterDropdown::Resolution => IconName::Maximize2,
        FilterDropdown::Orientation => IconName::AppWindow,
        FilterDropdown::Source => IconName::Layers,
    };

    div()
        .relative()
        .child(
            div()
                .id(format!("dropdown-trigger-{:?}", kind))
                .px_2p5()
                .py_1p5()
                .rounded_lg()
                .cursor_pointer()
                .flex()
                .items_center()
                .gap_1p5()
                .bg(if is_open || is_filtered {
                    palette.accent_soft
                } else {
                    palette.bg_surface
                })
                .border_1()
                .border_color(if is_open || is_filtered {
                    palette.accent
                } else {
                    palette.hairline
                })
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if is_open || is_filtered {
                    palette.text_primary
                } else {
                    palette.text_muted
                })
                .hover(|s| s.bg(palette.bg_elevated).text_color(palette.text_primary))
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.open_dropdown = if this.open_dropdown == Some(kind) {
                        None
                    } else {
                        Some(kind)
                    };
                    cx.notify();
                }))
                .child(Icon::new(icon).size(px(12.0)).text_color(if is_filtered {
                    palette.accent
                } else {
                    palette.text_subtle
                }))
                .child(label)
                .child(
                    Icon::new(IconName::ChevronDown)
                        .size(px(12.0))
                        .text_color(palette.text_subtle),
                ),
        )
        .when(is_open, |this| {
            this.child(deferred(render_dropdown_menu(app, kind, palette, cx)).with_priority(10))
        })
}

fn render_dropdown_menu(
    app: &WallzyApp,
    kind: FilterDropdown,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let mut items: Vec<AnyElement> = Vec::new();

    match kind {
        FilterDropdown::Resolution => {
            for res in ResolutionPreset::all().iter().copied() {
                let active = app.filter.resolution == res;
                let label = match res {
                    ResolutionPreset::All => "All Resolutions",
                    ResolutionPreset::Fhd => "1080p+ (Full HD)",
                    ResolutionPreset::Qhd => "1440p+ (2K QHD)",
                    ResolutionPreset::Uhd4k => "2160p+ (4K UHD)",
                    ResolutionPreset::Uhd5kPlus => "5K+ (Ultra HD)",
                };
                items.push(
                    menu_item(
                        format!("opt-res-{:?}", res),
                        label,
                        active,
                        palette,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.filter.resolution = res;
                            this.open_dropdown = None;
                            this.reset_gallery_scroll();
                            this.invalidate_visible_cache();
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
                );
            }
        }
        FilterDropdown::Orientation => {
            items.push(
                menu_item(
                    "opt-orient-all",
                    "Any Aspect Ratio",
                    app.filter.orientation.is_none(),
                    palette,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.filter.orientation = None;
                        this.open_dropdown = None;
                        this.reset_gallery_scroll();
                        this.invalidate_visible_cache();
                        cx.notify();
                    }),
                )
                .into_any_element(),
            );
            for orient in [
                Orientation::Landscape,
                Orientation::Ultrawide,
                Orientation::Portrait,
                Orientation::Square,
            ] {
                let active = app.filter.orientation == Some(orient);
                items.push(
                    menu_item(
                        format!("opt-orient-{:?}", orient),
                        orient.label(),
                        active,
                        palette,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.filter.orientation = Some(orient);
                            this.open_dropdown = None;
                            this.reset_gallery_scroll();
                            this.invalidate_visible_cache();
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
                );
            }
        }
        FilterDropdown::Source => {
            items.push(
                menu_item(
                    "opt-src-all",
                    "All Sources",
                    app.filter.source.is_none(),
                    palette,
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.filter.source = None;
                        this.open_dropdown = None;
                        this.reset_gallery_scroll();
                        this.invalidate_visible_cache();
                        cx.notify();
                    }),
                )
                .into_any_element(),
            );
            for source in WallpaperSource::all().iter().copied() {
                let active = app.filter.source == Some(source);
                items.push(
                    menu_item(
                        format!("opt-src-{:?}", source),
                        source.label(),
                        active,
                        palette,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.filter.source = Some(source);
                            this.open_dropdown = None;
                            this.reset_gallery_scroll();
                            this.invalidate_visible_cache();
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
                );
            }
        }
        FilterDropdown::Sort => {
            for sort in SortOrder::all().iter().copied() {
                let active = app.filter.sort == sort;
                items.push(
                    menu_item(
                        format!("opt-sort-{:?}", sort),
                        sort.label(),
                        active,
                        palette,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.filter.sort = sort;
                            this.open_dropdown = None;
                            this.reset_gallery_scroll();
                            this.invalidate_visible_cache();
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
                );
            }
        }
    }

    let menu = div()
        .id("filter-popover-menu")
        .absolute()
        .top_8()
        .right_0()
        .w(px(196.0))
        .p_1p5()
        .rounded_xl()
        .bg(palette.bg_popover)
        .border_1()
        .border_color(palette.hairline_strong)
        .shadow_xl()
        .flex()
        .flex_col()
        .gap_0p5()
        .on_click(|_, _, cx| cx.stop_propagation())
        .children(items);

    EffectTransition::new(Duration::from_millis(140))
        .ease(ease_out_cubic)
        .fade(0.0, 1.0)
        .slide_y(px(-6.0), px(0.0))
        .apply(menu, format!("dropdown-anim-{:?}", kind))
}

fn menu_item(
    id: impl Into<ElementId>,
    label: &'static str,
    active: bool,
    palette: ThemePalette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2p5()
        .py_1p5()
        .rounded_lg()
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_between()
        .bg(if active {
            palette.accent_soft
        } else {
            rgba(0x00000000)
        })
        .text_xs()
        .font_weight(if active {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        })
        .text_color(if active {
            palette.text_primary
        } else {
            palette.text_muted
        })
        .hover(|s| s.bg(palette.bg_hover).text_color(palette.text_primary))
        .on_click(on_click)
        .child(label)
        .when(active, |this| {
            this.child(
                Icon::new(IconName::Check)
                    .size(px(13.0))
                    .text_color(palette.accent),
            )
        })
}
