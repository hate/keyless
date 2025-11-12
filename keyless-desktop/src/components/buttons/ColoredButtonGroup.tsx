export interface ColoredButtonOption<T extends string> {
	id: T;
	label: string;
	color: {
		selected: string; // Hex color like '#50fa7b'
		hover: string; // Tailwind class like 'hover:border-[rgba(80,250,123,0.6)]'
	};
}

export interface ColoredButtonGroupProps<T extends string> {
	options: ColoredButtonOption<T>[];
	selected: T;
	onChange: (id: T) => void;
	className?: string;
}

/**
 * A button group component with colored borders for each option.
 * Used for output mode selection (paste, clipboard, file) and similar choices.
 */
export function ColoredButtonGroup<T extends string>({
	options,
	selected,
	onChange,
	className = '',
}: ColoredButtonGroupProps<T>) {
	return (
		<div className={`flex gap-2 ${className}`}>
			{options.map((option) => {
				const isSelected = selected === option.id;
				const baseClass = 'text-xs lowercase rounded-md px-3 py-2 bg-bgInput border-2 transition-colors outline-none focus:outline-none focus:ring-0 focus-visible:ring-0 btn-anim';

				if (isSelected) {
					return (
						<button
							key={option.id}
							onClick={() => onChange(option.id)}
							className={`${baseClass} text-textPrimary`}
							style={{ borderColor: option.color.selected }}
						>
							{option.label}
						</button>
					);
				}

				return (
					<button
						key={option.id}
						onClick={() => onChange(option.id)}
						className={`${baseClass} border-border ${option.color.hover} text-textSecondary`}
					>
						{option.label}
					</button>
				);
			})}
		</div>
	);
}

