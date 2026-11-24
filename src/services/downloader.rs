use crate::models::Wallpaper;
use crate::services::artwork::render_wallpaper_png;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of downloading or materializing a wallpaper to disk.
#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub wallpaper_id: String,
    pub destination: PathBuf,
    pub file_size_bytes: u64,
}

/// Computes a sanitized target file path inside `download_dir` for a wallpaper.
pub fn target_path_for(wallpaper: &Wallpaper, download_dir: &Path) -> PathBuf {
    let ext = wallpaper
        .download_url
        .as_deref()
        .and_then(|url| {
            url.split('?')
                .next()
                .and_then(|clean| clean.rsplit('.').next())
                .filter(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "webp"
                    )
                })
        })
        .unwrap_or("png");

    let clean_name: String = wallpaper
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = clean_name.trim_matches('_');
    let stem = if trimmed.is_empty() {
        wallpaper.id.as_str()
    } else {
        trimmed
    };
    download_dir.join(format!("{}_{}.{}", stem, sanitize_id(&wallpaper.id), ext))
}

/// Downloads a remote wallpaper via system `curl`, or renders a high-res PNG for curated items.
pub fn execute_download(wallpaper: &Wallpaper, dest: &Path) -> Result<DownloadOutcome, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create download folder {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    if let Some(local) = wallpaper.local_path.as_ref().filter(|p| p.exists()) {
        if local != dest {
            fs::copy(local, dest).map_err(|e| format!("Failed to copy local wallpaper: {}", e))?;
        }
    } else if let Some(url) = wallpaper.download_url.as_deref()
        && wallpaper.source.is_remote()
    {
        download_via_curl(url, dest)?;
    } else {
        // Render full-HD/QHD procedural artwork for curated wallpapers (capped at 2560px wide for speed).
        let (rw, rh) = capped_dimensions(wallpaper.width, wallpaper.height, 2560);
        render_wallpaper_png(wallpaper, rw, rh, dest)?;
    }

    let meta = fs::metadata(dest)
        .map_err(|e| format!("Downloaded file missing at {}: {}", dest.display(), e))?;
    Ok(DownloadOutcome {
        wallpaper_id: wallpaper.id.clone(),
        destination: dest.to_path_buf(),
        file_size_bytes: meta.len(),
    })
}

fn download_via_curl(url: &str, dest: &Path) -> Result<(), String> {
    let tmp_dest = dest.with_extension("part");
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "45",
            "-A",
            "Wallzy/0.1 (Open Source Wallpaper Manager)",
            "-o",
        ])
        .arg(&tmp_dest)
        .arg(url)
        .output()
        .map_err(|e| format!("Failed to invoke curl: {}", e))?;

    if !output.status.success() {
        let _ = fs::remove_file(&tmp_dest);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Download failed: {}", stderr.trim()));
    }

    fs::rename(&tmp_dest, dest).map_err(|e| {
        let _ = fs::remove_file(&tmp_dest);
        format!("Failed to finalize file {}: {}", dest.display(), e)
    })
}

fn capped_dimensions(w: u32, h: u32, max_edge: u32) -> (u32, u32) {
    let w = w.max(1280);
    let h = h.max(720);
    let longest = w.max(h);
    if longest <= max_edge {
        return (w, h);
    }
    let scale = max_edge as f32 / longest as f32;
    (
        ((w as f32) * scale).round() as u32,
        ((h as f32) * scale).round() as u32,
    )
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(16)
        .collect()
}
