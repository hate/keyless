/**
 * Settings data hook for app-level settings state.
 * 
 * Pre-warms settings data (bootstrap, hotkey, output mode) to avoid first-open lag.
 * Provides fast access to settings data for components that need it before Settings view opens.
 * 
 * Loads settings bootstrap on mount and keeps output mode in sync with backend changes.
 */

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { isEnabled as autostartIsEnabled } from '@tauri-apps/plugin-autostart';
import { logError } from '../utils/logger';
import { extractOutputMode } from '../utils/configHelpers';
import type { SettingsBootstrapPayload, ConfigResponse } from '../types';

interface UseSettingsResult {
	settingsBootstrap: SettingsBootstrapPayload | null;
	hotkey: string;
	outputMode: string;
}

export function useSettings(): UseSettingsResult {
	// Settings bootstrap data (loaded on mount for fast initial render).
	const [settingsBootstrap, setSettingsBootstrap] = useState<SettingsBootstrapPayload | null>(null);
	// Current hotkey (defaults to control+option).
	const [hotkey, setHotkey] = useState<string>("control+option");
	// Current output mode (defaults to paste).
	const [outputMode, setOutputMode] = useState<string>("paste");

	/**
	 * Pre-warm settings data to avoid first-open lag.
	 * 
	 * Loads settings bootstrap on mount (deferred with setTimeout to allow initial render).
	 * Falls back to individual API calls if bootstrap fails.
	 * Uses cancellation flag to prevent state updates if component unmounts.
	 */
	useEffect(() => {
		let cancelled = false;

		/**
		 * Fallback: load settings individually if bootstrap fails.
		 * 
		 * Fetches config, hotkey, devices, and autostart status separately.
		 * This ensures we have some data even if bootstrap endpoint fails.
		 */
		const fallbackPrime = () => {
			// Load output mode from config.
			invoke<ConfigResponse>('get_config')
				.then((cfg) => {
					if (!cancelled) {
						const mode = extractOutputMode(cfg);
						if (mode) setOutputMode(mode);
					}
				})
				.catch((error) => {
					logError('Failed to get config in fallbackPrime:', error);
				});

			// Pre-warm other settings (hotkey, devices, autostart) in parallel.
			// Using allSettled to ensure all requests complete even if some fail.
			Promise.allSettled([
				invoke<string>('get_hotkey'),
				invoke<string[]>('list_input_devices'),
				autostartIsEnabled().catch((error) => {
					logError('Failed to check autostart status:', error);
					return false;
				}),
			]).catch((error) => {
				logError('Failed to prime settings in fallbackPrime:', error);
			});
		};

		/**
		 * Load settings bootstrap (fast path, single API call).
		 * 
		 * Falls back to individual calls if bootstrap fails or is empty.
		 */
		const load = async () => {
			try {
				const payload = await invoke<SettingsBootstrapPayload>('load_settings_bootstrap');
				// Check cancellation and payload validity.
				if (cancelled || !payload) {
					if (!cancelled) {
						fallbackPrime();
					}
					return;
				}
				// Apply bootstrap data.
				setSettingsBootstrap(payload);
				if (typeof payload.hotkey === 'string' && payload.hotkey.length > 0) {
					setHotkey(payload.hotkey);
				}
				const mode = extractOutputMode(payload.config);
				if (mode) setOutputMode(mode);
			} catch (error) {
				logError('Failed to load settings bootstrap:', error);
				// Fall back to individual API calls.
				if (!cancelled) {
					fallbackPrime();
				}
			}
		};

		// Defer loading to allow initial render to complete (prevents blocking).
		const id = setTimeout(load, 0);
		return () => {
			cancelled = true;
			clearTimeout(id);
		};
	}, []);

	/**
	 * Load current hotkey on mount (fallback if bootstrap failed).
	 * 
	 * Uses bootstrap hotkey if available, otherwise fetches from backend.
	 */
	useEffect(() => {
		if (settingsBootstrap && typeof settingsBootstrap.hotkey === 'string') {
			setHotkey(settingsBootstrap.hotkey);
			return;
		}
		// Fallback: fetch hotkey from backend.
		invoke<string>('get_hotkey')
			.then((v) => {
				if (typeof v === 'string') {
					setHotkey(v);
				}
			})
			.catch((error) => {
				logError('Failed to get hotkey:', error);
			});
	}, [settingsBootstrap]);

	/**
	 * Keep output mode in sync with backend changes.
	 * 
	 * Loads initial output mode from bootstrap or config, then listens for
	 * output_mode_changed events to keep it synchronized.
	 */
	useEffect(() => {
		// Load initial output mode from bootstrap or config.
		if (settingsBootstrap && settingsBootstrap.config) {
			const mode = extractOutputMode(settingsBootstrap.config);
			if (mode) setOutputMode(mode);
		} else {
			// Fallback: fetch from backend if bootstrap unavailable.
			invoke<ConfigResponse>('get_config')
				.then((cfg) => {
					const mode = extractOutputMode(cfg);
					if (mode) setOutputMode(mode);
				})
				.catch((error) => {
					logError('Failed to get hotkey:', error);
				});
		}

		// Listen for output mode changes from backend (e.g., sink selection).
		let un: (() => void) | undefined;
		listen<string>('output_mode_changed', (e) => {
			if (typeof e.payload === 'string') setOutputMode(e.payload.toLowerCase());
		})
			.then((u) => (un = u))
			.catch((error) => {
				logError('Failed to attach output_mode_changed listener:', error);
			});
		return () => { if (un) un(); };
	}, [settingsBootstrap]);

	return {
		settingsBootstrap,
		hotkey,
		outputMode,
	};
}
