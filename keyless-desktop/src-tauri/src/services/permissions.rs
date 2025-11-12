//! System permissions checking and management service.
//!
//! Provides a trait-based abstraction for checking system permissions (microphone access)
//! and opening system settings panes. The implementation includes caching to avoid excessive
//! permission checks.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use keyless_audio::input::api::can_record;

/// Service trait for system permissions management.
///
/// Provides methods to check permissions and open system settings. The implementation
/// caches permission check results to avoid excessive system calls.
pub trait PermissionsService: Send + Sync {
    /// Check system permissions (microphone and accessibility).
    ///
    /// Returns a JSON object with `microphone`, `accessibility`, and `needsOnboarding` fields.
    /// Results are cached for 750ms to avoid excessive system calls.
    fn check(&self) -> Result<serde_json::Value, String>;

    /// Open a system settings pane.
    ///
    /// On macOS, opens the specified pane ("Microphone" or "Accessibility") in System Settings.
    /// On other platforms, returns an error.
    fn open_system_settings(&self, pane: &str) -> Result<(), String>;
}

/// Default implementation of `PermissionsService` using system APIs.
pub struct PermissionsServiceImpl;

/// Cached permissions check result with timestamp for TTL.
static PERMS_CACHE: OnceLock<Mutex<(Instant, serde_json::Value)>> = OnceLock::new();

impl PermissionsServiceImpl {
    /// Create a new `PermissionsServiceImpl` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PermissionsServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsService for PermissionsServiceImpl {
    fn check(&self) -> Result<serde_json::Value, String> {
        // Check if the audio pipeline is currently active.
        // If active, we assume permissions are granted (pipeline wouldn't work without them).
        let pipeline_active = crate::runtime::is_pipeline_active();

        // If pipeline is not active, check the cache to avoid excessive system calls.
        // Permission checks can be expensive, so we cache results for 750ms.
        if !pipeline_active {
            // Initialize cache if it doesn't exist (with a timestamp 5 seconds in the past
            // to ensure the first check always goes through).
            let cache = PERMS_CACHE.get_or_init(|| {
                Mutex::new((
                    Instant::now() - Duration::from_secs(5),
                    serde_json::json!({}),
                ))
            });

            // Try to use cached result if it's still fresh (< 750ms old) and valid.
            if let Ok(guard) = cache.lock() {
                let (ts, val) = &*guard;
                if ts.elapsed() < Duration::from_millis(750)
                    && !val.is_null()
                    && val.get("microphone").is_some()
                {
                    // Return cached result to avoid system call.
                    return Ok(val.clone());
                }
            }
        }

        // Actually check permissions:
        // - If pipeline is active, assume microphone permission is granted (pipeline needs it).
        // - Otherwise, check microphone permission via system API.
        let microphone = if pipeline_active { true } else { can_record() };
        // Check accessibility permission (needed for paste sink and PTT hotkey).
        let accessibility = keyless_core::permissions::has_accessibility_permission();

        // Determine if onboarding is needed (user hasn't granted required permissions).
        let needs_onboarding = !microphone || !accessibility;
        eprintln!(
            "[permissions] has_mic: {}, has_accessibility: {}, needsOnboarding: {}",
            microphone, accessibility, needs_onboarding
        );

        // In debug mode, warn that permissions might be inherited from terminal.
        // This helps developers understand why permissions might appear granted when they're not.
        #[cfg(debug_assertions)]
        if microphone && accessibility {
            eprintln!(
                "[permissions] WARNING: In dev mode, permissions may be inherited from terminal"
            );
            eprintln!(
                "[permissions] Please verify 'keyless-desktop' has permissions in System Settings"
            );
        }

        // Build the result JSON object with permission status.
        let result = serde_json::json!({
            "microphone": microphone,
            "accessibility": accessibility,
            "needsOnboarding": needs_onboarding,
        });

        // Update cache with fresh result (only if pipeline is not active, since active pipeline
        // means permissions are definitely granted and don't need caching).
        if !pipeline_active
            && let Some(cache) = PERMS_CACHE.get()
            && let Ok(mut guard) = cache.lock()
        {
            *guard = (Instant::now(), result.clone());
        }

        Ok(result)
    }

    fn open_system_settings(&self, pane: &str) -> Result<(), String> {
        crate::platform::open_system_settings(pane)
    }
}
