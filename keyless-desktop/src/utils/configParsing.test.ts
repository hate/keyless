import { describe, it, expect } from 'vitest';
import {
	parseOutputMode,
	parseDeviceName,
	parseVADConfig,
	parseLanguage,
	parseModelPath,
	parseHotkey,
	parseFullConfig,
} from './configParsing';

describe('parseOutputMode', () => {
	it('parses string format', () => {
		expect(parseOutputMode('paste')).toEqual({ mode: 'paste' });
		expect(parseOutputMode('clipboard')).toEqual({ mode: 'clipboard' });
		expect(parseOutputMode('file')).toEqual({ mode: 'file' });
		expect(parseOutputMode('PASTE')).toEqual({ mode: 'paste' }); // lowercase
	});

	it('parses object with boolean flags', () => {
		expect(parseOutputMode({ paste: true })).toEqual({ mode: 'paste' });
		expect(parseOutputMode({ clipboard: true })).toEqual({ mode: 'clipboard' });
		expect(parseOutputMode({ file: true })).toEqual({ mode: 'file' });
		expect(parseOutputMode({ paste: {} })).toEqual({ mode: 'paste' });
	});

	it('parses file mode with path', () => {
		expect(parseOutputMode({ file: '/path/to/file.txt' })).toEqual({
			mode: 'file',
			filePath: '/path/to/file.txt',
		});
	});

	it('parses type/path format', () => {
		expect(parseOutputMode({ type: 'paste' })).toEqual({ mode: 'paste' });
		expect(parseOutputMode({ type: 'file', path: '/test.txt' })).toEqual({
			mode: 'file',
			filePath: '/test.txt',
		});
		expect(parseOutputMode({ type: 'file' })).toEqual({ mode: 'file' }); // no path
	});

	it('handles invalid inputs', () => {
		expect(parseOutputMode(null)).toBeNull();
		expect(parseOutputMode(undefined)).toBeNull();
		expect(parseOutputMode({})).toBeNull();
		expect(parseOutputMode({ unknown: true })).toBeNull();
	});
});

describe('parseDeviceName', () => {
	it('parses deviceName (camelCase)', () => {
		expect(parseDeviceName({ deviceName: 'Microphone' })).toBe('Microphone');
	});

	it('parses device_name (snake_case)', () => {
		expect(parseDeviceName({ device_name: 'Microphone' })).toBe('Microphone');
	});

	it('prefers deviceName over device_name', () => {
		expect(parseDeviceName({ deviceName: 'First', device_name: 'Second' })).toBe('First');
	});

	it('handles invalid inputs', () => {
		expect(parseDeviceName({})).toBeNull();
		expect(parseDeviceName({ deviceName: 123 })).toBeNull();
		expect(parseDeviceName({ device_name: null })).toBeNull();
	});
});

describe('parseVADConfig', () => {
	it('returns defaults for missing/invalid VAD config', () => {
		expect(parseVADConfig({})).toEqual({
			startDb: -45.0,
			stopDb: -50.0,
			minDurationMs: 200,
			maxSilenceMs: 800,
		});
		expect(parseVADConfig({ vad: null })).toEqual({
			startDb: -45.0,
			stopDb: -50.0,
			minDurationMs: 200,
			maxSilenceMs: 800,
		});
	});

	it('parses camelCase keys', () => {
		expect(parseVADConfig({ vad: { startDb: -40, stopDb: -45, minDurationMs: 300, maxSilenceMs: 900 } })).toEqual({
			startDb: -40,
			stopDb: -45,
			minDurationMs: 300,
			maxSilenceMs: 900,
		});
	});

	it('parses snake_case keys', () => {
		expect(parseVADConfig({ vad: { start_db: -40, stop_db: -45, min_duration_ms: 300, max_silence_ms: 900 } })).toEqual({
			startDb: -40,
			stopDb: -45,
			minDurationMs: 300,
			maxSilenceMs: 900,
		});
	});

	it('prefers camelCase over snake_case', () => {
		expect(parseVADConfig({ vad: { startDb: -40, start_db: -50 } })).toEqual({
			startDb: -40,
			stopDb: -50.0,
			minDurationMs: 200,
			maxSilenceMs: 800,
		});
	});
});

describe('parseLanguage', () => {
	it('parses valid language strings', () => {
		expect(parseLanguage({ language: 'en' })).toBe('en');
		expect(parseLanguage({ language: 'fr' })).toBe('fr');
		expect(parseLanguage({ language: 'null' })).toBe('null'); // string "null"
	});

	it('handles null/undefined', () => {
		expect(parseLanguage({ language: null })).toBeNull();
		expect(parseLanguage({ language: undefined })).toBeNull();
		expect(parseLanguage({})).toBeNull();
	});

	it('converts non-string to string', () => {
		expect(parseLanguage({ language: 123 })).toBe('123');
		expect(parseLanguage({ language: true })).toBe('true');
	});
});

describe('parseModelPath', () => {
	it('parses modelPath (camelCase)', () => {
		expect(parseModelPath({ modelPath: '/path/to/model' })).toBe('/path/to/model');
	});

	it('parses model_path (snake_case)', () => {
		expect(parseModelPath({ model_path: '/path/to/model' })).toBe('/path/to/model');
	});

	it('prefers model_path over modelPath (current implementation)', () => {
		// Note: Current implementation uses ?? operator which prefers model_path first
		expect(parseModelPath({ modelPath: '/first', model_path: '/second' })).toBe('/second');
	});

	it('returns empty string for invalid inputs', () => {
		expect(parseModelPath({})).toBe('');
		expect(parseModelPath({ modelPath: 123 })).toBe('');
	});
});

describe('parseHotkey', () => {
	it('parses Control+Shift variants', () => {
		// Note: Implementation checks for 'control+shift' substring (lowercase)
		expect(parseHotkey({ hotkey: 'control+shift' })).toBe('ctrlShift');
		expect(parseHotkey({ hotkey: 'Control+Shift' })).toBe('ctrlShift'); // lowercased first
		// Note: 'ctrl+shift' doesn't contain 'control+shift', so it defaults to ctrlOption
		expect(parseHotkey({ hotkey: 'ctrl+shift' })).toBe('ctrlOption');
		expect(parseHotkey({ hotkey: 'Ctrl+Shift' })).toBe('ctrlOption'); // lowercased to 'ctrl+shift'
	});

	it('defaults to ctrlOption for other hotkeys', () => {
		expect(parseHotkey({ hotkey: 'Control+Option' })).toBe('ctrlOption');
		expect(parseHotkey({ hotkey: 'Ctrl+Alt' })).toBe('ctrlOption');
		expect(parseHotkey({ hotkey: 'anything else' })).toBe('ctrlOption');
	});

	it('handles invalid inputs', () => {
		expect(parseHotkey({})).toBeNull();
		expect(parseHotkey({ hotkey: 123 })).toBeNull();
		expect(parseHotkey({ hotkey: null })).toBeNull();
	});
});

describe('parseFullConfig', () => {
	it('parses complete config object', () => {
		const config = {
			output_mode: { paste: true },
			deviceName: 'Microphone',
			language: 'en',
			model_path: '/path/to/model',
			vad: { startDb: -40, stopDb: -45 },
			hotkey: 'Control+Shift',
		};
		const result = parseFullConfig(config);
		expect(result.outputMode).toEqual({ mode: 'paste' });
		expect(result.deviceName).toBe('Microphone');
		expect(result.language).toBe('en');
		expect(result.modelPath).toBe('/path/to/model');
		expect(result.vad?.startDb).toBe(-40);
		expect(result.hotkey).toBe('ctrlShift');
	});

	it('handles empty/invalid config', () => {
		expect(parseFullConfig(null)).toEqual({});
		expect(parseFullConfig(undefined)).toEqual({});
		expect(parseFullConfig({})).toEqual({
			language: null,
			modelPath: '',
			vad: { startDb: -45.0, stopDb: -50.0, minDurationMs: 200, maxSilenceMs: 800 },
		});
	});

	it('handles partial config', () => {
		const result = parseFullConfig({ language: 'fr' });
		expect(result.language).toBe('fr');
		expect(result.modelPath).toBe('');
		expect(result.vad).toBeDefined();
	});
});

