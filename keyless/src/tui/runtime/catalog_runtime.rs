//! Model catalog runtime state (background workers for size/discovery).

use std::sync::mpsc;

/// Model catalog runtime infrastructure (receivers and flags).
pub struct CatalogRuntime {
    pub rx_sizes_cached: Option<mpsc::Receiver<(String, Option<u64>)>>,
    pub rx_sizes_remote: Option<mpsc::Receiver<(String, Option<u64>)>>,
    pub sizes_started: bool,
    pub repos_started: bool,
}

impl CatalogRuntime {
    pub fn new() -> Self {
        Self {
            rx_sizes_cached: None,
            rx_sizes_remote: None,
            sizes_started: false,
            repos_started: false,
        }
    }
}

impl Default for CatalogRuntime {
    fn default() -> Self {
        Self::new()
    }
}
