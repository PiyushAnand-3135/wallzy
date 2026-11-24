use crate::models::{NavSection, Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes, percent_encode};
use serde::Deserialize;

/// Wallpaper Abyss / Alpha Coders v4.0 API provider (`https://api.alphacoders.com/4.0`).
///
/// Works out of the box using Alpha Coders' unauthenticated highest-rated endpoint,
/// and unlocks full search/sorting/pagination when an Alpha Coders API key is configured.
#[derive(Default)]
pub struct AlphaCodersProvider {
    api_key: String,
}

impl AlphaCodersProvider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl WallpaperProvider for AlphaCodersProvider {
    fn id(&self) -> &'static str {
        "alphacoders"
    }

    fn display_name(&self) -> &'static str {
        "Alpha Coders"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let key = self.api_key.trim();
        let search = query.search.trim();
        let page = query.page.max(1);
        let sort = match query.section {
            Some(NavSection::Latest) => "newest",
            _ => "highest_rated",
        };

        let url = if !key.is_empty() {
            if search.is_empty() {
                format!(
                    "https://api.alphacoders.com/4.0/contents?content_type=wallpaper&sort={}&page={}&auth={}",
                    sort,
                    page,
                    percent_encode(key)
                )
            } else {
                format!(
                    "https://api.alphacoders.com/4.0/search?content_type=wallpaper&sort={}&term={}&page={}&auth={}",
                    sort,
                    percent_encode(search),
                    page,
                    percent_encode(key)
                )
            }
        } else {
            // Public unauthenticated showcase endpoint documented at api.alphacoders.com/api/instructions
            "https://api.alphacoders.com/4.0/contents?content_type=wallpaper&sort=highest_rated"
                .to_string()
        };

        let bytes = curl_get_bytes(&url, 12)?;
        let local_filter = if key.is_empty() { search } else { "" };
        parse_alphacoders_response(&bytes, local_filter)
    }
}

pub(crate) fn parse_alphacoders_response(
    bytes: &[u8],
    search_filter: &str,
) -> Result<Vec<Wallpaper>, String> {
    let parsed: AlphaCodersResponse = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid Alpha Coders JSON response: {}", e))?;

    if parsed.success == Some(false) {
        return Err(parsed
            .error
            .unwrap_or_else(|| "Alpha Coders API returned an error".to_string()));
    }

    let q = search_filter.trim().to_lowercase();
    let mut items = Vec::new();

    for (idx, item) in parsed.contents.into_iter().enumerate() {
        if item.src.trim().is_empty() {
            continue;
        }
        let title = if item.name.trim().is_empty() {
            format!("Alpha Coders #{}", item.content_id)
        } else {
            item.name.trim().to_string()
        };
        let author = item
            .submitted_by
            .and_then(|u| u.name)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Alpha Coders".to_string());

        let mut tags = vec!["alphacoders".to_string()];
        for tag in item.tags.into_iter().take(4) {
            let clean = tag.name.trim().to_ascii_lowercase();
            if !clean.is_empty() && !tags.contains(&clean) {
                tags.push(clean);
            }
        }

        if !q.is_empty() {
            let matches_text = title.to_lowercase().contains(&q)
                || author.to_lowercase().contains(&q)
                || item.alt_text.to_lowercase().contains(&q)
                || tags.iter().any(|t| t.contains(&q));
            if !matches_text {
                continue;
            }
        }

        let category = infer_category(&tags, &title);
        let width = item.width.max(1);
        let height = item.height.max(1);
        let thumb_url = item
            .thumb_800
            .or(item.thumb_350)
            .or(item.thumb)
            .or_else(|| Some(item.src.clone()));

        items.push(Wallpaper {
            id: format!("ac-{}", item.content_id),
            name: title,
            author,
            category,
            tags,
            width,
            height,
            orientation: Orientation::from_dimensions(width, height),
            source: WallpaperSource::AlphaCoders,
            primary_hex: 0x1b2130,
            secondary_hex: 0x3b4968,
            thumb_url,
            download_url: Some(item.src),
            preview_path: None,
            local_path: None,
            file_size_bytes: item.file_size,
            popularity: item.upvote_count.saturating_add(item.favorite_count),
            created_order: 950u32.saturating_sub(idx as u32),
            is_favorite: false,
            is_downloaded: false,
        });
    }

    Ok(items)
}

fn infer_category(tags: &[String], title: &str) -> String {
    let hay = format!("{} {}", tags.join(" "), title.to_lowercase());
    if hay.contains("space") || hay.contains("nebula") || hay.contains("galaxy") {
        "Space".to_string()
    } else if hay.contains("minimal") {
        "Minimal".to_string()
    } else if hay.contains("anime") || hay.contains("abstract") || hay.contains("digital") {
        "Abstract".to_string()
    } else if hay.contains("city") || hay.contains("architecture") || hay.contains("building") {
        "Architecture".to_string()
    } else {
        "Nature".to_string()
    }
}

#[derive(Debug, Deserialize)]
struct AlphaCodersResponse {
    #[serde(default)]
    success: Option<bool>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    contents: Vec<AlphaCodersContent>,
}

#[derive(Debug, Deserialize)]
struct AlphaCodersContent {
    content_id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    alt_text: String,
    src: String,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    upvote_count: u32,
    #[serde(default)]
    favorite_count: u32,
    #[serde(default)]
    thumb: Option<String>,
    #[serde(default)]
    thumb_350: Option<String>,
    #[serde(default)]
    thumb_800: Option<String>,
    #[serde(default)]
    submitted_by: Option<AlphaCodersUser>,
    #[serde(default)]
    tags: Vec<AlphaCodersTag>,
}

#[derive(Debug, Deserialize)]
struct AlphaCodersUser {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AlphaCodersTag {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_alphacoders_v4_contents_json() {
        let sample = r#"{
            "success": true,
            "contents": [
                {
                    "content_id": 1699286,
                    "name": "Nebula Horizon",
                    "alt_text": "Cosmic nebula scene",
                    "src": "https://images.alphacoders.com/605/605592.png",
                    "width": 3840,
                    "height": 2160,
                    "file_size": 13547511,
                    "upvote_count": 1200,
                    "favorite_count": 300,
                    "thumb_800": "https://images.alphacoders.com/605/thumb-800-605592.png",
                    "submitted_by": { "name": "drak95" },
                    "tags": [{ "name": "Space" }, { "name": "Nebula" }]
                }
            ]
        }"#;

        let items = parse_alphacoders_response(sample.as_bytes(), "").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "ac-1699286");
        assert_eq!(items[0].source, WallpaperSource::AlphaCoders);
        assert_eq!(items[0].category, "Space");
        assert_eq!(items[0].author, "drak95");
        assert_eq!(items[0].width, 3840);
        assert_eq!(items[0].popularity, 1500);
    }
}
