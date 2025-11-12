/**
 * Centralized logging utility.
 * Only logs in development mode to avoid console noise in production.
 */

// Check if we're in development mode (Vite environment variable).
const isDev = import.meta.env.DEV;

/**
 * Log a debug message (only in development).
 * 
 * Suppressed in production to avoid console noise.
 */
export function logDebug(...args: unknown[]): void {
	if (isDev) {
		console.debug(...args);
	}
}

/**
 * Log an info message (only in development).
 * 
 * Suppressed in production to avoid console noise.
 */
export function logInfo(...args: unknown[]): void {
	if (isDev) {
		console.log(...args);
	}
}

/**
 * Log a warning message (only in development).
 * 
 * Suppressed in production to avoid console noise.
 */
export function logWarn(...args: unknown[]): void {
	if (isDev) {
		console.warn(...args);
	}
}

/**
 * Log an error message (always logged, even in production).
 * 
 * Errors are always logged because they indicate actual problems that need tracking.
 * Use this for actual errors, not for debug/info messages.
 */
export function logError(...args: unknown[]): void {
	console.error(...args);
}

