import { useEffect, useRef } from 'react';
import { BrandTitle } from '../components/BrandTitle';
import { formatDuration } from '../utils/format';
import { Card } from '../components/layout/Card';
import { LinkButton } from '../components/buttons/LinkButton';
import { ScrollFade } from '../components/ui/ScrollFade';
import { useScrollFade } from '../hooks/useScrollFade';
import { GeneralSection } from '../components/settings/GeneralSection';
import { AISection } from '../components/settings/AISection';
import { VADSection } from '../components/settings/VADSection';
import { useSettingsEventListeners } from '../hooks/useSettingsEventListeners';
import { useSettingsState, type SettingsBootstrapData } from '../hooks/useSettingsState';
import { AnimatedNumber } from '../components/ui/AnimatedNumber';

type SettingsViewProps = {
    onBack?: () => void;
    onOpenModels?: () => void;
    outputModeOverride?: string;
    stats?: { sessionWords: number; sessionTalkMs: number; lifetimeWords: number; lifetimeTalkMs: number; };
    overlayOffset?: number;
    initialSection?: 'ai' | 'general' | 'vad';
    bootstrap?: SettingsBootstrapData;
    missingModel?: boolean;
};

/**
 * Settings view component.
 * 
 * Displays all application settings organized into collapsible sections:
 * - General: Output mode, hotkey, autostart, input device, file path
 * - AI: Language, model selection
 * - VAD: Voice activity detection thresholds
 * 
 * Handles scrolling to specific sections when opened from external navigation (e.g., tray menu).
 */
export default function SettingsView({ onBack, onOpenModels, outputModeOverride, stats, overlayOffset, initialSection, bootstrap, missingModel }: SettingsViewProps) {
    // Consolidated settings state management (replaces multiple useState hooks).
    const { state, actions } = useSettingsState(outputModeOverride, bootstrap);
    // Reference to scrollable container (for scroll fade effect).
    const scrollRef = useRef<HTMLDivElement>(null);
    // Reference to AI section (for scrolling when opened from models view).
    const aiSectionRef = useRef<HTMLDivElement>(null);

    // Calculate fade alpha for bottom scroll fade effect.
    const bottomFadeAlpha = useScrollFade(scrollRef);

    // Listen for settings-related events from backend (output mode changes, model selection).
    useSettingsEventListeners({
        onOutputModeChanged: (mode) => actions.setOutputMode(mode),
        onModelSelected: (modelId) => {
            actions.updateConfig('modelPath', modelId);
        },
    });

    /**
     * Scroll to AI section when opened from models view.
     * 
     * Expands AI section, collapses others, then smoothly scrolls to the AI section.
     * Uses setTimeout(0) to ensure DOM has updated before scrolling.
     */
    useEffect(() => {
        if (initialSection === 'ai') {
            // Expand AI section, collapse others.
            actions.setExpandedSections({ general: false, ai: true, vad: false });
            // Scroll to AI section after DOM updates (setTimeout ensures layout is complete).
            setTimeout(() => {
                const container = scrollRef.current;
                const target = aiSectionRef.current;
                if (container && target) {
                    // Calculate scroll position (8px offset for visual spacing).
                    const top = Math.max(0, target.offsetTop - 8);
                    container.scrollTo({ top, behavior: 'smooth' });
                }
            }, 0);
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [initialSection]); // actions are stable



    return (
        <div>
            <Card className="overflow-hidden">
                <div className="pt-[26px] px-[22px] pb-[14px]">
                    <div className="mb-2">
                        <BrandTitle />
                    </div>
                    {stats && (
                        <div className="mb-6 text-[11px] text-textSecondary lowercase text-center">
                            lifetime • <AnimatedNumber value={stats.lifetimeWords} /> words • {formatDuration(stats.lifetimeTalkMs)}
                        </div>
                    )}

                    <div className="relative mb-2">
                        <div
                            ref={scrollRef}
                            className="space-y-6 pr-2 scrollbar-hide overflow-y-auto overflow-x-hidden relative"
                            style={{ maxHeight: overlayOffset ? `calc(100vh - ${overlayOffset + 240}px)` : 'calc(100vh - 240px)' }}
                        >
                            {/* General Section */}
                            <GeneralSection
                                expanded={state.expandedSections.general}
                                onToggle={() => actions.setExpandedSection('general', !state.expandedSections.general)}
                                autostart={state.autostart}
                                onToggleAutostart={actions.onToggleAutostart}
                                outputMode={state.outputMode}
                                onSelectSink={actions.onSelectSink}
                                filePath={state.filePath}
                                setFilePath={actions.setFilePath}
                                pttListener={state.pttListener}
                                setPttListener={actions.setPttListener}
                                devices={state.devices}
                                selectedDevice={state.selectedDevice}
                                onSelectDevice={actions.onSelectDevice}
                            />

                            {/* AI Section */}
                            <div ref={aiSectionRef}>
                                <AISection
                                    expanded={state.expandedSections.ai}
                                    onToggle={() => actions.setExpandedSection('ai', !state.expandedSections.ai)}
                                    config={{
                                        language: state.config.language,
                                        modelPath: state.config.modelPath,
                                    }}
                                    updateConfigValue={actions.updateConfigValue}
                                    onOpenModels={onOpenModels}
                                    missingModel={missingModel}
                                />
                            </div>

                            {/* VAD Section */}
                            <VADSection
                                expanded={state.expandedSections.vad}
                                onToggle={() => actions.setExpandedSection('vad', !state.expandedSections.vad)}
                                config={{
                                    vadStartDb: state.config.vadStartDb,
                                    vadStopDb: state.config.vadStopDb,
                                    vadMinDuration: state.config.vadMinDuration,
                                    vadMaxSilence: state.config.vadMaxSilence,
                                }}
                                updateConfigValue={actions.updateConfigValue}
                            />

                            {/* bottom fade over scroll content */}
                            <ScrollFade alpha={bottomFadeAlpha} />
                        </div>
                    </div>

                    {/* Footer: match Models back footer */}
                    <div className="sticky bottom-0 inset-x-0 text-center py-[2px]">
                        <LinkButton onClick={onBack}>[back to app]</LinkButton>
                    </div>
                </div>
            </Card>
        </div>
    );
}
