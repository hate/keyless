//! Small LRU-like cache for preview unit texts keyed by unit range and tail hash.
use std::collections::HashMap;
use std::ops::Range;

use keyless_core::error::KeylessResult;

/// Cache of decoded preview texts to avoid re-decoding overlapping units.
pub(crate) struct PreviewCache {
    /// Map from (start, end, tail_hash) to decoded text.
    map: HashMap<(usize, usize, u64), String>,
    /// Number of tail samples used for the unit hash.
    tail_len: usize,
}

impl PreviewCache {
    /// Create a new cache with the specified tail hash length.
    pub(crate) fn new(tail_len: usize) -> Self {
        Self {
            map: HashMap::new(),
            tail_len,
        }
    }

    /// Clear all cached entries.
    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    /// Build a cache key (start, end, hash) for a unit and PCM slice.
    fn make_key(&self, unit: &Range<usize>, pcm: &[f32]) -> (usize, usize, u64) {
        let start = unit.start;
        let end = unit.end.min(pcm.len());
        let hash = tail_hash_u64(&pcm[start..end], self.tail_len);
        (start, end, hash)
    }

    /// Get a cached text for the unit or decode and insert it; returns (text, reused).
    pub(crate) fn get_or_decode<F>(
        &mut self,
        unit: &Range<usize>,
        pcm: &[f32],
        mut decode: F,
    ) -> KeylessResult<(String, bool)>
    where
        F: FnMut() -> KeylessResult<String>,
    {
        let key = self.make_key(unit, pcm);
        if let Some(existing) = self.map.get(&key) {
            return Ok((existing.clone(), true));
        }
        let text = decode()?;
        if text.is_empty() {
            self.map.remove(&key);
        } else {
            self.map.insert(key, text.clone());
        }
        Ok((text, false))
    }
}

/// Compute a simple multiplicative hash over the tail of the slice.
fn tail_hash_u64(slice: &[f32], tail: usize) -> u64 {
    let start = slice.len().saturating_sub(tail);
    let mut h: u64 = 0x9E37_79B1_85EB_CA87;
    for &v in &slice[start..] {
        let bits = v.to_bits() as u64;
        h ^= bits.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).rotate_left(17);
        h = h.wrapping_mul(0x1656_6759_5DDE_EA3D);
    }
    h
}
