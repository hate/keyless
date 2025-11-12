/**
 * Statistics hook for tracking word counts and talk time.
 * 
 * Tracks session and lifetime statistics (words transcribed, time spent talking).
 * Loads initial lifetime stats from bootstrap, then listens for real-time updates
 * from the backend as the user speaks.
 */

import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { logError } from '../utils/logger';
import { numberOrZero } from '../utils/numberHelpers';
import type { DesktopStats, SettingsBootstrapPayload } from '../types';

export function useStats(settingsBootstrap: SettingsBootstrapPayload | null): DesktopStats {
	// Statistics state: session and lifetime word counts and talk time.
	const [stats, setStats] = useState<DesktopStats>({
		sessionWords: 0,
		sessionTalkMs: 0,
		lifetimeWords: 0,
		lifetimeTalkMs: 0,
	});

	/**
	 * Apply initial lifetime stats from bootstrap data.
	 * 
	 * Loads lifetime statistics (persisted across sessions) from bootstrap config.
	 * Session stats start at 0 and are updated via events.
	 */
	useEffect(() => {
		if (!settingsBootstrap?.config) {
			return;
		}
		const source = settingsBootstrap.config as Record<string, unknown>;
		// Update only lifetime stats (preserve session stats).
		setStats((prev) => ({
			...prev,
			// Handle both snake_case and camelCase keys.
			lifetimeWords: numberOrZero(
				source.lifetime_words ?? source.lifetimeWords,
			),
			lifetimeTalkMs: numberOrZero(
				source.lifetime_talk_ms ?? source.lifetimeTalkMs,
			),
		}));
	}, [settingsBootstrap]);

	/**
	 * Listen for real-time stats updates from backend.
	 * 
	 * Backend emits stats_update events as the user speaks, updating both
	 * session and lifetime statistics.
	 */
	useEffect(() => {
		let un: (() => void) | undefined;
		listen<unknown>('stats_update', (e) => {
			const payload = e.payload;
			// Type guard: ensure payload is an object with expected properties.
			if (!payload || typeof payload !== 'object') return;
			// Cast to Record for property access (payload structure validated by backend).
			const statsPayload = payload as Record<string, unknown>;
			// Update all stats from payload (replaces previous values).
			setStats({
				sessionWords: numberOrZero(statsPayload.sessionWords),
				sessionTalkMs: numberOrZero(statsPayload.sessionTalkMs),
				lifetimeWords: numberOrZero(statsPayload.lifetimeWords),
				lifetimeTalkMs: numberOrZero(statsPayload.lifetimeTalkMs),
			});
		})
			.then((u) => (un = u))
			.catch((error) => {
				logError('Failed to attach stats_update listener:', error);
			});
		return () => { if (un) un(); };
	}, []);

	return stats;
}
