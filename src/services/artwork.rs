use crate::models::Wallpaper;
use image::{ImageBuffer, Rgb};
use std::fs;
use std::path::{Path, PathBuf};

/// Generates a deterministic, atmospheric PNG preview thumbnail for a curated wallpaper
/// so cards display rich artwork even when offline.
pub fn ensure_curated_preview(wallpaper: &Wallpaper, cache_dir: &Path) -> Option<PathBuf> {
    fs::create_dir_all(cache_dir).ok()?;
    let safe_id: String = wallpaper
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = cache_dir.join(format!("{}_thumb.png", safe_id));
    if path.exists() {
        return Some(path);
    }

    let (w, h) = match wallpaper.orientation {
        crate::models::Orientation::Portrait => (300, 420),
        crate::models::Orientation::Ultrawide => (560, 240),
        crate::models::Orientation::Square => (360, 360),
        crate::models::Orientation::Landscape => (480, 270),
    };

    if render_wallpaper_png(wallpaper, w, h, &path).is_ok() {
        Some(path)
    } else {
        None
    }
}

/// Generates a crisp 720p HD preview image specifically for the Featured Hero Spotlight banner.
pub fn ensure_curated_hero(wallpaper: &Wallpaper, cache_dir: &Path) -> Option<PathBuf> {
    fs::create_dir_all(cache_dir).ok()?;
    let safe_id: String = wallpaper
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = cache_dir.join(format!("{}_hero.png", safe_id));
    if path.exists() {
        return Some(path);
    }
    if render_wallpaper_png(wallpaper, 1280, 720, &path).is_ok() {
        Some(path)
    } else {
        None
    }
}

/// Renders a smooth multi-layer procedural wallpaper PNG using the wallpaper's palette and category.
pub fn render_wallpaper_png(
    wallpaper: &Wallpaper,
    width: u32,
    height: u32,
    dest: &Path,
) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }

    let c1 = unpack_rgb(wallpaper.primary_hex);
    let c2 = unpack_rgb(wallpaper.secondary_hex);
    let seed = hash_id(&wallpaper.id);
    let style_kind = match wallpaper.category.as_str() {
        "Nature" => 0,
        "Architecture" => 1,
        "Minimal" => 2,
        "Abstract" => 3,
        "Space" => 4,
        _ => 5,
    };

    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(width, height);
    let wf = width.max(1) as f32;
    let hf = height.max(1) as f32;

    for (y, row) in img.rows_mut().enumerate() {
        let ny = y as f32 / hf;
        for (x, pixel) in row.enumerate() {
            let nx = x as f32 / wf;
            let (mut r, mut g, mut b) = match style_kind {
                // Nature: layered mountain ridges over atmospheric twilight sky
                0 => {
                    let sky_t = (ny * 1.15).clamp(0.0, 1.0);
                    let mut col = lerp_rgb(c1, c2, sky_t);
                    let sun_dx = nx - (0.35 + ((seed % 30) as f32) * 0.01);
                    let sun_dy = (ny - 0.34) * (hf / wf);
                    let glow = (-((sun_dx * sun_dx + sun_dy * sun_dy) * 14.0)).exp();
                    col = add_glow(col, (255.0, 215.0, 175.0), glow * 0.38);

                    let ridge1 =
                        0.56 + 0.09 * (nx * 5.5 + seed as f32).sin() + 0.04 * (nx * 13.0).cos();
                    let ridge2 = 0.72
                        + 0.07 * (nx * 7.2 + (seed as f32) * 1.7).cos()
                        + 0.03 * (nx * 18.0).sin();

                    if ny > ridge2 {
                        scale_rgb(c1, 0.36)
                    } else if ny > ridge1 {
                        scale_rgb(lerp_rgb(c1, c2, 0.4), 0.58)
                    } else {
                        col
                    }
                }
                // Architecture: crisp brutalist diagonal light shafts
                1 => {
                    let diag = (nx * 0.65 + ny * 0.85).clamp(0.0, 1.0);
                    let mut col = lerp_rgb(c1, c2, diag);
                    let beam = ((nx * 3.0 - ny * 1.6 + (seed % 5) as f32 * 0.2).sin() * 0.5 + 0.5)
                        .powf(4.0);
                    col = add_glow(col, (235.0, 240.0, 250.0), beam * 0.22);
                    if nx > 0.62 && ny > 0.25 {
                        col = scale_rgb(col, 0.72);
                    }
                    col
                }
                // Minimal: soft horizon orb & calm gradient
                2 => {
                    let t = (ny * 0.85 + nx * 0.15).clamp(0.0, 1.0);
                    let mut col = lerp_rgb(c1, c2, t);
                    let dx = nx - 0.5;
                    let dy = (ny - 0.46) * (hf / wf);
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < 0.19 {
                        let edge = ((0.19 - dist) * 60.0).clamp(0.0, 1.0);
                        col = lerp_rgb(col, (242.0, 236.0, 226.0), edge * 0.55);
                    }
                    col
                }
                // Abstract: flowing harmonic silk ribbons
                3 => {
                    let wave =
                        (nx * 4.5 + (ny * 3.2).sin() * 1.4 + seed as f32 * 0.3).sin() * 0.5 + 0.5;
                    let wave2 =
                        (ny * 5.0 - (nx * 2.8).cos() * 1.2 + seed as f32 * 0.7).cos() * 0.5 + 0.5;
                    let mix = (wave * 0.6 + wave2 * 0.4).clamp(0.0, 1.0);
                    lerp_rgb(c1, c2, mix)
                }
                // Space: deep nebula glow + deterministic starfield
                4 => {
                    let neb =
                        ((nx * 2.5 + seed as f32).sin() * (ny * 2.5).cos() * 0.5 + 0.5).powf(1.6);
                    let mut col = lerp_rgb(c1, c2, neb);
                    let star_hash = ((x as u32).wrapping_mul(374761393))
                        ^ ((y as u32).wrapping_mul(668265263))
                        ^ seed;
                    if star_hash.is_multiple_of(650) {
                        col = (245.0, 248.0, 255.0);
                    }
                    col
                }
                // Default smooth radial gradient
                _ => {
                    let t = (nx * 0.5 + ny * 0.5).clamp(0.0, 1.0);
                    lerp_rgb(c1, c2, t)
                }
            };

            // Subtle deterministic dithering to prevent gradient banding
            let dither = (((x ^ y).wrapping_mul(2654435761) >> 24) as f32 / 255.0 - 0.5) * 3.0;
            r = (r + dither).clamp(0.0, 255.0);
            g = (g + dither).clamp(0.0, 255.0);
            b = (b + dither).clamp(0.0, 255.0);
            *pixel = Rgb([r as u8, g as u8, b as u8]);
        }
    }

    img.save(dest)
        .map_err(|e| format!("Failed to save rendered PNG {}: {}", dest.display(), e))
}

fn unpack_rgb(hex: u32) -> (f32, f32, f32) {
    (
        ((hex >> 16) & 0xff) as f32,
        ((hex >> 8) & 0xff) as f32,
        (hex & 0xff) as f32,
    )
}

fn lerp_rgb(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

fn scale_rgb(a: (f32, f32, f32), factor: f32) -> (f32, f32, f32) {
    (a.0 * factor, a.1 * factor, a.2 * factor)
}

fn add_glow(base: (f32, f32, f32), glow: (f32, f32, f32), amount: f32) -> (f32, f32, f32) {
    (
        base.0 + (glow.0 - base.0) * amount,
        base.1 + (glow.1 - base.1) * amount,
        base.2 + (glow.2 - base.2) * amount,
    )
}

fn hash_id(id: &str) -> u32 {
    id.bytes().fold(2166136261u32, |acc, b| {
        acc.wrapping_mul(16777619) ^ b as u32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::curated::curated_catalog;

    #[test]
    fn procedural_artwork_renders_valid_png() {
        let temp_dir = std::env::temp_dir().join("wallzy_test_artwork");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let wp = curated_catalog().into_iter().next().unwrap();
        let out = temp_dir.join("preview.png");
        render_wallpaper_png(&wp, 160, 90, &out).unwrap();

        let (w, h) = image::image_dimensions(&out).unwrap();
        assert_eq!((w, h), (160, 90));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
