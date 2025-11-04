//! Small cross-crate utilities.

/// Current time in milliseconds since UNIX epoch (saturating to 0 on error).
pub fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Get duration since epoch; unwrap_or_default returns Duration::ZERO if system time < epoch.
    // This handles clock skew edge cases (shouldn't happen on normal systems).
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        // Convert to milliseconds (u128), then cast to u64 (truncates if > u64::MAX ms).
        // u64::MAX ms ≈ 584 million years, so truncation is not a practical concern.
        .as_millis() as u64
}

/// Format bytes into a human-readable string (KB/MB/GB) with one decimal place.
pub fn human_size(bytes: u64) -> String {
    // Binary units (1024-based), not decimal (1000-based); standard for file sizes.
    // KB: 1024 bytes
    // MB: 1024 * 1024 bytes
    // GB: 1024 * 1024 * 1024 bytes
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    // Convert to f64 for division; u64→f64 preserves precision up to 2^53 (plenty for file sizes).
    let b = bytes as f64;
    // Check largest unit first (descending order) to pick appropriate suffix.
    if b >= GB {
        // One decimal place via {:.1}; division produces value in GB units.
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        // Bytes: no decimal, use original u64 value (integer bytes).
        format!("{} B", bytes)
    }
}

/// Format milliseconds as HH:MM:SS or MM:SS if under one hour.
pub fn fmt_hms(ms: u64) -> String {
    // Integer division truncates fractional seconds (e.g., 1500ms → 1s, not 1.5s).
    let total_secs = ms / 1000;
    // Extract hours: total_secs / 3600 (integer division).
    let hours = total_secs / 3600;
    // Extract minutes: remainder after removing hours, then divide by 60.
    // Modulo 3600 gives seconds in current hour, divide by 60 gives minutes.
    let mins = (total_secs % 3600) / 60;
    // Extract seconds: remainder after removing hours and minutes.
    let secs = total_secs % 60;
    // Conditional formatting: show hours only if > 0 (compact display for short durations).
    if hours > 0 {
        // HH:MM:SS format; {:02} zero-pads minutes/seconds to 2 digits.
        format!("{}:{:02}:{:02}", hours, mins, secs)
    } else {
        // MM:SS format (no hours) for durations under 1 hour.
        format!("{:02}:{:02}", mins, secs)
    }
}
