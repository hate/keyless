import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export type DesktopStatus = 'idle' | 'listening';

export function useTauriStreams() {
    const [status, setStatus] = useState<DesktopStatus>('idle');
    const [eqBars, setEqBars] = useState<number[]>([]);
    const [transcript, setTranscript] = useState<string>('');
    const [speaking, setSpeaking] = useState<boolean>(false);

    useEffect(() => {
        const w = globalThis as unknown as { __TAURI_INTERNALS__?: unknown };
        // In tests (jsdom) tauri internals are absent; skip wiring to avoid errors
        if (!w.__TAURI_INTERNALS__) return;

        const unsubs: Array<() => void> = [];
        listen<string>('status_changed', (e) => {
            const s = (e.payload || '').toLowerCase();
            if (s === 'listening' || s === 'idle') setStatus(s as DesktopStatus);
        }).then((un) => unsubs.push(un)).catch(() => { });

        listen<number[]>('eq_update', (e) => {
            if (Array.isArray(e.payload)) setEqBars(e.payload.map((v) => Number(v) || 0));
        }).then((un) => unsubs.push(un)).catch(() => { });

        listen<string>('transcript_partial', (e) => {
            setTranscript(typeof e.payload === 'string' ? e.payload : '');
        }).then((un) => unsubs.push(un)).catch(() => { });

        listen<boolean>('vad_speaking', (e) => {
            setSpeaking(Boolean(e.payload));
        }).then((un) => unsubs.push(un)).catch(() => { });

        return () => {
            for (const un of unsubs) un();
        };
    }, []);

    return { status, eqBars, transcript, speaking };
}


