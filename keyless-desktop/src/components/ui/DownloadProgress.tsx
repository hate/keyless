export interface DownloadProgressProps {
	bytes: number;
	total?: number;
	className?: string;
}

/**
 * Download progress bar component.
 * Displays a progress bar with percentage based on bytes downloaded.
 */
export function DownloadProgress({ bytes, total, className = '' }: DownloadProgressProps) {
	const progressPercent = total
		? Math.min(100, (bytes / total) * 100)
		: bytes > 0
			? Math.min(100, (bytes / bytes) * 100)
			: 0;

	if (progressPercent === 0) {
		return null;
	}

	return (
		<div className={`mt-1 ${className}`}>
			<div className="h-1 rounded bg-bgTrack">
				<div
					className="h-1 rounded bg-textPrimary"
					style={{ width: `${progressPercent}%`, transition: 'width 180ms ease-out' }}
				/>
			</div>
		</div>
	);
}

