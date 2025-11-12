/**
 * Settings state management hook for the Settings view.
 * 
 * Consolidates all Settings view state into a single useReducer hook for better
 * state management. Handles:
 * - UI state (expanded sections)
 * - Settings state (PTT listener, output mode, autostart, devices, file path)
 * - Config state (language, model path, VAD settings)
 * 
 * Provides:
 * - Config hydration from bootstrap data and backend
 * - Backend action handlers (toggle autostart, select sink/device, update config)
 * - Batch state updates via reducer actions
 * - Reset capability
 */

import { useReducer, useMemo, useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isEnabled as autostartIsEnabled, enable as autostartEnable, disable as autostartDisable } from '@tauri-apps/plugin-autostart';
import { logError } from '../utils/logger';
import { safeInvoke } from '../utils/errorHandling';
import { showError } from '../utils/errorNotifications';
import { parseFullConfig } from '../utils/configParsing';
import { parseHotkeyToPttListener } from '../utils/hotkeyParsing';
import { getConfigPatch } from '../utils/configPatches';
import type { PttListener, ConfigValues, ConfigResponse } from '../types';

/**
 * Settings state interface.
 * 
 * Contains all state managed by the Settings view, including UI state (expanded sections),
 * settings state (PTT listener, output mode, autostart, devices), and config state (VAD, language, model).
 */
export interface SettingsState {
	// UI state: which settings sections are expanded.
	expandedSections: {
		general: boolean;
		ai: boolean;
		vad: boolean;
	};

	// Settings state: user preferences and selections.
	pttListener: PttListener;
	outputMode: string;
	autostart: boolean;
	devices: string[];
	selectedDevice: string | null;
	filePath: string;

	// Config state: AI model configuration (VAD, language, model path).
	config: ConfigValues;
}

/**
 * Settings reducer actions.
 * 
 * All actions that can be dispatched to update Settings state.
 */
export type SettingsAction =
	| { type: 'setExpandedSection'; section: 'general' | 'ai' | 'vad'; value: boolean }
	| { type: 'setExpandedSections'; value: { general: boolean; ai: boolean; vad: boolean } }
	| { type: 'setPttListener'; value: PttListener }
	| { type: 'setOutputMode'; value: string }
	| { type: 'setAutostart'; value: boolean }
	| { type: 'setDevices'; value: string[] }
	| { type: 'setSelectedDevice'; value: string | null }
	| { type: 'setFilePath'; value: string }
	| { type: 'setConfig'; value: ConfigValues }
	| { type: 'updateConfig'; key: keyof ConfigValues; value: ConfigValues[keyof ConfigValues] }
	| { type: 'reset'; outputModeOverride?: string };

/**
 * Create initial Settings state.
 * 
 * @param outputModeOverride - Optional output mode override (used when opening Settings from a specific mode)
 * @returns Initial Settings state with default values
 */
function createInitialState(outputModeOverride?: string): SettingsState {
	return {
		expandedSections: {
			general: true,
			ai: true,
			vad: false,
		},
		pttListener: 'ctrlOption',
		outputMode: outputModeOverride ?? 'paste',
		autostart: false,
		devices: [],
		selectedDevice: null,
		filePath: '',
		config: {
			language: null,
			modelPath: '',
			vadStartDb: -45.0,
			vadStopDb: -50.0,
			vadMinDuration: 200,
			vadMaxSilence: 800,
		},
	};
}

/**
 * Settings reducer function.
 * 
 * Handles all state updates for Settings view. Returns new state based on action type.
 * 
 * @param state - Current Settings state
 * @param action - Action to apply
 * @returns New Settings state
 */
function settingsReducer(state: SettingsState, action: SettingsAction): SettingsState {
	switch (action.type) {
		// Update a single expanded section.
		case 'setExpandedSection':
			return {
				...state,
				expandedSections: {
					...state.expandedSections,
					[action.section]: action.value,
				},
			};

		// Update all expanded sections at once.
		case 'setExpandedSections':
			return {
				...state,
				expandedSections: action.value,
			};

		// Update PTT listener (hotkey type: ctrlOption or ctrlShift).
		case 'setPttListener':
			return { ...state, pttListener: action.value };

		// Update output mode (paste, clipboard, or file).
		case 'setOutputMode':
			return { ...state, outputMode: action.value };

		// Update autostart enabled state.
		case 'setAutostart':
			return { ...state, autostart: action.value };

		// Update available input devices list.
		case 'setDevices':
			return { ...state, devices: action.value };

		// Update selected input device (microphone).
		case 'setSelectedDevice':
			return { ...state, selectedDevice: action.value };

		// Update file path (for file output mode).
		case 'setFilePath':
			return { ...state, filePath: action.value };

		// Replace entire config object.
		case 'setConfig':
			return { ...state, config: action.value };

		// Update a single config field.
		case 'updateConfig':
			return {
				...state,
				config: {
					...state.config,
					[action.key]: action.value,
				},
			};

		// Reset state to initial values (with optional output mode override).
		case 'reset':
			return createInitialState(action.outputModeOverride);

		default:
			return state;
	}
}

/**
 * Bootstrap data interface for Settings state hydration.
 * 
 * Contains pre-loaded data from the initial page load to avoid first-open lag.
 */
export interface SettingsBootstrapData {
	config?: Record<string, unknown>;
	hotkey?: string;
	autostart?: boolean;
	inputDevices?: string[];
}

/**
 * Consolidated state management hook for Settings view.
 * Replaces multiple useState hooks with a single useReducer for better state management.
 * Provides batch updates, reset capability, config hydration, and backend action handlers.
 */
export function useSettingsState(
	outputModeOverride?: string,
	bootstrap?: SettingsBootstrapData
) {
	const [state, dispatch] = useReducer(settingsReducer, outputModeOverride, createInitialState);
	const loadedFromConfigRef = useRef(false);

	/**
	 * Apply parsed config values to state.
	 * 
	 * Dispatches state updates for each config field found in the parsed config.
	 * Marks that config has been loaded to prevent outputModeOverride from overriding loaded values.
	 */
	const applyConfig = useCallback(
		(cfg: unknown) => {
			// Parse the config object (handles various formats: snake_case, camelCase, etc.).
			const parsed = parseFullConfig(cfg);

			// Apply output mode (paste, clipboard, or file).
			if (parsed.outputMode) {
				dispatch({ type: 'setOutputMode', value: parsed.outputMode.mode });
				// If file mode, also set the file path.
				if (parsed.outputMode.filePath) {
					dispatch({ type: 'setFilePath', value: parsed.outputMode.filePath });
				}
				// Mark that config has been loaded (prevents outputModeOverride from overriding).
				loadedFromConfigRef.current = true;
			}

			// Apply selected input device (microphone).
			if (parsed.deviceName) {
				dispatch({ type: 'setSelectedDevice', value: parsed.deviceName });
			}

			// Apply VAD (Voice Activity Detection) settings.
			if (parsed.vad) {
				dispatch({ type: 'updateConfig', key: 'vadStartDb', value: parsed.vad.startDb });
				dispatch({ type: 'updateConfig', key: 'vadStopDb', value: parsed.vad.stopDb });
				dispatch({
					type: 'updateConfig',
					key: 'vadMinDuration',
					value: parsed.vad.minDurationMs,
				});
				dispatch({
					type: 'updateConfig',
					key: 'vadMaxSilence',
					value: parsed.vad.maxSilenceMs,
				});
			}

			// Apply language setting (null = auto-detect).
			if (parsed.language !== undefined) {
				dispatch({ type: 'updateConfig', key: 'language', value: parsed.language ?? null });
			}

			// Apply model path (selected model ID).
			if (parsed.modelPath !== undefined) {
				dispatch({ type: 'updateConfig', key: 'modelPath', value: parsed.modelPath });
			}

			// Apply hotkey (PTT listener type: ctrlOption or ctrlShift).
			if (parsed.hotkey === 'ctrlOption' || parsed.hotkey === 'ctrlShift') {
				dispatch({ type: 'setPttListener', value: parsed.hotkey as PttListener });
			}
		},
		[]
	);

	/**
	 * Hydrate state from bootstrap data and backend on mount.
	 * 
	 * Loads initial state from three sources (in order):
	 * 1. Bootstrap data (fast, from initial page load)
	 * 2. Backend API calls (fallback if bootstrap incomplete)
	 * 3. Full config (most complete, includes all settings)
	 * 
	 * Uses cancellation flag to prevent state updates if component unmounts during async operations.
	 */
	useEffect(() => {
		let cancelled = false;

		/**
		 * Hydrate from bootstrap data (fast path, loaded on initial page load).
		 * Returns true if bootstrap was available, false otherwise.
		 */
		const hydrateFromBootstrap = () => {
			if (!bootstrap) {
				return false;
			}
			// Apply hotkey from bootstrap.
			if (typeof bootstrap.hotkey === 'string') {
				dispatch({
					type: 'setPttListener',
					value: parseHotkeyToPttListener(bootstrap.hotkey),
				});
			}
			// Apply autostart status from bootstrap.
			if (typeof bootstrap.autostart === 'boolean') {
				dispatch({ type: 'setAutostart', value: bootstrap.autostart });
			}
			// Apply input devices list from bootstrap.
			if (Array.isArray(bootstrap.inputDevices)) {
				dispatch({ type: 'setDevices', value: bootstrap.inputDevices });
			}
			// Apply full config from bootstrap (includes output mode, VAD, language, etc.).
			if (bootstrap.config) {
				applyConfig(bootstrap.config);
			}
			return true;
		};

		/**
		 * Load fallback state from backend API (if bootstrap incomplete).
		 * 
		 * Fetches hotkey, autostart status, and input devices in parallel.
		 * This is a fallback in case bootstrap data is missing or incomplete.
		 */
		const loadFallbackState = async () => {
			// Fetch all fallback data in parallel for better performance.
			const [hotkeyValue, autostartValue, devicesValue] = await Promise.all([
				invoke<string>('get_hotkey').catch((error) => {
					logError('Failed to get hotkey in loadFallbackState:', error);
					return undefined;
				}),
				autostartIsEnabled().catch((error) => {
					logError('Failed to check autostart in loadFallbackState:', error);
					return false;
				}),
				invoke<string[]>('list_input_devices').catch((error) => {
					logError('Failed to list input devices in loadFallbackState:', error);
					return undefined;
				}),
			]);
			// Check if component was unmounted during async operations.
			if (cancelled) {
				return;
			}
			// Apply fallback values if available.
			if (typeof hotkeyValue === 'string') {
				dispatch({
					type: 'setPttListener',
					value: parseHotkeyToPttListener(hotkeyValue),
				});
			}
			dispatch({ type: 'setAutostart', value: Boolean(autostartValue) });
			if (Array.isArray(devicesValue)) {
				dispatch({ type: 'setDevices', value: devicesValue });
			}
		};

		/**
		 * Load full config from backend (most complete source).
		 * 
		 * This includes all settings: output mode, VAD, language, model, etc.
		 * Applied last to ensure it takes precedence over bootstrap/fallback values.
		 */
		const loadConfig = async () => {
			try {
				const cfg = await invoke<ConfigResponse>('get_config');
				// Check cancellation before applying config.
				if (!cancelled) {
					applyConfig(cfg);
				}
			} catch (error) {
				logError('Failed to load config in useSettingsState:', error);
				// UI will rely on defaults if config load fails.
			}
		};

		// Execute all hydration steps (bootstrap is synchronous, others are async).
		hydrateFromBootstrap();
		loadFallbackState();
		loadConfig();

		// Cleanup: mark as cancelled to prevent state updates after unmount.
		return () => {
			cancelled = true;
		};
	}, [bootstrap, applyConfig]);

	/**
	 * Handle outputModeOverride prop changes.
	 * 
	 * Only applies override if config hasn't been loaded yet (prevents override from
	 * overwriting loaded config values). This allows parent components to set initial
	 * output mode before config is loaded.
	 */
	useEffect(() => {
		// Only apply override if config hasn't been loaded yet.
		if (!loadedFromConfigRef.current && typeof outputModeOverride === 'string') {
			const norm = outputModeOverride.toLowerCase();
			// Only update if value actually changed (prevents unnecessary dispatches).
			if (norm !== state.outputMode) {
				dispatch({ type: 'setOutputMode', value: norm });
			}
		}
	}, [outputModeOverride, state.outputMode]);

	const actions = useMemo(
		() => ({
			setExpandedSection: (section: 'general' | 'ai' | 'vad', value: boolean) =>
				dispatch({ type: 'setExpandedSection', section, value }),
			setExpandedSections: (value: { general: boolean; ai: boolean; vad: boolean }) =>
				dispatch({ type: 'setExpandedSections', value }),
			setPttListener: (value: PttListener) =>
				dispatch({ type: 'setPttListener', value }),
			setOutputMode: (value: string) => dispatch({ type: 'setOutputMode', value }),
			setAutostart: (value: boolean) => dispatch({ type: 'setAutostart', value }),
			setDevices: (value: string[]) => dispatch({ type: 'setDevices', value }),
			setSelectedDevice: (value: string | null) =>
				dispatch({ type: 'setSelectedDevice', value }),
			setFilePath: (value: string) => dispatch({ type: 'setFilePath', value }),
			setConfig: (value: ConfigValues) => dispatch({ type: 'setConfig', value }),
			updateConfig: (key: keyof ConfigValues, value: ConfigValues[keyof ConfigValues]) =>
				dispatch({ type: 'updateConfig', key, value }),
			reset: (outputModeOverride?: string) =>
				dispatch({ type: 'reset', outputModeOverride }),
			// Backend action handlers
			onToggleAutostart: async (next: boolean) => {
				dispatch({ type: 'setAutostart', value: next });
				try {
					if (next) await autostartEnable();
					else await autostartDisable();
				} catch (error) {
					logError('Failed to toggle autostart:', error);
				}
			},
			onSelectSink: async (id: string) => {
				dispatch({ type: 'setOutputMode', value: id });
				await safeInvoke('select_sink', { id }, true);
			},
			onSelectDevice: async (deviceName: string) => {
				dispatch({ type: 'setSelectedDevice', value: deviceName });
				await safeInvoke('update_config', { patch: { deviceName } }, true);
			},
			/**
			 * Update a config value and persist to backend.
			 * 
			 * Optimistically updates local state, then persists to backend.
			 * Handles special cases for language and modelPath (use dedicated commands).
			 */
			updateConfigValue: async (
				key: keyof ConfigValues,
				value: ConfigValues[keyof ConfigValues]
			) => {
				// Optimistically update local state (immediate UI feedback).
				dispatch({ type: 'updateConfig', key, value });

				// Language uses dedicated command (set_language).
				if (key === 'language') {
					try {
						await invoke('set_language', { language: value });
					} catch (e) {
						logError('set_language failed', e);
						await showError('Failed to update language setting', e);
					}
					return;
				}

				// Model path is handled via dedicated set_model command (called elsewhere).
				if (key === 'modelPath') {
					// Handled via dedicated set_model command (not here).
					return;
				}

				// All other config values use generic update_config with a patch.
				const patch = getConfigPatch(key, value);
				await safeInvoke('update_config', { patch }, true);
			},
		}),
		[state.outputMode] // Only include state.outputMode for outputModeOverride effect
	);

	return {
		state,
		actions,
	};
}

