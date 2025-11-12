/**
 * Error handling utilities for consistent error management across the application.
 */

import { logError } from './logger';
import { showError } from './errorNotifications';

/**
 * Safely invokes a Tauri command, logging errors but not throwing.
 * Use this for non-critical operations where failures can be gracefully ignored.
 * 
 * @param command - Tauri command name
 * @param args - Command arguments
 * @param showUserError - If true, shows a user-facing error toast on failure
 */
export async function safeInvoke<T = unknown>(
    command: string,
    args?: Record<string, unknown>,
    showUserError = false
): Promise<T | undefined> {
    try {
        // Dynamic import to avoid loading Tauri APIs in non-Tauri environments (e.g., tests).
        const { invoke } = await import('@tauri-apps/api/core');
        return await invoke<T>(command, args);
    } catch (error) {
        // Always log errors for debugging.
        logError(`Failed to invoke ${command}:`, error);
        // Optionally show user-facing error toast.
        if (showUserError) {
            // Extract error message or generate a user-friendly one from command name.
            const errorMessage = error instanceof Error ? error.message : `Failed to ${command.replace(/_/g, ' ')}`;
            await showError(errorMessage, error);
        }
        // Return undefined instead of throwing (allows graceful degradation).
        return undefined;
    }
}

/**
 * Safely executes an async function, logging errors but not throwing.
 * Use this for non-critical operations where failures can be gracefully ignored.
 */
export async function safeExecute<T>(
    fn: () => Promise<T>,
    context?: string
): Promise<T | undefined> {
    try {
        return await fn();
    } catch (error) {
        // Log error with context if provided (helps identify where error occurred).
        if (context) {
            logError(`Error in ${context}:`, error);
        } else {
            logError('Error executing function:', error);
        }
        // Return undefined instead of throwing (allows graceful degradation).
        return undefined;
    }
}


