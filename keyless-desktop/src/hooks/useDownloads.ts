/**
 * Download management hook for tracking active model downloads.
 * 
 * Listens to download events from the backend and maintains download state for the overlay.
 * Handles download lifecycle: started, progress, stage changes, completion, errors, cancellation, pausing.
 * Automatically removes completed/cancelled downloads after a delay.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { logError } from '../utils/logger';
import { isTauriAvailable } from '../utils/tauriHelpers';
import { extractModelId, extractStringField, extractObjectField } from '../utils/payloadHelpers';
import type { DownloadOverlayState, DownloadStatus } from '../types';

interface UseDownloadsResult {
	downloads: Record<string, DownloadOverlayState>;
	handlePause: (modelId: string) => void;
	handleResume: (modelId: string) => void;
	handleCancel: (modelId: string) => void;
	handleDismiss: (modelId: string) => void;
}

export function useDownloads(): UseDownloadsResult {
	// Map of modelId -> download state (for overlay display).
	const [downloads, setDownloads] = useState<Record<string, DownloadOverlayState>>({});
	// Timers for auto-removing completed/cancelled downloads after delay.
	const removalTimers = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

	/**
	 * Pause an active download (preserves partial files for resume).
	 */
	const handlePause = useCallback((modelId: string) => {
		invoke('pause_model_download', { modelId }).catch((error) => {
			logError('Failed to pause download:', error);
		});
	}, []);

	/**
	 * Resume a paused download (continues from where it left off).
	 */
	const handleResume = useCallback((modelId: string) => {
		invoke('resume_model_download', { modelId }).catch((error) => {
			logError('Failed to resume download:', error);
		});
	}, []);

	/**
	 * Cancel an active download (removes partial files).
	 */
	const handleCancel = useCallback((modelId: string) => {
		invoke('cancel_model_download', { modelId }).catch((error) => {
			logError('Failed to cancel download:', error);
		});
	}, []);

	/**
	 * Dismiss a download from the overlay (removes from UI immediately).
	 * 
	 * Used when user manually dismisses a completed/error download.
	 */
	const handleDismiss = useCallback((modelId: string) => {
		setDownloads((prev) => {
			const next = { ...prev };
			delete next[modelId];
			return next;
		});
	}, []);

	/**
	 * Set up event listeners for download lifecycle events.
	 * 
	 * Listens to all download events from the backend and updates local state.
	 * Handles auto-removal of completed/cancelled downloads after delays.
	 */
	useEffect(() => {
		if (!isTauriAvailable()) return;

		/**
		 * Safely clear a removal timer for a model (prevents duplicate timers).
		 */
		const safeClearTimer = (modelId: string) => {
			const timer = removalTimers.current[modelId];
			if (timer) {
				clearTimeout(timer);
				delete removalTimers.current[modelId];
			}
		};

		/**
		 * Schedule automatic removal of a download after a delay.
		 * 
		 * Used for completed/cancelled downloads that should disappear from the overlay
		 * after showing success/error state for a few seconds.
		 */
		const scheduleRemoval = (modelId: string, delay = 3000) => {
			// Clear any existing timer for this model.
			safeClearTimer(modelId);
			// Schedule removal after delay.
			removalTimers.current[modelId] = setTimeout(() => {
				setDownloads((prev) => {
					const next = { ...prev };
					delete next[modelId];
					return next;
				});
				delete removalTimers.current[modelId];
			}, delay);
		};

		/**
		 * Update download state, preserving existing fields and merging updates.
		 * 
		 * Always updates `updatedAt` timestamp and clears error unless explicitly set.
		 */
		const updateDownload = (
			modelId: string,
			updates: Partial<DownloadOverlayState>
		) => {
			setDownloads((prev) => {
				const current = prev[modelId];
				return {
					...prev,
					[modelId]: {
						modelId,
						status: current?.status ?? 'in-progress',
						stage: current?.stage,
						progress: current?.progress,
						error: undefined, // Clear error unless explicitly set in updates.
						updatedAt: Date.now(), // Always update timestamp.
						...updates,
					},
				};
			});
		};

		// Track all unsubscribe functions for cleanup.
		const unsubs: Array<() => void> = [];

		/**
		 * Attach an event listener and track it for cleanup.
		 */
		const attach = async (event: string, handler: (payload: unknown) => void) => {
			try {
				const un = await listen(event, (e) => handler(e.payload));
				unsubs.push(un);
			} catch (e) {
				logError(`Failed to attach listener for ${event}`, e);
			}
		};

		// Download started: initialize download state, clear any removal timer.
		attach('download_started', (payload) => {
			const modelId = extractModelId(payload);
			if (!modelId) return;
			safeClearTimer(modelId);
			updateDownload(modelId, {
				status: 'in-progress',
				stage: 'starting',
				progress: undefined,
			});
		});

		// Download stage changed (e.g., "downloading", "extracting"): update stage text.
		attach('download_stage', (payload) => {
			const modelId = extractModelId(payload);
			const text = extractStringField(payload, 'text');
			if (!modelId || !text) return;
			safeClearTimer(modelId);
			updateDownload(modelId, { stage: text });
		});

		// Download progress update: update progress bar (bytes, total, speed, ETA).
		attach('download_progress', (payload) => {
			const modelId = extractModelId(payload);
			const progress = extractObjectField(payload, 'progress');
			if (!modelId || !progress) return;
			safeClearTimer(modelId);
			updateDownload(modelId, {
				progress: {
					bytes: typeof progress.bytes === 'number' ? progress.bytes : 0,
					total: typeof progress.total === 'number' ? progress.total : undefined,
					mbps: typeof progress.mbps === 'number' ? progress.mbps : 0,
					etaSeconds: typeof progress.etaSeconds === 'number' ? progress.etaSeconds : 0,
				},
			});
		});

		// Download completed: mark as complete and auto-remove after 2 seconds.
		attach('download_complete', (payload) => {
			const modelId = extractModelId(payload);
			if (!modelId) return;
			updateDownload(modelId, {
				status: 'complete' as DownloadStatus,
				stage: 'ready',
			});
			// Auto-remove completed download after 2 seconds (shows success state briefly).
			scheduleRemoval(modelId, 2000);
		});

		// Download error: mark as error and auto-remove after 5 seconds (longer for user to read error).
		attach('download_error', (payload) => {
			const modelId = extractModelId(payload);
			const message = extractStringField(payload, 'message');
			if (!modelId || !message) return;
			updateDownload(modelId, {
				status: 'error' as DownloadStatus,
				stage: undefined,
				progress: undefined,
				error: message,
			});
			// Auto-remove error download after 5 seconds (gives user time to read error).
			scheduleRemoval(modelId, 5000);
		});

		// Download cancelled: mark as cancelled and auto-remove after 2 seconds.
		attach('download_cancelled', (payload) => {
			const modelId = extractModelId(payload);
			if (!modelId) return;
			updateDownload(modelId, {
				status: 'cancelled' as DownloadStatus,
				stage: undefined,
				progress: undefined,
			});
			// Auto-remove cancelled download after 2 seconds.
			scheduleRemoval(modelId, 2000);
		});

		// Download paused: mark as paused, clear removal timer (paused downloads stay visible).
		attach('download_paused', (payload) => {
			const modelId = extractModelId(payload);
			if (!modelId) return;
			// Clear removal timer (paused downloads should stay visible until resumed/dismissed).
			safeClearTimer(modelId);
			updateDownload(modelId, {
				status: 'paused' as DownloadStatus,
			});
		});

		// Cleanup: remove all event listeners and clear all timers on unmount.
		return () => {
			unsubs.forEach((un) => un());
			Object.values(removalTimers.current).forEach((timer) => clearTimeout(timer));
			removalTimers.current = {};
		};
	}, []);

	return {
		downloads,
		handlePause,
		handleResume,
		handleCancel,
		handleDismiss,
	};
}
