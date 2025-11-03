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
            runtime_list: Vec::new(),
            sizes: HashMap::new(),
            discovered: Vec::new(),
        }
    }
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}
