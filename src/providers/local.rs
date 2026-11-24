use crate::models::{Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider};
use image::imageops::FilterType;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Scans configured local directories (including Wallzy's download folder) for real image files.
///
/// Generates lightweight cached JPEG/PNG thumbnails in `preview_cache_dir` so scrolling
/// through large 4K/8K local collections remains instantaneous.
pub struct LocalProvider {
    directories: Vec<PathBuf>,
    preview_cache_dir: Option<PathBuf>,
}

impl LocalProvider {
    pub fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories,
            preview_cache_dir: None,
        }
    }

    pub fn with_preview_cache(mut self, cache_dir: PathBuf) -> Self {
        self.preview_cache_dir = Some(cache_dir);
        self
    }
}

impl WallpaperProvider for LocalProvider {
    fn id(&self) -> &'static str {
        "local"
    }

    fn display_name(&self) -> &'static str {
        "Local Library"
    }

    fn fetch(&self, _query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let mut seen = HashSet::new();
        let mut results = Vec::new();
        let mut order = 1000u32;

        for dir in &self.directories {
            if !dir.exists() {
                continue;
            }
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() || !is_supported_image(&path) {
                    continue;
                }
                let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if !seen.insert(canonical.clone()) {
                    continue;
                }

                let (width, height) = image::image_dimensions(&path).unwrap_or((1920, 1080));
                let file_size_bytes = entry.metadata().ok().map(|m| m.len());
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Local Wallpaper")
                    .replace(['_', '-'], " ");

                let hash = stable_path_hash(&canonical);
                let id = format!("local-{}", hash);
                order += 1;

                let preview_path = self
                    .preview_cache_dir
                    .as_deref()
                    .and_then(|cache| ensure_local_thumbnail(&path, hash, width, height, cache))
                    .or_else(|| Some(path.clone()));

                results.push(Wallpaper {
                    id,
                    name: title_case(&stem),
                    author: "Local File".to_string(),
                    category: "Local".to_string(),
                    tags: vec![
                        "local".to_string(),
                        path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("img")
                            .to_ascii_lowercase(),
                    ],
                    width,
                    height,
                    orientation: Orientation::from_dimensions(width, height),
                    source: WallpaperSource::Local,
                    primary_hex: 0x252a34,
                    secondary_hex: 0x3b4252,
                    thumb_url: None,
                    download_url: None,
                    preview_path,
                    local_path: Some(path),
                    file_size_bytes,
                    popularity: 50,
                    created_order: order,
                    is_favorite: false,
                    is_downloaded: true,
                });
            }
        }

        Ok(results)
    }
}

fn ensure_local_thumbnail(
    src: &Path,
    hash: u64,
    width: u32,
    height: u32,
    cache_dir: &Path,
) -> Option<PathBuf> {
    if width <= 640 && height <= 640 {
        return Some(src.to_path_buf());
    }
    fs::create_dir_all(cache_dir).ok()?;
    let thumb_path = cache_dir.join(format!("local_{}_thumb.jpg", hash));
    if thumb_path.exists() {
        return Some(thumb_path);
    }
    let img = image::open(src).ok()?;
    let resized = img.resize(540, 360, FilterType::Triangle);
    if resized.to_rgb8().save(&thumb_path).is_ok() {
        Some(thumb_path)
    } else {
        None
    }
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp"
            )
        })
}

fn stable_path_hash(path: &Path) -> u64 {
    path.to_string_lossy()
        .bytes()
        .fold(14695981039346656037u64, |acc, b| {
            acc.wrapping_mul(1099511628211) ^ (b as u64)
        })
}

fn title_case(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn local_provider_reads_real_image_dimensions() {
        let temp_dir = std::env::temp_dir().join("wallzy_test_local_provider");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let img_path = temp_dir.join("midnight_test.png");
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(320, 180);
        img.save(&img_path).unwrap();

        let provider = LocalProvider::new(vec![temp_dir.clone()]);
        let items = provider.fetch(&ProviderQuery::default()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].width, 320);
        assert_eq!(items[0].height, 180);
        assert_eq!(items[0].orientation, Orientation::Landscape);
        assert!(items[0].is_downloaded);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
