/**
 * Onboarding view component.
 * 
 * First-run screen that guides users through granting required system permissions.
 * Displays permission cards for:
 * - Microphone: Required for voice input and transcription
 * - Accessibility: Required for paste mode and global hotkeys
 * 
 * Users can click permission cards to open system settings, then recheck permissions
 * to verify they've been granted.
 */

import { invoke } from '@tauri-apps/api/core';
import { safeExecute } from '../utils/errorHandling';
import { BrandTitle } from '../components/BrandTitle';
import { PermissionCard } from '../components/PermissionCard';

export interface OnboardingViewProps {
    permissions: {
        microphone?: boolean;
        accessibility?: boolean;
    };
    onComplete: () => Promise<void> | void;
}

export default function OnboardingView({ permissions, onComplete }: OnboardingViewProps) {
    /**
     * Open system settings to the specified permission pane (platform-specific).
     * 
     * On macOS: Opens System Settings > Privacy & Security > [pane]
     * On Windows: Opens Settings > Privacy > [pane]
     * On Linux: Opens system settings app (GNOME preferred)
     * 
     * @param section - Settings pane name (e.g., 'Microphone', 'Accessibility')
     */
    async function openSystemSettings(section: string) {
        await safeExecute(
            () => invoke('open_system_settings', { pane: section }),
            `openSystemSettings(${section})`
        );
    }

    /**
     * Recheck permissions after user grants them in System Settings.
     * 
     * Calls the onComplete callback which triggers a permission check.
     * This allows the app to verify permissions were granted and proceed.
     */
    async function recheckPermissions() {
        await safeExecute(
            () => Promise.resolve(onComplete()),
            'recheckPermissions'
        );
    }

    return (
        <div className="">
            <div className="mx-3 mb-2 rounded-xl border border-border bg-bgCard overflow-hidden">
                <div className="py-[30px] px-[22px]">
                    {/* Brand title */}
                    <div className="mb-6">
                        <BrandTitle />
                    </div>

                    {/* Instructions */}
                    <div className="mb-6">
                        <p className="text-[13px] text-textSecondary lowercase leading-relaxed">
                            keyless needs a few permissions to function properly. please grant them in system settings.
                        </p>
                    </div>

                    {/* Permission cards: show status and allow opening System Settings */}
                    <div className="space-y-4 mb-6">
                        {/* Microphone permission: required for audio input */}
                        <PermissionCard
                            name="microphone"
                            granted={permissions.microphone ?? false}
                            description="required for voice input and transcription"
                            onOpenSettings={() => openSystemSettings('Microphone')}
                        />

                        {/* Accessibility permission: required for paste mode and hotkeys */}
                        <PermissionCard
                            name="accessibility"
                            granted={permissions.accessibility ?? false}
                            description="required for paste mode and global hotkeys"
                            onOpenSettings={() => openSystemSettings('Accessibility')}
                        />
                    </div>

                    {/* Recheck button: verifies permissions after user grants them */}
                    <button
                        onClick={recheckPermissions}
                        className="w-full text-[13px] lowercase px-4 py-3 rounded-lg bg-border hover:bg-bgHover text-textAlt border border-bgHoverAlt hover:border-white font-medium transition-colors"
                    >
                        recheck permissions
                    </button>
                </div>
            </div>
        </div>
    );
}
