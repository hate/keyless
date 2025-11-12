/**
 * Model status computation utilities.
 * Pure functions for determining available actions and status labels.
 */

export type ModelAction = 'download' | 'resume' | 'cancel' | 'delete' | 'use' | 'pause' | 'play';

export interface ModelState {
	downloaded: boolean;
	downloading: boolean;
	deleting?: boolean;
	failed?: boolean;
	paused?: boolean;
	pausable?: boolean;
	downloadProgress?: number;
}

/**
 * Determine which actions are available for a model based on its state.
 */
export function getAvailableActions(state: ModelState): ModelAction[] {
	const actions: ModelAction[] = [];

	// Available: can start download.
	if (!state.downloaded && !state.downloading && !state.failed) {
		actions.push('download');
	}

	// Failed with partial progress: can resume from where it left off.
	if (state.failed && (state.downloadProgress ?? 0) > 0) {
		actions.push('resume');
	}

	// Downloaded and not busy: can delete.
	if (state.downloaded && !state.downloading && !state.deleting) {
		actions.push('delete');
	}

	// Currently downloading: can cancel.
	if (state.downloading) {
		actions.push('cancel');
	}

	// Downloaded and not being deleted: can use.
	if (state.downloaded && !state.deleting) {
		actions.push('use');
	}

	// Downloading and pausable, not paused: can pause.
	if (state.downloading && state.pausable && !state.paused) {
		actions.push('pause');
	}

	// Downloading or paused, and pausable: can resume (play).
	if ((state.downloading || state.paused) && state.pausable) {
		actions.push('play');
	}

	return actions;
}

/**
 * Get human-readable label for an action.
 */
export function getActionLabel(action: ModelAction): string {
	const labels: Record<ModelAction, string> = {
		download: 'Download',
		resume: 'Resume',
		cancel: 'Cancel',
		delete: 'Delete',
		use: 'Use Model',
		pause: 'Pause',
		play: 'Play',
	};
	return labels[action];
}

/**
 * Get color class for an action button.
 */
export function getActionColor(action: ModelAction): string {
	const colors: Record<ModelAction, string> = {
		download: 'text-statusInfo',
		resume: 'text-statusWarning',
		cancel: 'text-error',
		delete: 'text-error',
		use: 'text-statusSuccess',
		pause: 'text-statusWarning',
		play: 'text-statusSuccess',
	};
	return colors[action];
}

/**
 * Get status label for a model.
 */
export function getStatusLabel(state: ModelState): string {
	// Priority order: installed > paused > downloading > available.
	if (state.downloaded) return 'installed';
	if (state.paused) return 'paused';
	if (state.downloading) return 'downloading';
	return 'available';
}

