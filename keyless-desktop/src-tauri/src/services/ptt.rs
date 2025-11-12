//! Push-to-talk hotkey management service.
//!
//! Provides a trait-based abstraction for managing the push-to-talk hotkey listener.
//! The implementation delegates to the runtime thread's PTT management functions.

/// Service trait for push-to-talk hotkey management.
///
/// Provides methods to update the PTT preset from config and check listening status.
/// All operations delegate to the runtime thread's PTT listener.
pub trait PttService: Send + Sync {
    /// Update the PTT hotkey preset from the current configuration.
    ///
    /// Reads the hotkey from config and updates the PTT listener's preset.
    fn update_preset_from_config(&self);

    /// Check whether the PTT hotkey is currently being held (listening state).
    fn is_listening(&self) -> bool;
}

/// Default implementation of `PttService` using the runtime thread API.
pub struct PttServiceImpl;

impl PttServiceImpl {
    /// Create a new `PttServiceImpl` instance.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PttServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl PttService for PttServiceImpl {
    fn update_preset_from_config(&self) {
        // Update the PTT hotkey preset by reading the current config and applying it to the listener.
        // This is called when the hotkey changes in settings, so the listener uses the new hotkey immediately.
        crate::runtime::update_ptt_selection();
    }

    fn is_listening(&self) -> bool {
        // Check if the PTT hotkey is currently being held down.
        // Returns true if the user is holding the hotkey (listening/recording), false otherwise.
        crate::runtime::is_listening()
    }
}
