/**
 * Idle view component.
 * 
 * Displays the home screen when the app is idle (PTT not held).
 * Shows session statistics, hotkey instructions, and a warning if no model is selected.
 * This is the default view when the app is not actively listening.
 */

import { BrandTitle } from '../components/BrandTitle';
import { formatDuration } from '../utils/format';
import { Card } from '../components/layout/Card';
import { LinkButton } from '../components/buttons/LinkButton';

export interface IdleViewProps {
    hotkey?: string;
    onOpenSettings?: () => void;
    onOpenModels?: () => void;
    stats?: {
        sessionWords: number;
        sessionTalkMs: number;
        lifetimeWords: number;
        lifetimeTalkMs: number;
    };
    missingModel?: boolean;
}

/**
 * Warning component displayed when no model is selected.
 * 
 * Shows an error-styled message prompting the user to download a model.
 * Includes a button to navigate to the models view.
 */
const MissingModelHint = ({ onOpenModels }: { onOpenModels?: () => void }) => (
    <div className="w-full bg-errorBg border border-errorBorder rounded-lg px-3 py-2 text-[11px] lowercase text-errorText">
        <b>no model selected.</b><br />download one from settings.
        <button
            onClick={onOpenModels}
            className="pt-5 text-xs text-errorText hover:text-errorTextHover lowercase underline focus:outline-none border-none"
            type="button"
        >
            manage models
        </button>
    </div>
);

export default function IdleView({ hotkey, onOpenSettings, onOpenModels, stats, missingModel }: IdleViewProps) {
    return (
        <>
            <Card className="min-h-[240px] flex flex-col py-[30px] px-[22px]">
                {/* Brand title */}
                <div className="mb-2">
                    <BrandTitle />
                </div>
                {/* Session statistics (only shown if model is selected) */}
                {stats && !missingModel && (
                    <div className="mb-6 text-[11px] text-textSecondary lowercase text-center">
                        session • {stats.sessionWords.toLocaleString()} words • {formatDuration(stats.sessionTalkMs)}
                    </div>
                )}
                {/* Main content area: shows either missing model warning or idle state */}
                <div className="flex-1 flex flex-col items-center justify-center text-center space-y-[18px]">
                    {missingModel ? (
                        // Missing model warning: prompts user to download a model.
                        <div className="flex flex-col items-center justify-center gap-3 w-full pt-5">
                            <MissingModelHint onOpenModels={onOpenModels} />
                        </div>
                    ) : (
                        // Normal idle state: shows "IDLE" heading and hotkey instructions.
                        <>
                            <div className="text-[36px] font-bold text-textPrimary tracking-[0.01em] uppercase">IDLE</div>
                            <p className="text-xs text-textSecondary lowercase font-semibold">
                                hold <span className="bg-border text-textPrimary px-2 py-0.5 rounded font-medium">{hotkey}</span> to start listening
                            </p>
                        </>
                    )}
                </div>
                {/* Settings link */}
                <div className="mt-6 text-center">
                    <LinkButton onClick={onOpenSettings}>[settings]</LinkButton>
                </div>
            </Card>
        </>
    );
}
