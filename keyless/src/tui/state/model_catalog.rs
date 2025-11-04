//! Model catalog state (available models, sizes, discovery).

use std::collections::HashMap;

/// Model catalog metadata for Config screen.
pub struct ModelCatalog {
    /// Runtime list of selectable model identifiers (official + discovered).
    pub runtime_list: Vec<String>,
    /// Known model sizes (bytes) keyed by model id.
    pub sizes: HashMap<String, u64>,
    /// Discovered local Whisper repos from the keyless cache.
    pub discovered: Vec<String>,
}

impl ModelCatalog {
    pub fn new() -> Self {
        Self {
            // Populated during init by merging official models + discovered local repos.
            runtime_list: Vec::new(),
            // Model sizes (bytes) keyed by model ID; populated asynchronously from cache/remote.
            sizes: HashMap::new(),
            // Local Whisper repos found in keyless cache directory.
            discovered: Vec::new(),
        }
    }
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}
