use crate::models::{NavSection, Orientation, Wallpaper, WallpaperSource};
use crate::providers::{ProviderQuery, WallpaperProvider, curl_get_bytes, percent_encode};
use serde::Deserialize;

/// Community wallpaper provider backed by Reddit's public JSON feeds (`r/wallpapers`, `r/MinimalWallpaper`).
pub struct RedditProvider;

impl WallpaperProvider for RedditProvider {
    fn id(&self) -> &'static str {
        "reddit"
    }

    fn display_name(&self) -> &'static str {
        "Reddit r/wallpapers"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        let q = query.search.trim();
        let url = if q.is_empty() {
            let listing = match query.section {
                Some(NavSection::Latest) => "new",
                Some(NavSection::Trending) => "top",
                _ => "hot",
            };
            format!(
                "https://www.reddit.com/r/wallpapers+MinimalWallpaper/{}.json?limit=25&t=month",
                listing
            )
        } else {
            format!(
                "https://www.reddit.com/r/wallpapers+MinimalWallpaper/search.json?q={}&restrict_sr=1&sort=top&limit=25",
                percent_encode(q)
            )
        };

        let bytes = curl_get_bytes(&url, 12)?;
        parse_reddit_response(&bytes)
    }
}

pub(crate) fn parse_reddit_response(bytes: &[u8]) -> Result<Vec<Wallpaper>, String> {
    let parsed: RedditListing = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid Reddit JSON response: {}", e))?;

    let mut items = Vec::new();

    for (idx, child) in parsed.data.children.into_iter().enumerate() {
        let post = child.data;
        if post.over_18 {
            continue;
        }
        let image_url = post
            .url_overridden_by_dest
            .filter(|u| is_direct_image_url(u));
        let Some(download_url) = image_url else {
            continue;
        };

        let (width, height) = post
            .preview
            .as_ref()
            .and_then(|p| p.images.first())
            .map(|img| (img.source.width, img.source.height))
            .unwrap_or((1920, 1080));

        let thumb_url = post
            .preview
            .as_ref()
            .and_then(|p| p.images.first())
            .and_then(|img| img.resolutions.last().or(Some(&img.source)))
            .map(|res| res.url.replace("&amp;", "&"))
            .or_else(|| Some(download_url.clone()));

        let category = infer_category(&post.title, &post.subreddit);
        let clean_name = clean_post_title(&post.title);

        items.push(Wallpaper {
            id: format!("reddit-{}", post.id),
            name: clean_name,
            author: format!("u/{}", post.author),
            category,
            tags: vec!["reddit".to_string(), post.subreddit.to_ascii_lowercase()],
            width,
            height,
            orientation: Orientation::from_dimensions(width, height),
            source: WallpaperSource::Reddit,
            primary_hex: 0x262936,
            secondary_hex: 0x4b536b,
            thumb_url,
            download_url: Some(download_url),
            preview_path: None,
            local_path: None,
            file_size_bytes: None,
            popularity: post.score.max(0) as u32,
            created_order: 800u32.saturating_sub(idx as u32),
            is_favorite: false,
            is_downloaded: false,
        });
    }

    Ok(items)
}

fn is_direct_image_url(url: &str) -> bool {
    let lower = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
}

fn clean_post_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.len() > 64 {
        format!("{}…", &trimmed[..63])
    } else {
        trimmed.to_string()
    }
}

fn infer_category(title: &str, subreddit: &str) -> String {
    if subreddit.eq_ignore_ascii_case("MinimalWallpaper") {
        return "Minimal".to_string();
    }
    let lower = title.to_ascii_lowercase();
    if lower.contains("space") || lower.contains("nebula") || lower.contains("galaxy") {
        "Space"
    } else if lower.contains("city") || lower.contains("cyber") || lower.contains("building") {
        "Architecture"
    } else if lower.contains("abstract") || lower.contains("render") {
        "Abstract"
    } else if lower.contains("minimal") {
        "Minimal"
    } else {
        "Nature"
    }
    .to_string()
}

#[derive(Debug, Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(Debug, Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    id: String,
    title: String,
    author: String,
    subreddit: String,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    over_18: bool,
    url_overridden_by_dest: Option<String>,
    preview: Option<RedditPreview>,
}

#[derive(Debug, Deserialize)]
struct RedditPreview {
    images: Vec<RedditPreviewImage>,
}

#[derive(Debug, Deserialize)]
struct RedditPreviewImage {
    source: RedditResolution,
    #[serde(default)]
    resolutions: Vec<RedditResolution>,
}

#[derive(Debug, Deserialize)]
struct RedditResolution {
    url: String,
    width: u32,
    height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reddit_json_and_filters_nsfw_and_non_images() {
        let sample = br#"{
            "data": {
                "children": [
                    {
                        "data": {
                            "id": "abc1",
                            "title": "Misty Forest Morning [3840x2160]",
                            "author": "forest_fan",
                            "subreddit": "wallpapers",
                            "score": 420,
                            "over_18": false,
                            "url_overridden_by_dest": "https://i.redd.it/sample1.jpg",
                            "preview": {
                                "images": [{
                                    "source": { "url": "https://preview.redd.it/orig.jpg", "width": 3840, "height": 2160 },
                                    "resolutions": [{ "url": "https://preview.redd.it/thumb.jpg?s=1&amp;w=640", "width": 640, "height": 360 }]
                                }]
                            }
                        }
                    },
                    {
                        "data": {
                            "id": "nsfw1",
                            "title": "Filtered",
                            "author": "someone",
                            "subreddit": "wallpapers",
                            "score": 100,
                            "over_18": true,
                            "url_overridden_by_dest": "https://i.redd.it/nsfw.jpg"
                        }
                    },
                    {
                        "data": {
                            "id": "gal1",
                            "title": "Gallery link",
                            "author": "someone",
                            "subreddit": "wallpapers",
                            "score": 50,
                            "over_18": false,
                            "url_overridden_by_dest": "https://www.reddit.com/gallery/xyz"
                        }
                    }
                ]
            }
        }"#;

        let items = parse_reddit_response(sample).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "reddit-abc1");
        assert_eq!(items[0].width, 3840);
        assert_eq!(items[0].height, 2160);
        assert_eq!(items[0].source, WallpaperSource::Reddit);
        assert_eq!(
            items[0].thumb_url.as_deref(),
            Some("https://preview.redd.it/thumb.jpg?s=1&w=640")
        );
    }
}
