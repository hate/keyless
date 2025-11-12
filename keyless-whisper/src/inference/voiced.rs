//! Voiced-mask detection and span/window utilities.
use std::ops::Range;

/// Configuration for voiced-span detection.
#[derive(Clone, Debug)]
pub struct VoicedCfg {
    /// Hop size in milliseconds between analysis frames.
    pub hop_ms: u32,
    /// dB threshold to open speech (higher = easier to open).
    pub open_db: f32,
    /// dB threshold to close speech (lower = harder to close).
    pub close_db: f32,
    /// Minimum continuous speech required to confirm opening (ms).
    pub min_speech_ms: u32,
    /// Maximum trailing silence tolerated before closing (ms).
    pub max_silence_ms: u32,
    /// Padding added before/after detected spans (ms).
    pub pad_ms: u32,
}

impl VoicedCfg {
    /// Reasonable defaults for real-time voiced span detection.
    pub const fn default() -> Self {
        Self {
            hop_ms: 25,
            open_db: -30.0,
            close_db: -35.0,
            min_speech_ms: 300,
            max_silence_ms: 800,
            pad_ms: 300,
        }
    }
}

/// Detect contiguous voiced spans (in samples) from raw PCM data.
pub fn build_voiced_spans(pcm: &[f32], sample_rate: usize, cfg: &VoicedCfg) -> Vec<Range<usize>> {
    if pcm.is_empty() || sample_rate == 0 {
        return Vec::new();
    }

    let hop_samples = ((sample_rate as f32) * (cfg.hop_ms as f32 / 1000.0)).round() as usize;
    let hop_samples = hop_samples.max(1);

    let num_frames = pcm.len().div_ceil(hop_samples);
    if num_frames == 0 {
        return Vec::new();
    }

    let min_frames = ms_to_frames(cfg.min_speech_ms, cfg.hop_ms).max(1);
    let max_silence_frames = ms_to_frames(cfg.max_silence_ms, cfg.hop_ms).max(1);

    let mut spans: Vec<Range<usize>> = Vec::new();
    let mut in_speech = false;
    let mut speech_start_frame = 0usize;
    let mut hold_frames = 0usize;
    let mut silence_frames = 0usize;

    for frame_idx in 0..num_frames {
        let frame_start = frame_idx * hop_samples;
        let frame_end = ((frame_idx + 1) * hop_samples).min(pcm.len());
        let rms = frame_rms(&pcm[frame_start..frame_end]);
        let db = if rms <= 0.0 {
            f32::NEG_INFINITY
        } else {
            20.0 * rms.log10()
        };

        if in_speech {
            if db >= cfg.close_db {
                silence_frames = 0;
            } else {
                silence_frames += 1;
                if silence_frames >= max_silence_frames {
                    let end_frame = frame_idx + 1 - silence_frames;
                    let start_sample = speech_start_frame * hop_samples;
                    let end_sample = (end_frame * hop_samples).min(pcm.len());
                    if end_sample > start_sample {
                        spans.push(start_sample..end_sample);
                    }
                    in_speech = false;
                    hold_frames = 0;
                    silence_frames = 0;
                }
            }
        } else if db >= cfg.open_db {
            hold_frames += 1;
            if hold_frames >= min_frames {
                in_speech = true;
                speech_start_frame = frame_idx + 1 - hold_frames;
                hold_frames = 0;
                silence_frames = 0;
            }
        } else {
            hold_frames = 0;
        }
    }

    if in_speech {
        let start_sample = speech_start_frame * hop_samples;
        spans.push(start_sample..pcm.len());
    }

    if spans.is_empty() {
        return spans;
    }

    let pad_samples = ((sample_rate as f32) * (cfg.pad_ms as f32 / 1000.0)).round() as usize;
    apply_padding_and_merge(spans, pad_samples, pcm.len())
}

/// Split voiced ranges into inference units with overlap (≤ window size).
pub fn units_from_spans(
    spans: &[Range<usize>],
    max_window_samples: usize,
    overlap_samples: usize,
) -> Vec<Range<usize>> {
    if spans.is_empty() {
        return Vec::new();
    }

    let mut units = Vec::new();
    for span in spans {
        if span.start >= span.end {
            continue;
        }
        let mut window_start = span.start;
        while window_start < span.end {
            let window_end = (window_start + max_window_samples).min(span.end);
            if window_end > window_start {
                units.push(window_start..window_end);
            }
            if window_end == span.end {
                break;
            }
            let next_start = window_end.saturating_sub(overlap_samples);
            if next_start <= window_start {
                window_start = window_end;
            } else {
                window_start = next_start;
            }
        }
    }
    units.sort_by_key(|r| r.start);
    units
}

/// Apply symmetric padding to spans and merge overlapping ones; clamps to total length.
fn apply_padding_and_merge(
    spans: Vec<Range<usize>>,
    pad_samples: usize,
    total_samples: usize,
) -> Vec<Range<usize>> {
    if spans.is_empty() {
        return spans;
    }

    let mut padded: Vec<Range<usize>> = Vec::with_capacity(spans.len());
    for span in spans {
        let start = span.start.saturating_sub(pad_samples);
        let end = (span.end + pad_samples).min(total_samples);
        if end > start {
            padded.push(start..end);
        }
    }

    if padded.is_empty() {
        return padded;
    }

    padded.sort_by_key(|r| r.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(padded.len());
    let mut current = padded[0].clone();
    for span in padded.into_iter().skip(1) {
        if span.start <= current.end {
            current.end = current.end.max(span.end);
        } else {
            merged.push(current);
            current = span;
        }
    }
    merged.push(current);
    merged
}

/// Compute RMS for a single analysis frame (f32 output for dB computation).
fn frame_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq = frame.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>();
    (sum_sq / frame.len() as f64).sqrt() as f32
}

/// Convert milliseconds to frame count given a hop size (ms), rounding up.
fn ms_to_frames(ms: u32, hop_ms: u32) -> usize {
    if hop_ms == 0 {
        return 0;
    }
    ms.div_ceil(hop_ms) as usize
}
