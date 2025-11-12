/**
 * Models state management hook for the Models view.
 * 
 * Consolidates all Models view state into a single useReducer hook for better
 * state management. Handles:
 * - Model catalog (list of available models)
 * - Model statuses (download state, progress, errors)
 * - Model sizes (for display and sorting)
 * - UI state (fetching flag, sort order, current model)
 * 
 * Provides:
 * - Model refresh logic (fetches catalog and statuses from backend)
 * - Download lifecycle handlers (start, progress, complete, error, cancel, pause)
 * - Backend action handlers (download, use, resume, cancel, delete)
 * - Event handler integration (for useModelEventListeners)
 */

import { useReducer, useMemo, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { logError } from '../utils/logger';
import { showError } from '../utils/errorNotifications';
import type { BackendModelInfo, BackendModelStatus, ModelId, ModelStatusResponse, BackendModelInfoWithSize, ConfigResponse } from '../types';
import { parseModelStatus } from '../utils/modelStatusParsing';
import {
	updateModelInArray,
	updateModelStatus,
	createDownloadStartedStatus,
	createDownloadProgressStatus,
	createDownloadCompleteStatus,
	createDownloadErrorStatus,
	createDownloadCancelledStatus,
	createDownloadPausedStatus,
} from '../utils/modelStateHelpers';

/**
 * Models state interface.
 * 
 * Contains all state managed by the Models view, including model catalog,
 * statuses, sizes, and UI state.
 */
export interface ModelsState {
	// Model catalog: list of all available models.
	models: BackendModelInfo[];
	// Model statuses: download state, progress, errors for each model.
	modelStatuses: Record<ModelId, BackendModelStatus>;
	// Model sizes: size in bytes for each model (for display and sorting).
	modelSizes: Record<ModelId, number | undefined>;
	// UI state: true when fetching models from backend.
	isFetching: boolean;
	// UI state: sort order for model list ('status' or 'size').
	sort: 'status' | 'size';
	// UI state: currently selected model ID (for highlighting).
	currentModel: string;
}

/**
 * Models reducer actions.
 * 
 * All actions that can be dispatched to update Models state.
 */
export type ModelsAction =
	| { type: 'setModels'; value: BackendModelInfo[] }
	| { type: 'setModelStatuses'; value: Record<ModelId, BackendModelStatus> }
	| { type: 'updateModelStatus'; modelId: string; updates: Partial<BackendModelStatus> }
	| { type: 'setModelSizes'; value: Record<ModelId, number | undefined> }
	| { type: 'updateModelSizes'; updates: Array<{ modelId: string; sizeBytes?: number }> }
	| { type: 'setIsFetching'; value: boolean }
	| { type: 'setSort'; value: 'status' | 'size' }
	| { type: 'setCurrentModel'; value: string }
	| { type: 'updateModelInArray'; modelId: string; updates: Partial<BackendModelInfo> }
	| { type: 'handleDownloadStarted'; modelId: string }
	| { type: 'handleDownloadStage'; modelId: string; stage: string }
	| { type: 'handleDownloadProgress'; modelId: string; progress: { bytes?: number; total?: number; mbps?: number; etaSeconds?: number } }
	| { type: 'handleDownloadComplete'; modelId: string }
	| { type: 'handleDownloadError'; modelId: string; message: string }
	| { type: 'handleDownloadCancelled'; modelId: string }
	| { type: 'handleDownloadPaused'; modelId: string };

/**
 * Create initial Models state.
 * 
 * @returns Initial Models state with empty arrays/objects and default values
 */
function createInitialState(): ModelsState {
	return {
		models: [],
		modelStatuses: {},
		modelSizes: {},
		isFetching: false,
		sort: 'status',
		currentModel: '',
	};
}

/**
 * Models reducer function.
 * 
 * Handles all state updates for Models view. Returns new state based on action type.
 * Handles download lifecycle events, model updates, and UI state changes.
 * 
 * @param state - Current Models state
 * @param action - Action to apply
 * @returns New Models state
 */
function modelsReducer(state: ModelsState, action: ModelsAction): ModelsState {
	switch (action.type) {
		// Replace entire models array.
		case 'setModels':
			return { ...state, models: action.value };

		// Replace entire model statuses map.
		case 'setModelStatuses':
			return { ...state, modelStatuses: action.value };

		// Update a single model's status (partial update).
		case 'updateModelStatus':
			return {
				...state,
				modelStatuses: updateModelStatus(state.modelStatuses, action.modelId, action.updates),
			};

		// Replace entire model sizes map.
		case 'setModelSizes':
			return { ...state, modelSizes: action.value };

		// Update model sizes from array of updates.
		case 'updateModelSizes':
			return {
				...state,
				modelSizes: (() => {
					const next = { ...state.modelSizes };
					for (const item of action.updates) {
						next[item.modelId] = item.sizeBytes;
					}
					return next;
				})(),
			};

		// Update fetching flag (for loading state).
		case 'setIsFetching':
			return { ...state, isFetching: action.value };

		// Update sort order.
		case 'setSort':
			return { ...state, sort: action.value };

		// Update currently selected model.
		case 'setCurrentModel':
			return { ...state, currentModel: action.value };

		// Update a single model in the models array (partial update).
		case 'updateModelInArray':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, action.updates),
			};

		// Download started: mark model as downloading and create started status.
		case 'handleDownloadStarted':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, { downloading: true }),
				modelStatuses: updateModelStatus(
					state.modelStatuses,
					action.modelId,
					createDownloadStartedStatus(state.modelStatuses[action.modelId])
				),
			};

		// Download stage changed: update stage text (e.g., "Downloading", "Extracting").
		case 'handleDownloadStage':
			return {
				...state,
				modelStatuses: updateModelStatus(state.modelStatuses, action.modelId, {
					downloaded: state.modelStatuses[action.modelId]?.downloaded ?? false,
					downloading: true,
					stage: action.stage,
					error: undefined,
				}),
			};

		// Download progress: update progress information (bytes, total, mbps, eta).
		case 'handleDownloadProgress':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, { downloading: true }),
				modelStatuses: updateModelStatus(
					state.modelStatuses,
					action.modelId,
					createDownloadProgressStatus(action.progress, state.modelStatuses[action.modelId])
				),
			};

		// Download completed: mark model as downloaded and not downloading.
		case 'handleDownloadComplete':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, {
					downloaded: true,
					downloading: false,
				}),
				modelStatuses: updateModelStatus(
					state.modelStatuses,
					action.modelId,
					createDownloadCompleteStatus()
				),
			};

		// Download error: mark as not downloading and store error message.
		case 'handleDownloadError':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, { downloading: false }),
				modelStatuses: updateModelStatus(
					state.modelStatuses,
					action.modelId,
					createDownloadErrorStatus(action.message, state.modelStatuses[action.modelId])
				),
			};

		// Download cancelled: mark as not downloading and create cancelled status.
		case 'handleDownloadCancelled':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, { downloading: false }),
				modelStatuses: updateModelStatus(
					state.modelStatuses,
					action.modelId,
					createDownloadCancelledStatus(state.modelStatuses[action.modelId])
				),
			};

		// Download paused: mark as not downloading and create paused status.
		case 'handleDownloadPaused':
			return {
				...state,
				models: updateModelInArray(state.models, action.modelId, { downloading: false }),
				modelStatuses: updateModelStatus(
					state.modelStatuses,
					action.modelId,
					createDownloadPausedStatus(state.modelStatuses[action.modelId])
				),
			};

		default:
			return state;
	}
}

/**
 * Consolidated state management hook for Models view.
 * Replaces multiple useState hooks with a single useReducer for better state management.
 * Provides refresh logic, action handlers, and event handler integration.
 */
export function useModelsState() {
	const [state, dispatch] = useReducer(modelsReducer, undefined, createInitialState);
	const refreshRef = useRef<(() => Promise<void>) | null>(null);

	/**
	 * Refresh model catalog and statuses from backend.
	 * 
	 * Fetches model list with sizes, then queries status for each model.
	 * This ensures we have accurate download state even if downloads started
	 * while the Models view was closed.
	 */
	const refreshModels = useCallback(async () => {
		// Set fetching flag to show loading state in UI.
		dispatch({ type: 'setIsFetching', value: true });
		try {
			// Fetch all models with size information (includes download status).
			const listWithSizes = await invoke<BackendModelInfoWithSize[]>('list_models_with_sizes');
			// Extract base model info (id, downloaded, downloading flags).
			const base: BackendModelInfo[] = listWithSizes.map((m) => ({
				id: m.id,
				downloaded: m.downloaded,
				downloading: m.downloading,
			}));
			dispatch({ type: 'setModels', value: base });

			// Extract size information into a map (modelId -> size in bytes).
			const sizes: Record<ModelId, number | undefined> = {};
			listWithSizes.forEach((m) => {
				sizes[m.id] = typeof m.sizeBytes === 'number' ? m.sizeBytes : undefined;
			});
			dispatch({ type: 'setModelSizes', value: sizes });

			// Query detailed status for each model (covers downloads started while view was closed).
			// This ensures we have accurate progress, stage, and error information.
			const statuses = await Promise.all(
				base.map(async (m) => {
					try {
						const s = await invoke<ModelStatusResponse>('get_model_status', { modelId: m.id });
						return [m.id, s] as const;
					} catch (error) {
						logError(`Failed to get status for model ${m.id}:`, error);
						// Return null on error (will use fallback status).
						return [m.id, null] as const;
					}
				})
			);

			// Parse statuses and create status map (modelId -> BackendModelStatus).
			const parsedStatuses: Record<ModelId, BackendModelStatus> = {};
			for (const [id, s] of statuses) {
				// Find fallback model info (for parsing status with defaults).
				const fallback = base.find((m) => m.id === id)!;
				// Parse status (handles null case with fallback).
				parsedStatuses[id] = parseModelStatus(s, fallback);
			}
			dispatch({ type: 'setModelStatuses', value: parsedStatuses });
		} catch (error) {
			logError('Failed to refresh models:', error);
		} finally {
			// Always clear fetching flag (even on error).
			dispatch({ type: 'setIsFetching', value: false });
		}
	}, []);

	// Store refresh function in ref so event handlers can access it (avoids stale closure issues).
	refreshRef.current = refreshModels;

	/**
	 * Load current model selection from config on mount.
	 * 
	 * Reads the model_path/modelPath field from config to determine which model
	 * is currently selected (for highlighting in the UI).
	 */
	useEffect(() => {
		invoke<ConfigResponse>('get_config')
			.then((cfg) => {
				if (cfg && typeof cfg === 'object') {
					// Handle both snake_case and camelCase keys.
					const val = cfg.model_path || cfg.modelPath;
					if (typeof val === 'string') {
						dispatch({ type: 'setCurrentModel', value: val });
					}
				}
			})
			.catch((error) => {
				logError('Failed to get current model from config:', error);
			});
	}, []);

	const actions = useMemo(
		() => ({
			refreshModels,
			setSort: (value: 'status' | 'size') => dispatch({ type: 'setSort', value }),
			setCurrentModel: (value: string) => dispatch({ type: 'setCurrentModel', value }),
			onDownload: async (id: string) => {
				try {
					await invoke('start_model_download', { modelId: id });
				} catch (error) {
					logError(`Failed to start download for model ${id}:`, error);
					await showError(`Failed to start download for ${id}`, error);
				}
			},
			onUse: async (id: string) => {
				// Store previous model ID for error recovery.
				const previousModelId = state.currentModel;
				// Optimistically update UI state immediately (prevents UI freeze).
				dispatch({ type: 'setCurrentModel', value: id });
				// Call backend asynchronously (don't block UI).
				// The backend will emit 'model_selected' event when done, which will update state again.
				invoke('set_model', { modelId: id }).catch((error) => {
					logError(`Failed to set model ${id}:`, error);
					// Revert optimistic update on error (restore previous model).
					dispatch({ type: 'setCurrentModel', value: previousModelId });
					showError(`Failed to select model ${id}`, error).catch(() => {});
				});
			},
			onResume: async (id: string) => {
				try {
					await invoke('resume_model_download', { modelId: id });
				} catch (error) {
					logError(`Failed to resume download for model ${id}:`, error);
					await showError(`Failed to resume download for ${id}`, error);
				}
			},
			onCancel: async (id: string) => {
				try {
					await invoke('cancel_model_download', { modelId: id });
				} catch (error) {
					logError(`Failed to cancel download for model ${id}:`, error);
					await showError(`Failed to cancel download for ${id}`, error);
				}
			},
			onDelete: async (id: string) => {
				try {
					await invoke('remove_model', { modelId: id });
					await refreshModels();
				} catch (error) {
					logError(`Failed to delete model ${id}:`, error);
					await showError(`Failed to delete model ${id}`, error);
				}
			},
			// Event handler actions (for useModelEventListeners)
			handleDownloadStarted: (modelId: string) =>
				dispatch({ type: 'handleDownloadStarted', modelId }),
			handleDownloadStage: (modelId: string, stage: string) =>
				dispatch({ type: 'handleDownloadStage', modelId, stage }),
			handleDownloadProgress: (
				modelId: string,
				progress: { bytes?: number; total?: number; mbps?: number; etaSeconds?: number }
			) => dispatch({ type: 'handleDownloadProgress', modelId, progress }),
			handleDownloadComplete: (modelId: string) => {
				dispatch({ type: 'handleDownloadComplete', modelId });
				refreshRef.current?.();
			},
			handleDownloadError: (modelId: string, message: string) =>
				dispatch({ type: 'handleDownloadError', modelId, message }),
			handleDownloadCancelled: (modelId: string) =>
				dispatch({ type: 'handleDownloadCancelled', modelId }),
			handleDownloadPaused: (modelId: string) =>
				dispatch({ type: 'handleDownloadPaused', modelId }),
			handleModelSelected: (modelId: string) => {
				dispatch({ type: 'setCurrentModel', value: modelId });
				refreshRef.current?.();
			},
			handleModelSizesUpdated: (updates: Array<{ modelId: string; sizeBytes?: number }>) =>
				dispatch({ type: 'updateModelSizes', updates }),
		}),
		[refreshModels]
	);

	return { state, actions };
}

