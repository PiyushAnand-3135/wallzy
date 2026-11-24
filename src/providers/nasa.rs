use crate::models::{Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes, percent_encode};
use serde::Deserialize;

/// Public-domain astronomy & Earth photography via NASA's Image and Video Library API.
pub struct NasaProvider;

impl WallpaperProvider for NasaProvider {
    fn id(&self) -> &'static str {
        "nasa"
    }

    fn display_name(&self) -> &'static str {
        "NASA Image Library"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let q = if query.search.trim().is_empty() {
            "nebula galaxy"
        } else {
            query.search.trim()
        };
        let url = format!(
            "https://images-api.nasa.gov/search?media_type=image&page=1&page_size=18&q={}",
            percent_encode(q)
        );
        let bytes = curl_get_bytes(&url, 12)?;
        parse_nasa_response(&bytes)
    }
}

pub(crate) fn parse_nasa_response(bytes: &[u8]) -> Result<Vec<Wallpaper>, String> {
    let parsed: NasaResponse =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid NASA JSON response: {}", e))?;

    let mut items = Vec::new();
    for (idx, item) in parsed.collection.items.into_iter().enumerate() {
        let Some(meta) = item.data.into_iter().next() else {
            continue;
        };
        let thumb_url = item
            .links
            .into_iter()
            .find(|l| l.render.as_deref() == Some("image"))
            .map(|l| l.href);
        let Some(preview_url) = thumb_url else {
            continue;
        };

        // NASA's `~thumb.jpg` / `~medium.jpg` links have a sibling `~orig.jpg` or `~large.jpg` asset.
        let download_url = if preview_url.contains("~thumb.") {
            preview_url.replace("~thumb.", "~large.")
        } else if preview_url.contains("~medium.") {
            preview_url.replace("~medium.", "~large.")
        } else {
            preview_url.clone()
        };

        let mut tags = vec!["nasa".to_string(), "space".to_string()];
        for kw in meta.keywords.into_iter().take(3) {
            let lower = kw.trim().to_ascii_lowercase();
            if !lower.is_empty() && !tags.contains(&lower) {
                tags.push(lower);
            }
        }

        items.push(Wallpaper {
            id: format!("nasa-{}", meta.nasa_id),
            name: meta.title,
            author: meta
                .center
                .map(|c| format!("NASA {}", c))
                .unwrap_or_else(|| "NASA".to_string()),
            category: "Space".to_string(),
            tags,
            width: 2560,
            height: 1440,
            orientation: Orientation::Landscape,
            source: WallpaperSource::Nasa,
            primary_hex: 0x0d1224,
            secondary_hex: 0x3b4d8f,
            thumb_url: Some(preview_url),
            download_url: Some(download_url),
            preview_path: None,
            local_path: None,
            file_size_bytes: None,
            popularity: 84,
            created_order: 650u32.saturating_sub(idx as u32),
            is_favorite: false,
            is_downloaded: false,
        });
    }

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct NasaResponse {
    collection: NasaCollection,
}

#[derive(Debug, Deserialize)]
struct NasaCollection {
    items: Vec<NasaItem>,
}

#[derive(Debug, Deserialize)]
struct NasaItem {
    #[serde(default)]
    data: Vec<NasaItemData>,
    #[serde(default)]
    links: Vec<NasaLink>,
}

#[derive(Debug, Deserialize)]
struct NasaItemData {
    nasa_id: String,
    title: String,
    center: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NasaLink {
    href: String,
    render: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nasa_library_json() {
        let sample = br#"{
            "collection": {
                "items": [
                    {
                        "data": [{
                            "nasa_id": "PIA12345",
                            "title": "Carina Nebula Cliffs",
                            "center": "GSFC",
                            "keywords": ["JWST", "Nebula"]
                        }],
                        "links": [{
                            "href": "https://images-assets.nasa.gov/image/PIA12345/PIA12345~thumb.jpg",
                            "render": "image"
                        }]
                    }
                ]
            }
        }"#;

        let items = parse_nasa_response(sample).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "nasa-PIA12345");
        assert_eq!(items[0].author, "NASA GSFC");
        assert_eq!(items[0].source, WallpaperSource::Nasa);
        assert_eq!(
            items[0].download_url.as_deref(),
            Some("https://images-assets.nasa.gov/image/PIA12345/PIA12345~large.jpg")
        );
    }
}
