/**
 * Hook for listening to runtime status and audio stream events from the Tauri backend.
 * 
 * Tracks:
 * - Runtime status (idle/listening)
 * - EQ visualization bars (audio frequency bands)
 * - Partial transcript (real-time transcription)
 * - Speaking state (VAD detection)
 * 
 * All data comes from Tauri events emitted by the runtime thread.
 */

import { useEffect, useState } from 'react';
import { logDebug, logInfo, logError } from '../utils/logger';
import { attachListeners } from '../utils/eventListeners';
import { isTauriAvailable } from '../utils/tauriHelpers';

export type DesktopStatus = 'idle' | 'listening';

export function useTauriStreams() {
    // Runtime status: 'idle' (PTT not held) or 'listening' (PTT held).
    const [status, setStatus] = useState<DesktopStatus>('idle');
    // EQ bars: array of frequency band amplitudes (0-1) for audio visualization.
    const [eqBars, setEqBars] = useState<number[]>([]);
    // Partial transcript: current transcription text (updates in real-time).
    const [transcript, setTranscript] = useState<string>('');
    // Speaking state: true when VAD detects speech, false otherwise.
    const [speaking, setSpeaking] = useState<boolean>(false);

    /**
     * Set up event listeners for runtime status and audio stream events.
     * 
     * Listens to events from the runtime thread:
     * - status_changed: PTT state changes (idle/listening)
     * - eq_update: Audio frequency band updates (for visualization)
     * - transcript_partial: Real-time transcription updates
     * - vad_speaking: Voice activity detection state
     * - log_message: Backend log messages (for debugging)
     */
    useEffect(() => {
        if (!isTauriAvailable()) return;

        let cleanup: (() => void) | undefined;

        attachListeners([
            {
                // Backend log messages (for debugging, forwarded to console).
                event: 'log_message',
                handler: (payload) => {
                    if (typeof payload === 'string') {
                        logInfo('[backend]', payload);
                    }
                },
            },
            {
                // Runtime status change: PTT state (idle or listening).
                event: 'status_changed',
                handler: (payload) => {
                    const s = String(payload || '').toLowerCase();
                    if (s === 'listening' || s === 'idle') {
                        setStatus(s as DesktopStatus);
                    }
                },
            },
            {
                // EQ update: frequency band amplitudes for audio visualization.
                event: 'eq_update',
                handler: (payload) => {
                    if (Array.isArray(payload)) {
                        // Convert payload values to numbers (fallback to 0 if invalid).
                        const bars = payload.map((v) => Number(v) || 0);
                        setEqBars(bars);
                        // Log first 8 bars for debugging (reduces log noise).
                        logDebug('[backend] eq_update', bars.slice(0, 8));
                    }
                },
            },
            {
                // Partial transcript: real-time transcription text (updates as speech is processed).
                event: 'transcript_partial',
                handler: (payload) => {
                    setTranscript(typeof payload === 'string' ? payload : '');
                },
            },
            {
                // VAD speaking state: true when speech is detected, false when silent.
                event: 'vad_speaking',
                handler: (payload) => {
                    setSpeaking(Boolean(payload));
                },
            },
        ]).then((fn) => {
            cleanup = fn;
        }).catch((error) => {
            logError('Failed to attach Tauri event listeners:', error);
        });

        // Cleanup: remove all event listeners on unmount.
        return () => {
            cleanup?.();
        };
    }, []);

    return { status, eqBars, transcript, speaking };
}
