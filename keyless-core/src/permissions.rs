//! Platform permissions checking and guidance.
//!
//! Provides helpers to check system permissions and generate user-facing guidance
//! for microphone access, accessibility permissions, etc.

/// Check if the process has macOS Accessibility permission.
/// Returns `true` if permission is granted, `false` otherwise.
/// On non-macOS platforms, always returns `true`.
#[cfg(target_os = "macos")]
pub fn has_accessibility_permission() -> bool {
    // SAFETY: Calling a system function that returns a boolean-like value.
    // Using AXIsProcessTrustedWithOptions(NULL) instead of AXIsProcessTrusted()
    // to get a fresh, non-cached result. This is critical for accurate permission checks.
    unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) != 0 }
}

/// Check if the process has macOS Accessibility permission.
/// Returns `true` if permission is granted, `false` otherwise.
/// On non-macOS platforms, always returns `true`.
#[cfg(not(target_os = "macos"))]
pub fn has_accessibility_permission() -> bool {
    true // Non-macOS platforms don't require this permission
}

/// Generate permission warnings/guidance for the user.
///
/// # Arguments
/// * `needs_paste_permission` - Whether the app needs accessibility permission (for paste/keystroke simulation)
/// * `has_input_device` - Whether an audio input device was detected
///
/// Returns a list of human-readable permission guidance strings.
pub fn gather_permission_warnings(
    needs_paste_permission: bool,
    has_input_device: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();

    // Microphone availability/permission hints
    if !has_input_device {
        warnings.push(
            "No input device detected. Ensure a microphone is connected and enabled.".to_string(),
        );

        #[cfg(target_os = "macos")]
        warnings.push(
            "macOS: System Settings → Privacy & Security → Microphone: allow your terminal/app"
                .to_string(),
        );

        #[cfg(target_os = "windows")]
        warnings.push(
            "Windows: Settings → Privacy → Microphone: allow desktop apps, check per‑app toggle"
                .to_string(),
        );

        #[cfg(target_os = "linux")]
        warnings.push(
            "Linux: Ensure PulseAudio/PipeWire/ALSA is installed and running; container/WSL often lack mic"
                .to_string(),
        );
    }

    // Accessibility permission for paste/keystroke simulation on macOS
    if needs_paste_permission {
        #[cfg(target_os = "macos")]
        {
            if !has_accessibility_permission() {
                warnings
                    .push("Accessibility disabled for this app (required for Paste).".to_string());
            }
            warnings
                .push("macOS: System Settings → Privacy & Security → Accessibility".to_string());
        }
    }

    warnings
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    /// AXIsProcessTrustedWithOptions(CFDictionaryRef) -> Boolean (typedef unsigned char)
    /// Pass NULL for a fresh, non-cached check.
    fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> u8;
}
