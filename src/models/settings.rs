use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application color theme preference.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    System,
}

impl ThemeMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }

    pub fn all() -> &'static [ThemeMode] {
        &[Self::Light, Self::Dark, Self::System]
    }
}

/// User-selectable accent color presets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum AccentColor {
    #[default]
    Indigo,
    Cyan,
    Emerald,
    Amber,
    Rose,
}

impl AccentColor {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Indigo => "Indigo",
            Self::Cyan => "Cyan",
            Self::Emerald => "Emerald",
            Self::Amber => "Amber",
            Self::Rose => "Rose",
        }
    }

    pub fn dark_hex(&self) -> u32 {
        match self {
            Self::Indigo => 0x6366f1,
            Self::Cyan => 0x06b6d4,
            Self::Emerald => 0x10b981,
            Self::Amber => 0xf59e0b,
            Self::Rose => 0xf43f5e,
        }
    }

    pub fn light_hex(&self) -> u32 {
        match self {
            Self::Indigo => 0x4f46e5,
            Self::Cyan => 0x0891b2,
            Self::Emerald => 0x059669,
            Self::Amber => 0xd97706,
            Self::Rose => 0xe11d48,
        }
    }

    pub fn all() -> &'static [AccentColor] {
        &[
            Self::Indigo,
            Self::Cyan,
            Self::Emerald,
            Self::Amber,
            Self::Rose,
        ]
    }
}

/// Grid card density / size presets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum GridDensity {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

impl GridDensity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Comfortable => "Comfortable",
            Self::Spacious => "Spacious",
        }
    }

    pub fn card_width(&self) -> f32 {
        match self {
            Self::Compact => 232.0,
            Self::Comfortable => 280.0,
            Self::Spacious => 340.0,
        }
    }

    pub fn thumb_height(&self) -> f32 {
        match self {
            Self::Compact => 142.0,
            Self::Comfortable => 172.0,
            Self::Spacious => 208.0,
        }
    }

    pub fn all() -> &'static [GridDensity] {
        &[Self::Compact, Self::Comfortable, Self::Spacious]
    }
}

/// Desktop wallpaper scaling / fit mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum WallpaperFit {
    #[default]
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
}

impl WallpaperFit {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fill => "Fill",
            Self::Fit => "Fit",
            Self::Stretch => "Stretch",
            Self::Center => "Center",
            Self::Tile => "Tile",
        }
    }

    pub fn all() -> &'static [WallpaperFit] {
        &[
            Self::Fill,
            Self::Fit,
            Self::Stretch,
            Self::Center,
            Self::Tile,
        ]
    }
}

/// User-configured custom HTTP JSON wallpaper provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
    pub enabled: bool,
}

/// Persisted application settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme_mode: ThemeMode,
    pub accent_color: AccentColor,
    pub grid_density: GridDensity,
    pub wallpaper_fit: WallpaperFit,
    pub download_dir: PathBuf,
    pub local_library_dirs: Vec<PathBuf>,
    pub enable_curated: bool,
    pub enable_local: bool,
    pub enable_wallhaven: bool,
    pub enable_alpha_coders: bool,
    pub enable_bing: bool,
    pub enable_reddit: bool,
    pub enable_wikimedia: bool,
    pub enable_nasa: bool,
    pub wallhaven_api_key: String,
    pub alpha_coders_api_key: String,
    pub custom_providers: Vec<CustomProviderConfig>,
    pub auto_apply_after_download: bool,
    pub show_resolution_badges: bool,
}

impl AppSettings {
    pub fn has_any_online_provider(&self) -> bool {
        self.enable_wallhaven
            || self.enable_alpha_coders
            || self.enable_bing
            || self.enable_reddit
            || self.enable_wikimedia
            || self.enable_nasa
            || self
                .custom_providers
                .iter()
                .any(|p| p.enabled && !p.endpoint_url.trim().is_empty())
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        let base_pictures = dirs::picture_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let download_dir = base_pictures.join("Wallzy");

        Self {
            theme_mode: ThemeMode::Dark,
            accent_color: AccentColor::Indigo,
            grid_density: GridDensity::Comfortable,
            wallpaper_fit: WallpaperFit::Fill,
            download_dir,
            local_library_dirs: Vec::new(),
            enable_curated: true,
            enable_local: true,
            enable_wallhaven: true,
            enable_alpha_coders: true,
            enable_bing: true,
            enable_reddit: true,
            enable_wikimedia: false,
            enable_nasa: true,
            wallhaven_api_key: String::new(),
            alpha_coders_api_key: String::new(),
            custom_providers: Vec::new(),
            auto_apply_after_download: false,
            show_resolution_badges: true,
        }
    }
}
