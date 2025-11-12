import type { ConfigValues } from '../types';

/**
 * Get the config patch object for a given config key and value.
 * Maps frontend config keys to backend config structure.
 */
export function getConfigPatch(
	key: keyof ConfigValues,
	value: ConfigValues[keyof ConfigValues]
): Record<string, unknown> {
	// Map frontend config keys to backend config structure.
	// VAD settings are nested under 'vad' object, others are top-level.
	const patches: Record<string, Record<string, unknown>> = {
		language: { language: value },
		modelPath: { modelPath: value },
		vadStartDb: { vad: { startDb: value } },
		vadStopDb: { vad: { stopDb: value } },
		vadMinDuration: { vad: { minDurationMs: value } },
		vadMaxSilence: { vad: { maxSilenceMs: value } },
	};

	// Return patch object for the given key, or empty object if key not found.
	return patches[key] || {};
}

