use crate::models::{
    AccentColor, AppSettings, DownloadItem, DownloadStatus, GridDensity, NavSection, SortOrder,
    ThemeMode, Wallpaper, WallpaperFilter, WallpaperFit, WallpaperSource,
};
use crate::providers::curated::CuratedProvider;
use crate::providers::local::LocalProvider;
use crate::providers::{ProviderQuery, WallpaperProvider, fetch_enabled_online};
use crate::services::downloader::{execute_download_with_cache, target_path_for};
use crate::services::preview_cache::{
    MAX_CONCURRENT_HD_FETCHES, MAX_TEMP_HD_PREVIEWS, PrefetchPlan, RollingHdCache, ViewportMetrics,
    cleanup_temp_hd_cache, compute_viewport_prefetch_plan, fetch_or_render_hd_preview,
};
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
use gpui_kit::component::animation::{EffectTransition, ease_out_cubic};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::scroll::ScrollbarHandle as _;
use gpui_kit::component::{TitleBar, VirtualListScrollHandle, WindowExt as _};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
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
    pub preview_image_cache: Entity<RetainAllImageCache>,
    pub gallery_scroll: VirtualListScrollHandle,
    cached_visible: Rc<Vec<Wallpaper>>,
    hd_cache: RollingHdCache,
    needs_prefetch_sync: bool,
    prefetch_generation: u64,
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
        cleanup_temp_hd_cache();
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
        let preview_image_cache = RetainAllImageCache::new(cx);
        let quit_sub = cx.on_app_quit(|_, _| async {
            cleanup_temp_hd_cache();
        });

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
            preview_image_cache,
            gallery_scroll: VirtualListScrollHandle::new(),
            cached_visible: Rc::new(Vec::new()),
            hd_cache: RollingHdCache::new(MAX_TEMP_HD_PREVIEWS),
            needs_prefetch_sync: true,
            prefetch_generation: 0,
            smooth_scroll_y: 0.0,
            scroll_target_y: 0.0,
            scroll_animating: false,
            favorite_ids: library.favorite_ids,
            downloaded_paths: library.downloaded_paths,
            search_input,
            download_dir_input,
            local_dir_input,
            wallhaven_key_input,
            _subscriptions: vec![search_sub, quit_sub],
        };

        app.reload_offline_catalog();
        app.sync_hd_prefetch(window, cx);
        app.fetch_online_providers_inner(Some(window), false, cx);
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
        self.needs_prefetch_sync = true;
    }

    /// Returns a pre-cached high-resolution preview file path for `id` if ready.
    pub fn hd_preview_path(&self, id: &str) -> Option<PathBuf> {
        self.hd_cache.get(id)
    }

    /// Returns a pre-cached high-resolution hero image path for the given wallpaper ID, if ready.
    pub fn hero_image_override(&self, id: &str) -> Option<PathBuf> {
        self.hd_cache.get(id)
    }

    /// Opens the preview modal and immediately prioritizes HD preview prefetch for it and its neighbors.
    pub fn open_preview(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_dropdown = None;
        self.preview_id = Some(id);
        self.sync_hd_prefetch(window, cx);
        cx.notify();
    }

    /// Closes the preview modal and drops decoded full-resolution modal textures from GPU memory.
    pub fn close_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.preview_id = None;
        self.preview_image_cache
            .update(cx, |cache, cx| cache.clear(window, cx));
        self.sync_hd_prefetch(window, cx);
        cx.notify();
    }

    fn compute_current_prefetch_plan(&self, window: &Window) -> PrefetchPlan {
        let scroll_y = (-f32::from(self.gallery_scroll.base_handle().offset().y)).max(0.0);
        let bounds_h: f32 = self
            .gallery_scroll
            .base_handle()
            .bounds()
            .size
            .height
            .into();
        let viewport_h = if bounds_h > 1.0 { bounds_h } else { 640.0 };
        let viewport_w: f32 = window.viewport_size().width.into();
        compute_viewport_prefetch_plan(
            &self.cached_visible,
            self.active_nav,
            self.filter.active_filter_count() > 0,
            self.preview_id.as_deref(),
            ViewportMetrics {
                scroll_offset_y: scroll_y,
                viewport_w,
                viewport_h,
                density: self.settings.grid_density,
            },
        )
    }

    fn drop_preview_textures(
        &mut self,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() {
            return;
        }
        self.preview_image_cache.update(cx, |cache, cx| {
            for path in paths {
                let res = Resource::Path(Arc::from(path.as_path()));
                cache.remove(&res, window, cx);
            }
        });
    }

    /// Prefetches HD previews for visible + buffered wallpapers and evicts older temp files left behind.
    pub fn sync_hd_prefetch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let plan = self.compute_current_prefetch_plan(window);
        let evicted = self
            .hd_cache
            .touch_and_prune(&plan.priority_ids, &plan.protected_ids);
        self.drop_preview_textures(evicted, window, cx);

        let win_handle = window.window_handle();
        for id in &plan.priority_ids {
            if self.hd_cache.active_fetches() >= MAX_CONCURRENT_HD_FETCHES {
                break;
            }
            if !self.hd_cache.needs_fetch(id) {
                continue;
            }
            let Some(wp) = self.wallpapers.iter().find(|w| &w.id == id).cloned() else {
                continue;
            };
            if wp.local_path.as_ref().is_some_and(|p| p.exists())
                || (!matches!(wp.source, WallpaperSource::Curated) && wp.download_url.is_none())
            {
                continue;
            }

            let target_id = wp.id.clone();
            self.hd_cache.mark_loading(&target_id);
            let entity = cx.entity().downgrade();
            cx.spawn(async move |_, cx| {
                let path = cx
                    .background_executor()
                    .spawn(async move { fetch_or_render_hd_preview(&wp) })
                    .await;

                let _ = win_handle.update(cx, |_, win, cx| {
                    if let Some(entity) = entity.upgrade() {
                        entity.update(cx, |app, cx| {
                            let plan = app.compute_current_prefetch_plan(win);
                            let should_repaint = app.preview_id.as_deref()
                                == Some(target_id.as_str())
                                || app
                                    .cached_visible
                                    .first()
                                    .is_some_and(|w| w.id == target_id);
                            let evicted =
                                app.hd_cache
                                    .finish_loading(&target_id, path, &plan.protected_ids);
                            app.drop_preview_textures(evicted, win, cx);
                            app.sync_hd_prefetch(win, cx);
                            if should_repaint {
                                cx.notify();
                            }
                        });
                    }
                });
            })
            .detach();
        }
    }

    fn schedule_scroll_prefetch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prefetch_generation = self.prefetch_generation.wrapping_add(1);
        let generation = self.prefetch_generation;
        let win_handle = window.window_handle();
        let entity = cx.entity().downgrade();

        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;

            let _ = win_handle.update(cx, |_, win, cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |app, cx| {
                        if app.prefetch_generation == generation && !app.scroll_animating {
                            app.sync_hd_prefetch(win, cx);
                        }
                    });
                }
            });
        })
        .detach();
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
        self.schedule_scroll_prefetch(window, cx);
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
                if !this.scroll_animating {
                    this.sync_hd_prefetch(window, cx);
                }
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
        self.needs_prefetch_sync = true;
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
        self.hd_cache.reset_failed();
        if self.settings.has_any_online_provider() {
            self.fetch_online_providers_inner(window, true, cx);
        } else if let Some(win) = window {
            win.push_notification(Notification::success("Local library rescanned"), cx);
        }
        cx.notify();
    }

    pub fn fetch_online_providers(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        self.fetch_online_providers_inner(window, true, cx);
    }

    fn fetch_online_providers_inner(
        &mut self,
        window: Option<&mut Window>,
        notify_errors: bool,
        cx: &mut Context<Self>,
    ) {
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
                        } else if notify_errors && !errors.is_empty() {
                            win.push_notification(
                                Notification::warning(format!(
                                    "Online providers unreachable: {}",
                                    errors.join(" | ")
                                )),
                                cx,
                            );
                        } else if notify_errors {
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
        let cached_hd = self.hd_cache.get(&wp.id);

        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { execute_download_with_cache(&wp, &dest, cached_hd.as_deref()) })
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

    pub fn step_preview(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current_id) = &self.preview_id else {
            return;
        };
        let visible = &self.cached_visible;
        if let Some(pos) = visible.iter().position(|w| &w.id == current_id) {
            let next = pos as isize + delta;
            if next >= 0 && (next as usize) < visible.len() {
                let next_id = visible[next as usize].id.clone();
                // Drop GPU texture of items left more than 1 step away from the new preview index.
                let mut stale_paths = Vec::new();
                let next_idx = next as usize;
                for offset in [-2isize, 2isize] {
                    let far = next + offset;
                    if far >= 0
                        && (far as usize) < visible.len()
                        && (far as usize).abs_diff(next_idx) > 1
                    {
                        let far_wp = &visible[far as usize];
                        if let Some(p) = self.hd_cache.get(&far_wp.id) {
                            stale_paths.push(p);
                        }
                    }
                }
                self.drop_preview_textures(stale_paths, window, cx);
                self.preview_id = Some(next_id);
                self.sync_hd_prefetch(window, cx);
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

    fn render_title_bar(&self, palette: ThemePalette, _cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new().child(
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
    }
}

impl Render for WallzyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_prefetch_sync && !self.scroll_animating {
            self.needs_prefetch_sync = false;
            self.sync_hd_prefetch(window, cx);
        }
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key.as_str() == "escape" {
                    if this.open_dropdown.is_some() {
                        this.open_dropdown = None;
                        cx.notify();
                    } else if this.preview_id.is_some() {
                        this.close_preview(window, cx);
                    }
                } else if this.preview_id.is_some() {
                    match event.keystroke.key.as_str() {
                        "left" => this.step_preview(-1, window, cx),
                        "right" => this.step_preview(1, window, cx),
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

impl Drop for WallzyApp {
    fn drop(&mut self) {
        cleanup_temp_hd_cache();
    }
}
