import { ReactNode } from 'react';

interface LinkButtonProps {
	children: ReactNode;
	onClick?: () => void;
	className?: string;
}

/**
 * A transparent button that looks like a link.
 * Used for secondary actions like [settings], [back], etc.
 */
export function LinkButton({ children, onClick, className = '' }: LinkButtonProps) {
	return (
		<button
			onClick={onClick}
			className={`text-xs text-textSecondary hover:text-textPrimary transition-colors lowercase bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none focus:outline-none focus:ring-0 focus-visible:outline-none active:bg-transparent active:text-textAlt ${className}`}
			type="button"
		>
			{children}
		</button>
	);
}
