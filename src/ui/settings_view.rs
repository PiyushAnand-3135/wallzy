use crate::app::WallzyApp;
use crate::models::{AccentColor, GridDensity, ThemeMode, WallpaperFit, WallpaperSource};
use crate::theme::ThemePalette;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_kit::component::input::Input;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::switch::Switch;
use gpui_kit::*;

pub fn render_settings_view(
    app: &WallzyApp,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let download_dir = app.settings.download_dir.clone();
    let extra_dirs = app.settings.local_library_dirs.clone();
    let mono_font = app.typography.mono_font.clone();

    div()
        .id("settings-scroll")
        .flex_1()
        .min_h_0()
        .overflow_y_scrollbar()
        .child(
            div()
                .max_w(px(860.0))
                .pb_8()
                .flex()
                .flex_col()
                .gap_5()
                // 1. Appearance Section
                .child(settings_card(
                    "Appearance & Typography",
                    "Customize theme polarity, accent color, and gallery density.",
                    IconName::Palette,
                    palette,
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(setting_row(
                            "Active Font Stack",
                            "Resolved automatically from modern installed system fonts.",
                            div()
                                .px_2p5()
                                .py_1()
                                .rounded_lg()
                                .bg(palette.bg_elevated)
                                .border_1()
                                .border_color(palette.hairline)
                                .font_family(mono_font.clone())
                                .text_xs()
                                .text_color(palette.text_muted)
                                .child(format!(
                                    "{} / {}",
                                    app.typography.ui_font, app.typography.mono_font
                                )),
                            palette,
                        ))
                        .child(setting_row(
                            "Theme Mode",
                            "Follow OS appearance or force Light / Dark mode.",
                            div().flex().gap_1p5().children(ThemeMode::all().iter().copied().map(
                                |mode| {
                                    let active = app.settings.theme_mode == mode;
                                    option_pill(
                                        format!("set-theme-{}", mode.label()),
                                        mode.label(),
                                        active,
                                        palette,
                                        cx.listener(move |this, _, window, cx| {
                                            this.set_theme_mode(mode, window, cx);
                                        }),
                                    )
                                },
                            )),
                            palette,
                        ))
                        .child(setting_row(
                            "Accent Color",
                            "Highlight color used for active tabs, badges, and primary actions.",
                            div().flex().gap_2().children(AccentColor::all().iter().copied().map(
                                |accent| {
                                    let active = app.settings.accent_color == accent;
                                    div()
                                        .id(format!("set-accent-{}", accent.label()))
                                        .px_2p5()
                                        .py_1()
                                        .rounded_lg()
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .gap_1p5()
                                        .bg(if active {
                                            palette.accent_soft
                                        } else {
                                            palette.bg_elevated
                                        })
                                        .border_1()
                                        .border_color(if active {
                                            palette.accent
                                        } else {
                                            palette.hairline
                                        })
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.set_accent_color(accent, window, cx);
                                        }))
                                        .child(
                                            div()
                                                .size(px(10.0))
                                                .rounded_full()
                                                .bg(rgb(accent.dark_hex())),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(palette.text_primary)
                                                .child(accent.label()),
                                        )
                                },
                            )),
                            palette,
                        ))
                        .child(setting_row(
                            "Grid Density",
                            "Adjust card dimensions and visual breathing room in the gallery.",
                            div().flex().gap_1p5().children(
                                GridDensity::all().iter().copied().map(|density| {
                                    let active = app.settings.grid_density == density;
                                    option_pill(
                                        format!("set-density-{}", density.label()),
                                        density.label(),
                                        active,
                                        palette,
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_grid_density(density, cx);
                                        }),
                                    )
                                }),
                            ),
                            palette,
                        ))
                        .child(setting_row(
                            "Show Quality Badges",
                            "Display 4K / 5K / Saved indicators on wallpaper thumbnails.",
                            Switch::new("toggle-badges")
                                .checked(app.settings.show_resolution_badges)
                                .on_change(cx.listener(|this, checked, _, cx| {
                                    this.settings.show_resolution_badges = *checked;
                                    this.persist_settings();
                                    cx.notify();
                                })),
                            palette,
                        )),
                ))
                // 2. Storage & Local Library
                .child(settings_card(
                    "Storage & Local Library",
                    "Manage where downloaded wallpapers are stored and which local folders are indexed.",
                    IconName::HardDrive,
                    palette,
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(palette.text_primary)
                                        .child("Download Directory"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().flex_1().child(Input::new(&app.download_dir_input).small()))
                                        .child(
                                            Button::new("save-dl-dir")
                                                .small()
                                                .primary()
                                                .rounded(ButtonRounded::Large)
                                                .label("Save Path")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.apply_download_dir_input(window, cx);
                                                })),
                                        )
                                        .child(
                                            Button::new("open-dl-dir")
                                                .small()
                                                .secondary()
                                                .rounded(ButtonRounded::Large)
                                                .icon(Icon::new(IconName::FolderOpen))
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.reveal_path(&download_dir, window, cx);
                                                })),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(palette.text_primary)
                                        .child("Additional Local Folders"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(div().flex_1().child(Input::new(&app.local_dir_input).small()))
                                        .child(
                                            Button::new("add-local-dir")
                                                .small()
                                                .secondary()
                                                .rounded(ButtonRounded::Large)
                                                .icon(Icon::new(IconName::FolderPlus))
                                                .label("Add Folder")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.add_local_library_dir(window, cx);
                                                })),
                                        ),
                                )
                                .child(
                                    div().flex().flex_wrap().gap_1p5().children(
                                        extra_dirs.into_iter().enumerate().map(|(idx, dir)| {
                                            div()
                                                .id(("extra-dir", idx))
                                                .px_2p5()
                                                .py_1()
                                                .rounded_lg()
                                                .bg(palette.bg_elevated)
                                                .border_1()
                                                .border_color(palette.hairline)
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .font_family(mono_font.clone())
                                                .text_xs()
                                                .text_color(palette.text_primary)
                                                .child(dir.to_string_lossy().to_string())
                                                .child(
                                                    div()
                                                        .id(("rm-extra-dir", idx))
                                                        .cursor_pointer()
                                                        .text_color(palette.text_muted)
                                                        .hover(|s| s.text_color(palette.danger))
                                                        .on_click(cx.listener(
                                                            move |this, _, window, cx| {
                                                                this.remove_local_library_dir(
                                                                    idx, window, cx,
                                                                );
                                                            },
                                                        ))
                                                        .child(Icon::new(IconName::X).size(px(12.0))),
                                                )
                                        }),
                                    ),
                                ),
                        ),
                ))
                // 3. Desktop Wallpaper Engine
                .child(settings_card(
                    "Wallpaper Engine",
                    "Control how wallpapers are scaled and applied to your desktop.",
                    IconName::Monitor,
                    palette,
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(setting_row(
                            "Default Fit Mode",
                            "Scaling mode applied when setting a desktop background.",
                            div().flex().gap_1p5().children(WallpaperFit::all().iter().copied().map(
                                |fit| {
                                    let active = app.settings.wallpaper_fit == fit;
                                    option_pill(
                                        format!("set-fit-{}", fit.label()),
                                        fit.label(),
                                        active,
                                        palette,
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_wallpaper_fit(fit, cx);
                                        }),
                                    )
                                },
                            )),
                            palette,
                        ))
                        .child(setting_row(
                            "Auto-Apply After Download",
                            "Automatically set a wallpaper on your desktop as soon as its download finishes.",
                            Switch::new("toggle-auto-apply")
                                .checked(app.settings.auto_apply_after_download)
                                .on_change(cx.listener(|this, checked, _, cx| {
                                    this.settings.auto_apply_after_download = *checked;
                                    this.persist_settings();
                                    cx.notify();
                                })),
                            palette,
                        )),
                ))
                // 4. Built-in & Custom Providers
                .child(settings_card(
                    "Wallpaper Providers",
                    "Toggle built-in sources, configure API keys, or add your own HTTP JSON wallpaper feeds.",
                    IconName::Globe,
                    palette,
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .pb_2()
                                .border_b_1()
                                .border_color(palette.hairline)
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(palette.text_muted)
                                        .child("BUILT-IN & ONLINE SOURCES"),
                                )
                                .child(
                                    Button::new("settings-sync-providers")
                                        .small()
                                        .secondary()
                                        .rounded(ButtonRounded::Large)
                                        .icon(Icon::new(IconName::RefreshCw))
                                        .label(if app.is_fetching {
                                            "Syncing..."
                                        } else {
                                            "Sync Enabled Sources"
                                        })
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.refresh_catalog(Some(window), cx);
                                        })),
                                ),
                        )
                        .child(setting_row(
                            "Wallzy Curated",
                            "Built-in atmospheric procedural wallpapers ready offline.",
                            Switch::new("toggle-curated")
                                .checked(app.settings.enable_curated)
                                .on_change(cx.listener(|this, checked, _, cx| {
                                    this.toggle_offline_provider(true, *checked, cx);
                                })),
                            palette,
                        ))
                        .child(setting_row(
                            "Local Library",
                            "Index images from your download folder and added local directories.",
                            Switch::new("toggle-local")
                                .checked(app.settings.enable_local)
                                .on_change(cx.listener(|this, checked, _, cx| {
                                    this.toggle_offline_provider(false, *checked, cx);
                                })),
                            palette,
                        ))
                        .child(setting_row(
                            "Bing Daily Wallpapers",
                            "Fetch daily UHD photography from Bing's image archive.",
                            Switch::new("toggle-bing")
                                .checked(app.settings.enable_bing)
                                .on_change(cx.listener(|this, checked, window, cx| {
                                    this.toggle_online_provider(
                                        WallpaperSource::Bing,
                                        *checked,
                                        window,
                                        cx,
                                    );
                                })),
                            palette,
                        ))
                        .child(setting_row(
                            "Alpha Coders (Wallpaper Abyss)",
                            "Fetch curated high-rated wallpapers (API key unlocks full search & sorting).",
                            Switch::new("toggle-alphacoders")
                                .checked(app.settings.enable_alpha_coders)
                                .on_change(cx.listener(|this, checked, window, cx| {
                                    this.toggle_online_provider(
                                        WallpaperSource::AlphaCoders,
                                        *checked,
                                        window,
                                        cx,
                                    );
                                })),
                            palette,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(palette.text_muted)
                                        .child("Alpha Coders API Key (Optional · api.alphacoders.com)"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div().flex_1().child(
                                                Input::new(&app.alpha_coders_key_input)
                                                    .small()
                                                    .mask_toggle(),
                                            ),
                                        )
                                        .child(
                                            Button::new("save-ac-key")
                                                .small()
                                                .secondary()
                                                .rounded(ButtonRounded::Large)
                                                .label("Save Key")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.apply_alpha_coders_key_input(window, cx);
                                                })),
                                        ),
                                ),
                        )
                        .child(setting_row(
                            "Reddit (r/wallpapers & r/MinimalWallpaper)",
                            "Fetch SFW community wallpaper posts via Reddit's public JSON feed.",
                            Switch::new("toggle-reddit")
                                .checked(app.settings.enable_reddit)
                                .on_change(cx.listener(|this, checked, window, cx| {
                                    this.toggle_online_provider(
                                        WallpaperSource::Reddit,
                                        *checked,
                                        window,
                                        cx,
                                    );
                                })),
                            palette,
                        ))
                        .child(setting_row(
                            "Wikimedia Commons",
                            "Browse high-resolution Creative Commons & public-domain featured pictures.",
                            Switch::new("toggle-wikimedia")
                                .checked(app.settings.enable_wikimedia)
                                .on_change(cx.listener(|this, checked, window, cx| {
                                    this.toggle_online_provider(
                                        WallpaperSource::Wikimedia,
                                        *checked,
                                        window,
                                        cx,
                                    );
                                })),
                            palette,
                        ))
                        .child(setting_row(
                            "NASA Image Library",
                            "Discover public-domain space, nebula, and Earth imagery from NASA.",
                            Switch::new("toggle-nasa")
                                .checked(app.settings.enable_nasa)
                                .on_change(cx.listener(|this, checked, window, cx| {
                                    this.toggle_online_provider(
                                        WallpaperSource::Nasa,
                                        *checked,
                                        window,
                                        cx,
                                    );
                                })),
                            palette,
                        ))
                        .child(setting_row(
                            "Wallhaven",
                            "Query Wallhaven.cc SFW community wallpapers when searching online.",
                            Switch::new("toggle-wallhaven")
                                .checked(app.settings.enable_wallhaven)
                                .on_change(cx.listener(|this, checked, window, cx| {
                                    this.toggle_online_provider(
                                        WallpaperSource::Wallhaven,
                                        *checked,
                                        window,
                                        cx,
                                    );
                                })),
                            palette,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(palette.text_muted)
                                        .child("Wallhaven API Key (Optional)"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div().flex_1().child(
                                                Input::new(&app.wallhaven_key_input)
                                                    .small()
                                                    .mask_toggle(),
                                            ),
                                        )
                                        .child(
                                            Button::new("save-wh-key")
                                                .small()
                                                .secondary()
                                                .rounded(ButtonRounded::Large)
                                                .label("Save Key")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.apply_wallhaven_key_input(window, cx);
                                                })),
                                        ),
                                ),
                        )
                        // Custom Providers Manager
                        .child(
                            div()
                                .pt_3()
                                .border_t_1()
                                .border_color(palette.hairline)
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text_primary)
                                                .child("Custom JSON Providers"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(palette.text_muted)
                                                .child("Add any HTTP JSON wallpaper feed or API endpoint. Use {query} in the URL to pass search terms."),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w(px(176.0))
                                                .child(Input::new(&app.custom_provider_name_input).small()),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .child(Input::new(&app.custom_provider_url_input).small()),
                                        )
                                        .child(
                                            Button::new("add-custom-provider")
                                                .small()
                                                .primary()
                                                .rounded(ButtonRounded::Large)
                                                .icon(Icon::new(IconName::Plus))
                                                .label("Add Source")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.add_custom_provider(window, cx);
                                                })),
                                        ),
                                )
                                .children(app.settings.custom_providers.iter().enumerate().map(
                                    |(idx, custom)| {
                                        let enabled = custom.enabled;
                                        div()
                                            .id(("custom-prov-row", idx))
                                            .p_3()
                                            .rounded_xl()
                                            .bg(palette.bg_elevated)
                                            .border_1()
                                            .border_color(palette.hairline)
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_0p5()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(palette.text_primary)
                                                            .truncate()
                                                            .child(custom.name.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family(mono_font.clone())
                                                            .text_xs()
                                                            .text_color(palette.text_muted)
                                                            .truncate()
                                                            .child(custom.endpoint_url.clone()),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        Switch::new(("toggle-custom", idx))
                                                            .checked(enabled)
                                                            .on_change(cx.listener(
                                                                move |this, checked, window, cx| {
                                                                    this.toggle_custom_provider(
                                                                        idx, *checked, window, cx,
                                                                    );
                                                                },
                                                            )),
                                                    )
                                                    .child(
                                                        Button::new(("rm-custom", idx))
                                                            .xsmall()
                                                            .ghost()
                                                            .rounded(ButtonRounded::Large)
                                                            .icon(Icon::new(IconName::X))
                                                            .on_click(cx.listener(
                                                                move |this, _, window, cx| {
                                                                    this.remove_custom_provider(
                                                                        idx, window, cx,
                                                                    );
                                                                },
                                                            )),
                                                    ),
                                            )
                                    },
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .flex_wrap()
                                .gap_1p5()
                                .children(
                                    crate::providers::registered_providers(
                                        crate::services::storage::preview_cache_dir(),
                                        app.settings.local_library_dirs.clone(),
                                    )
                                    .into_iter()
                                    .map(|(id, name)| {
                                        div()
                                            .px_2p5()
                                            .py_1()
                                            .rounded_full()
                                            .bg(palette.bg_elevated)
                                            .border_1()
                                            .border_color(palette.hairline)
                                            .text_xs()
                                            .text_color(palette.text_muted)
                                            .child(format!("{} · {}", name, id))
                                    }),
                                ),
                        ),
                ))
                // 5. Reset & Open Source Info
                .child(settings_card(
                    "Data & Open Source",
                    "Wallzy is open-source software built with Rust and GPUI Kit.",
                    IconName::RotateCcw,
                    palette,
                    setting_row(
                        "Reset Application State",
                        "Restore default settings and clear saved favorites & download history (files on disk are kept).",
                        Button::new("reset-app-state")
                            .small()
                            .danger()
                            .rounded(ButtonRounded::Large)
                            .label("Reset to Defaults")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reset_app_state(window, cx);
                            })),
                        palette,
                    ),
                )),
        )
}

fn settings_card(
    title: &'static str,
    subtitle: &'static str,
    icon: IconName,
    palette: ThemePalette,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .p_5()
        .rounded_2xl()
        .bg(palette.bg_surface)
        .border_1()
        .border_color(palette.hairline)
        .shadow_sm()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .size(px(34.0))
                        .rounded_xl()
                        .bg(palette.accent_soft)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(icon).size(px(16.0)).text_color(palette.accent)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(palette.text_primary)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.text_muted)
                                .child(subtitle),
                        ),
                ),
        )
        .child(content)
}

fn setting_row(
    title: &'static str,
    description: &'static str,
    control: impl IntoElement,
    palette: ThemePalette,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(palette.text_primary)
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(palette.text_muted)
                        .child(description),
                ),
        )
        .child(control)
}

fn option_pill(
    id: impl Into<ElementId>,
    label: &'static str,
    active: bool,
    palette: ThemePalette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
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
            palette.bg_elevated
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
        .hover(|s| {
            if active {
                s
            } else {
                s.text_color(palette.text_primary)
            }
        })
        .on_click(on_click)
        .child(label)
}
