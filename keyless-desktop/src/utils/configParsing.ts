import type { PttListener } from '../types';
import { logWarn } from './logger';

/**
 * Configuration parsing utilities.
 * Handles parsing of config objects from various formats (snake_case, camelCase, object variants).
 * Pure functions that can be unit tested independently.
 */

export interface ParsedOutputMode {
	mode: string;
	filePath?: string;
}

export interface ParsedVADConfig {
	startDb: number;
	stopDb: number;
	minDurationMs: number;
	maxSilenceMs: number;
}

export interface ParsedConfig {
	outputMode?: ParsedOutputMode;
	deviceName?: string;
	language?: string | null;
	modelPath?: string;
	vad?: ParsedVADConfig;
	hotkey?: string;
}

const VAD_DEFAULTS = {
	startDb: -45.0,
	stopDb: -50.0,
	minDurationMs: 200,
	maxSilenceMs: 800,
} as const;


/**
 * Parse output mode from various format variations.
 * Handles:
 * - String: "paste", "clipboard", "file"
 * - Object with keys: { paste: true }, { clipboard: true }, { file: "path" }
 * - Object with type: { type: "file", path: "..." }
 */
export function parseOutputMode(raw: unknown): ParsedOutputMode | null {
	// Handle null/undefined/empty values.
	if (!raw) return null;

	// Handle string format: "paste", "clipboard", "file".
	if (typeof raw === 'string') {
		return { mode: raw.toLowerCase() };
	}

	// Handle object formats (various shapes from backend).
	if (typeof raw === 'object' && raw !== null) {
		const obj = raw as Record<string, unknown>;

		// Format: { paste: true } or { paste: {} }
		if ('paste' in obj) {
			return { mode: 'paste' };
		}

		// Format: { clipboard: true } or { clipboard: {} }
		if ('clipboard' in obj) {
			return { mode: 'clipboard' };
		}

		// Format: { file: "path/to/file" } or { file: true }
		if ('file' in obj) {
			const filePath = typeof obj.file === 'string' ? obj.file : undefined;
			return { mode: 'file', filePath };
		}

		// Format: { type: "file", path: "..." } or { type: "paste" }
		if (typeof obj.type === 'string') {
			const mode = obj.type.toLowerCase();
			// Only extract path if mode is "file" and path exists.
			const filePath = mode === 'file' && typeof obj.path === 'string' ? obj.path : undefined;
			return { mode, filePath };
		}
	}

	// Log warning for unrecognized formats (helps debug config issues).
	logWarn('[configParsing] unrecognized output_mode shape', raw);

	return null;
}

/**
 * Parse device name from config (handles both deviceName and device_name keys).
 */
export function parseDeviceName(data: Record<string, unknown>): string | null {
	const deviceName = data.deviceName ?? data.device_name;
	return typeof deviceName === 'string' ? deviceName : null;
}

/**
 * Parse VAD config with fallback to defaults.
 * Handles both camelCase and snake_case keys.
 */
export function parseVADConfig(data: Record<string, unknown>): ParsedVADConfig {
	const vad = data.vad;
	if (!vad || typeof vad !== 'object' || vad === null) {
		return { ...VAD_DEFAULTS };
	}

	const vadObj = vad as Record<string, unknown>;
	return {
		startDb: (vadObj.startDb ?? vadObj.start_db ?? VAD_DEFAULTS.startDb) as number,
		stopDb: (vadObj.stopDb ?? vadObj.stop_db ?? VAD_DEFAULTS.stopDb) as number,
		minDurationMs: (vadObj.minDurationMs ?? vadObj.min_duration_ms ?? VAD_DEFAULTS.minDurationMs) as number,
		maxSilenceMs: (vadObj.maxSilenceMs ?? vadObj.max_silence_ms ?? VAD_DEFAULTS.maxSilenceMs) as number,
	};
}

/**
 * Parse language from config (handles null/undefined).
 */
export function parseLanguage(data: Record<string, unknown>): string | null {
	const lang = data.language;
	return lang === null || lang === undefined ? null : String(lang);
}

/**
 * Parse model path from config (handles both model_path and modelPath keys).
 */
export function parseModelPath(data: Record<string, unknown>): string {
	const path = data.model_path ?? data.modelPath;
	return typeof path === 'string' ? path : '';
}

/**
 * Parse hotkey from config and normalize to 'ctrlOption' or 'ctrlShift'.
 */
export function parseHotkey(data: Record<string, unknown>): PttListener | null {
	const hotkey = data.hotkey;
	if (typeof hotkey !== 'string') return null;

	const hk = hotkey.toLowerCase();
	return hk.includes('control+shift') ? 'ctrlShift' : 'ctrlOption';
}

/**
 * Main entry point: parse full config object.
 * Returns a fully typed config object with all parsed values.
 */
export function parseFullConfig(cfg: unknown): ParsedConfig {
	if (!cfg || typeof cfg !== 'object' || cfg === null) {
		return {};
	}

	const data = cfg as Record<string, unknown>;
	const result: ParsedConfig = {};

	const outputMode = parseOutputMode(data.outputMode ?? data.output_mode);
	if (outputMode) {
		result.outputMode = outputMode;
	}

	const deviceName = parseDeviceName(data);
	if (deviceName) {
		result.deviceName = deviceName;
	}

	result.language = parseLanguage(data);
	result.modelPath = parseModelPath(data);
	result.vad = parseVADConfig(data);

	const hotkey = parseHotkey(data);
	if (hotkey) {
		result.hotkey = hotkey;
	}

	return result;
}

