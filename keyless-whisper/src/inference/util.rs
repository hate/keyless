//! Small numeric helpers for inference.

/// Compute the root-mean-square (RMS) of a slice; returns 0 for empty input.
pub(crate) fn compute_rms(slice: &[f32]) -> f64 {
    if slice.is_empty() {
        0.0
    } else {
        let sum = slice.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>();
        (sum / slice.len() as f64).sqrt()
    }
}
