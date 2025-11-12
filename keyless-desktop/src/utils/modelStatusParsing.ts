import type { BackendModelStatus, BackendModelInfo } from '../types';

/**
 * Parse raw model status from backend into typed BackendModelStatus.
 * Handles various formats and provides fallback to base model info.
 */
export function parseModelStatus(
	raw: unknown,
	fallback: BackendModelInfo
): BackendModelStatus {
	// Type guard: if raw status is invalid, use fallback from model info.
	if (!raw || typeof raw !== 'object') {
		return {
			downloaded: fallback.downloaded,
			downloading: fallback.downloading,
		};
	}

	const s = raw as Record<string, unknown>;

	// Parse status fields with type guards and fallbacks.
	return {
		downloaded: !!s.downloaded,
		downloading: !!s.downloading,
		stage: typeof s.stage === 'string' ? s.stage : undefined,
		progress: parseProgress(s.progress),
		error: typeof s.error === 'string' ? s.error : undefined,
		paused: !!s.paused,
	};
}

/**
 * Parse progress object from backend response.
 */
function parseProgress(raw: unknown): BackendModelStatus['progress'] {
	// Type guard: ensure progress is an object.
	if (!raw || typeof raw !== 'object') {
		return undefined;
	}

	const p = raw as Record<string, unknown>;

	// Parse progress fields with type guards (bytes/mbps/etaSeconds default to 0, total can be undefined).
	return {
		bytes: Number(p.bytes ?? 0),
		total: typeof p.total === 'number' ? p.total : undefined,
		mbps: Number(p.mbps ?? 0),
		etaSeconds: Number(p.etaSeconds ?? 0),
	};
}

