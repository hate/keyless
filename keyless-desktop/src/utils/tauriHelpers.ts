/**
 * Utility functions for Tauri API interactions.
 */

/**
 * Checks if Tauri is available in the current environment.
 * Useful for skipping Tauri-specific code in tests or non-Tauri environments.
 * 
 * Uses type assertion to access Tauri internals safely.
 * The double assertion (`as unknown as`) is necessary because `__TAURI_INTERNALS__`
 * is not part of the standard `globalThis` type definition, but is injected by
 * Tauri at runtime. This is a safe pattern for feature detection.
 * 
 * @returns true if Tauri APIs are available, false otherwise
 */
export function isTauriAvailable(): boolean {
	// Type assertion needed because __TAURI_INTERNALS__ is not in globalThis types
	const w = globalThis as unknown as { __TAURI_INTERNALS__?: unknown };
	return !!w.__TAURI_INTERNALS__;
}

