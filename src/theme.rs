use crate::models::{AccentColor, ThemeMode};
use gpui_kit::component::{
    Colorize as _, Theme as ComponentTheme, ThemeMode as ComponentThemeMode,
};
use gpui_kit::*;

/// Resolved UI and monospace font families selected from installed system fonts.
#[derive(Clone, Debug)]
pub struct ResolvedTypography {
    pub ui_font: SharedString,
    pub mono_font: SharedString,
}

impl ResolvedTypography {
    /// Picks the cleanest modern sans-serif and monospace fonts available on the host OS.
    pub fn detect(cx: &App) -> Self {
        let installed = cx.text_system().all_font_names();

        let ui_candidates = [
            "Inter",
            "Plus Jakarta Sans",
            "Geist",
            "SF Pro Display",
            "SF Pro Text",
            "Segoe UI Variable Display",
            "Segoe UI Variable Text",
            "Segoe UI Variable",
            "Ubuntu Sans",
            "Cantarell",
            "Noto Sans",
            "Segoe UI",
        ];
        let mono_candidates = [
            "JetBrains Mono",
            "Cascadia Mono",
            "Cascadia Code",
            "Ubuntu Sans Mono",
            "SF Mono",
            "Consolas",
            "DejaVu Sans Mono",
        ];

        let ui_font = ui_candidates
            .iter()
            .find(|name| installed.iter().any(|f| f.eq_ignore_ascii_case(name)))
            .map(|s| SharedString::new_static(s))
            .unwrap_or_else(|| SharedString::new_static(".SystemUIFont"));

        let mono_font = mono_candidates
            .iter()
            .find(|name| installed.iter().any(|f| f.eq_ignore_ascii_case(name)))
            .map(|s| SharedString::new_static(s))
            .unwrap_or_else(|| SharedString::new_static("Consolas"));

        Self { ui_font, mono_font }
    }
}

/// Resolved modern surface, hairline, glass, and typography color tokens for Wallzy.
#[derive(Clone, Copy)]
pub struct ThemePalette {
    pub bg_app: Rgba,
    pub bg_app_top_glow: Rgba,
    pub bg_sidebar: Rgba,
    pub bg_surface: Rgba,
    pub bg_elevated: Rgba,
    pub bg_popover: Rgba,
    pub bg_hover: Rgba,
    pub hairline: Rgba,
    pub hairline_strong: Rgba,
    pub text_primary: Rgba,
    pub text_muted: Rgba,
    pub text_subtle: Rgba,
    pub text_on_accent: Rgba,
    pub accent: Rgba,
    pub accent_soft: Rgba,
    pub glass_dark: Rgba,
    pub glass_border: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
    pub danger: Rgba,
}

impl ThemePalette {
    pub fn is_dark_mode(mode: ThemeMode, window: &Window) -> bool {
        match mode {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
        }
    }

    pub fn resolve(mode: ThemeMode, accent: AccentColor, window: &Window) -> Self {
        let is_dark = Self::is_dark_mode(mode, window);

        if is_dark {
            let accent_hex = accent.dark_hex();
            Self {
                bg_app: rgb(0x0a0b0f),
                bg_app_top_glow: rgba((accent_hex << 8) | 0x16),
                bg_sidebar: rgb(0x0f1117),
                bg_surface: rgb(0x161821),
                bg_elevated: rgb(0x1d202b),
                bg_popover: rgb(0x191c26),
                bg_hover: rgba(0xffffff0d),
                hairline: rgba(0xffffff12),
                hairline_strong: rgba(0xffffff26),
                text_primary: rgb(0xf4f5f8),
                text_muted: rgb(0x9ca3b4),
                text_subtle: rgb(0x6b7280),
                text_on_accent: rgb(0xffffff),
                accent: rgb(accent_hex),
                accent_soft: rgba((accent_hex << 8) | 0x26),
                glass_dark: rgba(0x0b0d12cc),
                glass_border: rgba(0xffffff24),
                success: rgb(0x22c55e),
                warning: rgb(0xf59e0b),
                danger: rgb(0xef4444),
            }
        } else {
            let accent_hex = accent.light_hex();
            Self {
                bg_app: rgb(0xf6f7fb),
                bg_app_top_glow: rgba((accent_hex << 8) | 0x12),
                bg_sidebar: rgb(0xedeff5),
                bg_surface: rgb(0xffffff),
                bg_elevated: rgb(0xf3f5f9),
                bg_popover: rgb(0xffffff),
                bg_hover: rgba(0x0f172a0b),
                hairline: rgba(0x0f172a12),
                hairline_strong: rgba(0x0f172a26),
                text_primary: rgb(0x111827),
                text_muted: rgb(0x5b6475),
                text_subtle: rgb(0x8c95a6),
                text_on_accent: rgb(0xffffff),
                accent: rgb(accent_hex),
                accent_soft: rgba((accent_hex << 8) | 0x1a),
                glass_dark: rgba(0x0b0d12b8),
                glass_border: rgba(0xffffff30),
                success: rgb(0x16a34a),
                warning: rgb(0xd97706),
                danger: rgb(0xdc2626),
            }
        }
    }

    /// Synchronizes `gpui-component`'s global theme, typography, and radii with Wallzy.
    pub fn sync_component_theme(
        mode: ThemeMode,
        accent: AccentColor,
        typography: &ResolvedTypography,
        window: &mut Window,
        cx: &mut App,
    ) {
        let is_dark = Self::is_dark_mode(mode, window);
        let target = if is_dark {
            ComponentThemeMode::Dark
        } else {
            ComponentThemeMode::Light
        };
        if ComponentTheme::global(cx).mode != target {
            ComponentTheme::change(target, Some(window), cx);
        }

        let palette = Self::resolve(mode, accent, window);
        let accent_hsla: Hsla = palette.accent.into();
        let accent_hover: Hsla = accent_hsla.lighten(0.08);
        let accent_active: Hsla = accent_hsla.darken(0.08);
        let surface_hsla: Hsla = palette.bg_surface.into();
        let elevated_hsla: Hsla = palette.bg_elevated.into();
        let primary_fg: Hsla = palette.text_on_accent.into();
        let text_hsla: Hsla = palette.text_primary.into();
        let muted_fg: Hsla = palette.text_muted.into();

        ComponentTheme::update(cx, |theme| {
            theme.font_family = typography.ui_font.clone();
            theme.mono_font_family = typography.mono_font.clone();
            theme.radius = px(10.0);
            theme.radius_lg = px(14.0);
            theme.colors.background = palette.bg_app.into();
            theme.colors.foreground = text_hsla;
            theme.colors.muted = elevated_hsla;
            theme.colors.muted_foreground = muted_fg;
            theme.colors.title_bar = palette.bg_sidebar.into();
            theme.colors.title_bar_border = palette.hairline.into();
            theme.colors.primary = accent_hsla;
            theme.colors.primary_hover = accent_hover;
            theme.colors.primary_active = accent_active;
            theme.colors.primary_foreground = primary_fg;
            theme.colors.button_primary = accent_hsla;
            theme.colors.button_primary_hover = accent_hover;
            theme.colors.button_primary_active = accent_active;
            theme.colors.button_primary_foreground = primary_fg;
            theme.colors.secondary = surface_hsla;
            theme.colors.secondary_hover = elevated_hsla;
            theme.colors.secondary_foreground = text_hsla;
            theme.colors.button_secondary = surface_hsla;
            theme.colors.button_secondary_hover = elevated_hsla;
            theme.colors.button_secondary_foreground = text_hsla;
            theme.colors.ring = accent_hsla;
            theme.colors.border = palette.hairline.into();
            theme.colors.input = palette.hairline_strong.into();
        });
    }
}
