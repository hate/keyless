import { listen } from '@tauri-apps/api/event';
import { logError } from './logger';

/**
 * Utility for attaching Tauri event listeners with consistent error handling.
 * Returns a cleanup function that unsubscribes all listeners.
 */
export async function attachListeners(
	listeners: Array<{ event: string; handler: (payload: unknown) => void }>
): Promise<() => void> {
	// Track all unsubscribe functions for cleanup.
	const unsubs: Array<() => void> = [];

	// Attach each listener, handling errors gracefully (one failure doesn't stop others).
	for (const { event, handler } of listeners) {
		try {
			const un = await listen(event, (e) => handler(e.payload));
			unsubs.push(un);
		} catch (error) {
			// Log error but continue attaching other listeners.
			logError(`Failed to attach event listener for ${event}:`, error);
		}
	}

	// Return cleanup function that unsubscribes all listeners.
	return () => {
		unsubs.forEach((un) => un());
	};
}

