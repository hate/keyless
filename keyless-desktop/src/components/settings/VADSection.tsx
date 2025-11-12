import { SettingsSection } from '../layout/SettingsSection';
import { SliderField } from '../fields/SliderField';

export interface VADSectionConfigValues {
	vadStartDb: number;
	vadStopDb: number;
	vadMinDuration: number;
	vadMaxSilence: number;
}

export interface VADSectionProps {
	expanded: boolean;
	onToggle: () => void;
	config: VADSectionConfigValues;
	updateConfigValue: <K extends keyof VADSectionConfigValues>(key: K, value: VADSectionConfigValues[K]) => void;
}

const DEFAULTS = {
	vadStartDb: -45.0,
	vadStopDb: -50.0,
	vadMinDuration: 200,
	vadMaxSilence: 800,
} as const;

/**
 * Voice Activity Detection (VAD) settings section containing all VAD threshold sliders.
 */
export function VADSection({
	expanded,
	onToggle,
	config,
	updateConfigValue,
}: VADSectionProps) {
	return (
		<SettingsSection title="voice detection" expanded={expanded} onToggle={onToggle}>
			<SliderField
				label="start threshold"
				value={config.vadStartDb}
				min={-50}
				max={-30}
				step={0.5}
				unit="dB"
				defaultValue={DEFAULTS.vadStartDb}
				onChange={(value) => updateConfigValue('vadStartDb', value)}
				formatValue={(v) => v.toFixed(1)}
				helpText="lower (more negative) = more sensitive"
				onReset={() => updateConfigValue('vadStartDb', DEFAULTS.vadStartDb)}
			/>

			<SliderField
				label="stop threshold"
				value={config.vadStopDb}
				min={-60}
				max={-35}
				step={0.5}
				unit="dB"
				defaultValue={DEFAULTS.vadStopDb}
				onChange={(value) => updateConfigValue('vadStopDb', value)}
				formatValue={(v) => v.toFixed(1)}
				helpText="should be lower than start threshold"
				onReset={() => updateConfigValue('vadStopDb', DEFAULTS.vadStopDb)}
			/>

			<SliderField
				label="min speech duration"
				value={config.vadMinDuration}
				min={0}
				max={1000}
				step={50}
				unit="ms"
				defaultValue={DEFAULTS.vadMinDuration}
				onChange={(value) => updateConfigValue('vadMinDuration', value)}
				onReset={() => updateConfigValue('vadMinDuration', DEFAULTS.vadMinDuration)}
			/>

			<SliderField
				label="maximum silence"
				value={config.vadMaxSilence}
				min={0}
				max={2000}
				step={100}
				unit="ms"
				defaultValue={DEFAULTS.vadMaxSilence}
				onChange={(value) => updateConfigValue('vadMaxSilence', value)}
				onReset={() => updateConfigValue('vadMaxSilence', DEFAULTS.vadMaxSilence)}
			/>
		</SettingsSection>
	);
}

