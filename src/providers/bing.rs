use crate::models::{Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes};
use serde::Deserialize;

/// Bing Daily Wallpaper provider via the public `HPImageArchive` JSON endpoint.
///
/// Serves UHD (`3840x2160`) daily photography with rich titles and photographer credits.
pub struct BingProvider;

impl WallpaperProvider for BingProvider {
    fn id(&self) -> &'static str {
        "bing"
    }

    fn display_name(&self) -> &'static str {
        "Bing Daily"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let url = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=8&mkt=en-US";
        let bytes = curl_get_bytes(url, 10)?;
        parse_bing_response(&bytes, &query.search)
    }
}

pub(crate) fn parse_bing_response(bytes: &[u8], search: &str) -> Result<Vec<Wallpaper>, String> {
    let parsed: BingResponse =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid Bing JSON response: {}", e))?;

    let q = search.trim().to_lowercase();
    let mut items = Vec::new();

    for (idx, img) in parsed.images.into_iter().enumerate() {
        let (title, author) = split_title_copyright(&img.title, &img.copyright);
        if !q.is_empty()
            && !title.to_lowercase().contains(&q)
            && !author.to_lowercase().contains(&q)
        {
            continue;
        }

        let base = format!("https://www.bing.com{}", img.urlbase);
        let download_url = format!("{}_UHD.jpg", base);
        let thumb_url = format!("{}_800x480.jpg", base);

        items.push(Wallpaper {
            id: format!("bing-{}", img.startdate),
            name: title,
            author,
            category: "Nature".to_string(),
            tags: vec!["bing".to_string(), "daily".to_string(), "uhd".to_string()],
            width: 3840,
            height: 2160,
            orientation: Orientation::Landscape,
            source: WallpaperSource::Bing,
            primary_hex: 0x1d2d3e,
            secondary_hex: 0x456b8c,
            thumb_url: Some(thumb_url),
            download_url: Some(download_url),
            preview_path: None,
            local_path: None,
            file_size_bytes: None,
            popularity: 92u32.saturating_sub(idx as u32 * 2),
            created_order: 900u32.saturating_sub(idx as u32),
            is_favorite: false,
            is_downloaded: false,
        });
    }

    Ok(items)
}

fn split_title_copyright(title: &str, copyright: &str) -> (String, String) {
    let clean_title = if title.trim().is_empty() {
        copyright
            .split('(')
            .next()
            .unwrap_or("Bing Daily Spotlight")
            .trim()
            .to_string()
    } else {
        title.trim().to_string()
    };

    let author = copyright
        .split('(')
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .map(|s| s.trim_start_matches("© ").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Bing Archive".to_string());

    (clean_title, author)
}

#[derive(Debug, Deserialize)]
struct BingResponse {
    images: Vec<BingImage>,
}

#[derive(Debug, Deserialize)]
struct BingImage {
    startdate: String,
    urlbase: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    copyright: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bing_daily_json() {
        let sample = r#"{
            "images": [
                {
                    "startdate": "20261124",
                    "urlbase": "/th?id=OHR.AlpineGlow_EN-US1234",
                    "title": "Alpine Glow",
                    "copyright": "Dolomites at dusk (© Jane Doe/Getty Images)"
                }
            ]
        }"#
        .as_bytes();

        let items = parse_bing_response(sample, "").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "bing-20261124");
        assert_eq!(items[0].name, "Alpine Glow");
        assert_eq!(items[0].author, "Jane Doe/Getty Images");
        assert_eq!(items[0].source, WallpaperSource::Bing);
        assert!(
            items[0]
                .download_url
                .as_deref()
                .unwrap()
                .ends_with("_UHD.jpg")
        );
    }
}
