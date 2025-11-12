/**
 * Utility functions for parsing and converting hotkey strings.
 */

import type { PttListener } from '../types';

/**
 * Parse a hotkey string and convert it to a PTT listener type.
 * Handles various formats: "control+shift", "Control+Shift", "ctrl+shift", etc.
 */
export function parseHotkeyToPttListener(hotkey: string): PttListener {
	// Normalize to lowercase for case-insensitive matching.
	const normalized = hotkey.toLowerCase();
	// Check for Control+Shift variants (defaults to Control+Option if not found).
	return normalized.includes('control+shift') || normalized.includes('ctrl+shift')
		? 'ctrlShift'
		: 'ctrlOption';
}

