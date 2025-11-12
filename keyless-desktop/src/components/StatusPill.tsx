export type StatusType = 'success' | 'error' | 'warning' | 'info';

export interface StatusPillProps {
	active: boolean;
	activeLabel?: string;
	inactiveLabel?: string;
	type?: StatusType;
}

const TYPE_CONFIG = {
	success: {
		activeBg: 'bg-statusBgSuccess',
		inactiveBg: 'bg-statusBgError',
		activeDot: 'bg-statusSuccess',
		inactiveDot: 'bg-statusError',
		activeText: 'text-statusTextSuccess',
		inactiveText: 'text-statusTextError',
	},
	error: {
		activeBg: 'bg-statusBgError',
		inactiveBg: 'bg-statusBgError',
		activeDot: 'bg-statusError',
		inactiveDot: 'bg-statusError',
		activeText: 'text-statusTextError',
		inactiveText: 'text-statusTextError',
	},
	warning: {
		activeBg: 'bg-statusBgWarning',
		inactiveBg: 'bg-statusBgWarning',
		activeDot: 'bg-statusWarning',
		inactiveDot: 'bg-statusWarning',
		activeText: 'text-statusTextWarning',
		inactiveText: 'text-statusTextWarning',
	},
	info: {
		activeBg: 'bg-statusBgInfo',
		inactiveBg: 'bg-statusBgInfo',
		activeDot: 'bg-statusInfo',
		inactiveDot: 'bg-statusInfo',
		activeText: 'text-statusTextInfo',
		inactiveText: 'text-statusTextInfo',
	},
} as const;

/**
 * Status pill component that displays an active/inactive state with colored dot and label.
 * Used for VAD status, connection status, etc.
 */
export function StatusPill({
	active,
	activeLabel = 'Active',
	inactiveLabel = 'Idle',
	type = 'success',
}: StatusPillProps) {
	const config = TYPE_CONFIG[type];
	const bgClass = active ? config.activeBg : config.inactiveBg;
	const dotClass = active ? config.activeDot : config.inactiveDot;
	const textClass = active ? config.activeText : config.inactiveText;
	const label = active ? activeLabel : inactiveLabel;

	return (
		<div className={`px-2 py-[3px] rounded-full text-[11px] font-semibold lowercase flex items-center gap-1.5 ${bgClass}`}>
			<span className={`inline-block h-[6px] w-[6px] rounded-full ${dotClass} ${active ? 'vad-pulse' : ''}`} />
			<span className={textClass}>{label}</span>
		</div>
	);
}

