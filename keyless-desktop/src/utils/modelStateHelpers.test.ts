import { describe, it, expect } from 'vitest';
import {
	updateModelInArray,
	updateModelStatus,
	createDownloadStartedStatus,
	createDownloadProgressStatus,
	createDownloadCompleteStatus,
	createDownloadErrorStatus,
	createDownloadCancelledStatus,
	createDownloadPausedStatus,
} from './modelStateHelpers';
import type { BackendModelInfo, BackendModelStatus } from '../types';

const mockModel: BackendModelInfo = {
	id: 'test-model',
	name: 'Test Model',
	size: 1000000,
	downloaded: false,
	downloading: false,
};

describe('updateModelInArray', () => {
	it('updates a single model in array', () => {
		const models = [mockModel, { ...mockModel, id: 'other-model' }];
		const result = updateModelInArray(models, 'test-model', { downloaded: true });
		expect(result[0].downloaded).toBe(true);
		expect(result[1].downloaded).toBe(false); // unchanged
	});

	it('leaves other models unchanged', () => {
		const models = [mockModel, { ...mockModel, id: 'other-model', name: 'Other' }];
		const result = updateModelInArray(models, 'test-model', { name: 'Updated' });
		expect(result[0].name).toBe('Updated');
		expect(result[1].name).toBe('Other');
	});

	it('returns new array (immutability)', () => {
		const models = [mockModel];
		const result = updateModelInArray(models, 'test-model', { downloaded: true });
		expect(result).not.toBe(models);
		expect(models[0].downloaded).toBe(false); // original unchanged
	});

	it('handles non-existent model id', () => {
		const models = [mockModel];
		const result = updateModelInArray(models, 'non-existent', { downloaded: true });
		expect(result).toEqual(models);
	});
});

describe('updateModelStatus', () => {
	it('updates status for existing model', () => {
		const statuses = { 'test-model': { downloaded: false, downloading: false } };
		const result = updateModelStatus(statuses, 'test-model', { downloading: true });
		expect(result['test-model'].downloading).toBe(true);
		expect(result['test-model'].downloaded).toBe(false); // preserved
	});

	it('creates status for new model', () => {
		const statuses: Record<string, BackendModelStatus> = {};
		const result = updateModelStatus(statuses, 'new-model', { downloading: true });
		expect(result['new-model'].downloading).toBe(true);
	});

	it('merges updates with existing status', () => {
		const statuses = {
			'test-model': {
				downloaded: false,
				downloading: true,
				stage: 'downloading',
				progress: { bytes: 1000, mbps: 1.0, etaSeconds: 10 },
			},
		};
		const result = updateModelStatus(statuses, 'test-model', { stage: 'completed' });
		expect(result['test-model'].stage).toBe('completed');
		expect(result['test-model'].downloading).toBe(true); // preserved
		expect(result['test-model'].progress).toBeDefined(); // preserved
	});

	it('returns new object (immutability)', () => {
		const statuses = { 'test-model': { downloaded: false, downloading: false } };
		const result = updateModelStatus(statuses, 'test-model', { downloading: true });
		expect(result).not.toBe(statuses);
		expect(statuses['test-model'].downloading).toBe(false); // original unchanged
	});
});

describe('createDownloadStartedStatus', () => {
	it('creates initial download status', () => {
		const status = createDownloadStartedStatus();
		expect(status.downloaded).toBe(false);
		expect(status.downloading).toBe(true);
		expect(status.stage).toBe('starting');
		expect(status.progress).toBeUndefined();
		expect(status.error).toBeUndefined();
		expect(status.paused).toBe(false);
	});

	it('preserves downloaded flag from existing status', () => {
		const existing = { downloaded: true, downloading: false };
		const status = createDownloadStartedStatus(existing);
		expect(status.downloaded).toBe(true);
		expect(status.downloading).toBe(true);
	});
});

describe('createDownloadProgressStatus', () => {
	it('creates progress status with all fields', () => {
		const status = createDownloadProgressStatus({
			bytes: 500000,
			total: 1000000,
			mbps: 1.5,
			etaSeconds: 30,
		});
		expect(status.downloading).toBe(true);
		expect(status.progress).toEqual({
			bytes: 500000,
			total: 1000000,
			mbps: 1.5,
			etaSeconds: 30,
		});
		expect(status.error).toBeUndefined();
	});

	it('handles partial progress data', () => {
		const status = createDownloadProgressStatus({ bytes: 1000 });
		expect(status.progress).toEqual({
			bytes: 1000,
			total: undefined,
			mbps: 0,
			etaSeconds: 0,
		});
	});

	it('preserves stage from existing status', () => {
		const existing = { downloaded: false, downloading: true, stage: 'downloading' };
		const status = createDownloadProgressStatus({ bytes: 1000 }, existing);
		expect(status.stage).toBe('downloading');
	});

	it('clears error on progress update', () => {
		const existing = { downloaded: false, downloading: true, error: 'Previous error' };
		const status = createDownloadProgressStatus({ bytes: 1000 }, existing);
		expect(status.error).toBeUndefined();
	});
});

describe('createDownloadCompleteStatus', () => {
	it('creates completed download status', () => {
		const status = createDownloadCompleteStatus();
		expect(status.downloaded).toBe(true);
		expect(status.downloading).toBe(false);
		expect(status.stage).toBe('ready');
		expect(status.progress).toBeUndefined();
		expect(status.error).toBeUndefined();
		expect(status.paused).toBe(false);
	});
});

describe('createDownloadErrorStatus', () => {
	it('creates error status with message', () => {
		const status = createDownloadErrorStatus('Network error');
		expect(status.downloading).toBe(false);
		expect(status.error).toBe('Network error');
		expect(status.stage).toBeUndefined();
		expect(status.progress).toBeUndefined();
	});

	it('preserves downloaded flag from existing status', () => {
		const existing = { downloaded: true, downloading: true };
		const status = createDownloadErrorStatus('Error', existing);
		expect(status.downloaded).toBe(true);
		expect(status.downloading).toBe(false);
	});
});

describe('createDownloadCancelledStatus', () => {
	it('creates cancelled status', () => {
		const status = createDownloadCancelledStatus();
		expect(status.downloading).toBe(false);
		expect(status.stage).toBeUndefined();
		expect(status.progress).toBeUndefined();
		expect(status.error).toBeUndefined();
		expect(status.paused).toBe(false);
	});

	it('preserves downloaded flag from existing status', () => {
		const existing = { downloaded: true, downloading: true };
		const status = createDownloadCancelledStatus(existing);
		expect(status.downloaded).toBe(true);
		expect(status.downloading).toBe(false);
	});
});

describe('createDownloadPausedStatus', () => {
	it('creates paused status preserving all state', () => {
		const existing = {
			downloaded: false,
			downloading: true,
			stage: 'downloading',
			progress: { bytes: 500000, mbps: 1.0, etaSeconds: 30 },
			error: undefined,
			paused: false,
		};
		const status = createDownloadPausedStatus(existing);
		expect(status.downloading).toBe(false);
		expect(status.paused).toBe(true);
		expect(status.stage).toBe('downloading'); // preserved
		expect(status.progress).toEqual(existing.progress); // preserved
	});

	it('preserves downloaded flag', () => {
		const existing = { downloaded: true, downloading: true };
		const status = createDownloadPausedStatus(existing);
		expect(status.downloaded).toBe(true);
	});
});

