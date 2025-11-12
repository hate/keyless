import { describe, it, expect } from 'vitest';
import { getConfigPatch } from './configPatches';
import type { ConfigValues } from '../types';

describe('getConfigPatch', () => {
	it('maps language to top-level language field', () => {
		expect(getConfigPatch('language', 'en')).toEqual({ language: 'en' });
		expect(getConfigPatch('language', 'fr')).toEqual({ language: 'fr' });
		expect(getConfigPatch('language', null)).toEqual({ language: null });
	});

	it('maps modelPath to top-level modelPath field', () => {
		expect(getConfigPatch('modelPath', '/path/to/model')).toEqual({ modelPath: '/path/to/model' });
		expect(getConfigPatch('modelPath', '')).toEqual({ modelPath: '' });
	});

	it('maps VAD settings to nested vad object', () => {
		expect(getConfigPatch('vadStartDb', -40)).toEqual({ vad: { startDb: -40 } });
		expect(getConfigPatch('vadStopDb', -45)).toEqual({ vad: { stopDb: -45 } });
		expect(getConfigPatch('vadMinDuration', 300)).toEqual({ vad: { minDurationMs: 300 } });
		expect(getConfigPatch('vadMaxSilence', 900)).toEqual({ vad: { maxSilenceMs: 900 } });
	});

	it('returns empty object for unknown keys', () => {
		// TypeScript should prevent this, but test runtime behavior
		expect(getConfigPatch('unknownKey' as keyof ConfigValues, 'value')).toEqual({});
	});

	it('handles all valid ConfigValues keys', () => {
		// Ensure all keys in ConfigValues are handled
		const testCases: Array<[keyof ConfigValues, ConfigValues[keyof ConfigValues], Record<string, unknown>]> = [
			['language', 'en', { language: 'en' }],
			['modelPath', '/test', { modelPath: '/test' }],
			['vadStartDb', -40, { vad: { startDb: -40 } }],
			['vadStopDb', -45, { vad: { stopDb: -45 } }],
			['vadMinDuration', 300, { vad: { minDurationMs: 300 } }],
			['vadMaxSilence', 900, { vad: { maxSilenceMs: 900 } }],
		];

		testCases.forEach(([key, value, expected]) => {
			expect(getConfigPatch(key, value)).toEqual(expected);
		});
	});
});

