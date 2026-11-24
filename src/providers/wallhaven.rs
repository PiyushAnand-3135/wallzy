use crate::models::{NavSection, Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes, percent_encode};
use serde::Deserialize;

/// Live community wallpaper provider backed by Wallhaven's public v1 JSON API.
///
/// Uses the system `curl` binary so no extra TLS/HTTP client crates are required.
pub struct WallhavenProvider;

impl WallpaperProvider for WallhavenProvider {
    fn id(&self) -> &'static str {
        "wallhaven"
    }

    fn display_name(&self) -> &'static str {
        "Wallhaven"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let sorting = match query.section {
            Some(NavSection::Trending) => "toplist",
            Some(NavSection::Latest) => "date_added",
            _ => {
                if query.search.trim().is_empty() {
                    "toplist"
                } else {
                    "relevance"
                }
            }
        };

        let page = query.page.max(1);
        let encoded_q = percent_encode(query.search.trim());
        let mut url = format!(
            "https://wallhaven.cc/api/v1/search?sorting={}&purity=100&categories=110&page={}",
            sorting, page
        );
        if !encoded_q.is_empty() {
            url.push_str("&q=");
            url.push_str(&encoded_q);
        }
        if !query.api_key.trim().is_empty() {
            url.push_str("&apikey=");
            url.push_str(&percent_encode(query.api_key.trim()));
        }

        let bytes =
            curl_get_bytes(&url, 12).map_err(|err| format!("Wallhaven request failed: {}", err))?;

        let parsed: WallhavenResponse = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Invalid Wallhaven JSON response: {}", e))?;

        Ok(parsed
            .data
            .into_iter()
            .enumerate()
            .map(|(idx, item)| item.into_wallpaper(idx))
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct WallhavenResponse {
    data: Vec<WallhavenItem>,
}

#[derive(Debug, Deserialize)]
struct WallhavenItem {
    id: String,
    path: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    dimension_x: u32,
    #[serde(default)]
    dimension_y: u32,
    #[serde(default)]
    file_size: u64,
    #[serde(default)]
    favorites: u32,
    #[serde(default)]
    colors: Vec<String>,
    thumbs: WallhavenThumbs,
}

#[derive(Debug, Deserialize)]
struct WallhavenThumbs {
    large: String,
}

impl WallhavenItem {
    fn into_wallpaper(self, index: usize) -> Wallpaper {
        let primary_hex = self
            .colors
            .first()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(0x222630);
        let secondary_hex = self
            .colors
            .get(1)
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(0x41485a);

        let category = match self.category.to_lowercase().as_str() {
            "anime" => "Abstract",
            "people" => "Architecture",
            _ => "Nature",
        }
        .to_string();

        Wallpaper {
            id: format!("wh-{}", self.id),
            name: format!("Wallhaven #{}", self.id),
            author: "Wallhaven Community".to_string(),
            category,
            tags: vec!["wallhaven".to_string(), self.category],
            width: self.dimension_x,
            height: self.dimension_y,
            orientation: Orientation::from_dimensions(self.dimension_x, self.dimension_y),
            source: WallpaperSource::Wallhaven,
            primary_hex,
            secondary_hex,
            thumb_url: Some(self.thumbs.large),
            download_url: Some(self.path),
            preview_path: None,
            local_path: None,
            file_size_bytes: Some(self.file_size),
            popularity: self.favorites,
            created_order: 500u32.saturating_sub(index as u32),
            is_favorite: false,
            is_downloaded: false,
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<u32> {
    u32::from_str_radix(hex.trim_start_matches('#'), 16).ok()
}
