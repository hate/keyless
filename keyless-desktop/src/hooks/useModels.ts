/**
 * Model catalog hook for app-level model state.
 * 
 * Tracks installed models and model sizes for fast access across the app.
 * Handles auto-selection of first installed model and listens for model events
 * (downloads, removals, catalog refreshes).
 * 
 * Used by App.tsx to determine if models are available and to pass model sizes
 * to the DownloadOverlay component.
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { logError } from '../utils/logger';
import { extractModelId } from '../utils/payloadHelpers';
import type { SettingsBootstrapPayload, BackendModelInfoWithSize } from '../types';

interface UseModelsResult {
	installedModels: string[];
	modelSizes: Record<string, number | undefined>;
	hasInstalledModel: boolean;
	autoSelectModel: (modelId: string) => void;
}

export function useModels(settingsBootstrap: SettingsBootstrapPayload | null): UseModelsResult {
	// List of installed model IDs.
	const [installedModels, setInstalledModels] = useState<string[]>([]);
	// Map of modelId -> size in bytes (for download overlay).
	const [modelSizes, setModelSizes] = useState<Record<string, number | undefined>>({});
	// Ref to installed models (for event handlers to access current state).
	const installedModelsRef = useRef<string[]>([]);

	// Computed: true if any models are installed.
	const hasInstalledModel = installedModels.length > 0;

	/**
	 * Apply installed models list (deduplicates and validates).
	 * 
	 * Only updates state if the list actually changed (prevents unnecessary re-renders).
	 */
	const applyInstalledModels = useCallback((ids: string[]) => {
		// Deduplicate and filter out invalid IDs.
		const deduped = Array.from(new Set(ids.filter((id) => typeof id === 'string' && id.length > 0)));
		setInstalledModels((prev) => {
			// Only update if list changed (shallow comparison).
			if (prev.length === deduped.length && prev.every((id, index) => id === deduped[index])) {
				return prev;
			}
			return deduped;
		});
	}, []);

	// Keep ref in sync with state (for event handlers).
	useEffect(() => {
		installedModelsRef.current = installedModels;
	}, [installedModels]);

	/**
	 * Auto-select a model (used when first model is installed).
	 * 
	 * Automatically sets the model as active when user installs their first model.
	 */
	const autoSelectModel = useCallback((modelId: string) => {
		invoke('set_model', { modelId }).catch((error) => {
			logError('Failed to auto-select model:', error);
		});
	}, []);

	/**
	 * Apply installed models from bootstrap data (fast path).
	 */
	useEffect(() => {
		if (!settingsBootstrap) {
			return;
		}
		applyInstalledModels(settingsBootstrap.installedModels ?? []);
	}, [settingsBootstrap, applyInstalledModels]);

	/**
	 * Fallback: load models if bootstrap failed.
	 * 
	 * Fetches model list with sizes and extracts installed models.
	 */
	useEffect(() => {
		if (settingsBootstrap) {
			return;
		}
		invoke<BackendModelInfoWithSize[]>('list_models_with_sizes')
			.then((list) => {
				if (!Array.isArray(list)) {
					return;
				}
				// Extract model sizes and installed model IDs.
				const next: Record<string, number | undefined> = {};
				const installed: string[] = [];
				for (const item of list) {
					const id = item?.id;
					if (typeof id === 'string') {
						next[id] =
							typeof item?.sizeBytes === 'number' ? item.sizeBytes : undefined;
						if (item?.downloaded) {
							installed.push(id);
						}
					}
				}
				setModelSizes(next);
				applyInstalledModels(installed);
			})
			.catch((error) => {
				logError('Failed to list models with sizes:', error);
			});
	}, [settingsBootstrap, applyInstalledModels]);

	/**
	 * Apply model sizes from bootstrap data (fast path).
	 */
	useEffect(() => {
		if (!settingsBootstrap?.models) {
			return;
		}
		// Extract model sizes from bootstrap models array.
		const next: Record<string, number | undefined> = {};
		for (const entry of settingsBootstrap.models) {
			if (entry && typeof entry.id === 'string') {
				next[entry.id] =
					typeof entry.sizeBytes === 'number' ? entry.sizeBytes : undefined;
			}
		}
		setModelSizes(next);
	}, [settingsBootstrap]);

	/**
	 * Listen for model-related events from backend.
	 * 
	 * Handles:
	 * - download_complete: Add to installed list, auto-select if first model
	 * - model_removed: Remove from installed list
	 * - catalog_refreshed: Clear installed list (will be repopulated)
	 * - model_sizes_updated: Update size information
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
			} catch (e) {
				logError(`Failed to attach listener for ${event}`, e);
			}
		};

		// Download completed: add to installed list, auto-select if first model.
		attach('download_complete', (payload) => {
			const modelId = extractModelId(payload);
			if (!modelId) return;
			const prevInstalled = installedModelsRef.current;
			const hadModelsBefore = prevInstalled.length > 0;
			const alreadyInstalled = prevInstalled.includes(modelId);

			if (!alreadyInstalled) {
				setInstalledModels((prev) => (prev.includes(modelId) ? prev : [...prev, modelId]));
				// Auto-select if this is the first installed model.
				if (!hadModelsBefore) {
					autoSelectModel(modelId);
				}
			}
		});

		// Model removed: remove from installed list.
		attach('model_removed', (payload) => {
			const modelId = extractModelId(payload);
			if (!modelId) return;
			setInstalledModels((prev) => prev.filter((id) => id !== modelId));
		});

		// Catalog refreshed: clear installed list (will be repopulated by other events).
		attach('catalog_refreshed', () => {
			setInstalledModels([]);
		});

		// Model sizes updated: update size map.
		attach('model_sizes_updated', (payload) => {
			if (!Array.isArray(payload)) return;
			setModelSizes((prev) => {
				const next = { ...prev } as Record<string, number | undefined>;
				for (const item of payload) {
					const id = item?.modelId;
					if (typeof id === 'string') {
						next[id] = typeof item?.sizeBytes === 'number' ? item.sizeBytes : undefined;
					}
				}
				return next;
			});
		});

		// Cleanup: remove all event listeners on unmount.
		return () => {
			unsubs.forEach((un) => un());
		};
	}, [autoSelectModel]);

	return {
		installedModels,
		modelSizes,
		hasInstalledModel,
		autoSelectModel,
	};
}
