import { ReactNode } from 'react';

interface SecondaryButtonProps {
	children: ReactNode;
	onClick?: () => void;
	className?: string;
	disabled?: boolean;
	type?: 'button' | 'submit' | 'reset';
}

/**
 * A secondary button with border styling.
 * Used for primary actions like Download, Browse, etc.
 */
export function SecondaryButton({
	children,
	onClick,
	className = '',
	disabled = false,
	type = 'button',
}: SecondaryButtonProps) {
	return (
		<button
			type={type}
			onClick={onClick}
			disabled={disabled}
			className={`text-[11px] lowercase rounded-md px-3 py-2 bg-bgInput border-2 border-border transition-colors outline-none focus:outline-none focus:ring-0 focus-visible:ring-0 btn-anim ${className}`}
		>
			{children}
		</button>
	);
}
