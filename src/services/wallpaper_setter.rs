use crate::models::WallpaperFit;
use std::path::Path;
use std::process::Command;

/// Sets the active desktop wallpaper using the host operating system's native mechanism.
pub fn set_desktop_wallpaper(image_path: &Path, fit: WallpaperFit) -> Result<(), String> {
    if !image_path.exists() {
        return Err(format!(
            "Wallpaper file does not exist: {}",
            image_path.display()
        ));
    }
    let abs_path = std::fs::canonicalize(image_path).unwrap_or_else(|_| image_path.to_path_buf());
    // Strip Windows extended-length prefix (\\?\) for Win32 SystemParametersInfo compatibility.
    let path_str = abs_path.to_string_lossy();
    let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);

    #[cfg(target_os = "windows")]
    {
        set_windows_wallpaper(clean_path, fit)
    }

    #[cfg(target_os = "macos")]
    {
        let _ = fit;
        set_macos_wallpaper(clean_path)
    }

    #[cfg(target_os = "linux")]
    {
        set_linux_wallpaper(clean_path, fit)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = (clean_path, fit);
        Err("Desktop wallpaper setting is not implemented for this OS yet.".to_string())
    }
}

/// Reveals a file in the system file manager, or opens a directory directly.
pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let target = if path.exists() {
        path
    } else if let Some(parent) = path.parent().filter(|p| p.exists()) {
        parent
    } else {
        return Err(format!("Path does not exist: {}", path.display()));
    };

    #[cfg(target_os = "windows")]
    {
        let path_str = target.to_string_lossy();
        let clean = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
        if target.is_file() {
            Command::new("explorer")
                .arg(format!("/select,{}", clean))
                .spawn()
                .map_err(|e| format!("Failed to launch Explorer: {}", e))?;
        } else {
            Command::new("explorer")
                .arg(clean)
                .spawn()
                .map_err(|e| format!("Failed to launch Explorer: {}", e))?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let mut cmd = Command::new("open");
        if target.is_file() {
            cmd.arg("-R");
        }
        cmd.arg(target)
            .spawn()
            .map_err(|e| format!("Failed to launch Finder: {}", e))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let dir = if target.is_file() {
            target.parent().unwrap_or(target)
        } else {
            target
        };
        Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Unsupported operating system".to_string())
}

#[cfg(target_os = "windows")]
fn set_windows_wallpaper(path: &str, fit: WallpaperFit) -> Result<(), String> {
    let (style, tile) = match fit {
        WallpaperFit::Fill => ("10", "0"),
        WallpaperFit::Fit => ("6", "0"),
        WallpaperFit::Stretch => ("2", "0"),
        WallpaperFit::Center => ("0", "0"),
        WallpaperFit::Tile => ("0", "1"),
    };

    let escaped = path.replace('\'', "''");
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name WallpaperStyle -Value '{style}'
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name TileWallpaper -Value '{tile}'
Add-Type -TypeDefinition @"
using System.Runtime.InteropServices;
public class WallzyNative {{
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool SystemParametersInfoW(uint uiAction, uint uiParam, string pvParam, uint fWinIni);
}}
"@
$ok = [WallzyNative]::SystemParametersInfoW(0x0014, 0, '{escaped}', 0x01 -bor 0x02)
if (-not $ok) {{ throw "SystemParametersInfoW failed" }}
"#
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|e| format!("Failed to invoke PowerShell: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Windows wallpaper update failed: {}",
            stderr.trim()
        ))
    }
}

#[cfg(target_os = "macos")]
fn set_macos_wallpaper(path: &str) -> Result<(), String> {
    let escaped = path.replace('"', r#"\""#);
    let script = format!(
        r#"tell application "System Events" to tell every desktop to set picture to "{escaped}""#
    );
    let output = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Failed to invoke osascript: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(target_os = "linux")]
fn set_linux_wallpaper(path: &str, fit: WallpaperFit) -> Result<(), String> {
    let uri = format!("file://{}", path);
    let gnome_opt = match fit {
        WallpaperFit::Fill => "zoom",
        WallpaperFit::Fit => "scaled",
        WallpaperFit::Stretch => "stretched",
        WallpaperFit::Center => "centered",
        WallpaperFit::Tile => "wallpaper",
    };

    if Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
        .status()
        .is_ok_and(|s| s.success())
    {
        let _ = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.desktop.background",
                "picture-uri-dark",
                &uri,
            ])
            .status();
        let _ = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.desktop.background",
                "picture-options",
                gnome_opt,
            ])
            .status();
        return Ok(());
    }

    if Command::new("plasma-apply-wallpaperimage")
        .arg(path)
        .status()
        .is_ok_and(|s| s.success())
    {
        return Ok(());
    }

    if Command::new("feh")
        .args(["--bg-fill", path])
        .status()
        .is_ok_and(|s| s.success())
    {
        return Ok(());
    }

    Err("No supported Linux wallpaper backend found (tried gsettings, plasma-apply-wallpaperimage, feh).".to_string())
}
