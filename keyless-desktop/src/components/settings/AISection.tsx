import { SettingsSection } from '../layout/SettingsSection';
import { SelectField } from '../fields/SelectField';
import { LinkButton } from '../buttons/LinkButton';

export interface AISectionConfigValues {
	language: string | null;
	modelPath: string;
}

export interface AISectionProps {
	expanded: boolean;
	onToggle: () => void;
	config: AISectionConfigValues;
	updateConfigValue: <K extends keyof AISectionConfigValues>(key: K, value: AISectionConfigValues[K]) => void;
	onOpenModels?: () => void;
	missingModel?: boolean;
}

/**
 * AI settings section containing language selection and model management.
 */
export function AISection({
	expanded,
	onToggle,
	config,
	updateConfigValue,
	onOpenModels,
	missingModel,
}: AISectionProps) {
	return (
		<SettingsSection title="ai" expanded={expanded} onToggle={onToggle}>
			{/* Language */}
			<SelectField
				label="language"
				value={config.language || 'auto'}
				onChange={(v) => updateConfigValue('language', v === 'auto' ? null : v)}
				help="auto-detect is slower and less accurate"
			>
				<option value="auto">auto-detect</option>
				<option value="en">english (en)</option>
				<option value="es">spanish (es)</option>
				<option value="fr">french (fr)</option>
				<option value="de">german (de)</option>
				<option value="zh">chinese (zh)</option>
				<option value="ja">japanese (ja)</option>
				<option value="ko">korean (ko)</option>
				<option value="pt">portuguese (pt)</option>
				<option value="ru">russian (ru)</option>
			</SelectField>

			{/* Model Management */}
			<div className="space-y-2">
				<div className="flex items-center justify-between">
					<div className="text-[13px] text-textAlt lowercase">model</div>
					{onOpenModels && (
						<LinkButton onClick={onOpenModels}>
							manage models
						</LinkButton>
					)}
				</div>
				<div className="text-[11px] text-textSecondary lowercase">
					<div>current model:</div>
					<div className="font-mono text-textPrimary break-words">
						{missingModel || !config.modelPath ? 'none' : config.modelPath}
					</div>
				</div>
			</div>
		</SettingsSection>
	);
}

