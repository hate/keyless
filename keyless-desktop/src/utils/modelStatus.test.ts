import { describe, it, expect } from 'vitest';
import { getAvailableActions, getActionLabel, getActionColor, getStatusLabel } from './modelStatus';
import type { ModelState } from './modelStatus';

describe('getAvailableActions', () => {
	it('returns download for available model', () => {
		const state: ModelState = { downloaded: false, downloading: false };
		expect(getAvailableActions(state)).toContain('download');
		expect(getAvailableActions(state)).not.toContain('delete');
		expect(getAvailableActions(state)).not.toContain('use');
	});

	it('returns delete and use for downloaded model', () => {
		const state: ModelState = { downloaded: true, downloading: false };
		const actions = getAvailableActions(state);
		expect(actions).toContain('delete');
		expect(actions).toContain('use');
		expect(actions).not.toContain('download');
	});

	it('returns cancel for downloading model', () => {
		const state: ModelState = { downloaded: false, downloading: true };
		const actions = getAvailableActions(state);
		expect(actions).toContain('cancel');
		expect(actions).not.toContain('download');
		expect(actions).not.toContain('delete');
	});

	it('returns resume for failed download with progress', () => {
		const state: ModelState = {
			downloaded: false,
			downloading: false,
			failed: true,
			downloadProgress: 50,
		};
		const actions = getAvailableActions(state);
		expect(actions).toContain('resume');
		expect(actions).not.toContain('download');
	});

	it('does not return resume or download for failed download without progress', () => {
		// Note: When failed=true, download is not available (user must use resume if there's progress)
		const state: ModelState = {
			downloaded: false,
			downloading: false,
			failed: true,
			downloadProgress: 0,
		};
		const actions = getAvailableActions(state);
		expect(actions).not.toContain('resume');
		expect(actions).not.toContain('download'); // failed prevents download
	});

	it('returns pause for downloading pausable model', () => {
		const state: ModelState = {
			downloaded: false,
			downloading: true,
			pausable: true,
			paused: false,
		};
		const actions = getAvailableActions(state);
		expect(actions).toContain('pause');
		expect(actions).toContain('cancel');
	});

	it('returns play for paused pausable model', () => {
		const state: ModelState = {
			downloaded: false,
			downloading: false,
			pausable: true,
			paused: true,
		};
		const actions = getAvailableActions(state);
		expect(actions).toContain('play');
		expect(actions).not.toContain('pause');
	});

	it('returns play for downloading pausable model (resume)', () => {
		const state: ModelState = {
			downloaded: false,
			downloading: true,
			pausable: true,
			paused: false,
		};
		const actions = getAvailableActions(state);
		expect(actions).toContain('play'); // can resume
		expect(actions).toContain('pause');
	});

	it('does not return delete or use when deleting', () => {
		const state: ModelState = {
			downloaded: true,
			downloading: false,
			deleting: true,
		};
		const actions = getAvailableActions(state);
		expect(actions).not.toContain('delete');
		expect(actions).not.toContain('use');
	});

	it('does not return use when deleting', () => {
		const state: ModelState = {
			downloaded: true,
			downloading: false,
			deleting: true,
		};
		const actions = getAvailableActions(state);
		expect(actions).not.toContain('use');
	});

	it('handles complex state: downloaded, not downloading, not deleting', () => {
		const state: ModelState = {
			downloaded: true,
			downloading: false,
			deleting: false,
		};
		const actions = getAvailableActions(state);
		expect(actions).toContain('delete');
		expect(actions).toContain('use');
		expect(actions).not.toContain('download');
		expect(actions).not.toContain('cancel');
	});
});

describe('getActionLabel', () => {
	it('returns correct labels for all actions', () => {
		expect(getActionLabel('download')).toBe('Download');
		expect(getActionLabel('resume')).toBe('Resume');
		expect(getActionLabel('cancel')).toBe('Cancel');
		expect(getActionLabel('delete')).toBe('Delete');
		expect(getActionLabel('use')).toBe('Use Model');
		expect(getActionLabel('pause')).toBe('Pause');
		expect(getActionLabel('play')).toBe('Play');
	});
});

describe('getActionColor', () => {
	it('returns correct colors for all actions', () => {
		expect(getActionColor('download')).toBe('text-statusInfo');
		expect(getActionColor('resume')).toBe('text-statusWarning');
		expect(getActionColor('cancel')).toBe('text-error');
		expect(getActionColor('delete')).toBe('text-error');
		expect(getActionColor('use')).toBe('text-statusSuccess');
		expect(getActionColor('pause')).toBe('text-statusWarning');
		expect(getActionColor('play')).toBe('text-statusSuccess');
	});
});

describe('getStatusLabel', () => {
	it('returns "installed" for downloaded model', () => {
		expect(getStatusLabel({ downloaded: true, downloading: false })).toBe('installed');
		expect(getStatusLabel({ downloaded: true, downloading: true })).toBe('installed'); // priority
	});

	it('returns "paused" for paused model', () => {
		expect(getStatusLabel({ downloaded: false, downloading: false, paused: true })).toBe('paused');
	});

	it('returns "downloading" for downloading model', () => {
		expect(getStatusLabel({ downloaded: false, downloading: true })).toBe('downloading');
	});

	it('returns "available" for available model', () => {
		expect(getStatusLabel({ downloaded: false, downloading: false })).toBe('available');
	});

	it('prioritizes installed over paused', () => {
		expect(getStatusLabel({ downloaded: true, downloading: false, paused: true })).toBe('installed');
	});

	it('prioritizes paused over downloading', () => {
		expect(getStatusLabel({ downloaded: false, downloading: true, paused: true })).toBe('paused');
	});
});

