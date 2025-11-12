export interface SliderFieldProps {
	label: string;
	value: number;
	min: number;
	max: number;
	step: number;
	onChange: (value: number) => void;
	defaultValue: number;
	unit?: string;
	formatValue?: (value: number) => string;
	helpText?: string;
	isDifferent?: (a: number, b: number) => boolean;
	onReset?: () => void;
}

/**
 * Reusable slider field component combining label, slider, value display, and optional reset button.
 * Eliminates duplication of the VAD slider pattern used 4 times in Settings (start threshold, stop threshold, min duration, max silence).
 */
export function SliderField({
	label,
	value,
	min,
	max,
	step,
	onChange,
	defaultValue,
	unit = '',
	formatValue,
	helpText,
	isDifferent = (a, b) => Math.abs(a - b) > 1e-6,
	onReset,
}: SliderFieldProps) {
	const displayValue = formatValue
		? formatValue(value)
		: unit
		? `${value} ${unit}`
		: value.toString();
	const displayDefault = formatValue
		? formatValue(defaultValue)
		: unit
		? `${defaultValue} ${unit}`
		: defaultValue.toString();

	return (
		<div className="space-y-2">
			<div className="flex items-center justify-between">
				<div className="text-[13px] text-textAlt lowercase">{label}</div>
				<div className="flex items-baseline gap-2">
					<span className="text-[12px] text-textPrimary font-mono">{displayValue}</span>
					{isDifferent(value, defaultValue) && onReset && (
						<button
							type="button"
							title="reset to default"
							onClick={onReset}
							className="text-[11px] text-textDisabled font-mono cursor-pointer bg-transparent border-0 p-0 m-0 outline-none focus:outline-none focus:ring-0 appearance-none"
						>
							{displayDefault}
						</button>
					)}
				</div>
			</div>
			<div className="relative">
				<input
					type="range"
					min={min}
					max={max}
					step={step}
					value={value}
					onChange={(e) => onChange(parseFloat(e.target.value))}
					className="w-full"
				/>
			</div>
			{helpText && <p className="text-[11px] text-textSecondary lowercase">{helpText}</p>}
		</div>
	);
}

