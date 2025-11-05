import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { isEnabled as autostartIsEnabled, enable as autostartEnable, disable as autostartDisable } from '@tauri-apps/plugin-autostart';
import { BrandTitle } from '../components/BrandTitle';

interface ConfigValues {
    language: string | null;
    modelPath: string;
    vadStartDb: number;
    vadStopDb: number;
    vadMinDuration: number;
    vadMaxSilence: number;
    eqBands: number;
    eqNoiseReduction: number;
    eqWindowDb: number;
    eqGamma: number;
    eqAttack: number;
    eqDecay: number;
}

export default function SettingsView({ onBack }: { onBack?: () => void }) {
    const [hotkey, setHotkey] = useState<string>('control+option');
    const [outputMode, setOutputMode] = useState<string>('paste');
    const [autostart, setAutostart] = useState<boolean>(false);
    const [devices, setDevices] = useState<string[]>([]);
    const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
    const [config, setConfig] = useState<ConfigValues>({
        language: null,
        modelPath: 'openai/whisper-large-v3-turbo',
        vadStartDb: -45.0,
        vadStopDb: -50.0,
        vadMinDuration: 200,
        vadMaxSilence: 800,
        eqBands: 64,
        eqNoiseReduction: 0.46,
        eqWindowDb: 50.0,
        eqGamma: 1.3,
        eqAttack: 0.35,
        eqDecay: 0.12,
    });
    const [generalExpanded, setGeneralExpanded] = useState<boolean>(true);
    const [aiExpanded, setAiExpanded] = useState<boolean>(true);
    const [vadExpanded, setVadExpanded] = useState<boolean>(false);
    const [eqExpanded, setEqExpanded] = useState<boolean>(false);

    useEffect(() => {
        // Load hotkey
        invoke<string>('get_hotkey').then((v) => typeof v === 'string' && setHotkey(v)).catch(() => { });
        
        // Load full config
        invoke<any>('get_config')
            .then((cfg) => {
                if (!cfg || typeof cfg !== 'object') return;
                
                // Output mode
                if (cfg.output_mode) {
                    const om = typeof cfg.output_mode === 'string' ? cfg.output_mode : cfg.output_mode?.type;
                    if (typeof om === 'string') setOutputMode(om.toLowerCase());
                }
                
                // Device name
                if (cfg.device_name && typeof cfg.device_name === 'string') {
                    setSelectedDevice(cfg.device_name);
                }
                
                // Config values
                setConfig({
                    language: cfg.language || null,
                    modelPath: cfg.model_path || 'openai/whisper-large-v3-turbo',
                    vadStartDb: cfg.vad?.start_db ?? -45.0,
                    vadStopDb: cfg.vad?.stop_db ?? -50.0,
                    vadMinDuration: cfg.vad?.min_duration_ms ?? 200,
                    vadMaxSilence: cfg.vad?.max_silence_ms ?? 800,
                    eqBands: cfg.eq?.bands ?? 64,
                    eqNoiseReduction: cfg.eq?.noise_reduction ?? 0.46,
                    eqWindowDb: cfg.eq?.window_db ?? 50.0,
                    eqGamma: cfg.eq?.gamma ?? 1.3,
                    eqAttack: cfg.eq?.attack ?? 0.35,
                    eqDecay: cfg.eq?.decay ?? 0.12,
                });
            })
            .catch(() => { });
        
        // Load autostart status
        autostartIsEnabled().then(setAutostart).catch(() => { });
        
        // Load input devices
        invoke<string[]>('list_input_devices')
            .then((devs) => setDevices(devs))
            .catch(() => { });
    }, []);

    async function onToggleAutostart(next: boolean) {
        setAutostart(next);
        try {
            if (next) await autostartEnable();
            else await autostartDisable();
        } catch { }
    }

    async function onSelectSink(id: string) {
        setOutputMode(id);
        try {
            await invoke('select_sink', { id });
            await invoke('update_config', { patch: { outputMode: id } });
        } catch { }
    }

    async function onSelectDevice(deviceName: string) {
        setSelectedDevice(deviceName);
        try {
            await invoke('update_config', { patch: { deviceName } });
        } catch { }
    }

    async function updateConfigValue(key: keyof ConfigValues, value: any) {
        setConfig(prev => ({ ...prev, [key]: value }));
        
        // Map frontend keys to backend patch format
        const patches: Record<string, any> = {
            language: { language: value },
            modelPath: { modelPath: value },
            vadStartDb: { vad: { startDb: value } },
            vadStopDb: { vad: { stopDb: value } },
            vadMinDuration: { vad: { minDurationMs: value } },
            vadMaxSilence: { vad: { maxSilenceMs: value } },
            eqBands: { eq: { bands: value } },
            eqNoiseReduction: { eq: { noiseReduction: value } },
            eqWindowDb: { eq: { windowDb: value } },
            eqGamma: { eq: { gamma: value } },
            eqAttack: { eq: { attack: value } },
            eqDecay: { eq: { decay: value } },
        };
        
        try {
            await invoke('update_config', { patch: patches[key] });
        } catch { }
    }

    return (
        <div className="">
            <div className="mx-3 mb-3 rounded-xl border border-[#2a2a2a] bg-[#0f0f0f] overflow-hidden">
                <div className="pt-[30px] px-[22px] pb-[30px]">
                    <div className="mb-6">
                        <BrandTitle />
                    </div>

                    <div className="relative mb-2">
                        <div className="space-y-6 max-h-[350px] overflow-y-auto overflow-x-hidden pr-2 pb-9 scrollbar-hide">
                            {/* General Section */}
                            <div>
                                <button
                                    onClick={() => setGeneralExpanded(!generalExpanded)}
                                    className="flex items-center gap-2 w-full text-left group bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none cursor-pointer pb-2 select-none focus:outline-none focus:ring-0 focus-visible:outline-none active:bg-transparent active:text-[#e8e8e8]"
                                >
                                    <span className="text-[#a0a0a0] text-sm flex items-center leading-none">{generalExpanded ? '▼' : '▶'}</span>
                                    <h3 className="text-[15px] font-medium text-[#e8e8e8] lowercase leading-none">general</h3>
                                </button>
                                <div className="border-b border-[#2a2a2a] mb-4"></div>

                                {generalExpanded && (
                                    <div className="mt-4 space-y-5">
                                        {/* Output Destination */}
                                        <div className="space-y-2">
                                            <div className="text-[13px] text-[#e8e8e8] lowercase">output</div>
                                            <div className="flex gap-2">
                                                {(['paste', 'clipboard', 'file'] as const).map((id) => {
                                                    const selected = outputMode === id;
                                                    const base = 'text-xs lowercase rounded-md px-3 py-2 bg-[#1a1a1a] border-2 transition-colors outline-none focus:outline-none focus:ring-0 focus-visible:ring-0';
                                                    if (selected) {
                                                        const color = id === 'paste'
                                                            ? 'border-[#50fa7b] hover:border-[#50fa7b]'
                                                            : id === 'clipboard'
                                                                ? 'border-[#f1fa8c] hover:border-[#f1fa8c]'
                                                                : 'border-[#4ea3ff] hover:border-[#4ea3ff]';
                                                        return (
                                                            <button key={id} onClick={() => onSelectSink(id)} className={`${base} ${color} text-[#e5e5e5]`}>
                                                                {id}
                                                            </button>
                                                        );
                                                    }
                                                    const hover = id === 'paste'
                                                        ? 'hover:border-[rgba(80,250,123,0.6)]'
                                                        : id === 'clipboard'
                                                            ? 'hover:border-[rgba(241,250,140,0.6)]'
                                                            : 'hover:border-[rgba(78,163,255,0.6)]';
                                                    return (
                                                        <button key={id} onClick={() => onSelectSink(id)} className={`${base} border-[#2a2a2a] ${hover} text-[#a0a0a0]`}>
                                                            {id}
                                                        </button>
                                                    );
                                                })}
                                            </div>
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">
                                                {outputMode === "paste" && "simulate keystrokes into the active window"}
                                                {outputMode === "clipboard" && "copy text to system clipboard"}
                                                {outputMode === "file" && "append text to a specified file"}
                                            </p>
                                        </div>

                                        {/* Push-to-talk Hotkey */}
                                        <div className="space-y-2">
                                            <div className="text-[13px] text-[#e8e8e8] lowercase">push to talk</div>
                                            <div className="relative">
                                                <select
                                                    value={hotkey}
                                                    onChange={(e) => {
                                                        const v = e.target.value;
                                                        setHotkey(v);
                                                        invoke('set_hotkey', { label: v }).catch(() => { });
                                                        invoke('update_config', { patch: { hotkey: v } }).catch(() => { });
                                                    }}
                                                    className="text-[13px] font-mono text-[#e8e8e8] bg-[#1a1a1a] border border-[#2a2a2a] rounded px-2 py-2 w-full pr-6 appearance-none outline-none"
                                                >
                                                    <option value="control+option">control+option</option>
                                                    <option value="control+shift">control+shift</option>
                                                </select>
                                                <div className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[#a0a0a0] text-xs">▾</div>
                                            </div>
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">
                                                format: modifier+key (e.g., control+option, ctrl+shift+d)
                                            </p>
                                        </div>

                                        {/* Input Device */}
                                        <div className="space-y-2">
                                            <div className="text-[13px] text-[#e8e8e8] lowercase">input</div>
                                            <div className="relative">
                                                <select
                                                    value={selectedDevice || ''}
                                                    onChange={(e) => onSelectDevice(e.target.value)}
                                                    className="text-[13px] text-[#e8e8e8] bg-[#1a1a1a] border border-[#2a2a2a] rounded px-2 py-2 w-full pr-6 appearance-none outline-none"
                                                >
                                                    <option value="">system default</option>
                                                    {devices.map((dev, idx) => (
                                                        <option key={idx} value={dev}>{dev}</option>
                                                    ))}
                                                </select>
                                                <div className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[#a0a0a0] text-xs">▾</div>
                                            </div>
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">if unavailable, os default device is used</p>
                                        </div>

                                        {/* Start at Login */}
                                        <div className="space-y-2">
                                            <div className="text-[13px] text-[#e8e8e8] lowercase">start at login</div>
                                            <label className="inline-flex items-center gap-2 text-xs text-[#a0a0a0]">
                                                <input
                                                    type="checkbox"
                                                    checked={autostart}
                                                    onChange={(e) => onToggleAutostart(e.target.checked)}
                                                />
                                                enable
                                            </label>
                                        </div>
                                    </div>
                                )}
                            </div>

                            {/* AI Section */}
                            <div>
                                <button
                                    onClick={() => setAiExpanded(!aiExpanded)}
                                    className="flex items-center gap-2 w-full text-left group bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none cursor-pointer pb-2 select-none focus:outline-none focus:ring-0 focus-visible:outline-none active:bg-transparent active:text-[#e8e8e8]"
                                >
                                    <span className="text-[#a0a0a0] text-sm flex items-center leading-none">{aiExpanded ? '▼' : '▶'}</span>
                                    <h3 className="text-[15px] font-medium text-[#e8e8e8] lowercase leading-none">ai</h3>
                                </button>
                                <div className="border-b border-[#2a2a2a] mb-4"></div>

                                {aiExpanded && (
                                    <div className="mt-4 space-y-5">
                                        {/* Language */}
                                        <div className="space-y-2">
                                            <div className="text-[13px] text-[#e8e8e8] lowercase">language</div>
                                            <div className="relative">
                                                <select
                                                    value={config.language || 'auto'}
                                                    onChange={(e) => {
                                                        const v = e.target.value === 'auto' ? null : e.target.value;
                                                        updateConfigValue('language', v);
                                                    }}
                                                    className="text-[13px] text-[#e8e8e8] bg-[#1a1a1a] border border-[#2a2a2a] rounded px-2 py-2 w-full pr-6 appearance-none outline-none"
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
                                                </select>
                                                <div className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[#a0a0a0] text-xs">▾</div>
                                            </div>
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">auto-detect is slower and less accurate</p>
                                        </div>

                                        {/* Model */}
                                        <div className="space-y-2">
                                            <div className="text-[13px] text-[#e8e8e8] lowercase">model</div>
                                            <div className="relative">
                                                <select
                                                    value={config.modelPath}
                                                    onChange={(e) => updateConfigValue('modelPath', e.target.value)}
                                                    className="text-[13px] font-mono text-[#e8e8e8] bg-[#1a1a1a] border border-[#2a2a2a] rounded px-2 py-2 w-full pr-6 appearance-none outline-none"
                                                >
                                                    <option value="openai/whisper-large-v3-turbo">openai/whisper-large-v3-turbo</option>
                                                    <option value="openai/whisper-large-v3">openai/whisper-large-v3</option>
                                                    <option value="openai/whisper-medium">openai/whisper-medium</option>
                                                    <option value="openai/whisper-small">openai/whisper-small</option>
                                                    <option value="openai/whisper-base">openai/whisper-base</option>
                                                    <option value="openai/whisper-tiny">openai/whisper-tiny</option>
                                                </select>
                                                <div className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[#a0a0a0] text-xs">▾</div>
                                            </div>
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">larger models are more accurate but slower</p>
                                        </div>
                                    </div>
                                )}
                            </div>

                            {/* VAD Section */}
                            <div>
                                <button
                                    onClick={() => setVadExpanded(!vadExpanded)}
                                    className="flex items-center gap-2 w-full text-left group bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none cursor-pointer pb-2 select-none focus:outline-none focus:ring-0 focus-visible:outline-none active:bg-transparent active:text-[#e8e8e8]"
                                >
                                    <span className="text-[#a0a0a0] text-sm flex items-center leading-none">{vadExpanded ? '▼' : '▶'}</span>
                                    <h3 className="text-[15px] font-medium text-[#e8e8e8] lowercase leading-none">voice detection (vad)</h3>
                                </button>
                                <div className="border-b border-[#2a2a2a] mb-4"></div>

                                {vadExpanded && (
                                    <div className="mt-4 space-y-5">
                                        {/* VAD Start dB */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">start threshold</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.vadStartDb.toFixed(1)} dB</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="-50"
                                                max="-30"
                                                step="0.5"
                                                value={config.vadStartDb}
                                                onChange={(e) => updateConfigValue('vadStartDb', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">lower (more negative) = more sensitive</p>
                                        </div>

                                        {/* VAD Stop dB */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">stop threshold</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.vadStopDb.toFixed(1)} dB</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="-60"
                                                max="-35"
                                                step="0.5"
                                                value={config.vadStopDb}
                                                onChange={(e) => updateConfigValue('vadStopDb', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                            <p className="text-[11px] text-[#a0a0a0] lowercase">should be lower than start threshold</p>
                                        </div>

                                        {/* Min Duration */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">min speech duration</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.vadMinDuration} ms</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="0"
                                                max="1000"
                                                step="50"
                                                value={config.vadMinDuration}
                                                onChange={(e) => updateConfigValue('vadMinDuration', parseInt(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>

                                        {/* Max Silence */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">maximum silence</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.vadMaxSilence} ms</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="0"
                                                max="2000"
                                                step="100"
                                                value={config.vadMaxSilence}
                                                onChange={(e) => updateConfigValue('vadMaxSilence', parseInt(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>
                                    </div>
                                )}
                            </div>

                            {/* EQ Section */}
                            <div>
                                <button
                                    onClick={() => setEqExpanded(!eqExpanded)}
                                    className="flex items-center gap-2 w-full text-left group bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none cursor-pointer pb-2 select-none focus:outline-none focus:ring-0 focus-visible:outline-none active:bg-transparent active:text-[#e8e8e8]"
                                >
                                    <span className="text-[#a0a0a0] text-sm flex items-center leading-none">{eqExpanded ? '▼' : '▶'}</span>
                                    <h3 className="text-[15px] font-medium text-[#e8e8e8] lowercase leading-none">audio visualization (eq)</h3>
                                </button>
                                <div className="border-b border-[#2a2a2a] mb-4"></div>

                                {eqExpanded && (
                                    <div className="mt-4 space-y-5">
                                        {/* EQ Bands */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">bands</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.eqBands}</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="16"
                                                max="128"
                                                step="8"
                                                value={config.eqBands}
                                                onChange={(e) => updateConfigValue('eqBands', parseInt(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>

                                        {/* Noise Reduction */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">noise reduction</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.eqNoiseReduction.toFixed(2)}</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="0"
                                                max="1"
                                                step="0.01"
                                                value={config.eqNoiseReduction}
                                                onChange={(e) => updateConfigValue('eqNoiseReduction', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>

                                        {/* Window dB */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">window</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.eqWindowDb.toFixed(1)} dB</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="10"
                                                max="80"
                                                step="1"
                                                value={config.eqWindowDb}
                                                onChange={(e) => updateConfigValue('eqWindowDb', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>

                                        {/* Gamma */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">gamma</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.eqGamma.toFixed(1)}</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="0.8"
                                                max="2"
                                                step="0.1"
                                                value={config.eqGamma}
                                                onChange={(e) => updateConfigValue('eqGamma', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>

                                        {/* Attack */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">attack</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.eqAttack.toFixed(2)}</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="0.05"
                                                max="1"
                                                step="0.05"
                                                value={config.eqAttack}
                                                onChange={(e) => updateConfigValue('eqAttack', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>

                                        {/* Decay */}
                                        <div className="space-y-2">
                                            <div className="flex items-center justify-between">
                                                <div className="text-[13px] text-[#e8e8e8] lowercase">decay</div>
                                                <span className="text-[12px] text-[#a0a0a0] font-mono">{config.eqDecay.toFixed(2)}</span>
                                            </div>
                                            <input
                                                type="range"
                                                min="0.05"
                                                max="1"
                                                step="0.05"
                                                value={config.eqDecay}
                                                onChange={(e) => updateConfigValue('eqDecay', parseFloat(e.target.value))}
                                                className="w-full"
                                            />
                                        </div>
                                    </div>
                                )}
                            </div>
                        </div>
                        {/* Scroll indicator gradient */}
                        <div className="absolute bottom-0 left-0 right-2 h-12 bg-gradient-to-t from-[#0f0f0f] via-[#0f0f0f]/80 to-transparent pointer-events-none" />
                    </div>

                    <div className="text-center mt-2">
                        <button
                            onClick={onBack}
                            className="text-xs text-[#a0a0a0] hover:text-[#e5e5e5] transition-colors lowercase bg-transparent border-0 p-0 m-0 shadow-none rounded-none appearance-none"
                        >
                            [back to home]
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}
