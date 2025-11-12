export interface ScrollFadeProps {
	alpha: number;
	color?: 'dark' | 'light';
	className?: string;
}

/**
 * Scroll fade component that creates a gradient fade effect at the bottom of scrollable content.
 * Used to indicate when there's more content below.
 */
export function ScrollFade({ alpha, color = 'dark', className = '' }: ScrollFadeProps) {
	const colorClass = color === 'dark'
		? 'from-bgCard via-bgCard/90'
		: 'from-white via-white/90';

	return (
		<div
			className={`sticky bottom-0 inset-x-0 h-4 bg-gradient-to-t ${colorClass} to-transparent pointer-events-none z-10 ${className}`}
			style={{ opacity: alpha }}
		/>
	);
}

