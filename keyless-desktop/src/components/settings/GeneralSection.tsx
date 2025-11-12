import { useCallback } from 'react';
import { open, type OpenDialogOptions } from '@tauri-apps/plugin-dialog';
import { logError } from '../../utils/logger';
import { safeInvoke } from '../../utils/errorHandling';
import { showError } from '../../utils/errorNotifications';
import { COLORS } from '../../constants/colors';
import { SettingsSection } from '../layout/SettingsSection';
import { ColoredButtonGroup } from '../buttons/ColoredButtonGroup';
import { SelectField } from '../fields/SelectField';
import { SecondaryButton } from '../buttons/SecondaryButton';
import type { PttListener } from '../../types';

export interface GeneralSectionProps {
	expanded: boolean;
	onToggle: () => void;
	autostart: boolean;
	onToggleAutostart: (value: boolean) => void;
	outputMode: string;
	onSelectSink: (id: string) => void;
	filePath: string;
	setFilePath: (path: string) => void;
	pttListener: PttListener;
	setPttListener: (value: PttListener) => void;
	devices: string[];
	selectedDevice: string | null;
	onSelectDevice: (deviceName: string) => void;
}

/**
 * General settings section containing output mode, PTT listener, input device, and autostart.
 */
export function GeneralSection({
	expanded,
	onToggle,
	autostart,
	onToggleAutostart,
	outputMode,
	onSelectSink,
	filePath,
	setFilePath,
	pttListener,
	setPttListener,
	devices,
	selectedDevice,
	onSelectDevice,
}: GeneralSectionProps) {
	const handleBrowseForOutputFile = useCallback(async () => {
		await safeInvoke('suppress_popover_blur', { value: true });
		await safeInvoke('set_popover_visibility', { visible: false });
		try {
			const selection = await open({
				title: 'Select output file',
				defaultPath: filePath || undefined,
				directory: false,
				multiple: false,
			} satisfies OpenDialogOptions);

			if (selection === null) {
				return;
			}

			const chosenPath = Array.isArray(selection) ? selection[0] : selection;
			if (typeof chosenPath !== 'string') {
				return;
			}

			const normalized = chosenPath.trim();
			if (normalized.length === 0) {
				return;
			}

			setFilePath(normalized);
			const result = await safeInvoke('update_config', { patch: { outputMode: 'file', filePath: normalized } }, true);
			if (result === undefined) {
				await showError('Failed to save file path setting');
			}
		} catch (err) {
			logError('file dialog failed', err);
			await showError('Failed to select output file', err);
		} finally {
			await safeInvoke('set_popover_visibility', { visible: true });
			await safeInvoke('suppress_popover_blur', { value: false });
		}
	}, [filePath, setFilePath]);

	return (
		<SettingsSection title="general" expanded={expanded} onToggle={onToggle}>
			{/* Start at Login */}
			<div className="space-y-2">
				<div className="flex items-center justify-between">
					<div className="text-[13px] text-textAlt lowercase">start at login</div>
					<input
						aria-label="start at login"
						title="start at login"
						type="checkbox"
						checked={autostart}
						onChange={(e) => onToggleAutostart(e.target.checked)}
						className="h-[14px] w-[14px] appearance-none border border-border rounded-[3px] bg-bgInput checked:bg-textPrimary checked:border-textPrimary transition-colors cursor-pointer"
					/>
				</div>
			</div>

			{/* Output Destination */}
			<div className="space-y-2">
				<div className="text-[13px] text-textAlt lowercase">output</div>
				<ColoredButtonGroup
					options={[
						{
							id: 'paste',
							label: 'paste',
							color: {
								selected: COLORS.sink.paste,
								hover: 'hover:border-[rgba(80,250,123,0.6)]',
							},
						},
						{
							id: 'clipboard',
							label: 'clipboard',
							color: {
								selected: COLORS.sink.clipboard,
								hover: 'hover:border-[rgba(241,250,140,0.6)]',
							},
						},
						{
							id: 'file',
							label: 'file',
							color: {
								selected: COLORS.sink.file,
								hover: 'hover:border-[rgba(78,163,255,0.6)]',
							},
						},
					]}
					selected={outputMode as 'paste' | 'clipboard' | 'file'}
					onChange={onSelectSink}
				/>
				<p className="text-[11px] text-textSecondary lowercase">
					{outputMode === "paste" && "simulate keystrokes into the active window"}
					{outputMode === "clipboard" && "copy text to system clipboard"}
					{outputMode === "file" && "append text to a specified file"}
				</p>
			</div>

			{/* File Path (when output is file) */}
			{outputMode === 'file' && (
				<div className="space-y-2">
					<div className="text-[13px] text-textAlt lowercase">file path</div>
					<div className="flex gap-2">
						<input
							type="text"
							value={filePath}
							onChange={(e) => setFilePath(e.target.value)}
							placeholder="/path/to/output.txt"
							className="w-full text-[13px] text-textAlt bg-bgInput border border-border rounded px-2 py-2 outline-none"
							readOnly
							aria-label="output file path (read-only, use browse button to change)"
						/>
						<SecondaryButton onClick={handleBrowseForOutputFile}>
							browse…
						</SecondaryButton>
					</div>
					<p className="text-[11px] text-textSecondary lowercase">file will be created if missing; ensure directory exists</p>
				</div>
			)}

			{/* Push-to-talk Listener */}
			<SelectField
				label="push to talk"
				value={pttListener}
				onChange={async (v) => {
					const newValue = (v as PttListener) || 'ctrlOption';
					setPttListener(newValue);
					const newHotkey = newValue === 'ctrlShift' ? 'control+shift' : 'control+option';
					const result = await safeInvoke('update_config', { patch: { hotkey: newHotkey } }, true);
					if (result === undefined) {
						await showError('Failed to save push-to-talk setting');
					}
				}}
			>
				<option value="ctrlOption">control+option</option>
				<option value="ctrlShift">control+shift</option>
			</SelectField>

			{/* Input Device */}
			<SelectField
				label="input device"
				value={selectedDevice || null}
				onChange={(v) => v && onSelectDevice(v)}
			>
				<option value="">system default</option>
				{devices.map((dev, idx) => (
					<option key={idx} value={dev}>{dev}</option>
				))}
			</SelectField>
		</SettingsSection>
	);
}

