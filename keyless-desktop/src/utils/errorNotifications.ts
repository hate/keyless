import { emit } from '@tauri-apps/api/event';
import { logError } from './logger';

/**
 * Show a user-facing error notification.
 * Emits an event that the ErrorToast component listens to.
 * 
 * @param message - Error message to display to the user
 * @param error - Optional error object for logging
 */
export async function showError(message: string, error?: unknown): Promise<void> {
	// Always log the error for debugging (even if toast display fails).
	if (error) {
		logError(`User-facing error: ${message}`, error);
	} else {
		logError(`User-facing error: ${message}`);
	}

	// Emit Tauri event that ErrorToast component listens to.
	// This decouples error handling from UI display.
	try {
		await emit('error_notification', { message });
	} catch (emitError) {
		// If emit fails, log but don't throw (non-critical - error is already logged above).
		// This prevents error handling from causing additional errors.
		logError('Failed to emit error notification:', emitError);
	}
}

