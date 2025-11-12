use std::sync::atomic::{AtomicBool, Ordering};

/// Tracks whether the popover window should ignore temporary blur events (e.g., file dialogs).
#[derive(Default)]
pub struct PopoverBlurGuard {
    /// Whether blur events are currently suppressed.
    suppressed: AtomicBool,
}

impl PopoverBlurGuard {
    /// Set the blur suppression state.
    pub fn set(&self, value: bool) {
        self.suppressed.store(value, Ordering::SeqCst);
    }

    /// Check if blur events are currently suppressed.
    pub fn is_suppressed(&self) -> bool {
        self.suppressed.load(Ordering::SeqCst)
    }
}
