use gpui_kit::assets::IconName;
use serde::{Deserialize, Serialize};

/// Primary navigation sections in Wallzy's sidebar.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum NavSection {
    Discover,
    Trending,
    Latest,
    MyWallpapers,
    Favorites,
    Downloads,
    Settings,
}

impl NavSection {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Discover => "Discover",
            Self::Trending => "Trending",
            Self::Latest => "Latest",
            Self::MyWallpapers => "My Wallpapers",
            Self::Favorites => "Favorites",
            Self::Downloads => "Downloads",
            Self::Settings => "Settings",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Self::Discover => "Curated and online wallpapers tailored for your desktop.",
            Self::Trending => "Most popular wallpapers across the community.",
            Self::Latest => "Freshly added and recently discovered wallpapers.",
            Self::MyWallpapers => "Wallpapers scanned from your local folders and downloads.",
            Self::Favorites => "Your starred collection for quick access and rotation.",
            Self::Downloads => "Active download queue and saved wallpaper files.",
            Self::Settings => "Configure appearance, storage paths, providers, and behavior.",
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Discover => IconName::Globe,
            Self::Trending => IconName::ChartColumn,
            Self::Latest => IconName::Star,
            Self::MyWallpapers => IconName::FolderOpen,
            Self::Favorites => IconName::Heart,
            Self::Downloads => IconName::Download,
            Self::Settings => IconName::Settings2,
        }
    }

    pub fn browse_sections() -> &'static [NavSection] {
        &[Self::Discover, Self::Trending, Self::Latest]
    }

    pub fn library_sections() -> &'static [NavSection] {
        &[Self::MyWallpapers, Self::Favorites, Self::Downloads]
    }
}
