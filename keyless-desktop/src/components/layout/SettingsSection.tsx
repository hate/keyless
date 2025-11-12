import { ReactNode } from 'react';

export interface SettingsSectionProps {
    title: string;
    expanded: boolean;
    onToggle: () => void;
    children: ReactNode;
}

/**
 * A reusable expandable settings section component.
 * Eliminates duplication of the expand/collapse pattern used 3+ times in Settings.tsx.
 * Each section (General, AI, VAD) in Settings now uses this component.
 */
export function SettingsSection({
	title,
	expanded,
	onToggle,
	children,
}: SettingsSectionProps) {
	const sectionId = `settings-section-${title.toLowerCase().replace(/\s+/g, '-')}`;
	
	return (
		<div>
			<button
				onClick={onToggle}
				aria-expanded={expanded}
				aria-controls={sectionId}
				className="flex items-center gap-2 w-full text-left group bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none cursor-pointer pb-2 select-none focus:outline-none focus:ring-0 focus-visible:outline-none active:bg-transparent active:text-textAlt"
			>
				<span className="text-textSecondary text-sm flex items-center leading-none" aria-hidden="true">
					{expanded ? '▼' : '▶'}
				</span>
				<h3 className="text-[15px] font-medium text-textAlt lowercase leading-none">
					{title}
				</h3>
			</button>
			<div className="border-b border-border mb-4"></div>
			{expanded && (
				<div id={sectionId} className="mt-4 space-y-5 slide-up">
					{children}
				</div>
			)}
		</div>
	);
}

