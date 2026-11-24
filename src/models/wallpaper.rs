use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Origin of a wallpaper entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum WallpaperSource {
    Curated,
    Local,
    Bing,
    Wallhaven,
    Reddit,
    Wikimedia,
    Nasa,
}

impl WallpaperSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Curated => "Curated",
            Self::Local => "Local",
            Self::Bing => "Bing Daily",
            Self::Wallhaven => "Wallhaven",
            Self::Reddit => "Reddit",
            Self::Wikimedia => "Wikimedia",
            Self::Nasa => "NASA",
        }
    }

    pub fn is_remote(&self) -> bool {
        !matches!(self, Self::Curated | Self::Local)
    }

    pub fn all() -> &'static [WallpaperSource] {
        &[
            Self::Curated,
            Self::Local,
            Self::Bing,
            Self::Wallhaven,
            Self::Reddit,
            Self::Wikimedia,
            Self::Nasa,
        ]
    }
}

/// Screen orientation derived from width/height or provider metadata.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum Orientation {
    Landscape,
    Portrait,
    Ultrawide,
    Square,
}

impl Orientation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Landscape => "Landscape",
            Self::Portrait => "Portrait",
            Self::Ultrawide => "Ultrawide",
            Self::Square => "Square",
        }
    }

    pub fn from_dimensions(width: u32, height: u32) -> Self {
        if width == 0 || height == 0 {
            return Self::Landscape;
        }
        let ratio = width as f32 / height as f32;
        if ratio >= 2.05 {
            Self::Ultrawide
        } else if ratio > 1.08 {
            Self::Landscape
        } else if ratio < 0.92 {
            Self::Portrait
        } else {
            Self::Square
        }
    }
}

/// Resolution filter buckets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ResolutionPreset {
    #[default]
    All,
    Fhd,
    Qhd,
    Uhd4k,
    Uhd5kPlus,
}

impl ResolutionPreset {
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::All => "All Res",
            Self::Fhd => "1080p+",
            Self::Qhd => "2K+",
            Self::Uhd4k => "4K+",
            Self::Uhd5kPlus => "5K+",
        }
    }

    pub fn matches(&self, width: u32, height: u32) -> bool {
        let max_dim = width.max(height);
        let min_dim = width.min(height);
        match self {
            Self::All => true,
            Self::Fhd => max_dim >= 1920 && min_dim >= 1080,
            Self::Qhd => max_dim >= 2560 && min_dim >= 1440,
            Self::Uhd4k => max_dim >= 3840 && min_dim >= 2160,
            Self::Uhd5kPlus => max_dim >= 5120,
        }
    }

    pub fn all() -> &'static [ResolutionPreset] {
        &[
            Self::All,
            Self::Fhd,
            Self::Qhd,
            Self::Uhd4k,
            Self::Uhd5kPlus,
        ]
    }
}

/// Sort order applied to the visible wallpaper grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Featured,
    Trending,
    Newest,
    ResolutionDesc,
    NameAsc,
}

impl SortOrder {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Featured => "Featured",
            Self::Trending => "Most Popular",
            Self::Newest => "Newest",
            Self::ResolutionDesc => "Highest Res",
            Self::NameAsc => "Name (A-Z)",
        }
    }

    pub fn all() -> &'static [SortOrder] {
        &[
            Self::Featured,
            Self::Trending,
            Self::Newest,
            Self::ResolutionDesc,
            Self::NameAsc,
        ]
    }
}

/// Core wallpaper domain entity used across providers, UI, and persistence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallpaper {
    pub id: String,
    pub name: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub width: u32,
    pub height: u32,
    pub orientation: Orientation,
    pub source: WallpaperSource,
    /// Primary hex color used as placeholder/tint and palette swatch.
    pub primary_hex: u32,
    /// Secondary hex color used for procedural artwork & rich card previews.
    pub secondary_hex: u32,
    /// Optional remote thumbnail URL.
    pub thumb_url: Option<String>,
    /// Optional remote full-resolution download URL.
    pub download_url: Option<String>,
    /// Path to cached preview thumbnail on disk (generated or fetched).
    pub preview_path: Option<PathBuf>,
    /// Path to downloaded or local full-resolution image on disk.
    pub local_path: Option<PathBuf>,
    pub file_size_bytes: Option<u64>,
    pub popularity: u32,
    pub created_order: u32,
    pub is_favorite: bool,
    pub is_downloaded: bool,
}

impl Wallpaper {
    pub fn resolution_label(&self) -> String {
        if self.width > 0 && self.height > 0 {
            format!("{}×{}", self.width, self.height)
        } else {
            "Unknown".to_string()
        }
    }

    pub fn quality_badge(&self) -> Option<&'static str> {
        let max_dim = self.width.max(self.height);
        if max_dim >= 5120 {
            Some("5K")
        } else if max_dim >= 3840 {
            Some("4K")
        } else if max_dim >= 2560 {
            Some("2K")
        } else if max_dim >= 1920 {
            Some("HD")
        } else {
            None
        }
    }

    pub fn aspect_ratio_label(&self) -> String {
        if self.width == 0 || self.height == 0 {
            return "—".to_string();
        }
        let g = gcd(self.width, self.height);
        let aw = self.width / g;
        let ah = self.height / g;
        if aw <= 32 && ah <= 32 {
            format!("{}:{}", aw, ah)
        } else {
            format!("{:.2}:1", self.width as f32 / self.height as f32)
        }
    }

    pub fn formatted_size(&self) -> String {
        match self.file_size_bytes {
            Some(bytes) if bytes >= 1024 * 1024 => {
                format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
            }
            Some(bytes) if bytes >= 1024 => format!("{:.0} KB", bytes as f64 / 1024.0),
            Some(bytes) => format!("{} B", bytes),
            None => "Procedural / Remote".to_string(),
        }
    }

    pub fn pixel_count(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }

    /// Best local image path available for preview rendering (`preview_path` preferred for crisp
    /// lightweight gallery scrolling, falling back to `local_path`).
    pub fn display_image_path(&self) -> Option<&PathBuf> {
        self.preview_path
            .as_ref()
            .filter(|p| p.exists())
            .or_else(|| self.local_path.as_ref().filter(|p| p.exists()))
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

/// Active query/filter state applied to the wallpaper catalog.
#[derive(Clone, Debug, Default)]
pub struct WallpaperFilter {
    pub query: String,
    pub category: String,
    pub resolution: ResolutionPreset,
    pub orientation: Option<Orientation>,
    pub source: Option<WallpaperSource>,
    pub sort: SortOrder,
}

impl WallpaperFilter {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            category: "All".to_string(),
            resolution: ResolutionPreset::All,
            orientation: None,
            source: None,
            sort: SortOrder::Featured,
        }
    }

    pub fn active_filter_count(&self) -> usize {
        let mut count = 0;
        if !self.query.trim().is_empty() {
            count += 1;
        }
        if self.category != "All" {
            count += 1;
        }
        if self.resolution != ResolutionPreset::All {
            count += 1;
        }
        if self.orientation.is_some() {
            count += 1;
        }
        if self.source.is_some() {
            count += 1;
        }
        count
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.category = "All".to_string();
        self.resolution = ResolutionPreset::All;
        self.orientation = None;
        self.source = None;
    }

    pub fn matches(&self, wp: &Wallpaper) -> bool {
        if self.category != "All" && !wp.category.eq_ignore_ascii_case(&self.category) {
            return false;
        }
        if !self.resolution.matches(wp.width, wp.height) {
            return false;
        }
        if let Some(orientation) = self.orientation
            && wp.orientation != orientation
        {
            return false;
        }
        if let Some(source) = self.source
            && wp.source != source
        {
            return false;
        }
        let q = self.query.trim().to_lowercase();
        if !q.is_empty() {
            let matches_name = wp.name.to_lowercase().contains(&q);
            let matches_author = wp.author.to_lowercase().contains(&q);
            let matches_cat = wp.category.to_lowercase().contains(&q);
            let matches_tag = wp.tags.iter().any(|t| t.to_lowercase().contains(&q));
            if !(matches_name || matches_author || matches_cat || matches_tag) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::curated::curated_catalog;

    #[test]
    fn orientation_detects_aspect_ratios() {
        assert_eq!(
            Orientation::from_dimensions(3840, 2160),
            Orientation::Landscape
        );
        assert_eq!(
            Orientation::from_dimensions(3440, 1440),
            Orientation::Ultrawide
        );
        assert_eq!(
            Orientation::from_dimensions(2160, 3840),
            Orientation::Portrait
        );
        assert_eq!(
            Orientation::from_dimensions(2000, 2000),
            Orientation::Square
        );
    }

    #[test]
    fn resolution_presets_and_filters_work() {
        let catalog = curated_catalog();
        assert!(catalog.len() >= 10);

        let mut filter = WallpaperFilter::new();
        filter.resolution = ResolutionPreset::Uhd4k;
        filter.category = "Nature".to_string();
        filter.query = "alpine".to_string();

        let matches: Vec<_> = catalog.iter().filter(|w| filter.matches(w)).collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "Alpine Dusk");
        assert_eq!(filter.active_filter_count(), 3);

        filter.reset();
        assert_eq!(filter.active_filter_count(), 0);
    }
}
