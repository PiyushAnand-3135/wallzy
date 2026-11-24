use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Lifecycle state for a wallpaper download job.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Completed,
    Failed(String),
}

impl DownloadStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Queued | Self::Downloading)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Queued => "Queued",
            Self::Downloading => "Downloading",
            Self::Completed => "Completed",
            Self::Failed(_) => "Failed",
        }
    }
}

/// Tracks a wallpaper download task and its local destination.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadItem {
    pub wallpaper_id: String,
    pub wallpaper_name: String,
    pub resolution: String,
    pub preview_hex: u32,
    pub destination: PathBuf,
    pub progress_percent: f32,
    pub status: DownloadStatus,
    pub file_size_bytes: Option<u64>,
}
