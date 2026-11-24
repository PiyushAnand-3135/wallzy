use crate::models::{CustomProviderConfig, Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes, percent_encode};
use serde_json::Value;

/// Generic HTTP JSON wallpaper feed provider configured by users in Settings.
///
/// Accepts JSON arrays or objects containing `wallpapers`, `contents`, `items`, `data`,
/// `results`, `images`, or `posts`, and supports `{query}` URL templates.
pub struct CustomJsonProvider {
    config: CustomProviderConfig,
}

impl CustomJsonProvider {
    pub fn new(config: CustomProviderConfig) -> Self {
        Self { config }
    }
}

impl WallpaperProvider for CustomJsonProvider {
    fn id(&self) -> &'static str {
        "custom"
    }

    fn display_name(&self) -> &'static str {
        "Custom Feed"
    }

    fn display_name_owned(&self) -> String {
        self.config.name.clone()
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let template = self.config.endpoint_url.trim();
        if !(template.starts_with("http://") || template.starts_with("https://")) {
            return Err("Endpoint URL must start with http:// or https://".to_string());
        }

        let has_query_placeholder = template.contains("{query}");
        let url = if has_query_placeholder {
            template.replace("{query}", &percent_encode(query.search.trim()))
        } else {
            template.to_string()
        };

        let bytes = curl_get_bytes(&url, 12)?;
        let local_filter = if has_query_placeholder {
            ""
        } else {
            query.search.trim()
        };
        parse_custom_json_response(&bytes, &self.config, local_filter)
    }
}

pub(crate) fn parse_custom_json_response(
    bytes: &[u8],
    config: &CustomProviderConfig,
    search_filter: &str,
) -> Result<Vec<Wallpaper>, String> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid JSON from {}: {}", config.name, e))?;

    let raw_items = extract_item_array(&root).ok_or_else(|| {
        "Expected a JSON array or object with items/wallpapers/data/results".to_string()
    })?;

    let q = search_filter.trim().to_lowercase();
    let provider_tag = slugify(&config.name);
    let mut results = Vec::new();

    for (idx, item) in raw_items.iter().enumerate() {
        let obj = item.get("data").filter(|d| d.is_object()).unwrap_or(item);
        let Some(download_url) = pick_url(
            obj,
            &[
                "download_url",
                "url_image",
                "src",
                "image_url",
                "full_url",
                "path",
                "hdurl",
                "original",
                "url",
            ],
        ) else {
            continue;
        };

        let thumb_url = pick_url(
            obj,
            &[
                "thumb_url",
                "url_thumb",
                "thumb_800",
                "thumb_350",
                "thumb",
                "thumbnail",
                "small_url",
            ],
        )
        .or_else(|| Some(download_url.clone()));

        let name = pick_str(obj, &["name", "title", "caption", "alt_text"])
            .unwrap_or_else(|| format!("{} #{}", config.name, idx + 1));

        let author = pick_str(obj, &["author", "photographer", "creator", "copyright"])
            .or_else(|| {
                obj.get("submitted_by")
                    .and_then(|u| u.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| config.name.clone());

        let category = pick_str(obj, &["category"]).unwrap_or_else(|| "Nature".to_string());
        let width = pick_u32(obj, &["width", "dimension_x"])
            .unwrap_or(1920)
            .max(1);
        let height = pick_u32(obj, &["height", "dimension_y"])
            .unwrap_or(1080)
            .max(1);

        let mut tags = vec![provider_tag.clone()];
        if let Some(arr) = obj.get("tags").and_then(|t| t.as_array()) {
            for entry in arr.iter().take(4) {
                let tag_text = entry
                    .as_str()
                    .or_else(|| entry.get("name").and_then(|n| n.as_str()));
                if let Some(t) = tag_text {
                    let clean = t.trim().to_ascii_lowercase();
                    if !clean.is_empty() && !tags.contains(&clean) {
                        tags.push(clean);
                    }
                }
            }
        }

        if !q.is_empty() {
            let matches = name.to_lowercase().contains(&q)
                || author.to_lowercase().contains(&q)
                || category.to_lowercase().contains(&q)
                || tags.iter().any(|t| t.contains(&q));
            if !matches {
                continue;
            }
        }

        let item_key = pick_str(obj, &["id", "content_id"])
            .or_else(|| {
                obj.get("id")
                    .and_then(|v| v.as_u64())
                    .map(|n| n.to_string())
            })
            .unwrap_or_else(|| idx.to_string());

        results.push(Wallpaper {
            id: format!("custom-{}-{}", config.id, slugify(&item_key)),
            name,
            author,
            category,
            tags,
            width,
            height,
            orientation: Orientation::from_dimensions(width, height),
            source: WallpaperSource::Custom,
            primary_hex: 0x202532,
            secondary_hex: 0x4b556e,
            thumb_url,
            download_url: Some(download_url),
            preview_path: None,
            local_path: None,
            file_size_bytes: obj.get("file_size").and_then(|v| v.as_u64()),
            popularity: pick_u32(obj, &["popularity", "score", "upvote_count"]).unwrap_or(60),
            created_order: 600u32.saturating_sub(idx as u32),
            is_favorite: false,
            is_downloaded: false,
        });
    }

    Ok(results)
}

fn extract_item_array(root: &Value) -> Option<&Vec<Value>> {
    if let Some(arr) = root.as_array() {
        return Some(arr);
    }
    for key in [
        "wallpapers",
        "contents",
        "items",
        "data",
        "results",
        "images",
        "posts",
    ] {
        if let Some(arr) = root.get(key).and_then(|v| v.as_array()) {
            return Some(arr);
        }
    }
    root.get("data")
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
}

fn pick_str(obj: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn pick_url(obj: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
            let trimmed = val.trim();
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn pick_u32(obj: &Value, keys: &[&str]) -> Option<u32> {
    for key in keys {
        if let Some(n) = obj.get(*key).and_then(|v| v.as_u64()) {
            return Some(n.min(u32::MAX as u64) as u32);
        }
    }
    None
}

fn slugify(input: &str) -> String {
    let s: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = s.trim_matches('-');
    if trimmed.is_empty() {
        "feed".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_custom_json_arrays_and_wrapped_objects() {
        let cfg = CustomProviderConfig {
            id: "myfeed".to_string(),
            name: "Studio Picks".to_string(),
            endpoint_url: "https://example.com/wallpapers.json".to_string(),
            enabled: true,
        };

        let sample = r#"{
            "wallpapers": [
                {
                    "id": "aurora-1",
                    "title": "Northern Glow",
                    "author": "Kira",
                    "category": "Nature",
                    "image_url": "https://cdn.example.com/aurora.jpg",
                    "thumb_url": "https://cdn.example.com/aurora_sm.jpg",
                    "width": 3840,
                    "height": 2160,
                    "tags": ["aurora", "night"]
                }
            ]
        }"#;

        let items = parse_custom_json_response(sample.as_bytes(), &cfg, "").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source, WallpaperSource::Custom);
        assert_eq!(items[0].name, "Northern Glow");
        assert_eq!(
            items[0].download_url.as_deref(),
            Some("https://cdn.example.com/aurora.jpg")
        );
        assert_eq!(items[0].width, 3840);
    }
}
