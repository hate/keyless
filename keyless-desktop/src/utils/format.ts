/**
 * Formatting utilities for displaying numbers, durations, and file sizes.
 */

/**
 * Format bytes as human-readable file size (MB or GB).
 * 
 * @param bytes - Number of bytes (can be undefined or NaN)
 * @returns Formatted string (e.g., "123.4 MB", "1.23 GB", "?" for invalid)
 */
export function formatBytes(bytes?: number): string {
	// Handle invalid values.
	if (bytes === undefined || Number.isNaN(bytes)) return '?';
	if (bytes <= 0) return '0 MB';
	// Convert to megabytes.
	const mb = bytes / 1_000_000;
	// Format as GB if >= 1000 MB, otherwise MB.
	if (mb >= 1000) {
		return `${(mb / 1000).toFixed(2)} GB`;
	}
	return `${mb.toFixed(1)} MB`;
}

/**
 * Format estimated time remaining as human-readable string (e.g., "5m 30s", "45s").
 * 
 * @param etaSeconds - Estimated seconds remaining (can be undefined or NaN)
 * @returns Formatted string (e.g., "5m 30s", "45s", "" for invalid)
 */
export function formatEta(etaSeconds?: number): string {
	// Handle invalid values.
	if (etaSeconds === undefined || Number.isNaN(etaSeconds)) return '';
	// Round to nearest second, clamp to non-negative.
	const seconds = Math.max(0, Math.round(etaSeconds));
	// Format as "Xm Ys" if >= 60 seconds, otherwise just seconds.
	if (seconds >= 60) {
		const minutes = Math.floor(seconds / 60);
		const secs = seconds % 60;
		return `${minutes}m ${secs}s`;
	}
	return `${seconds}s`;
}

/**
 * Format duration in milliseconds as human-readable string (e.g., "1h 30m", "5m 30s", "45s").
 * 
 * @param ms - Duration in milliseconds (can be undefined or <= 0)
 * @returns Formatted string (e.g., "1h 30m", "5m 30s", "45s", "0s" for invalid)
 */
export function formatDuration(ms?: number): string {
	// Handle invalid values.
	if (!ms || ms <= 0) return '0s';
	// Convert to total seconds.
	const totalSeconds = Math.floor(ms / 1000);
	// Calculate hours, minutes, and seconds.
	const hours = Math.floor(totalSeconds / 3600);
	const minutes = Math.floor((totalSeconds % 3600) / 60);
	const seconds = totalSeconds % 60;
	// Format based on duration: include hours if > 0, otherwise minutes if > 0, otherwise just seconds.
	if (hours > 0) {
		return `${hours}h ${minutes}m`;
	}
	if (minutes > 0) {
		return `${minutes}m ${seconds}s`;
	}
	return `${seconds}s`;
}
