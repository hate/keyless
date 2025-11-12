import { ReactNode } from 'react';

export interface SelectFieldProps {
	label: string;
	value: string | null;
	onChange: (value: string | null) => void;
	children: ReactNode;
	help?: string;
}

/**
 * Reusable select field component combining label, select, and help text.
 * Eliminates duplication of the select + custom dropdown styling pattern used 3+ times in Settings.
 */
export function SelectField({
	label,
	value,
	onChange,
	children,
	help,
}: SelectFieldProps) {
	return (
		<div className="space-y-2">
			<div className="text-[13px] text-textAlt lowercase">{label}</div>
			<div className="relative">
				<select
					value={value || ''}
					onChange={(e) => {
						const newValue = e.target.value;
						onChange(newValue === '' ? null : newValue);
					}}
					className="text-[13px] text-textAlt bg-bgInput border border-border rounded px-2 py-2 w-full pr-6 appearance-none outline-none font-sans antialiased"
				>
					{children}
				</select>
				<div className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-textSecondary text-xs">
					▾
				</div>
			</div>
			{help && <p className="text-[11px] text-textSecondary lowercase">{help}</p>}
		</div>
	);
}

