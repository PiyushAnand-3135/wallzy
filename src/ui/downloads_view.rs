use crate::app::WallzyApp;
use crate::models::{DownloadItem, DownloadStatus};
use crate::theme::ThemePalette;
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonRounded, ButtonVariants as _};
use gpui_kit::component::empty::{
    Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::progress::Progress;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub fn render_downloads_view(
    app: &WallzyApp,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement {
    let active_count = app
        .downloads
        .iter()
        .filter(|d| d.status.is_active())
        .count();
    let completed_count = app
        .downloads
        .iter()
        .filter(|d| d.status.is_completed())
        .count();
    let download_dir = app.settings.download_dir.clone();
    let mono_font = app.typography.mono_font.clone();

    let mut rows = Vec::with_capacity(app.downloads.len());
    for item in app.downloads.iter().cloned() {
        rows.push(render_download_row(item, mono_font.clone(), palette, cx));
    }

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap_4()
        // Summary Banner
        .child(
            div()
                .p_4()
                .rounded_2xl()
                .bg(palette.bg_surface)
                .border_1()
                .border_color(palette.hairline)
                .shadow_sm()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_6()
                        .child(stat_block(
                            "Completed",
                            completed_count.to_string(),
                            &mono_font,
                            palette,
                        ))
                        .child(stat_block(
                            "Active Queue",
                            active_count.to_string(),
                            &mono_font,
                            palette,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(palette.text_subtle)
                                        .child("Destination Folder"),
                                )
                                .child(
                                    div()
                                        .font_family(mono_font.clone())
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(palette.text_primary)
                                        .child(download_dir.to_string_lossy().to_string()),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new("dl-open-folder")
                                .small()
                                .secondary()
                                .rounded(ButtonRounded::Large)
                                .icon(Icon::new(IconName::FolderOpen))
                                .label("Open Folder")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.reveal_path(&download_dir, window, cx);
                                })),
                        )
                        .when(completed_count > 0, |this| {
                            this.child(
                                Button::new("dl-clear-history")
                                    .small()
                                    .ghost()
                                    .rounded(ButtonRounded::Large)
                                    .label("Clear History")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.clear_completed_downloads(cx);
                                    })),
                            )
                        }),
                ),
        )
        // Download Items List or Empty State
        .child(if app.downloads.is_empty() {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Empty::new().header(
                        EmptyHeader::new()
                            .media(
                                EmptyMedia::new()
                                    .with_variant(EmptyMediaVariant::Icon)
                                    .child(Icon::new(IconName::Download)),
                            )
                            .title(EmptyTitle::new().child("No downloads yet"))
                            .description(EmptyDescription::new().child(
                                "Download any wallpaper from Discover or Trending to manage it here.",
                            )),
                    ),
                )
                .into_any_element()
        } else {
            div()
                .id("downloads-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(div().pb_6().flex().flex_col().gap_2p5().children(rows))
                .into_any_element()
        })
}

fn stat_block(
    label: &'static str,
    value: String,
    mono_font: &SharedString,
    palette: ThemePalette,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(div().text_xs().text_color(palette.text_subtle).child(label))
        .child(
            div()
                .font_family(mono_font.clone())
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(palette.text_primary)
                .child(value),
        )
}

fn render_download_row(
    item: DownloadItem,
    mono_font: SharedString,
    palette: ThemePalette,
    cx: &mut Context<WallzyApp>,
) -> impl IntoElement + use<> {
    let wp_id_apply = item.wallpaper_id.clone();
    let wp_id_retry = item.wallpaper_id.clone();
    let dest_reveal = item.destination.clone();

    let status_color = match &item.status {
        DownloadStatus::Completed => palette.success,
        DownloadStatus::Failed(_) => palette.danger,
        _ => palette.accent,
    };

    let size_text = item
        .file_size_bytes
        .map(|b| format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)))
        .unwrap_or_else(|| "—".to_string());

    div()
        .p_3p5()
        .rounded_xl()
        .bg(palette.bg_surface)
        .border_1()
        .border_color(palette.hairline)
        .shadow_sm()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3p5()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .w(px(72.0))
                        .h(px(46.0))
                        .rounded_lg()
                        .overflow_hidden()
                        .bg(rgb(item.preview_hex))
                        .border_1()
                        .border_color(palette.hairline)
                        .when(item.destination.exists(), |this| {
                            this.child(
                                img(item.destination.clone())
                                    .size_full()
                                    .object_fit(ObjectFit::Cover),
                            )
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(palette.text_primary)
                                        .truncate()
                                        .child(item.wallpaper_name.clone()),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(palette.bg_elevated)
                                        .text_xs()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(status_color)
                                        .child(item.status.label().to_string()),
                                ),
                        )
                        .child(
                            div()
                                .font_family(mono_font)
                                .text_xs()
                                .text_color(palette.text_muted)
                                .truncate()
                                .child(match &item.status {
                                    DownloadStatus::Failed(err) => err.clone(),
                                    _ => format!(
                                        "{} · {} · {}",
                                        item.resolution,
                                        size_text,
                                        item.destination.to_string_lossy()
                                    ),
                                }),
                        )
                        .when(item.status.is_active(), |this| {
                            this.child(
                                Progress::new(format!("dl-progress-{}", item.wallpaper_id))
                                    .xsmall()
                                    .loading(true)
                                    .value(item.progress_percent),
                            )
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .when(item.status.is_completed(), |this| {
                    this.child(
                        Button::new(format!("dl-apply-{}", wp_id_apply))
                            .small()
                            .primary()
                            .rounded(ButtonRounded::Large)
                            .icon(Icon::new(IconName::Monitor))
                            .label("Set Wallpaper")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.apply_wallpaper(&wp_id_apply, window, cx);
                            })),
                    )
                    .child(
                        Button::new(format!("dl-reveal-{}", item.wallpaper_id))
                            .small()
                            .secondary()
                            .rounded(ButtonRounded::Large)
                            .icon(Icon::new(IconName::FolderOpen))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.reveal_path(&dest_reveal, window, cx);
                            })),
                    )
                })
                .when(matches!(item.status, DownloadStatus::Failed(_)), |this| {
                    this.child(
                        Button::new(format!("dl-retry-{}", wp_id_retry))
                            .small()
                            .secondary()
                            .rounded(ButtonRounded::Large)
                            .icon(Icon::new(IconName::RefreshCw))
                            .label("Retry")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.download_wallpaper(&wp_id_retry, false, window, cx);
                            })),
                    )
                }),
        )
}
