use crate::models::{AppSettings, DownloadItem};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Persisted user library metadata (favorites, downloaded file mappings, history).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryState {
    pub favorite_ids: HashSet<String>,
    pub downloaded_paths: HashMap<String, PathBuf>,
    pub download_history: Vec<DownloadItem>,
}

/// Returns Wallzy's configuration directory (`~/.config/wallzy` or `%APPDATA%\wallzy`).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wallzy")
}

/// Returns Wallzy's cache directory for generated/fetched preview thumbnails.
pub fn preview_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wallzy")
        .join("previews")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

fn library_state_path() -> PathBuf {
    config_dir().join("library_state.json")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    write_json(&settings_path(), settings)
}

pub fn load_library_state() -> LibraryState {
    let path = library_state_path();
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => LibraryState::default(),
    }
}

pub fn save_library_state(state: &LibraryState) -> Result<(), String> {
    write_json(&library_state_path(), state)
}

pub fn reset_all_persisted_data() -> Result<(), String> {
    for path in [settings_path(), library_state_path()] {
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}
