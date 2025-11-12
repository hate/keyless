import { memo } from 'react';
import { IconButton } from './buttons/IconButton';
import { DownloadProgress } from './ui/DownloadProgress';
import { formatBytes } from '../utils/format';
import { getStatusLabel } from '../utils/modelStatus';
import type { BackendModelInfo, BackendModelStatus } from '../types';

export interface ModelRowProps {
	model: BackendModelInfo;
	status?: BackendModelStatus;
	size?: number;
	isCurrent: boolean;
	index: number;
	hoverDeleteId: string | null;
	onUse: (id: string) => void;
	onDownload: (id: string) => void;
	onDelete: (id: string) => void;
	onResume: (id: string) => void;
	onCancel: (id: string) => void;
	onHoverDelete: (id: string | null) => void;
}

/**
 * Single model row component displaying model info, status, and actions.
 * Handles all the complex conditional rendering for model states.
 * Memoized to prevent unnecessary re-renders when parent updates.
 */
export const ModelRow = memo(function ModelRow({
	model,
	status,
	size,
	isCurrent,
	index,
	hoverDeleteId,
	onUse,
	onDownload,
	onDelete,
	onResume,
	onCancel,
	onHoverDelete,
}: ModelRowProps) {
	const downloading = status?.downloading ?? model.downloading;
	const paused = status?.paused ?? false;
	const installed = status?.downloaded ?? model.downloaded;
	const canUse = installed && !downloading && !isCurrent;

	const containerBorderClass = hoverDeleteId === model.id
		? 'border-errorDanger/60'
		: (isCurrent ? 'border-statusSuccess' : 'border-border');

	const hoverBorderClass = canUse && hoverDeleteId !== model.id
		? 'hover:border-statusSuccess/60 transition-colors cursor-pointer'
		: (canUse ? 'transition-colors cursor-pointer' : '');

	const titleTextClass = isCurrent
		? 'text-statusSuccess'
		: installed
			? 'text-textAlt'
			: (!downloading ? 'text-textMuted' : 'text-textAlt');

	const statusTextClass = installed
		? 'text-textSecondary'
		: (!downloading ? 'text-textMuted' : 'text-textSecondary');

	const delayMs = Math.min(index, 6) * 40;
	const statusLabel = getStatusLabel({ downloaded: installed, downloading, paused });
	const progress = status?.progress;
	const showProgress = (downloading || paused) && progress && (progress.total ? true : progress.bytes > 0);

	return (
		<div
			className={`slide-up flex items-center justify-between bg-bgRow border ${containerBorderClass} ${hoverBorderClass} rounded px-3 py-2`}
			style={{ animationDelay: `${delayMs}ms` }}
			onClick={canUse ? () => onUse(model.id) : undefined}
		>
			<div className="min-w-0">
				<div className={`${titleTextClass} text-[12px] truncate`} title={model.id} aria-label={model.id}>
					{model.id}
				</div>
				<div className={`${statusTextClass} text-[11px] lowercase`}>
					{statusLabel} • {formatBytes(size)}
				</div>
				{showProgress && progress && (
					<DownloadProgress bytes={progress.bytes} total={progress.total} />
				)}
				{status?.error && (
					<div className="text-[11px] text-errorTextAlt lowercase mt-1">{status.error}</div>
				)}
			</div>
			<div className="flex items-center gap-2">
				{!installed && !downloading && !paused && (
					<IconButton
						icon="download"
						label="download"
						onClick={(e) => {
							e.stopPropagation();
							onDownload(model.id);
						}}
						variant="secondary"
						size="sm"
					/>
				)}
				{installed && !downloading && !isCurrent && (
					<IconButton
						icon="delete"
						label="delete"
						onClick={(e) => {
							e.stopPropagation();
							onHoverDelete(null);
							onDelete(model.id);
						}}
						variant="danger"
						size="sm"
						onMouseEnter={() => onHoverDelete(model.id)}
						onMouseLeave={() => onHoverDelete(null)}
					/>
				)}
				{paused && (
					<div className="flex items-center gap-1.5">
						<IconButton
							icon="resume"
							label="resume"
							onClick={(e) => {
								e.stopPropagation();
								onResume(model.id);
							}}
							variant="secondary"
							size="sm"
						/>
						<IconButton
							icon="cancel"
							label="cancel"
							onClick={(e) => {
								e.stopPropagation();
								onCancel(model.id);
							}}
							variant="danger"
							size="sm"
						/>
					</div>
				)}
			</div>
		</div>
	);
});

