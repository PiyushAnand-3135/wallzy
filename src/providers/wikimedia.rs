use crate::models::{Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes, percent_encode};
use serde::Deserialize;
use std::collections::HashMap;

/// Open-source / Creative Commons Featured Pictures provider via Wikimedia Commons' public MediaWiki API.
pub struct WikimediaProvider;

impl WallpaperProvider for WikimediaProvider {
    fn id(&self) -> &'static str {
        "wikimedia"
    }

    fn display_name(&self) -> &'static str {
        "Wikimedia Commons"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let q = query.search.trim();
        let url = if q.is_empty() {
            "https://commons.wikimedia.org/w/api.php?action=query&generator=categorymembers&gcmtitle=Category:Featured_pictures_on_Wikimedia_Commons&gcmtype=file&gcmlimit=20&prop=imageinfo&iiprop=url|size|mime&iiurlwidth=640&format=json"
                .to_string()
        } else {
            format!(
                "https://commons.wikimedia.org/w/api.php?action=query&generator=search&gsrnamespace=6&gsrsearch={}%20filetype:bitmap&gsrlimit=20&prop=imageinfo&iiprop=url|size|mime&iiurlwidth=640&format=json",
                percent_encode(q)
            )
        };

        let bytes = curl_get_bytes(&url, 12)?;
        parse_wikimedia_response(&bytes)
    }
}

pub(crate) fn parse_wikimedia_response(bytes: &[u8]) -> Result<Vec<Wallpaper>, String> {
    let parsed: WikimediaResponse = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid Wikimedia JSON response: {}", e))?;

    let Some(query) = parsed.query else {
        return Ok(Vec::new());
    };

    let mut items = Vec::new();
    for (idx, (_, page)) in query.pages.into_iter().enumerate() {
        let Some(info) = page.imageinfo.into_iter().next() else {
            continue;
        };
        if !matches!(
            info.mime.as_deref(),
            Some("image/jpeg" | "image/png" | "image/webp")
        ) {
            continue;
        }
        if info.width < 1280 || info.height < 720 {
            continue;
        }

        let clean_name = page
            .title
            .strip_prefix("File:")
            .unwrap_or(&page.title)
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .unwrap_or(&page.title)
            .replace('_', " ");

        items.push(Wallpaper {
            id: format!("wiki-{}", page.pageid),
            name: clean_name,
            author: "Wikimedia Commons".to_string(),
            category: "Nature".to_string(),
            tags: vec![
                "wikimedia".to_string(),
                "commons".to_string(),
                "cc".to_string(),
            ],
            width: info.width,
            height: info.height,
            orientation: Orientation::from_dimensions(info.width, info.height),
            source: WallpaperSource::Wikimedia,
            primary_hex: 0x242c38,
            secondary_hex: 0x52657d,
            thumb_url: info.thumburl.or_else(|| Some(info.url.clone())),
            download_url: Some(info.url),
            preview_path: None,
            local_path: None,
            file_size_bytes: info.size,
            popularity: 78,
            created_order: 700u32.saturating_sub(idx as u32),
            is_favorite: false,
            is_downloaded: false,
        });
    }

    Ok(items)
}

#[derive(Debug, Deserialize)]
struct WikimediaResponse {
    query: Option<WikimediaQuery>,
}

#[derive(Debug, Deserialize)]
struct WikimediaQuery {
    #[serde(default)]
    pages: HashMap<String, WikimediaPage>,
}

#[derive(Debug, Deserialize)]
struct WikimediaPage {
    pageid: u64,
    title: String,
    #[serde(default)]
    imageinfo: Vec<WikimediaImageInfo>,
}

#[derive(Debug, Deserialize)]
struct WikimediaImageInfo {
    url: String,
    thumburl: Option<String>,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    size: Option<u64>,
    mime: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wikimedia_commons_json() {
        let sample = br#"{
            "query": {
                "pages": {
                    "42": {
                        "pageid": 42,
                        "title": "File:Milky_Way_Arch.jpg",
                        "imageinfo": [{
                            "url": "https://upload.wikimedia.org/example/Milky_Way_Arch.jpg",
                            "thumburl": "https://upload.wikimedia.org/example/640px-Milky_Way_Arch.jpg",
                            "width": 4000,
                            "height": 2250,
                            "size": 3145728,
                            "mime": "image/jpeg"
                        }]
                    }
                }
            }
        }"#;

        let items = parse_wikimedia_response(sample).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "wiki-42");
        assert_eq!(items[0].name, "Milky Way Arch");
        assert_eq!(items[0].source, WallpaperSource::Wikimedia);
        assert_eq!(items[0].width, 4000);
    }
}
