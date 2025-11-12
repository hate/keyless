import { useEffect, useRef } from 'react';
import { logError } from '../utils/logger';
import { attachListeners } from '../utils/eventListeners';

export interface SettingsEventHandlers {
	onOutputModeChanged?: (mode: string) => void;
	onModelSelected?: (modelId: string) => void;
}

/**
 * Consolidated hook for Settings event listeners.
 * 
 * Handles output_mode_changed and model_selected events with proper cleanup.
 * Uses refs to access handlers (allows handlers to change without re-attaching listeners).
 * 
 * Used by Settings view to listen for output mode changes and model selection.
 */
export function useSettingsEventListeners(handlers: SettingsEventHandlers) {
	// Store handlers in ref (allows handlers to change without re-attaching listeners).
	const handlersRef = useRef(handlers);
	useEffect(() => {
		handlersRef.current = handlers;
	}, [handlers]);

	/**
	 * Set up event listeners for settings-related events.
	 * 
	 * Only attaches listeners for handlers that are provided (all handlers are optional).
	 * Uses refs to access handlers, so listeners don't need to be re-attached when
	 * handlers change (improves performance).
	 */
	useEffect(() => {
		const listeners = [];

		// Listen for output mode changes (e.g., sink selection).
		if (handlersRef.current.onOutputModeChanged) {
			listeners.push({
				event: 'output_mode_changed',
				handler: (payload: unknown) => {
					if (typeof payload === 'string') {
						handlersRef.current.onOutputModeChanged?.(payload.toLowerCase());
					}
				},
			});
		}

		// Listen for model selection (e.g., model selected from tray menu).
		if (handlersRef.current.onModelSelected) {
			listeners.push({
				event: 'model_selected',
				handler: (payload: unknown) => {
					if (typeof payload === 'string') {
						handlersRef.current.onModelSelected?.(payload);
					}
				},
			});
		}

		// Early return if no listeners to attach.
		if (listeners.length === 0) return;

		// Attach listeners and store cleanup function.
		let cleanup: (() => void) | undefined;
		attachListeners(listeners)
			.then((fn) => {
				cleanup = fn;
			})
			.catch((error) => {
				logError('Failed to attach settings event listeners:', error);
			});

		// Cleanup: remove all listeners on unmount.
		return () => {
			cleanup?.();
		};
	}, []); // Empty dependency array - handlers are accessed via ref
}

