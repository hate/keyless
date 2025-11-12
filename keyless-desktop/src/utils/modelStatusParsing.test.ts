import { describe, it, expect } from 'vitest';
import { parseModelStatus } from './modelStatusParsing';
import type { BackendModelInfo } from '../types';

const mockModelInfo: BackendModelInfo = {
	id: 'test-model',
	downloaded: false,
	downloading: false,
};

describe('parseModelStatus', () => {
	it('uses fallback for invalid status', () => {
		expect(parseModelStatus(null, mockModelInfo)).toEqual({
			downloaded: false,
			downloading: false,
		});
		expect(parseModelStatus(undefined, mockModelInfo)).toEqual({
			downloaded: false,
			downloading: false,
		});
		expect(parseModelStatus('not an object', mockModelInfo)).toEqual({
			downloaded: false,
			downloading: false,
		});
	});

	it('parses valid status object', () => {
		const status = {
			downloaded: true,
			downloading: false,
			stage: 'downloading',
			progress: { bytes: 500000, total: 1000000, mbps: 1.5, etaSeconds: 30 },
			error: null,
			paused: false,
		};
		const result = parseModelStatus(status, mockModelInfo);
		expect(result.downloaded).toBe(true);
		expect(result.downloading).toBe(false);
		expect(result.stage).toBe('downloading');
		expect(result.progress).toEqual({ bytes: 500000, total: 1000000, mbps: 1.5, etaSeconds: 30 });
		expect(result.error).toBeUndefined();
		expect(result.paused).toBe(false);
	});

	it('handles partial status objects', () => {
		const result = parseModelStatus({ downloaded: true }, mockModelInfo);
		expect(result.downloaded).toBe(true);
		expect(result.downloading).toBe(false);
		expect(result.stage).toBeUndefined();
		expect(result.progress).toBeUndefined();
	});

	it('handles invalid progress object', () => {
		const result = parseModelStatus({ progress: 'invalid' }, mockModelInfo);
		expect(result.progress).toBeUndefined();
	});

	it('handles partial progress object', () => {
		const result = parseModelStatus({ progress: { bytes: 1000 } }, mockModelInfo);
		expect(result.progress).toEqual({ bytes: 1000, total: undefined, mbps: 0, etaSeconds: 0 });
	});
});

