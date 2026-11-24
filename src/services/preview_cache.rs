use crate::models::{GridDensity, NavSection, Wallpaper, WallpaperSource};
use crate::services::artwork::ensure_curated_hero;
use crate::services::storage::preview_cache_dir;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const HERO_ROW_HEIGHT: f32 = 236.0;
pub const CARD_ROW_GAP: f32 = 16.0;
pub const GRID_SIDE_CHROME: f32 = 232.0 + 56.0;

pub const MAX_TEMP_HD_PREVIEWS: usize = 24;
pub const MAX_CONCURRENT_HD_FETCHES: usize = 3;

/// Returns the isolated directory for temporary rolling HD previews.
pub fn temp_hd_cache_dir() -> PathBuf {
    preview_cache_dir().join("hd_temp")
}

/// Removes all temporary HD preview files from disk (used on startup and exit).
pub fn cleanup_temp_hd_cache() {
    let dir = temp_hd_cache_dir();
    if dir.exists() {
        let _ = fs::remove_dir_all(&dir);
    }
}

/// Viewport geometry used to map scroll offsets to visible gallery rows.
#[derive(Debug, Clone, Copy)]
pub struct ViewportMetrics {
    pub scroll_offset_y: f32,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub density: GridDensity,
}

/// Priority-ordered prefetch plan derived from the user's current scroll viewport and preview modal.
#[derive(Debug, Clone)]
pub struct PrefetchPlan {
    pub priority_ids: Vec<String>,
    pub protected_ids: HashSet<String>,
}

/// Computes which wallpaper IDs should be fetched/retained for the active viewport window and preview modal.
pub fn compute_viewport_prefetch_plan(
    items: &[Wallpaper],
    active_nav: NavSection,
    has_active_filters: bool,
    preview_id: Option<&str>,
    viewport: ViewportMetrics,
) -> PrefetchPlan {
    let mut priority_ids = Vec::new();
    let mut protected_ids = HashSet::new();

    let push_id = |id: &str, out: &mut Vec<String>, seen: &mut HashSet<String>| {
        if seen.insert(id.to_string()) {
            out.push(id.to_string());
        }
    };

    // 1. Highest priority: active preview modal and immediate ±1 carousel neighbors.
    if let Some(pid) = preview_id {
        push_id(pid, &mut priority_ids, &mut protected_ids);
        if let Some(pos) = items.iter().position(|w| w.id == pid) {
            if pos + 1 < items.len() {
                push_id(&items[pos + 1].id, &mut priority_ids, &mut protected_ids);
            }
            if pos > 0 {
                push_id(&items[pos - 1].id, &mut priority_ids, &mut protected_ids);
            }
        }
    }

    if items.is_empty() {
        return PrefetchPlan {
            priority_ids,
            protected_ids,
        };
    }

    let card_width = viewport.density.card_width();
    let card_height = viewport.density.thumb_height() + 26.0;
    let row_stride = card_height + CARD_ROW_GAP;
    let avail_w = (viewport.viewport_w - GRID_SIDE_CHROME).max(card_width);
    let cols = ((avail_w + CARD_ROW_GAP) / (card_width + CARD_ROW_GAP))
        .floor()
        .max(1.0) as usize;

    let show_hero = active_nav == NavSection::Discover && !has_active_filters && items.len() > 1;
    let grid_offset = if show_hero { 1 } else { 0 };

    if show_hero {
        protected_ids.insert(items[0].id.clone());
    }

    let scroll_top = viewport.scroll_offset_y.max(0.0);
    let scroll_bottom = scroll_top + viewport.viewport_h.max(400.0);
    let hero_span = if show_hero { HERO_ROW_HEIGHT } else { 0.0 };

    if show_hero && scroll_top < HERO_ROW_HEIGHT {
        push_id(&items[0].id, &mut priority_ids, &mut protected_ids);
    }

    let grid_count = items.len().saturating_sub(grid_offset);
    let card_rows = grid_count.div_ceil(cols);
    if card_rows > 0 {
        let vis_start_row = ((scroll_top - hero_span).max(0.0) / row_stride).floor() as usize;
        let vis_end_row =
            (((scroll_bottom - hero_span).max(0.0) / row_stride).ceil() as usize).min(card_rows);

        // First push visibly on-screen rows so the viewport sharpens first.
        for r in vis_start_row..vis_end_row {
            let start = grid_offset + r * cols;
            let end = (start + cols).min(items.len());
            for wp in &items[start..end] {
                push_id(&wp.id, &mut priority_ids, &mut protected_ids);
            }
        }

        // Lookahead 2 rows below and 1 row above so scrolling/clicking ahead stays seamless.
        let buf_start = vis_start_row.saturating_sub(1);
        let buf_end = (vis_end_row + 2).min(card_rows);
        for r in vis_end_row..buf_end {
            let start = grid_offset + r * cols;
            let end = (start + cols).min(items.len());
            for wp in &items[start..end] {
                push_id(&wp.id, &mut priority_ids, &mut protected_ids);
            }
        }
        for r in buf_start..vis_start_row {
            let start = grid_offset + r * cols;
            let end = (start + cols).min(items.len());
            for wp in &items[start..end] {
                push_id(&wp.id, &mut priority_ids, &mut protected_ids);
            }
        }
    }

    PrefetchPlan {
        priority_ids,
        protected_ids,
    }
}

/// Bounded LRU cache for temporary HD preview files on disk.
///
/// Retains the active viewport window + recently passed items up to `max_entries`,
/// deleting evicted temporary files from disk as the user scrolls past or leaves.
pub struct RollingHdCache {
    entries: HashMap<String, Option<PathBuf>>,
    lru: VecDeque<String>,
    in_flight: HashSet<String>,
    max_entries: usize,
}

impl RollingHdCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            lru: VecDeque::new(),
            in_flight: HashSet::new(),
            max_entries: max_entries.max(4),
        }
    }

    /// Returns the cached temporary HD file path if it exists on disk.
    pub fn get(&self, id: &str) -> Option<PathBuf> {
        self.entries
            .get(id)
            .and_then(|opt| opt.as_ref())
            .filter(|p| p.exists())
            .cloned()
    }

    pub fn needs_fetch(&self, id: &str) -> bool {
        if self.in_flight.contains(id) {
            return false;
        }
        match self.entries.get(id) {
            Some(Some(path)) => !path.exists(),
            Some(None) => false,
            None => true,
        }
    }

    pub fn active_fetches(&self) -> usize {
        self.in_flight.len()
    }

    pub fn mark_loading(&mut self, id: &str) {
        self.in_flight.insert(id.to_string());
    }

    /// Records the completion of a background HD fetch and evicts stale files outside `protected`.
    pub fn finish_loading(
        &mut self,
        id: &str,
        result: Option<PathBuf>,
        protected: &HashSet<String>,
    ) -> Vec<PathBuf> {
        self.in_flight.remove(id);
        self.entries.insert(id.to_string(), result);
        self.touch_lru(id);
        self.evict_excess(protected)
    }

    /// Refreshes LRU ordering for active viewport items and prunes older items left behind.
    pub fn touch_and_prune(
        &mut self,
        active_ids: &[String],
        protected: &HashSet<String>,
    ) -> Vec<PathBuf> {
        for id in active_ids {
            if self.entries.contains_key(id) {
                self.touch_lru(id);
            }
        }
        self.evict_excess(protected)
    }

    /// Clears failed entries so a manual catalog refresh can retry them.
    pub fn reset_failed(&mut self) {
        self.entries.retain(|_, v| v.is_some());
        let entries = &self.entries;
        self.lru.retain(|id| entries.contains_key(id));
    }

    fn touch_lru(&mut self, id: &str) {
        if let Some(pos) = self.lru.iter().position(|existing| existing == id) {
            self.lru.remove(pos);
        }
        self.lru.push_back(id.to_string());
    }

    fn evict_excess(&mut self, protected: &HashSet<String>) -> Vec<PathBuf> {
        let mut evicted_paths = Vec::new();
        if self.entries.len() <= self.max_entries {
            return evicted_paths;
        }

        let mut i = 0;
        while self.entries.len() > self.max_entries && i < self.lru.len() {
            let candidate = &self.lru[i];
            if protected.contains(candidate) {
                i += 1;
                continue;
            }
            if let Some(victim_id) = self.lru.remove(i)
                && let Some(Some(path)) = self.entries.remove(&victim_id)
            {
                let _ = fs::remove_file(&path);
                evicted_paths.push(path);
            }
        }
        evicted_paths
    }
}

/// Fetches or renders a high-resolution preview file inside `temp_hd_cache_dir()`.
pub fn fetch_or_render_hd_preview(wallpaper: &Wallpaper) -> Option<PathBuf> {
    if let Some(local) = wallpaper.local_path.as_ref().filter(|p| p.exists()) {
        return Some(local.clone());
    }

    let dir = temp_hd_cache_dir();
    if fs::create_dir_all(&dir).is_err() {
        return None;
    }

    if matches!(wallpaper.source, WallpaperSource::Curated) {
        return ensure_curated_hero(wallpaper, &dir);
    }

    let url = wallpaper.download_url.as_deref()?;
    let safe_id = sanitize_cache_id(&wallpaper.id);
    let ext = extension_from_url(url);
    let dest = dir.join(format!("hd_{}.{}", safe_id, ext));
    if dest.metadata().is_ok_and(|m| m.is_file() && m.len() > 1024) {
        return Some(dest);
    }

    let tmp_dest = dir.join(format!("hd_{}.{}.part", safe_id, ext));
    let status = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "20",
            "-A",
            "Wallzy/0.1 (Open Source Wallpaper Manager)",
            "-o",
        ])
        .arg(&tmp_dest)
        .arg(url)
        .status()
        .ok()?;

    if status.success()
        && tmp_dest
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() > 1024)
        && fs::rename(&tmp_dest, &dest).is_ok()
    {
        Some(dest)
    } else {
        let _ = fs::remove_file(&tmp_dest);
        None
    }
}

fn sanitize_cache_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn extension_from_url(url: &str) -> &'static str {
    let clean = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if clean.ends_with(".png") {
        "png"
    } else if clean.ends_with(".webp") {
        "webp"
    } else {
        "jpg"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::curated::curated_catalog;

    #[test]
    fn rolling_cache_evicts_oldest_unprotected_entries() {
        let temp_dir = std::env::temp_dir().join("wallzy_rolling_cache_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mut cache = RollingHdCache::new(4);
        let mut protected = HashSet::new();
        protected.insert("wp-0".to_string());

        for i in 0..6 {
            let p = temp_dir.join(format!("hd_{}.png", i));
            fs::write(&p, b"test-image-payload").unwrap();
            cache.mark_loading(&format!("wp-{}", i));
            cache.finish_loading(&format!("wp-{}", i), Some(p), &protected);
        }

        // Protected item `wp-0` stays even though it's oldest; `wp-1` and `wp-2` are evicted.
        assert!(cache.get("wp-0").is_some());
        assert!(cache.get("wp-1").is_none());
        assert!(cache.get("wp-2").is_none());
        assert!(cache.get("wp-3").is_some());
        assert!(cache.get("wp-4").is_some());
        assert!(cache.get("wp-5").is_some());
        assert!(!temp_dir.join("hd_1.png").exists());
        assert!(!temp_dir.join("hd_2.png").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn prefetch_plan_prioritizes_preview_and_neighbors() {
        let catalog = curated_catalog();
        let target_id = catalog[3].id.clone();
        let next_id = catalog[4].id.clone();
        let prev_id = catalog[2].id.clone();

        let plan = compute_viewport_prefetch_plan(
            &catalog,
            NavSection::Discover,
            false,
            Some(&target_id),
            ViewportMetrics {
                scroll_offset_y: 0.0,
                viewport_w: 1340.0,
                viewport_h: 800.0,
                density: GridDensity::Comfortable,
            },
        );

        assert_eq!(plan.priority_ids[0], target_id);
        assert!(plan.priority_ids[1..=2].contains(&next_id));
        assert!(plan.priority_ids[1..=2].contains(&prev_id));
        assert!(plan.protected_ids.contains(&catalog[0].id));
    }
}
