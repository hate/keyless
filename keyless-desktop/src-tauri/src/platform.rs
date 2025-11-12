//! Platform-specific detection and utilities.
//!
//! Centralizes all cross-platform differences in one place for easier maintenance.

#[cfg(target_os = "linux")]
mod linux {

    /// Detected desktop environment on Linux.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DesktopEnvironment {
        /// GNOME desktop environment.
        Gnome,
        /// KDE Plasma desktop environment.
        Kde,
        /// XFCE desktop environment.
        Xfce,
        /// Other or unknown desktop environment.
        Other,
    }

    /// Detected init system on Linux.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InitSystem {
        /// systemd init system.
        Systemd,
        /// OpenRC init system.
        OpenRc,
        /// runit init system.
        Runit,
        /// Other or unknown init system.
        Other,
    }

    /// Detect the current desktop environment.
    ///
    /// Checks the `XDG_CURRENT_DESKTOP` and `XDG_SESSION_DESKTOP` environment variables,
    /// falling back to `DESKTOP_SESSION` if needed.
    ///
    /// Returns `DesktopEnvironment::Other` if detection fails.
    pub fn detect_desktop_environment() -> DesktopEnvironment {
        // Check XDG_CURRENT_DESKTOP (most reliable, set by most modern DEs).
        if let Ok(val) = std::env::var("XDG_CURRENT_DESKTOP") {
            let val_lower = val.to_lowercase();
            if val_lower.contains("gnome") {
                return DesktopEnvironment::Gnome;
            }
            if val_lower.contains("kde") {
                return DesktopEnvironment::Kde;
            }
            if val_lower.contains("xfce") {
                return DesktopEnvironment::Xfce;
            }
        }

        // Check XDG_SESSION_DESKTOP (alternative variable).
        if let Ok(val) = std::env::var("XDG_SESSION_DESKTOP") {
            let val_lower = val.to_lowercase();
            if val_lower.contains("gnome") {
                return DesktopEnvironment::Gnome;
            }
            if val_lower.contains("kde") {
                return DesktopEnvironment::Kde;
            }
            if val_lower.contains("xfce") {
                return DesktopEnvironment::Xfce;
            }
        }

        // Fallback to DESKTOP_SESSION (older systems).
        if let Ok(val) = std::env::var("DESKTOP_SESSION") {
            let val_lower = val.to_lowercase();
            if val_lower.contains("gnome") {
                return DesktopEnvironment::Gnome;
            }
            if val_lower.contains("kde") {
                return DesktopEnvironment::Kde;
            }
            if val_lower.contains("xfce") {
                return DesktopEnvironment::Xfce;
            }
        }

        DesktopEnvironment::Other
    }

    /// Detect the current init system.
    ///
    /// Checks for systemd by looking for `/run/systemd/system` or `systemctl` command.
    /// Checks for OpenRC by looking for `/run/openrc` or `openrc-status` command.
    /// Checks for runit by looking for `/run/runit` or `runit` command.
    ///
    /// Returns `InitSystem::Other` if detection fails.
    pub fn detect_init_system() -> InitSystem {
        // Check for systemd (most common).
        // Check paths first (faster), then fall back to which command.
        if std::path::Path::new("/run/systemd/system").exists()
            || std::path::Path::new("/usr/lib/systemd").exists()
        {
            return InitSystem::Systemd;
        }
        // Check for systemctl command (more reliable detection).
        if which::which("systemctl").is_ok() {
            return InitSystem::Systemd;
        }

        // Check for OpenRC (Gentoo, Alpine, etc.).
        if std::path::Path::new("/run/openrc").exists()
            || std::path::Path::new("/etc/init.d").exists()
        {
            return InitSystem::OpenRc;
        }
        // Check for openrc-status command.
        if which::which("openrc-status").is_ok() {
            return InitSystem::OpenRc;
        }

        // Check for runit (Void Linux, etc.).
        if std::path::Path::new("/run/runit").exists()
            || std::path::Path::new("/etc/runit").exists()
        {
            return InitSystem::Runit;
        }
        // Check for runit command.
        if which::which("runit").is_ok() {
            return InitSystem::Runit;
        }

        InitSystem::Other
    }

    /// Get the system settings command for a given desktop environment and pane.
    ///
    /// Returns a vector of command and arguments, or `None` if no command is available
    /// for the given DE and pane combination.
    pub fn get_settings_command(de: DesktopEnvironment, pane: &str) -> Option<Vec<String>> {
        match (de, pane) {
            (DesktopEnvironment::Gnome, "Microphone") => {
                Some(vec!["gnome-control-center".into(), "privacy".into()])
            }
            (DesktopEnvironment::Gnome, "Accessibility") => Some(vec![
                "gnome-control-center".into(),
                "universal-access".into(),
            ]),
            (DesktopEnvironment::Kde, "Microphone") => {
                Some(vec!["systemsettings5".into(), "kcm_microphone".into()])
            }
            (DesktopEnvironment::Kde, "Accessibility") => {
                Some(vec!["systemsettings5".into(), "kcm_accessibility".into()])
            }
            (DesktopEnvironment::Xfce, "Microphone") => {
                Some(vec!["xfce4-settings-manager".into(), "sound".into()])
            }
            (DesktopEnvironment::Xfce, "Accessibility") => Some(vec![
                "xfce4-settings-manager".into(),
                "accessibility".into(),
            ]),
            // Fallback: try gnome-control-center even on unknown DEs (might work).
            (DesktopEnvironment::Other, "Microphone") => {
                Some(vec!["gnome-control-center".into(), "privacy".into()])
            }
            (DesktopEnvironment::Other, "Accessibility") => Some(vec![
                "gnome-control-center".into(),
                "universal-access".into(),
            ]),
            _ => None,
        }
    }
}

/// Open system settings to the specified pane (platform-specific).
///
/// # Arguments
/// * `pane` - Settings pane name ("Microphone" or "Accessibility")
///
/// # Errors
/// Returns an error if the settings pane cannot be opened.
pub fn open_system_settings(pane: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Map pane name to macOS System Settings URL scheme.
        let url = match pane {
            "Microphone" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            "Accessibility" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
            }
            other => return Err(format!("Unknown settings pane: {other}")),
        };

        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open System Settings: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        // Map pane name to Windows Settings URI.
        let uri = match pane {
            "Microphone" => "ms-settings:privacy-microphone",
            "Accessibility" => "ms-settings:privacy-accessibility",
            other => return Err(format!("Unknown settings pane: {other}")),
        };

        std::process::Command::new("cmd")
            .args(["/C", "start", "", uri])
            .spawn()
            .map_err(|e| format!("Failed to open Settings: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use linux::{detect_desktop_environment, get_settings_command};

        let de = detect_desktop_environment();
        eprintln!("[platform] Detected desktop environment: {:?}", de);

        let command = match get_settings_command(de, pane) {
            Some(cmd) => cmd,
            None => {
                return Err(format!("Unknown settings pane: {pane} (DE: {:?})", de));
            }
        };

        let cmd_name = &command[0];
        let args = &command[1..];

        eprintln!(
            "[platform] Opening system settings: {} {:?}",
            cmd_name, args
        );

        match std::process::Command::new(cmd_name).args(args).spawn() {
            Ok(_) => Ok(()),
            Err(e) => {
                let error_msg = format!(
                    "Failed to open system settings ({} may not be installed for {:?}): {}",
                    cmd_name, de, e
                );
                eprintln!("[platform] {}", error_msg);
                Err(error_msg)
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = pane;
        Err("System Settings opening is not supported on this platform".into())
    }
}

/// Get platform-specific overlay bottom offset (in logical pixels).
pub fn overlay_bottom_offset() -> f64 {
    #[cfg(target_os = "macos")]
    {
        4.0
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        12.0
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        24.0
    }
}

/// Get platform-specific default tray arrow direction.
///
/// Returns "up" if popover is below tray, "down" if above tray.
pub fn default_tray_arrow_direction() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "up" // macOS tray is always at top
    }
    #[cfg(not(target_os = "macos"))]
    {
        "down" // Windows/Linux: default to DOWN (most common: tray at bottom)
    }
}

/// Configure platform-specific app settings.
///
/// Called during Tauri app setup to apply platform-specific configurations.
pub fn configure_app(app: &mut tauri::App) {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        eprintln!("[platform] Setting activation policy to Accessory...");
        app.set_activation_policy(ActivationPolicy::Accessory);
    }
    #[cfg(not(target_os = "macos"))]
    {
        // No platform-specific configuration needed for other platforms.
        let _ = app;
    }
}
