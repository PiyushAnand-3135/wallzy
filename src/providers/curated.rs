use crate::models::{Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider};
use crate::services::artwork::ensure_curated_preview;
use std::path::PathBuf;

/// Built-in curated wallpaper catalog with procedural artwork previews so Wallzy
/// works out of the box even without an internet connection.
pub struct CuratedProvider {
    preview_cache_dir: PathBuf,
}

impl CuratedProvider {
    pub fn new(preview_cache_dir: PathBuf) -> Self {
        Self { preview_cache_dir }
    }
}

impl WallpaperProvider for CuratedProvider {
    fn id(&self) -> &'static str {
        "curated"
    }

    fn display_name(&self) -> &'static str {
        "Wallzy Curated"
    }

    fn fetch(&self, _query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let mut items = curated_catalog();
        for item in &mut items {
            item.preview_path = ensure_curated_preview(item, &self.preview_cache_dir);
        }
        Ok(items)
    }
}

struct CuratedSeed {
    id: &'static str,
    name: &'static str,
    author: &'static str,
    category: &'static str,
    tags: &'static [&'static str],
    width: u32,
    height: u32,
    primary_hex: u32,
    secondary_hex: u32,
    popularity: u32,
    created_order: u32,
}

pub fn curated_catalog() -> Vec<Wallpaper> {
    const SEEDS: &[CuratedSeed] = &[
        CuratedSeed {
            id: "curated-alpine-dusk",
            name: "Alpine Dusk",
            author: "Lukas Meyer",
            category: "Nature",
            tags: &["mountains", "twilight", "blue", "peaks"],
            width: 3840,
            height: 2160,
            primary_hex: 0x1e2c3d,
            secondary_hex: 0x5c6b82,
            popularity: 96,
            created_order: 12,
        },
        CuratedSeed {
            id: "curated-monolith-concrete",
            name: "Monolith Concrete",
            author: "Elena Vance",
            category: "Architecture",
            tags: &["brutalist", "monochrome", "shadow", "minimal"],
            width: 5120,
            height: 2880,
            primary_hex: 0x2f3339,
            secondary_hex: 0x626770,
            popularity: 88,
            created_order: 10,
        },
        CuratedSeed {
            id: "curated-quiet-dunes",
            name: "Quiet Dunes",
            author: "Amir Farah",
            category: "Minimal",
            tags: &["desert", "warm", "sand", "calm"],
            width: 3840,
            height: 2160,
            primary_hex: 0x8d6e53,
            secondary_hex: 0xd2b393,
            popularity: 92,
            created_order: 14,
        },
        CuratedSeed {
            id: "curated-nordic-mist",
            name: "Nordic Mist",
            author: "Soren Lindqvist",
            category: "Nature",
            tags: &["fjord", "fog", "green", "ultrawide"],
            width: 3440,
            height: 1440,
            primary_hex: 0x263834,
            secondary_hex: 0x5d7d73,
            popularity: 94,
            created_order: 11,
        },
        CuratedSeed {
            id: "curated-obsidian-waves",
            name: "Obsidian Waves",
            author: "Kira Sato",
            category: "Abstract",
            tags: &["dark", "silk", "indigo", "3d"],
            width: 3840,
            height: 2160,
            primary_hex: 0x161826,
            secondary_hex: 0x4f46e5,
            popularity: 98,
            created_order: 15,
        },
        CuratedSeed {
            id: "curated-kyoto-rain",
            name: "Kyoto Rain",
            author: "Haruto Tanaka",
            category: "Architecture",
            tags: &["street", "neon", "portrait", "night"],
            width: 2160,
            height: 3840,
            primary_hex: 0x2c2233,
            secondary_hex: 0x79557a,
            popularity: 85,
            created_order: 9,
        },
        CuratedSeed {
            id: "curated-solar-eclipse",
            name: "Amber Horizon",
            author: "Nadia Bloom",
            category: "Minimal",
            tags: &["sun", "amber", "gradient", "clean"],
            width: 5120,
            height: 2880,
            primary_hex: 0x2e2336,
            secondary_hex: 0xd97742,
            popularity: 91,
            created_order: 16,
        },
        CuratedSeed {
            id: "curated-deep-nebula",
            name: "Carina Drift",
            author: "Orion Studio",
            category: "Space",
            tags: &["cosmos", "stars", "nebula", "dark"],
            width: 3840,
            height: 2160,
            primary_hex: 0x0b0e1a,
            secondary_hex: 0x3b4d8f,
            popularity: 95,
            created_order: 17,
        },
        CuratedSeed {
            id: "curated-emerald-valley",
            name: "Emerald Ridge",
            author: "Maya Lin",
            category: "Nature",
            tags: &["forest", "valley", "morning", "1440p"],
            width: 2560,
            height: 1440,
            primary_hex: 0x1d3b32,
            secondary_hex: 0x4b8770,
            popularity: 83,
            created_order: 8,
        },
        CuratedSeed {
            id: "curated-prism-flow",
            name: "Prism Flow",
            author: "Devon Reed",
            category: "Abstract",
            tags: &["colorful", "cyan", "waves", "ultrawide"],
            width: 5120,
            height: 2160,
            primary_hex: 0x0e2a38,
            secondary_hex: 0x14b8a6,
            popularity: 90,
            created_order: 13,
        },
        CuratedSeed {
            id: "curated-lunar-orbit",
            name: "Lunar Orbit",
            author: "Orion Studio",
            category: "Space",
            tags: &["moon", "space", "minimal", "1080p"],
            width: 1920,
            height: 1080,
            primary_hex: 0x111319,
            secondary_hex: 0x4b5563,
            popularity: 79,
            created_order: 7,
        },
        CuratedSeed {
            id: "curated-glass-pavilion",
            name: "Glass Pavilion",
            author: "Clara Jensen",
            category: "Architecture",
            tags: &["glass", "light", "modern", "4k"],
            width: 3840,
            height: 2160,
            primary_hex: 0x26323d,
            secondary_hex: 0x8ea4b8,
            popularity: 86,
            created_order: 18,
        },
    ];

    SEEDS
        .iter()
        .map(|s| Wallpaper {
            id: s.id.to_string(),
            name: s.name.to_string(),
            author: s.author.to_string(),
            category: s.category.to_string(),
            tags: s.tags.iter().map(|t| (*t).to_string()).collect(),
            width: s.width,
            height: s.height,
            orientation: Orientation::from_dimensions(s.width, s.height),
            source: WallpaperSource::Curated,
            primary_hex: s.primary_hex,
            secondary_hex: s.secondary_hex,
            thumb_url: None,
            download_url: None,
            preview_path: None,
            local_path: None,
            file_size_bytes: None,
            popularity: s.popularity,
            created_order: s.created_order,
            is_favorite: false,
            is_downloaded: false,
        })
        .collect()
}
