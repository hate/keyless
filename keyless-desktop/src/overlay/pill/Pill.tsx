/**
 * Pill overlay component - audio visualization overlay window.
 * 
 * Displays a floating audio visualization (EQ bars) that appears when PTT is held.
 * Shows real-time audio frequency bands with smooth animations and state transitions.
 * 
 * Features:
 * - Real-time EQ visualization (6 bars from audio frequency bands)
 * - Smooth bar animations with exponential smoothing
 * - State transitions (idle, listening, finalizing, exiting)
 * - Auto-hide when PTT is released
 */

import { CSSProperties, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { safeExecute } from "../../utils/errorHandling";
import "./Pill.css";

// Target number of bars to display (reduced from raw EQ data for cleaner visualization).
const TARGET_BARS = 6;
// Center index for finalizing animation (bars animate outward from center).
const CENTER_INDEX = (TARGET_BARS - 1) / 2;
// Offset for even number of bars (centers animation between two middle bars).
const EVEN_CENTER_OFFSET = TARGET_BARS % 2 === 0 ? 0.5 : 0;

/**
 * Clamp value to [0, 1] range.
 */
const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

/**
 * Max pooling: reduce array to target length by taking max values from slices.
 * 
 * Used to downsample EQ frequency bands (e.g., 32 bands -> 6 bars) while preserving
 * peak values for better visualization.
 */
const maxPool = (values: number[], target: number): number[] => {
    // Handle empty input: return array of zeros.
    if (values.length === 0) {
        return Array.from({ length: target }, () => 0);
    }
    // If already smaller or equal, return as-is.
    if (values.length <= target) {
        return values.slice();
    }
    // Downsample by taking max of each slice.
    return Array.from({ length: target }, (_, index) => {
        const start = Math.floor((index / target) * values.length);
        const end = Math.floor(((index + 1) / target) * values.length);
        const slice = values.slice(start, Math.max(end, start + 1));
        // Take maximum value from slice (preserves peaks).
        return slice.reduce((acc, value) => (value > acc ? value : acc), 0);
    });
};

/**
 * Normalize EQ bars to [0, 1] range.
 * 
 * Converts raw EQ values (0-100) to normalized [0, 1] range for visualization.
 * Handles invalid values (NaN, Infinity) by defaulting to 0.
 */
const normaliseBars = (bars: number[]) =>
    bars.map((value) => clamp01((Number.isFinite(value) ? value : 0) / 100));

export default function Pill() {
    // Visibility state: controls whether overlay is shown.
    const [visible, setVisible] = useState(false);
    // Exiting state: true during exit animation (scaleOut).
    const [exiting, setExiting] = useState(false);
    // VAD speaking state: true when speech is detected (affects bar appearance).
    const [vadOpen, setVadOpen] = useState(false);
    // Finalizing state: true when PTT released but waiting for final transcription.
    const [finalizing, setFinalizing] = useState(false);
    // Current bar values (normalized [0, 1]) for rendering.
    const [bars, setBars] = useState<number[]>(() =>
        Array.from({ length: TARGET_BARS }, () => 0),
    );

    // Smoothed bar values (for exponential smoothing between updates).
    const smoothedRef = useRef<number[]>(Array.from({ length: TARGET_BARS }, () => 0));
    // Timer for exit animation delay.
    const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    // Current runtime status (idle or listening) for state management.
    const statusRef = useRef<"idle" | "listening">("idle");

    /**
     * Set up event listeners for runtime status, transcription, VAD, and EQ updates.
     * 
     * Handles:
     * - status_changed: Show/hide overlay based on PTT state
     * - final_transcription: Hide overlay after final transcription
     * - vad_speaking: Update VAD state (affects bar appearance)
     * - eq_update: Update bar values with smooth animation
     */
    useEffect(() => {
        const subscriptions: Array<() => void> = [];

        /**
         * Reset all bars to zero (used when hiding overlay).
         */
        const resetBars = () => {
            const zeros = Array.from({ length: TARGET_BARS }, () => 0);
            smoothedRef.current = zeros;
            setBars(zeros);
        };

        /**
         * Clear exit timer (prevents duplicate timers).
         */
        const clearExitTimer = () => {
            if (exitTimerRef.current) {
                clearTimeout(exitTimerRef.current);
                exitTimerRef.current = null;
            }
        };

        /**
         * Schedule hide sequence: start exit animation, then hide after animation completes.
         */
        const scheduleHide = () => {
            setExiting(true);
            clearExitTimer();
            // Hide after scaleOut animation completes (150ms).
            exitTimerRef.current = window.setTimeout(() => {
                setVisible(false);
                setExiting(false);
                setVadOpen(false);
                setFinalizing(false);
                resetBars();
            }, 150); // Match scaleOut animation duration
        };

        /**
         * Start finalizing state (PTT released, waiting for final transcription).
         */
        const startFinalizing = () => {
            setFinalizing(true);
            setVisible(true);
            setExiting(false);
        };

        /**
         * Stop finalizing state (final transcription received).
         */
        const stopFinalizing = () => {
            setFinalizing(false);
        };

        /**
         * Handle runtime status changes (idle/listening).
         * 
         * When listening: show overlay, clear timers, reset VAD state.
         * When idle: start finalizing (wait for final transcription before hiding).
         */
        const handleStatus = (status: string) => {
            const normalized = status === "listening" ? "listening" : "idle";
            statusRef.current = normalized;
            if (normalized === "listening") {
                // PTT held: show overlay immediately.
                clearExitTimer();
                stopFinalizing();
                setExiting(false);
                setVadOpen(false); // Reset to idle state when opening
                setVisible(true);
            } else {
                // PTT released: enter finalizing state (wait for final transcription).
                startFinalizing();
            }
        };

        safeExecute(
            () => listen<string>("status_changed", (event) => {
                const status = String(event.payload || "").toLowerCase();
                handleStatus(status);
            }).then((unsub) => subscriptions.push(unsub)),
            'Pill status_changed listener'
        );

        safeExecute(
            () => listen("final_transcription", () => {
                stopFinalizing();
                if (statusRef.current === "idle") {
                    scheduleHide();
                }
            }).then((unsub) => subscriptions.push(unsub)),
            'Pill final_transcription listener'
        );

        safeExecute(
            () => listen<boolean>("vad_speaking", (event) =>
                setVadOpen(Boolean(event.payload)),
            ).then((unsub) => subscriptions.push(unsub)),
            'Pill vad_speaking listener'
        );

        // Listen for EQ updates: update bar values with exponential smoothing.
        safeExecute(
            () => listen<number[]>("eq_update", (event) => {
                // Extract and normalize EQ values.
                const values = Array.isArray(event.payload) ? event.payload : [];
                const clamped = normaliseBars(values.map((value) => Number(value) || 0));
                // Downsample to target bar count using max pooling.
                const pooled = maxPool(clamped, TARGET_BARS);
                // Apply exponential smoothing (faster attack, slower decay for natural feel).
                const smoothed = pooled.map((value, index) => {
                    const previous = smoothedRef.current[index] ?? 0;
                    // Faster smoothing when rising (0.45), slower when falling (0.3).
                    const alpha = value > previous ? 0.45 : 0.3;
                    return previous + (value - previous) * alpha;
                });
                smoothedRef.current = smoothed;
                setBars(smoothed);
            }).then((unsub) => subscriptions.push(unsub)),
            'Pill eq_update listener'
        );
        return () => {
            clearExitTimer();
            subscriptions.forEach((unsubscribe) => unsubscribe());
        };
    }, []);

    // Don't render if not visible (reduces DOM overhead).
    if (!visible) {
        return null;
    }

    // Build CSS class list based on state.
    const pillClassName = ["pill"];
    if (exiting) {
        pillClassName.push("exiting");
    }
    if (finalizing) {
        pillClassName.push("finalizing");
    }

    return (
        <div className={pillClassName.join(" ")}>
            <div className="bars">
                {bars.map((value, index) => {
                    // Apply square root easing for more natural bar movement.
                    const eased = Math.sqrt(clamp01(value));
                    // Calculate height percentage (minimum 10% for visibility).
                    const heightPercent = Math.max(10, Math.round(eased * 100));
                    // Calculate opacity (0.5 to 1.0 based on value).
                    const opacity = 0.5 + (eased * 0.5);
                    // Determine if bars should be in idle state (no VAD, not finalizing).
                    const idle = !vadOpen && !finalizing;
                    const classes = ["bar"];
                    if (idle) {
                        classes.push("idle");
                    }
                    if (finalizing) {
                        classes.push("finalizing");
                    }
                    const barStyle: CSSProperties = {};
                    if (finalizing) {
                        // Finalizing animation: bars animate outward from center with staggered delays.
                        const distanceFromCenter = Math.max(0, Math.abs(index - CENTER_INDEX) - EVEN_CENTER_OFFSET);
                        const delayMs = distanceFromCenter * 90;
                        barStyle.animationDelay = `${delayMs}ms`;
                        barStyle.animationDuration = `${900 + distanceFromCenter * 120}ms`;
                    } else if (!idle) {
                        // Active state: set height and opacity based on smoothed values.
                        barStyle.height = `${heightPercent}%`;
                        barStyle.opacity = opacity;
                    }
                    return (
                        <div
                            key={index}
                            className={classes.join(" ")}
                            style={barStyle}
                        />
                    );
                })}
            </div>
        </div>
    );
}

