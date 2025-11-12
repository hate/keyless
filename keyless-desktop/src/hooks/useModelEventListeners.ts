import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { logError } from '../utils/logger';
import { extractModelId, extractStringField, extractObjectField } from '../utils/payloadHelpers';

export interface ModelEventPayload {
	modelId?: string;
	model_id?: string;
	progress?: {
		bytes?: number;
		total?: number;
		mbps?: number;
		etaSeconds?: number;
	};
	text?: string;
	message?: string;
}

export interface ModelEventHandlers {
	onDownloadStarted?: (modelId: string) => void;
	onDownloadStage?: (modelId: string, stage: string) => void;
	onDownloadProgress?: (modelId: string, progress: ModelEventPayload['progress']) => void;
	onDownloadComplete?: (modelId: string) => void;
	onDownloadError?: (modelId: string, message: string) => void;
	onDownloadCancelled?: (modelId: string) => void;
	onDownloadPaused?: (modelId: string) => void;
	onModelSelected?: (modelId: string) => void;
	onModelSizesUpdated?: (updates: Array<{ modelId: string; sizeBytes?: number }>) => void;
}

/**
 * Consolidated hook for model event listeners.
 * 
 * Handles all model-related events in one place with proper cleanup.
 * Only attaches listeners for handlers that are provided (optional handlers).
 * 
 * Used by Models view to listen for download progress, completion, errors, etc.
 */
export function useModelEventListeners(handlers: ModelEventHandlers) {
	/**
	 * Set up event listeners for model-related events.
	 * 
	 * Only attaches listeners for handlers that are provided (all handlers are optional).
	 * This allows components to only listen to events they care about.
	 */
	useEffect(() => {
		const unsubs: Array<() => void> = [];

		/**
		 * Attach event listener and track for cleanup.
		 */
		const attach = async (event: string, handler: (payload: unknown) => void) => {
			try {
				const un = await listen(event, (e) => handler(e.payload));
				unsubs.push(un);
			} catch (error) {
				logError(`Failed to attach model event listener for ${event}:`, error);
			}
		};

		// Download started: notify handler if provided.
		if (handlers.onDownloadStarted) {
			attach('download_started', (payload) => {
				const modelId = extractModelId(payload);
				if (modelId) handlers.onDownloadStarted?.(modelId);
			});
		}

		// Download stage changed: notify handler with stage text.
		if (handlers.onDownloadStage) {
			attach('download_stage', (payload) => {
				const modelId = extractModelId(payload);
				const text = extractStringField(payload, 'text');
				if (modelId && text) {
					handlers.onDownloadStage?.(modelId, text);
				}
			});
		}

		// Download progress: notify handler with progress object.
		if (handlers.onDownloadProgress) {
			attach('download_progress', (payload) => {
				const modelId = extractModelId(payload);
				const progress = extractObjectField(payload, 'progress');
				if (modelId && progress) {
					handlers.onDownloadProgress?.(modelId, progress as ModelEventPayload['progress']);
				}
			});
		}

		// Download completed: notify handler.
		if (handlers.onDownloadComplete) {
			attach('download_complete', (payload) => {
				const modelId = extractModelId(payload);
				if (modelId) handlers.onDownloadComplete?.(modelId);
			});
		}

		// Download error: notify handler with error message.
		if (handlers.onDownloadError) {
			attach('download_error', (payload) => {
				const modelId = extractModelId(payload);
				const message = extractStringField(payload, 'message');
				if (modelId) {
					handlers.onDownloadError?.(modelId, message || 'download failed');
				}
			});
		}

		// Download cancelled: notify handler.
		if (handlers.onDownloadCancelled) {
			attach('download_cancelled', (payload) => {
				const modelId = extractModelId(payload);
				if (modelId) handlers.onDownloadCancelled?.(modelId);
			});
		}

		// Download paused: notify handler.
		if (handlers.onDownloadPaused) {
			attach('download_paused', (payload) => {
				const modelId = extractModelId(payload);
				if (modelId) handlers.onDownloadPaused?.(modelId);
			});
		}

		// Model selected: notify handler (handles both string and object payloads).
		if (handlers.onModelSelected) {
			attach('model_selected', (payload) => {
				const modelId = typeof payload === 'string' ? payload : extractModelId(payload);
				if (modelId) handlers.onModelSelected?.(modelId);
			});
		}

		// Model sizes updated: notify handler with array of size updates.
		if (handlers.onModelSizesUpdated) {
			attach('model_sizes_updated', (payload) => {
				if (Array.isArray(payload)) {
					const updates: Array<{ modelId: string; sizeBytes?: number }> = [];
					for (const item of payload) {
						const id = item?.modelId;
						if (typeof id === 'string') {
							updates.push({
								modelId: id,
								sizeBytes: typeof item?.sizeBytes === 'number' ? item.sizeBytes : undefined,
							});
						}
					}
					if (updates.length > 0) {
						handlers.onModelSizesUpdated?.(updates);
					}
				}
			});
		}

		// Cleanup: remove all event listeners on unmount.
		return () => {
			unsubs.forEach((un) => un());
		};
	}, [handlers]);
}

