use crate::models::{
    AccentColor, AppSettings, DownloadItem, DownloadStatus, GridDensity, NavSection, SortOrder,
    ThemeMode, Wallpaper, WallpaperFilter, WallpaperFit, WallpaperSource,
};
use crate::providers::curated::CuratedProvider;
use crate::providers::local::LocalProvider;
use crate::providers::{ProviderQuery, WallpaperProvider, fetch_enabled_online};
use crate::services::downloader::{execute_download, target_path_for};
use crate::services::storage::{
    LibraryState, load_library_state, load_settings, preview_cache_dir, reset_all_persisted_data,
    save_library_state, save_settings,
};
use crate::services::wallpaper_setter::{reveal_in_file_manager, set_desktop_wallpaper};
use crate::theme::{ResolvedTypography, ThemePalette};
use crate::ui::{
    downloads_view::render_downloads_view,
    filter_bar::{FilterDropdown, render_filter_bar},
    gallery::render_gallery,
    preview_modal::render_preview_modal,
    settings_view::render_settings_view,
    sidebar::render_sidebar,
    topbar::render_topbar,
};
use gpui_kit::assets::IconName;
use gpui_kit::component::animation::{EffectTransition, ease_out_cubic};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::scroll::ScrollbarHandle as _;
use gpui_kit::component::{Icon, TitleBar, VirtualListScrollHandle, WindowExt as _};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

/// Root GPUI view and application state coordinator for Wallzy.
pub struct WallzyApp {
    pub active_nav: NavSection,
    pub settings: AppSettings,
    pub typography: ResolvedTypography,
    pub filter: WallpaperFilter,
    pub wallpapers: Vec<Wallpaper>,
    pub downloads: Vec<DownloadItem>,
    pub preview_id: Option<String>,
    pub open_dropdown: Option<FilterDropdown>,
    pub is_fetching: bool,
    pub gallery_image_cache: Entity<RetainAllImageCache>,
    pub gallery_scroll: VirtualListScrollHandle,
    cached_visible: Rc<Vec<Wallpaper>>,
    smooth_scroll_y: f32,
    scroll_target_y: f32,
    scroll_animating: bool,

    favorite_ids: HashSet<String>,
    downloaded_paths: HashMap<String, PathBuf>,

    pub search_input: Entity<InputState>,
    pub download_dir_input: Entity<InputState>,
    pub local_dir_input: Entity<InputState>,
    pub wallhaven_key_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl WallzyApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = load_settings();
        let library = load_library_state();
        let typography = ResolvedTypography::detect(cx);

        ThemePalette::sync_component_theme(
            settings.theme_mode,
            settings.accent_color,
            &typography,
            window,
            cx,
        );

        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search wallpapers, authors, or #tags...")
        });
        let download_dir_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Download folder path");
            state.set_value(
                settings.download_dir.to_string_lossy().to_string(),
                window,
                cx,
            );
            state
        });
        let local_dir_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Add local wallpaper folder path...")
        });
        let wallhaven_key_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Optional Wallhaven API key");
            if !settings.wallhaven_api_key.is_empty() {
                state.set_value(settings.wallhaven_api_key.clone(), window, cx);
            }
            state
        });

        let search_sub = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    this.filter.query = state.read(cx).value().to_string();
                    this.invalidate_visible_cache();
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    this.filter.query = state.read(cx).value().to_string();
                    if this.settings.has_any_online_provider()
                        && !this.filter.query.trim().is_empty()
                    {
                        this.fetch_online_providers(Some(window), cx);
                    }
                    cx.notify();
                }
                _ => {}
            },
        );

        let gallery_image_cache = RetainAllImageCache::new(cx);

        let mut app = Self {
            active_nav: NavSection::Discover,
            settings,
            typography,
            filter: WallpaperFilter::new(),
            wallpapers: Vec::new(),
            downloads: library.download_history,
            preview_id: None,
            open_dropdown: None,
            is_fetching: false,
            gallery_image_cache,
            gallery_scroll: VirtualListScrollHandle::new(),
            cached_visible: Rc::new(Vec::new()),
            smooth_scroll_y: 0.0,
            scroll_target_y: 0.0,
            scroll_animating: false,
            favorite_ids: library.favorite_ids,
            downloaded_paths: library.downloaded_paths,
            search_input,
            download_dir_input,
            local_dir_input,
            wallhaven_key_input,
            _subscriptions: vec![search_sub],
        };

        app.reload_offline_catalog();
        app
    }

    /// Loads built-in curated wallpapers and scans configured local directories.
    pub fn reload_offline_catalog(&mut self) {
        let curated = CuratedProvider::new(preview_cache_dir())
            .fetch(&ProviderQuery::default())
            .unwrap_or_default();

        let mut local_dirs = vec![self.settings.download_dir.clone()];
        local_dirs.extend(self.settings.local_library_dirs.iter().cloned());
        let local = LocalProvider::new(local_dirs)
            .with_preview_cache(preview_cache_dir())
            .fetch(&ProviderQuery::default())
            .unwrap_or_default();

        // Keep any previously fetched remote wallpapers when refreshing local/curated items.
        let remote: Vec<Wallpaper> = self
            .wallpapers
            .iter()
            .filter(|w| w.source.is_remote())
            .cloned()
            .collect();

        let mut merged = Vec::with_capacity(curated.len() + local.len() + remote.len());
        merged.extend(curated);
        merged.extend(remote);
        merged.extend(local);

        self.wallpapers = merged;
        self.apply_persisted_flags();
        self.invalidate_visible_cache();
    }

    pub fn invalidate_visible_cache(&mut self) {
        self.cached_visible = Rc::new(self.filtered_wallpapers());
    }

    fn apply_persisted_flags(&mut self) {
        for wp in &mut self.wallpapers {
            wp.is_favorite = self.favorite_ids.contains(&wp.id);
            if let Some(path) = self.downloaded_paths.get(&wp.id)
                && path.exists()
            {
                wp.is_downloaded = true;
                wp.local_path = Some(path.clone());
                if let Ok(meta) = fs::metadata(path) {
                    wp.file_size_bytes = Some(meta.len());
                }
            }
        }
    }

    pub fn filtered_wallpapers(&self) -> Vec<Wallpaper> {
        let mut items: Vec<Wallpaper> = self
            .wallpapers
            .iter()
            .filter(|w| match self.active_nav {
                NavSection::Favorites => w.is_favorite,
                NavSection::MyWallpapers => w.is_downloaded || w.source == WallpaperSource::Local,
                _ => true,
            })
            .filter(|w| self.filter.matches(w))
            .cloned()
            .collect();

        let effective_sort = match (self.active_nav, self.filter.sort) {
            (NavSection::Trending, SortOrder::Featured) => SortOrder::Trending,
            (NavSection::Latest, SortOrder::Featured) => SortOrder::Newest,
            (_, sort) => sort,
        };

        match effective_sort {
            SortOrder::Featured => items.sort_by(|a, b| {
                b.is_favorite
                    .cmp(&a.is_favorite)
                    .then(b.popularity.cmp(&a.popularity))
                    .then(a.name.cmp(&b.name))
            }),
            SortOrder::Trending => items.sort_by_key(|a| std::cmp::Reverse(a.popularity)),
            SortOrder::Newest => items.sort_by_key(|a| std::cmp::Reverse(a.created_order)),
            SortOrder::ResolutionDesc => items.sort_by_key(|a| std::cmp::Reverse(a.pixel_count())),
            SortOrder::NameAsc => items.sort_by(|a, b| a.name.cmp(&b.name)),
        }

        items
    }

    pub fn select_nav(&mut self, section: NavSection, cx: &mut Context<Self>) {
        self.active_nav = section;
        self.open_dropdown = None;
        self.reset_gallery_scroll();
        self.invalidate_visible_cache();
        cx.notify();
    }

    pub fn reset_gallery_scroll(&mut self) {
        self.smooth_scroll_y = 0.0;
        self.scroll_target_y = 0.0;
        self.scroll_animating = false;
        self.gallery_scroll.set_offset(point(px(0.), px(0.)));
    }

    /// Interpolates discrete mouse-wheel and trackpad scroll events at display refresh rate.
    pub fn handle_gallery_scroll(
        &mut self,
        event: &ScrollEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let content_h: f32 = self.gallery_scroll.content_size().height.into();
        let viewport_h: f32 = self
            .gallery_scroll
            .base_handle()
            .bounds()
            .size
            .height
            .into();
        let max_extent = (content_h - viewport_h).max(0.0);
        if max_extent <= 0.0 {
            return;
        }

        let raw_delta: f32 = event.delta.pixel_delta(window.line_height()).y.into();
        if raw_delta.abs() < 0.01 {
            return;
        }

        let current_raw_y: f32 = self.gallery_scroll.base_handle().offset().y.into();
        if !self.scroll_animating {
            self.smooth_scroll_y = (current_raw_y - raw_delta).clamp(-max_extent, 0.0);
            self.scroll_target_y = self.smooth_scroll_y;
        }

        let boost = match event.delta {
            ScrollDelta::Lines(_) => 1.85,
            ScrollDelta::Pixels(_) => 1.0,
        };
        self.scroll_target_y = (self.scroll_target_y + raw_delta * boost).clamp(-max_extent, 0.0);
        self.gallery_scroll
            .set_offset(point(px(0.), px(self.smooth_scroll_y)));

        if !self.scroll_animating {
            self.scroll_animating = true;
            self.schedule_smooth_scroll_frame(window, cx);
        }
        cx.notify();
    }

    fn schedule_smooth_scroll_frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let Some(entity) = entity.upgrade() else {
                return;
            };
            entity.update(cx, |this, cx| {
                if !this.scroll_animating {
                    return;
                }
                let content_h: f32 = this.gallery_scroll.content_size().height.into();
                let viewport_h: f32 = this
                    .gallery_scroll
                    .base_handle()
                    .bounds()
                    .size
                    .height
                    .into();
                let max_extent = (content_h - viewport_h).max(0.0);
                this.scroll_target_y = this.scroll_target_y.clamp(-max_extent, 0.0);

                let diff = this.scroll_target_y - this.smooth_scroll_y;
                if diff.abs() < 0.5 {
                    this.smooth_scroll_y = this.scroll_target_y;
                    this.scroll_animating = false;
                } else {
                    this.smooth_scroll_y += diff * 0.24;
                    this.schedule_smooth_scroll_frame(window, cx);
                }

                this.gallery_scroll
                    .set_offset(point(px(0.), px(this.smooth_scroll_y)));
                cx.notify();
            });
        });
    }

    pub fn set_theme_mode(&mut self, mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.theme_mode = mode;
        ThemePalette::sync_component_theme(
            mode,
            self.settings.accent_color,
            &self.typography,
            window,
            cx,
        );
        self.persist_settings();
        cx.notify();
    }

    pub fn set_accent_color(
        &mut self,
        accent: AccentColor,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings.accent_color = accent;
        ThemePalette::sync_component_theme(
            self.settings.theme_mode,
            accent,
            &self.typography,
            window,
            cx,
        );
        self.persist_settings();
        cx.notify();
    }

    pub fn set_grid_density(&mut self, density: GridDensity, cx: &mut Context<Self>) {
        self.settings.grid_density = density;
        self.persist_settings();
        cx.notify();
    }

    pub fn set_wallpaper_fit(&mut self, fit: WallpaperFit, cx: &mut Context<Self>) {
        self.settings.wallpaper_fit = fit;
        self.persist_settings();
        cx.notify();
    }

    pub fn set_search_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.filter.query = query.to_string();
        self.open_dropdown = None;
        self.reset_gallery_scroll();
        self.search_input.update(cx, |state, cx| {
            state.set_value(query.to_string(), window, cx);
        });
        self.invalidate_visible_cache();
        cx.notify();
    }

    pub fn clear_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.filter.reset();
        self.open_dropdown = None;
        self.reset_gallery_scroll();
        self.search_input.update(cx, |state, cx| {
            state.set_value(String::new(), window, cx);
        });
        self.invalidate_visible_cache();
        cx.notify();
    }

    pub fn toggle_favorite(
        &mut self,
        wallpaper_id: &str,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let mut added = false;
        let mut title = String::new();
        if let Some(wp) = self.wallpapers.iter_mut().find(|w| w.id == wallpaper_id) {
            wp.is_favorite = !wp.is_favorite;
            added = wp.is_favorite;
            title = wp.name.clone();
            if added {
                self.favorite_ids.insert(wp.id.clone());
            } else {
                self.favorite_ids.remove(&wp.id);
            }
        }
        self.persist_library_state();
        self.invalidate_visible_cache();
        if let Some(win) = window
            && !title.is_empty()
        {
            let msg = if added {
                format!("Added \"{}\" to Favorites", title)
            } else {
                format!("Removed \"{}\" from Favorites", title)
            };
            win.push_notification(Notification::info(msg), cx);
        }
        cx.notify();
    }

    pub fn refresh_catalog(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        self.reload_offline_catalog();
        if self.settings.has_any_online_provider() {
            self.fetch_online_providers(window, cx);
        } else if let Some(win) = window {
            win.push_notification(Notification::success("Local library rescanned"), cx);
        }
        cx.notify();
    }

    pub fn fetch_online_providers(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        if self.is_fetching || !self.settings.has_any_online_provider() {
            return;
        }
        self.is_fetching = true;
        cx.notify();

        let query = ProviderQuery {
            search: self.filter.query.clone(),
            section: Some(self.active_nav),
            api_key: self.settings.wallhaven_api_key.clone(),
            page: 1,
        };
        let settings = self.settings.clone();
        let cache_dir = preview_cache_dir();
        let win_handle = window.map(|w| w.window_handle());

        cx.spawn(async move |this, cx| {
            let (mut remote_items, errors) = cx
                .background_executor()
                .spawn(async move {
                    let (mut items, errors) = fetch_enabled_online(&settings, &query);
                    std::thread::scope(|scope| {
                        let cache_ref = &cache_dir;
                        let jobs: Vec<_> = items
                            .iter_mut()
                            .map(|item| {
                                scope.spawn(move || {
                                    if let Some(thumb) = item.thumb_url.as_deref() {
                                        item.preview_path =
                                            fetch_cached_thumb(&item.id, thumb, cache_ref);
                                    }
                                })
                            })
                            .collect();
                        for job in jobs {
                            let _ = job.join();
                        }
                    });
                    (items, errors)
                })
                .await;

            let _ = this.update(cx, |app, cx| {
                app.is_fetching = false;
                let count = remote_items.len();
                if count > 0 {
                    app.wallpapers.retain(|w| !w.source.is_remote());
                    app.wallpapers.append(&mut remote_items);
                    app.apply_persisted_flags();
                    app.invalidate_visible_cache();
                }

                if let Some(handle) = win_handle {
                    let _ = handle.update(cx, |_, win, cx| {
                        if count > 0 && errors.is_empty() {
                            win.push_notification(
                                Notification::success(format!(
                                    "Loaded {} wallpapers from online providers",
                                    count
                                )),
                                cx,
                            );
                        } else if count > 0 {
                            win.push_notification(
                                Notification::info(format!(
                                    "Loaded {} wallpapers ({} provider(s) unavailable)",
                                    count,
                                    errors.len()
                                )),
                                cx,
                            );
                        } else if !errors.is_empty() {
                            win.push_notification(
                                Notification::warning(format!(
                                    "Online providers unreachable: {}",
                                    errors.join(" | ")
                                )),
                                cx,
                            );
                        } else {
                            win.push_notification(
                                Notification::info("No matching online wallpapers found"),
                                cx,
                            );
                        }
                    });
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn download_wallpaper(
        &mut self,
        wallpaper_id: &str,
        apply_after: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(wp) = self
            .wallpapers
            .iter()
            .find(|w| w.id == wallpaper_id)
            .cloned()
        else {
            return;
        };

        let dest = wp
            .local_path
            .clone()
            .unwrap_or_else(|| target_path_for(&wp, &self.settings.download_dir));

        let item = DownloadItem {
            wallpaper_id: wp.id.clone(),
            wallpaper_name: wp.name.clone(),
            resolution: wp.resolution_label(),
            preview_hex: wp.primary_hex,
            destination: dest.clone(),
            progress_percent: 30.0,
            status: DownloadStatus::Downloading,
            file_size_bytes: wp.file_size_bytes,
        };
        self.upsert_download_item(item);
        cx.notify();

        let fit = self.settings.wallpaper_fit;
        let auto_apply = apply_after || self.settings.auto_apply_after_download;
        let win_handle = window.window_handle();
        let wp_id = wp.id.clone();

        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { execute_download(&wp, &dest) })
                .await;

            let _ = this.update(cx, |app, cx| {
                match outcome {
                    Ok(done) => {
                        app.downloaded_paths
                            .insert(done.wallpaper_id.clone(), done.destination.clone());
                        if let Some(w) = app
                            .wallpapers
                            .iter_mut()
                            .find(|w| w.id == done.wallpaper_id)
                        {
                            w.is_downloaded = true;
                            w.local_path = Some(done.destination.clone());
                            w.file_size_bytes = Some(done.file_size_bytes);
                        }
                        if let Some(entry) = app
                            .downloads
                            .iter_mut()
                            .find(|d| d.wallpaper_id == done.wallpaper_id)
                        {
                            entry.status = DownloadStatus::Completed;
                            entry.progress_percent = 100.0;
                            entry.destination = done.destination.clone();
                            entry.file_size_bytes = Some(done.file_size_bytes);
                        }
                        app.persist_library_state();
                        app.invalidate_visible_cache();

                        if auto_apply {
                            let apply_res = set_desktop_wallpaper(&done.destination, fit);
                            let _ = win_handle.update(cx, |_, win, cx| match apply_res {
                                Ok(()) => win.push_notification(
                                    Notification::success("Wallpaper downloaded and applied!"),
                                    cx,
                                ),
                                Err(err) => win.push_notification(
                                    Notification::error(format!(
                                        "Saved, but failed to apply: {}",
                                        err
                                    )),
                                    cx,
                                ),
                            });
                        } else {
                            let _ = win_handle.update(cx, |_, win, cx| {
                                win.push_notification(
                                    Notification::success(format!(
                                        "Saved to {}",
                                        done.destination.display()
                                    )),
                                    cx,
                                );
                            });
                        }
                    }
                    Err(err) => {
                        if let Some(entry) =
                            app.downloads.iter_mut().find(|d| d.wallpaper_id == wp_id)
                        {
                            entry.status = DownloadStatus::Failed(err.clone());
                            entry.progress_percent = 0.0;
                        }
                        app.persist_library_state();
                        let _ = win_handle.update(cx, |_, win, cx| {
                            win.push_notification(Notification::error(err), cx);
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn apply_wallpaper(
        &mut self,
        wallpaper_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(wp) = self
            .wallpapers
            .iter()
            .find(|w| w.id == wallpaper_id)
            .cloned()
        else {
            return;
        };

        if let Some(local) = wp.local_path.as_ref().filter(|p| p.exists()) {
            let path = local.clone();
            let fit = self.settings.wallpaper_fit;
            let name = wp.name.clone();
            let win_handle = window.window_handle();
            cx.spawn(async move |_, cx| {
                let res = cx
                    .background_executor()
                    .spawn(async move { set_desktop_wallpaper(&path, fit) })
                    .await;
                let _ = win_handle.update(cx, |_, win, cx| match res {
                    Ok(()) => win.push_notification(
                        Notification::success(format!("Applied \"{}\" to desktop", name)),
                        cx,
                    ),
                    Err(err) => win.push_notification(Notification::error(err), cx),
                });
            })
            .detach();
        } else {
            self.download_wallpaper(wallpaper_id, true, window, cx);
        }
    }

    pub fn remove_downloaded_wallpaper(
        &mut self,
        wallpaper_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(path) = self.downloaded_paths.remove(wallpaper_id)
            && path.exists()
        {
            let _ = fs::remove_file(&path);
        }
        if let Some(wp) = self.wallpapers.iter_mut().find(|w| w.id == wallpaper_id) {
            wp.is_downloaded = false;
            wp.local_path = None;
        }
        self.downloads.retain(|d| d.wallpaper_id != wallpaper_id);
        self.persist_library_state();
        self.invalidate_visible_cache();
        window.push_notification(Notification::info("Removed saved wallpaper file"), cx);
        cx.notify();
    }

    pub fn reveal_path(&self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(err) = reveal_in_file_manager(path) {
            window.push_notification(Notification::error(err), cx);
        }
    }

    pub fn clear_completed_downloads(&mut self, cx: &mut Context<Self>) {
        self.downloads.retain(|d| d.status.is_active());
        self.persist_library_state();
        self.invalidate_visible_cache();
        cx.notify();
    }

    pub fn step_preview(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(current_id) = &self.preview_id else {
            return;
        };
        let visible = &self.cached_visible;
        if let Some(pos) = visible.iter().position(|w| &w.id == current_id) {
            let next = pos as isize + delta;
            if next >= 0 && (next as usize) < visible.len() {
                self.preview_id = Some(visible[next as usize].id.clone());
                cx.notify();
            }
        }
    }

    pub fn apply_download_dir_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.download_dir_input.read(cx).value().trim().to_string();
        if raw.is_empty() {
            window.push_notification(
                Notification::warning("Download directory cannot be empty"),
                cx,
            );
            return;
        }
        self.settings.download_dir = PathBuf::from(raw);
        self.persist_settings();
        self.reload_offline_catalog();
        window.push_notification(Notification::success("Download folder updated"), cx);
        cx.notify();
    }

    pub fn add_local_library_dir(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.local_dir_input.read(cx).value().trim().to_string();
        if raw.is_empty() {
            return;
        }
        let path = PathBuf::from(&raw);
        if !path.is_dir() {
            window.push_notification(
                Notification::warning(format!("Folder does not exist: {}", path.display())),
                cx,
            );
            return;
        }
        if !self.settings.local_library_dirs.contains(&path) {
            self.settings.local_library_dirs.push(path);
            self.persist_settings();
            self.reload_offline_catalog();
            self.local_dir_input.update(cx, |state, cx| {
                state.set_value(String::new(), window, cx);
            });
            window.push_notification(Notification::success("Added local library folder"), cx);
            cx.notify();
        }
    }

    pub fn remove_local_library_dir(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index < self.settings.local_library_dirs.len() {
            self.settings.local_library_dirs.remove(index);
            self.persist_settings();
            self.reload_offline_catalog();
            window.push_notification(Notification::info("Removed local library folder"), cx);
            cx.notify();
        }
    }

    pub fn apply_wallhaven_key_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.wallhaven_api_key =
            self.wallhaven_key_input.read(cx).value().trim().to_string();
        self.persist_settings();
        window.push_notification(Notification::success("Wallhaven API key saved"), cx);
        cx.notify();
    }

    pub fn reset_app_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = reset_all_persisted_data();
        self.settings = AppSettings::default();
        self.favorite_ids.clear();
        self.downloaded_paths.clear();
        self.downloads.clear();
        self.filter.reset();
        ThemePalette::sync_component_theme(
            self.settings.theme_mode,
            self.settings.accent_color,
            &self.typography,
            window,
            cx,
        );

        let dl_dir = self.settings.download_dir.to_string_lossy().to_string();
        self.download_dir_input
            .update(cx, |s, cx| s.set_value(dl_dir, window, cx));
        self.search_input
            .update(cx, |s, cx| s.set_value(String::new(), window, cx));
        self.wallhaven_key_input
            .update(cx, |s, cx| s.set_value(String::new(), window, cx));

        self.reload_offline_catalog();
        window.push_notification(Notification::info("Restored default settings"), cx);
        cx.notify();
    }

    pub fn persist_settings(&self) {
        let _ = save_settings(&self.settings);
    }

    fn persist_library_state(&self) {
        let state = LibraryState {
            favorite_ids: self.favorite_ids.clone(),
            downloaded_paths: self.downloaded_paths.clone(),
            download_history: self.downloads.clone(),
        };
        let _ = save_library_state(&state);
    }

    fn upsert_download_item(&mut self, item: DownloadItem) {
        if let Some(existing) = self
            .downloads
            .iter_mut()
            .find(|d| d.wallpaper_id == item.wallpaper_id)
        {
            *existing = item;
        } else {
            self.downloads.insert(0, item);
        }
        self.persist_library_state();
        self.invalidate_visible_cache();
    }

    fn render_title_bar(&self, palette: ThemePalette, cx: &mut Context<Self>) -> impl IntoElement {
        let modes = [
            (ThemeMode::Light, IconName::Sun, "Light"),
            (ThemeMode::Dark, IconName::Moon, "Dark"),
            (ThemeMode::System, IconName::Monitor, "System"),
        ];

        TitleBar::new()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(palette.text_primary)
                            .child("Wallzy"),
                    )
                    .child(div().text_xs().text_color(palette.text_subtle).child("/"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(palette.text_muted)
                            .child(self.active_nav.label()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pr_2()
                    // Compact Theme Polarity Toggle inside TitleBar
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_0p5()
                            .p_0p5()
                            .rounded_lg()
                            .bg(palette.bg_elevated)
                            .border_1()
                            .border_color(palette.hairline)
                            .children(modes.into_iter().map(|(mode, icon, label)| {
                                let active = self.settings.theme_mode == mode;
                                div()
                                    .id(format!("tb-theme-{}", label))
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .bg(if active {
                                        palette.bg_surface
                                    } else {
                                        rgba(0x00000000)
                                    })
                                    .when(active, |s| s.shadow_sm())
                                    .text_xs()
                                    .text_color(if active {
                                        palette.text_primary
                                    } else {
                                        palette.text_subtle
                                    })
                                    .hover(|s| s.text_color(palette.text_primary))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.set_theme_mode(mode, window, cx);
                                    }))
                                    .child(Icon::new(icon).size(px(12.0)))
                                    .child(label)
                            })),
                    ),
            )
    }
}

impl Render for WallzyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette =
            ThemePalette::resolve(self.settings.theme_mode, self.settings.accent_color, window);

        let visible_wallpapers = self.cached_visible.clone();
        let visible_count = visible_wallpapers.len();

        let preview_info = self.preview_id.as_ref().and_then(|id| {
            let pos = visible_wallpapers.iter().position(|w| &w.id == id);
            let wp = pos
                .map(|i| visible_wallpapers[i].clone())
                .or_else(|| self.wallpapers.iter().find(|w| &w.id == id).cloned())?;
            let has_prev = pos.is_some_and(|i| i > 0);
            let has_next = pos.is_some_and(|i| i + 1 < visible_wallpapers.len());
            Some((wp, has_prev, has_next))
        });

        let active_content =
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(match self.active_nav {
                    NavSection::Downloads => {
                        render_downloads_view(self, palette, cx).into_any_element()
                    }
                    NavSection::Settings => {
                        render_settings_view(self, palette, cx).into_any_element()
                    }
                    _ => render_gallery(self, visible_wallpapers, palette, window, cx)
                        .into_any_element(),
                });
        let animated_content = EffectTransition::new(Duration::from_millis(180))
            .ease(ease_out_cubic)
            .fade(0.0, 1.0)
            .slide_y(px(8.0), px(0.0))
            .apply(active_content, format!("view-enter-{:?}", self.active_nav));

        div()
            .id("wallzy-root")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .font_family(self.typography.ui_font.clone())
            .bg(palette.bg_app)
            .text_color(palette.text_primary)
            .on_click(cx.listener(|this, _, _, cx| {
                if this.open_dropdown.is_some() {
                    this.open_dropdown = None;
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.as_str() == "escape" {
                    if this.open_dropdown.is_some() {
                        this.open_dropdown = None;
                        cx.notify();
                    } else if this.preview_id.is_some() {
                        this.preview_id = None;
                        cx.notify();
                    }
                } else if this.preview_id.is_some() {
                    match event.keystroke.key.as_str() {
                        "left" => this.step_preview(-1, cx),
                        "right" => this.step_preview(1, cx),
                        _ => {}
                    }
                }
            }))
            // Unified Custom Window Chrome TitleBar
            .child(self.render_title_bar(palette, cx))
            // Main Body Split (Sidebar + Studio Canvas)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(render_sidebar(self, palette, cx))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            // Subtle ambient top accent wash
                            .child(div().absolute().top_0().left_0().right_0().h(px(200.0)).bg(
                                linear_gradient(
                                    180.,
                                    linear_color_stop(palette.bg_app_top_glow, 0.),
                                    linear_color_stop(rgba(0x00000000), 1.),
                                ),
                            ))
                            .child(
                                div()
                                    .relative()
                                    .flex_1()
                                    .min_h_0()
                                    .flex()
                                    .flex_col()
                                    .px_7()
                                    .pt_5()
                                    .gap_4()
                                    .child(render_topbar(self, visible_count, palette, cx))
                                    .when(
                                        !matches!(
                                            self.active_nav,
                                            NavSection::Settings | NavSection::Downloads
                                        ),
                                        |this| this.child(render_filter_bar(self, palette, cx)),
                                    )
                                    .child(animated_content),
                            ),
                    ),
            )
            // Preview Inspector Modal
            .when_some(preview_info, |this, (wp, has_prev, has_next)| {
                this.child(render_preview_modal(
                    self, &wp, has_prev, has_next, palette, cx,
                ))
            })
    }
}

fn fetch_cached_thumb(id: &str, url: &str, cache_dir: &Path) -> Option<PathBuf> {
    if fs::create_dir_all(cache_dir).is_err() {
        return None;
    }
    let path = cache_dir.join(format!("{}_wh_thumb.jpg", id));
    if path.exists() {
        return Some(path);
    }
    let status = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "8", "-o"])
        .arg(&path)
        .arg(url)
        .status()
        .ok()?;
    if status.success() && path.exists() {
        Some(path)
    } else {
        let _ = fs::remove_file(&path);
        None
    }
}
