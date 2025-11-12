/**
 * Utility functions for parsing configuration values.
 */

/**
 * Extracts output mode from a config object, handling various formats.
 * Returns the output mode string in lowercase, or undefined if not found.
 */
export function extractOutputMode(cfg: unknown): string | undefined {
	// Type guard: ensure config is an object.
	if (!cfg || typeof cfg !== 'object') {
		return undefined;
	}
	const source = cfg as Record<string, unknown>;
	// Handle multiple formats:
	// - String: source.output_mode (snake_case)
	// - Object with type: source.output_mode.type
	// - camelCase: source.outputMode
	const raw =
		typeof source.output_mode === 'string'
			? source.output_mode
			: (source.output_mode as Record<string, unknown>)?.type ?? source.outputMode;
	// Normalize to lowercase string or undefined.
	return typeof raw === 'string' ? raw.toLowerCase() : undefined;
}

