import { ReactNode } from 'react';

export type IconName = 'download' | 'delete' | 'pause' | 'play' | 'check' | 'resume' | 'cancel' | 'use';

interface IconProps {
	name: IconName;
	size?: 'sm' | 'md' | 'lg';
}

function Icon({ name, size = 'md' }: IconProps) {
	const sizeClass = size === 'sm' ? 'w-3.5 h-3.5' : size === 'md' ? 'w-3.5 h-3.5' : 'w-4 h-4';

	const icons: Record<IconName, ReactNode> = {
		download: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path
					d="M12 3v12m0 0l-4-4m4 4l4-4M5 21h14"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
				/>
			</svg>
		),
		delete: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M10 11v6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
				<path d="M14 11v6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
				<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" stroke="currentColor" strokeWidth="2" />
				<path d="M3 6h18" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
				<path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" stroke="currentColor" strokeWidth="2" />
			</svg>
		),
		pause: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M10 4h4v16h-4z" stroke="currentColor" strokeWidth="2" />
				<path d="M14 4h4v16h-4z" stroke="currentColor" strokeWidth="2" />
			</svg>
		),
		play: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M8 5v14l11-7z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
			</svg>
		),
		check: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M20 6L9 17l-5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
			</svg>
		),
		resume: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M8 5v14l11-7z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
			</svg>
		),
		cancel: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M18 6L6 18M6 6l12 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
			</svg>
		),
		use: (
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" className={sizeClass} aria-hidden="true">
				<path d="M20 6L9 17l-5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
			</svg>
		),
	};

	return <>{icons[name]}</>;
}

export interface IconButtonProps {
	icon: IconName;
	label: string;
	onClick: (e: React.MouseEvent) => void;
	disabled?: boolean;
	variant?: 'primary' | 'secondary' | 'danger';
	size?: 'sm' | 'md' | 'lg';
	showLabel?: boolean;
	className?: string;
	onMouseEnter?: () => void;
	onMouseLeave?: () => void;
}

/**
 * Icon button component with consistent styling and icon support.
 * Used for model actions and other icon-based buttons.
 */
export function IconButton({
	icon,
	label,
	onClick,
	disabled = false,
	variant = 'secondary',
	size = 'md',
	showLabel = false,
	className = '',
	onMouseEnter,
	onMouseLeave,
}: IconButtonProps) {
	const baseClass = size === 'sm' 
		? 'inline-flex items-center justify-center rounded-md p-1.5 transition-colors outline-none focus:outline-none focus:ring-0 btn-anim'
		: 'inline-flex items-center gap-2 rounded-md px-2 py-1 transition-colors outline-none focus:outline-none focus:ring-0 btn-anim';

	const variantClass = {
		primary: 'bg-textPrimary text-bgInput hover:bg-white',
		secondary: 'bg-bgInput border border-border text-textPrimary hover:border-textPrimary',
		danger: 'bg-bgInput border border-border text-errorDanger hover:border-errorDanger',
	};

	const disabledClass = disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer';

	return (
		<button
			onClick={onClick}
			onMouseEnter={onMouseEnter}
			onMouseLeave={onMouseLeave}
			disabled={disabled}
			aria-label={label}
			title={label}
			className={`${baseClass} ${variantClass[variant]} ${disabledClass} ${className}`}
		>
			<Icon name={icon} size={size} />
			{showLabel && <span className="text-[11px] lowercase">{label}</span>}
		</button>
	);
}

