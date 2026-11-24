pub mod alphacoders;
pub mod bing;
pub mod curated;
pub mod custom;
pub mod local;
pub mod nasa;
pub mod reddit;
pub mod wallhaven;
pub mod wikimedia;

use crate::models::{AppSettings, NavSection, Wallpaper};
use std::process::Command;

/// Query parameters passed to wallpaper providers.
#[derive(Clone, Debug, Default)]
pub struct ProviderQuery {
    pub search: String,
    pub section: Option<NavSection>,
    pub api_key: String,
    pub page: u32,
}

/// Extension point for adding new wallpaper sources to Wallzy.
///
/// Contributors can implement this trait in `src/providers/<source>.rs` and register
/// the provider metadata in [`registered_providers`].
pub trait WallpaperProvider: Send + Sync {
    /// Stable lowercase identifier for this provider (e.g. `"wallhaven"`).
    fn id(&self) -> &'static str;

    /// Human-readable provider name displayed in the UI.
    fn display_name(&self) -> &'static str;

    /// Owned display name (overridden by user-configured dynamic providers).
    fn display_name_owned(&self) -> String {
        self.display_name().to_string()
    }

    /// Fetches wallpapers matching the requested query.
    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String>;
}

/// Returns metadata (`id`, `display_name`) for all built-in providers.
pub fn registered_providers(
    preview_cache_dir: std::path::PathBuf,
    local_dirs: Vec<std::path::PathBuf>,
) -> Vec<(&'static str, &'static str)> {
    let providers: Vec<Box<dyn WallpaperProvider>> = vec![
        Box::new(curated::CuratedProvider::new(preview_cache_dir)),
        Box::new(local::LocalProvider::new(local_dirs)),
        Box::new(bing::BingProvider),
        Box::new(wallhaven::WallhavenProvider),
        Box::new(alphacoders::AlphaCodersProvider::default()),
        Box::new(reddit::RedditProvider),
        Box::new(wikimedia::WikimediaProvider),
        Box::new(nasa::NasaProvider),
    ];
    providers
        .into_iter()
        .map(|p| (p.id(), p.display_name()))
        .collect()
}

/// Fetches all enabled online providers concurrently and returns merged results plus any per-provider errors.
pub fn fetch_enabled_online(
    settings: &AppSettings,
    query: &ProviderQuery,
) -> (Vec<Wallpaper>, Vec<String>) {
    let mut active: Vec<Box<dyn WallpaperProvider>> = Vec::new();
    if settings.enable_bing {
        active.push(Box::new(bing::BingProvider));
    }
    if settings.enable_wallhaven {
        active.push(Box::new(wallhaven::WallhavenProvider));
    }
    if settings.enable_alpha_coders {
        active.push(Box::new(alphacoders::AlphaCodersProvider::new(
            settings.alpha_coders_api_key.clone(),
        )));
    }
    if settings.enable_reddit {
        active.push(Box::new(reddit::RedditProvider));
    }
    if settings.enable_wikimedia {
        active.push(Box::new(wikimedia::WikimediaProvider));
    }
    if settings.enable_nasa {
        active.push(Box::new(nasa::NasaProvider));
    }
    for custom in &settings.custom_providers {
        if custom.enabled && !custom.endpoint_url.trim().is_empty() {
            active.push(Box::new(custom::CustomJsonProvider::new(custom.clone())));
        }
    }

    if active.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut merged = Vec::new();
    let mut errors = Vec::new();

    std::thread::scope(|scope| {
        let handles: Vec<_> = active
            .iter()
            .map(|provider| {
                let name = provider.display_name_owned();
                let q = query.clone();
                scope.spawn(move || (name, provider.fetch(&q)))
            })
            .collect();

        for handle in handles {
            match handle.join() {
                Ok((_, Ok(items))) => merged.extend(items),
                Ok((name, Err(err))) => errors.push(format!("{}: {}", name, err)),
                Err(_) => errors.push("Provider worker thread panicked".to_string()),
            }
        }
    });

    (merged, errors)
}

/// Shared HTTP GET helper via system `curl` so providers stay dependency-light.
pub(crate) fn curl_get_bytes(url: &str, timeout_secs: u32) -> Result<Vec<u8>, String> {
    let timeout = timeout_secs.to_string();
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            &timeout,
            "-A",
            "Wallzy/0.1 (Open Source Wallpaper Manager; https://github.com/PiyushAnand/wallzy)",
            url,
        ])
        .output()
        .map_err(|e| format!("Failed to invoke curl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_string());
    }
    Ok(output.stdout)
}

/// Shared URL query encoder for provider endpoints.
pub(crate) fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}
