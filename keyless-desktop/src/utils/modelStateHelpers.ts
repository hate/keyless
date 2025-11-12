import type { BackendModelInfo, BackendModelStatus, ModelId } from '../types';

/**
 * Helper functions for updating model state in Models.tsx.
 * Reduces repetition in event handlers.
 */

/**
 * Update a single model in the models array.
 */
export function updateModelInArray(
	models: BackendModelInfo[],
	modelId: string,
	updates: Partial<BackendModelInfo>
): BackendModelInfo[] {
	// Immutably update a single model in the array by merging updates.
	return models.map((m) => (m.id === modelId ? { ...m, ...updates } : m));
}

/**
 * Update model status in the statuses record.
 */
export function updateModelStatus(
	statuses: Record<ModelId, BackendModelStatus>,
	modelId: string,
	updates: Partial<BackendModelStatus>
): Record<ModelId, BackendModelStatus> {
	// Immutably update a single model's status by merging updates with existing status.
	return {
		...statuses,
		[modelId]: {
			...statuses[modelId],
			...updates,
		},
	};
}

/**
 * Create initial status for a model starting download.
 */
export function createDownloadStartedStatus(
	existingStatus?: BackendModelStatus
): BackendModelStatus {
	// Create initial download status: preserve downloaded flag, set downloading to true, clear progress/error.
	return {
		downloaded: existingStatus?.downloaded ?? false,
		downloading: true,
		stage: 'starting',
		progress: undefined,
		error: undefined,
		paused: false,
	};
}

/**
 * Create status for download progress.
 */
export function createDownloadProgressStatus(
	progress: { bytes?: number; total?: number; mbps?: number; etaSeconds?: number },
	existingStatus?: BackendModelStatus
): BackendModelStatus {
	// Update download progress: preserve downloaded flag and stage, update progress, clear error.
	return {
		downloaded: existingStatus?.downloaded ?? false,
		downloading: true,
		stage: existingStatus?.stage,
		progress: {
			bytes: Number(progress.bytes ?? 0),
			total: typeof progress.total === 'number' ? progress.total : undefined,
			mbps: Number(progress.mbps ?? 0),
			etaSeconds: Number(progress.etaSeconds ?? 0),
		},
		error: undefined,
		paused: existingStatus?.paused ?? false,
	};
}

/**
 * Create status for completed download.
 */
export function createDownloadCompleteStatus(): BackendModelStatus {
	return {
		downloaded: true,
		downloading: false,
		stage: 'ready',
		progress: undefined,
		error: undefined,
		paused: false,
	};
}

/**
 * Create status for download error.
 */
export function createDownloadErrorStatus(
	message: string,
	existingStatus?: BackendModelStatus
): BackendModelStatus {
	return {
		downloaded: existingStatus?.downloaded ?? false,
		downloading: false,
		stage: undefined,
		progress: undefined,
		error: message,
		paused: existingStatus?.paused ?? false,
	};
}

/**
 * Create status for cancelled download.
 */
export function createDownloadCancelledStatus(
	existingStatus?: BackendModelStatus
): BackendModelStatus {
	return {
		downloaded: existingStatus?.downloaded ?? false,
		downloading: false,
		stage: undefined,
		progress: undefined,
		error: undefined,
		paused: false,
	};
}

/**
 * Create status for paused download.
 */
export function createDownloadPausedStatus(
	existingStatus?: BackendModelStatus
): BackendModelStatus {
	// Pause download: preserve all state (stage, progress, error), set paused to true, downloading to false.
	return {
		downloaded: existingStatus?.downloaded ?? false,
		downloading: false,
		stage: existingStatus?.stage,
		progress: existingStatus?.progress,
		error: existingStatus?.error,
		paused: true,
	};
}

