import type { ViewKey } from '../types';

/**
 * View transition duration in milliseconds.
 */
export const TRANSITION_DURATION_MS = 350;

/**
 * View hierarchy: determines order and direction of view transitions.
 * Higher views animate down (forward), lower views animate up (backward).
 */
export const VIEW_HIERARCHY: ViewKey[] = ["onboarding", "idle", "listening", "settings", "models"];
