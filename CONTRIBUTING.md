# Contributing to Wallzy

Thank you for helping build **Wallzy**! The codebase is intentionally split into small, single-responsibility modules so new features and wallpaper sources are straightforward to add.

## Architecture Overview

- `src/models/`: Pure data structures (`Wallpaper`, `WallpaperFilter`, `AppSettings`, `DownloadItem`, `NavSection`). Keep UI and IO logic out of `models/`.
- `src/providers/`: Implements the `WallpaperProvider` trait (`src/providers/mod.rs`). Each wallpaper source lives in its own file.
- `src/services/`: IO and OS integrations (`storage.rs`, `downloader.rs`, `wallpaper_setter.rs`, `artwork.rs`).
- `src/ui/`: Stateless view renderers that take `&WallzyApp`, `ThemePalette`, and `&mut Context<WallzyApp>`.

## How to Add a New Wallpaper Provider

1. Create a new module in `src/providers/<your_source>.rs` and export it in `src/providers/mod.rs`.
2. Add a variant to `WallpaperSource` in `src/models/wallpaper.rs` (and wire its label in `WallpaperSource::label`).
3. Implement `WallpaperProvider`:

```rust
use crate::models::Wallpaper;
use crate::providers::{ProviderQuery, WallpaperProvider};

pub struct MySourceProvider;

impl WallpaperProvider for MySourceProvider {
    fn id(&self) -> &'static str {
        "my-source"
    }

    fn display_name(&self) -> &'static str {
        "My Source"
    }

    fn fetch(&self, query: &ProviderQuery) -> Result<Vec<Wallpaper>, String> {
        todo!("Fetch and map wallpapers into Vec<Wallpaper>")
    }
}
```

4. Register the provider fetch call in `WallzyApp` (`src/app.rs`) and add any optional settings fields to `AppSettings` (`src/models/settings.rs`).

## How to Add a UI Section or View

1. Add a variant to `NavSection` in `src/models/navigation.rs` with its `label()`, `subtitle()`, and `icon()`.
2. Create a dedicated render module in `src/ui/<view_name>.rs`.
3. Route the new `NavSection` in `WallzyApp::render` (`src/app.rs`).

## Development Checks

Before opening a pull request, please verify:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```
