# Wallzy

An open-source, fast desktop wallpaper downloader and manager written in **Rust** and built on **GPUI** (`gpui-kit`).

## Features

- **Curated & Offline-Ready Gallery**: Ships with procedural atmospheric wallpapers and cached PNG previews so the app works immediately out of the box.
- **7 Built-in Wallpaper Providers**:
  - **Wallzy Curated**: Offline-ready atmospheric wallpapers with procedural PNG rendering.
  - **Local Library**: Scans your download directory and custom folders with real image dimension detection.
  - **Bing Daily**: UHD (`3840x2160`) daily photography via Bing's image archive.
  - **Wallhaven**: Community wallpapers via Wallhaven's v1 API (optional API key support).
  - **Reddit**: SFW community posts from `r/wallpapers` and `r/MinimalWallpaper`.
  - **Wikimedia Commons**: High-resolution Creative Commons & public-domain featured pictures.
  - **NASA Image Library**: Public-domain deep space, nebula, and Earth imagery.
- **Powerful Filtering & Sorting**: Real-time search across names, authors, and `#tags`, plus filter pills for Category, Resolution (`1080p+`, `2K+`, `4K+`, `5K+`), Aspect Ratio, Source, and Sort Order.
- **Preview Inspector Modal**: High-contrast preview canvas with Previous/Next navigation (`←` / `→`, `Esc` to close), color palette swatches, clickable tags, resolution/aspect ratio specs, and fit mode selection.
- **Download Manager**: Dedicated Downloads queue with progress feedback, destination folder management, retry support, and file-manager reveal.
- **Native Desktop Wallpaper Setter**: Apply wallpapers directly to your desktop with configurable fit modes (`Fill`, `Fit`, `Stretch`, `Center`, `Tile`) on Windows, macOS, and Linux.
- **Customizable UI**: Light, Dark, and System theme modes, 5 accent color presets, 3 grid density presets (`Compact`, `Comfortable`, `Spacious`), and persistent JSON configuration.

## Project Structure

```text
src/
├── main.rs                 # App entry point and GPUI window setup
├── app.rs                  # Root WallzyApp state & async task coordinator
├── theme.rs                # Color palette tokens & gpui-component theme sync
├── models/                 # Domain structs, filters, settings, and download types
│   ├── download.rs
│   ├── navigation.rs
│   ├── settings.rs
│   └── wallpaper.rs
├── providers/              # Pluggable wallpaper sources (`WallpaperProvider` trait)
│   ├── bing.rs
│   ├── curated.rs
│   ├── local.rs
│   ├── nasa.rs
│   ├── reddit.rs
│   ├── wallhaven.rs
│   └── wikimedia.rs
├── services/               # Persistence, procedural artwork, downloads, OS integration
│   ├── artwork.rs
│   ├── downloader.rs
│   ├── storage.rs
│   └── wallpaper_setter.rs
└── ui/                     # Modular GPUI views and components
    ├── card.rs
    ├── downloads_view.rs
    ├── filter_bar.rs
    ├── gallery.rs
    ├── preview_modal.rs
    ├── settings_view.rs
    ├── sidebar.rs
    └── topbar.rs
```

## Getting Started

### Prerequisites

- Rust toolchain (Edition 2024, stable `1.85+`)
- `curl` in `PATH` (optional, used for fetching online Wallhaven wallpapers)

### Run Locally

```bash
cargo run
```

### Run Tests

```bash
cargo test
```

## Contributing

Contributions are warmly welcomed! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on adding new wallpaper providers, UI views, and running checks.

## License

MIT
