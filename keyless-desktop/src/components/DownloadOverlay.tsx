import { useMemo } from 'react';
import type { DownloadOverlayState } from '../types';
import { formatBytes, formatEta } from '../utils/format';

interface DownloadOverlayProps {
	downloads: Record<string, DownloadOverlayState>;
	modelSizes: Record<string, number | undefined>;
	onPause: (modelId: string) => void;
	onResume: (modelId: string) => void;
	onCancel: (modelId: string) => void;
	onDismiss: (modelId: string) => void;
}

export function DownloadOverlay({
	downloads,
	modelSizes,
	onPause,
	onResume,
	onCancel,
	onDismiss,
}: DownloadOverlayProps) {
	const activeDownload = useMemo(() => {
		const entries = Object.values(downloads);
		if (entries.length === 0) return null;
		return entries.sort((a, b) => b.updatedAt - a.updatedAt)[0];
	}, [downloads]);

	if (!activeDownload) {
		return null;
	}

	const activeProgress = activeDownload.progress;
	const inferredTotal =
		activeProgress?.total ??
		(activeDownload.modelId ? modelSizes[activeDownload.modelId] : undefined);
	const totalForProgress =
		typeof inferredTotal === 'number' && inferredTotal > 0 ? inferredTotal : undefined;
	const percentComplete =
		activeProgress && totalForProgress
			? Math.min(
				100,
				Math.floor(
					((activeProgress.bytes ?? 0) / totalForProgress) * 100,
				),
			)
			: undefined;

	return (
		<div className="mx-3 rounded-lg border border-border bg-bgCard px-4 py-3 text-xs text-textPrimary lowercase fade-in slide-up">
			<div className="flex items-center justify-between">
				<span className="font-medium text-textAlt">
					{activeDownload.status === 'complete'
						? 'download complete'
						: activeDownload.status === 'error'
							? 'download failed'
							: activeDownload.status === 'cancelled'
								? 'download cancelled'
								: activeDownload.status === 'paused'
									? 'download paused'
									: 'downloading model'}
				</span>
				<div className="flex items-center gap-2">
					{activeDownload.status === 'paused' && (
						<button
							aria-label="dismiss download notification"
							className="text-textSecondary hover:text-textPrimary text-sm leading-none px-1 py-0.5 border border-transparent hover:border-border rounded"
							onClick={() => onDismiss(activeDownload.modelId)}
						>
							×
						</button>
					)}
				</div>
			</div>
			{activeDownload.stage && (
				<div className="mt-1 text-[11px] text-textSecondary">{activeDownload.modelId}</div>
			)}
			{activeProgress && totalForProgress && (
				<div className="mt-2">
					<div className="h-1 rounded bg-bgTrack">
						<div
							className="h-1 rounded bg-textPrimary"
							style={{ width: `${percentComplete ?? 0}%`, transition: 'width 180ms ease-out' }}
						/>
					</div>
					<div className="mt-1 flex justify-between text-[10px] text-textMuted">
						<span>
							{percentComplete ?? 0}% • {formatBytes(activeProgress.bytes)} / {formatBytes(totalForProgress)}
						</span>
						<span>
							{activeProgress.mbps.toFixed(1)} MB/s
							{activeProgress.etaSeconds > 0 && ` • eta ${formatEta(activeProgress.etaSeconds)}`}
						</span>
					</div>
				</div>
			)}
			{activeProgress && !totalForProgress && activeDownload.status === 'in-progress' && (
				<div className="mt-2 text-[11px] text-textSecondary">
					{formatBytes(activeProgress.bytes)} downloaded
					{activeProgress.mbps > 0 && ` • ${activeProgress.mbps.toFixed(1)} MB/s`}
				</div>
			)}
			{activeDownload.error && (
				<div className="mt-2 text-[11px] text-errorTextAlt">{activeDownload.error}</div>
			)}
			<div className="mt-2 flex gap-2">
				{activeDownload.status === 'in-progress' && (
					<button
						aria-label="pause download"
						className="px-2 py-1 rounded border border-border bg-bgInput text-textPrimary hover:border-textPrimary btn-anim"
						onClick={() => onPause(activeDownload.modelId)}
					>
						pause
					</button>
				)}
				{activeDownload.status === 'paused' && (
					<>
						<button
							aria-label="resume download"
							className="px-2 py-1 rounded border border-border bg-bgInput text-textPrimary hover:border-textPrimary btn-anim"
							onClick={() => onResume(activeDownload.modelId)}
						>
							resume
						</button>
						<button
							aria-label="cancel download"
							className="px-2 py-1 rounded border border-border bg-bgInput text-errorTextAlt hover:border-errorTextAlt btn-anim"
							onClick={() => onCancel(activeDownload.modelId)}
						>
							cancel
						</button>
					</>
				)}
			</div>
		</div>
	);
}
