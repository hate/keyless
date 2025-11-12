/**
 * Toast overlay component - transcription display overlay window.
 * 
 * Displays a floating toast notification showing transcribed text after PTT is released.
 * Shows:
 * - Partial transcript (real-time updates while speaking)
 * - Final transcript (with word count and output sink indicator)
 * - Background flash animation on final transcription
 * - Auto-dismiss after 5 seconds
 * 
 * The toast appears at the top-right of the primary monitor and automatically hides
 * when new speech starts or after a timeout.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { logWarn } from "../../utils/logger";
import { COLORS } from "../../constants/colors";
import "./Toast.css";

type ToastData = {
  text: string;
  wordCount: number;
  sink: string;
};

/**
 * Count words in a string (splits on whitespace).
 * 
 * @param value - String to count words in
 * @returns Number of words (0 for empty/whitespace-only strings)
 */
const countWords = (value: string) => {
  const trimmed = value.trim();
  if (!trimmed) {
    return 0;
  }
  return trimmed.split(/\s+/).length;
};

export default function Toast() {
  // Visibility state: controls whether toast is shown.
  const [visible, setVisible] = useState(false);
  // Exiting state: true during exit animation (slideOut).
  const [exiting, setExiting] = useState(false);
  // Current transcript text to display.
  const [text, setText] = useState("");
  // Word count for the transcript.
  const [wordCount, setWordCount] = useState(0);
  // Output sink (paste/clipboard/file) for final transcription.
  const [sink, setSink] = useState("paste");
  // Whether to show sink indicator (only for final transcription).
  const [sinkVisible, setSinkVisible] = useState(false);
  // Background flash animation state (triggers on final transcription).
  const [bgFlash, setBgFlash] = useState(false);
  // Key for sink animation (forces re-render for animation restart).
  const [sinkAnimationKey, setSinkAnimationKey] = useState(0);

  // Timer for auto-hide delay (5 seconds).
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Timer for exit animation delay.
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Timer for background flash animation (700ms).
  const bgFlashTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Reference to container element (for potential DOM operations).
  const containerRef = useRef<HTMLDivElement | null>(null);

  /**
   * Clear all hide/exit timers (used when new transcription starts).
   */
  const clearHideTimers = useCallback(() => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    if (exitTimerRef.current) {
      clearTimeout(exitTimerRef.current);
      exitTimerRef.current = null;
    }
  }, []);

  /**
   * Reset final transcription visuals (sink indicator and flash).
   */
  const resetFinalVisuals = useCallback(() => {
    setSinkVisible(false);
    setBgFlash(false);
  }, []);

  /**
   * Trigger background flash animation (700ms duration).
   * Used when final transcription is received.
   */
  const triggerBgFlash = useCallback(() => {
    if (bgFlashTimerRef.current) {
      clearTimeout(bgFlashTimerRef.current);
      bgFlashTimerRef.current = null;
    }
    setBgFlash(true);
    bgFlashTimerRef.current = window.setTimeout(() => {
      setBgFlash(false);
      bgFlashTimerRef.current = null;
    }, 700);
  }, []);

  /**
   * Start auto-hide sequence: wait 5 seconds, then start exit animation.
   */
  const startHideSequence = useCallback(() => {
    hideTimerRef.current = window.setTimeout(() => {
      setExiting(true);
      // Hide after slideOut animation completes (300ms).
      exitTimerRef.current = window.setTimeout(() => {
        setVisible(false);
        setExiting(false);
        resetFinalVisuals();
        setText("");
        setWordCount(0);
      }, 300); // Match slideOut animation duration
    }, 5000);
  }, [resetFinalVisuals]);

  /**
   * Handle partial transcript (real-time updates while speaking).
   * 
   * Shows toast immediately, clears hide timers, resets final visuals.
   * Updates text and word count as user speaks.
   */
  const handlePreview = useCallback((preview: string) => {
    const trimmed = preview.trim();
    if (!trimmed) {
      return;
    }
    clearHideTimers();
    setVisible(true);
    setExiting(false);
    resetFinalVisuals();
    setText(preview);
    setWordCount(countWords(preview));
  }, [clearHideTimers, resetFinalVisuals]);

  /**
   * Handle final transcript (after PTT release and processing completes).
   * 
   * Shows sink indicator, triggers flash animation, starts auto-hide sequence.
   * Uses word count from payload if available, otherwise counts words.
   */
  const handleFinal = useCallback((payload: ToastData) => {
    if (!payload) {
      return;
    }
    clearHideTimers();
    setVisible(true);
    setExiting(false);
    // Show sink indicator (paste/clipboard/file) with animation.
    setSinkVisible(true);
    setSinkAnimationKey((value) => value + 1);
    setText(payload.text);
    // Use word count from payload if valid, otherwise count words.
    setWordCount(
      typeof payload.wordCount === "number" && payload.wordCount > 0
        ? payload.wordCount
        : countWords(payload.text),
    );
    setSink(payload.sink || "paste");
    triggerBgFlash();
    startHideSequence();
  }, [clearHideTimers, triggerBgFlash, startHideSequence]);

  /**
   * Set up event listeners for transcript updates.
   * 
   * Listens to:
   * - transcript_partial: Real-time transcript updates (shows preview)
   * - final_transcription: Final transcript after processing (shows with sink indicator)
   */
  useEffect(() => {
    const subscriptions: Array<Promise<() => void>> = [];

    // Listen for partial transcript (real-time updates).
    subscriptions.push(
      listen<string>("transcript_partial", (event) => {
        const payload = typeof event.payload === "string" ? event.payload : "";
        handlePreview(payload);
      }),
    );

    // Listen for final transcript (after processing completes).
    subscriptions.push(
      listen<ToastData>("final_transcription", (event) => {
        handleFinal(event.payload);
      }),
    );

    // Cleanup: clear timers and unsubscribe from all events.
    return () => {
      clearHideTimers();
      if (bgFlashTimerRef.current) {
        clearTimeout(bgFlashTimerRef.current);
        bgFlashTimerRef.current = null;
      }
      subscriptions.forEach((subscription) => {
        subscription
          .then((unsub) => unsub())
          .catch((error) => {
            // Subscription cleanup errors are non-critical
            logWarn('Failed to unsubscribe from toast event:', error);
          });
      });
    };
  }, [handlePreview, handleFinal, clearHideTimers]);

  /**
   * Listen for VAD speaking state.
   * 
   * When speech starts, clear hide timers and reset visuals (allows new transcript
   * to replace old one immediately).
   */
  useEffect(() => {
    const subscription = listen<boolean>("vad_speaking", (event) => {
      if (event.payload) {
        clearHideTimers();
        resetFinalVisuals();
      }
    });
    return () => {
      subscription
        .then((unsub) => unsub())
        .catch((error) => {
          // Subscription cleanup errors are non-critical
          logWarn('Failed to unsubscribe from toast event:', error);
        });
    };
  }, [clearHideTimers, resetFinalVisuals]);

  /**
   * Listen for runtime status changes.
   * 
   * When PTT is held (listening), clear hide timers and reset visuals (allows
   * new transcript to replace old one immediately).
   */
  useEffect(() => {
    const subscription = listen<string>("status_changed", (event) => {
      const payload = typeof event.payload === "string" ? event.payload : "";
      if (payload.toLowerCase() === "listening") {
        clearHideTimers();
        resetFinalVisuals();
      }
    });
    return () => {
      subscription
        .then((unsub) => unsub())
        .catch((error) => {
          // Subscription cleanup errors are non-critical
          logWarn('Failed to unsubscribe from toast event:', error);
        });
    };
  }, [clearHideTimers, resetFinalVisuals]);

  // Don't render if not visible or text is empty (reduces DOM overhead).
  if (!visible || !text.trim()) {
    return null;
  }

  /**
   * Get color for sink indicator based on output mode.
   * 
   * @param value - Output sink name (paste/clipboard/file)
   * @returns Color string for the sink indicator
   */
  const getSinkColor = (value: string) => {
    const lowerSink = value.toLowerCase();
    if (lowerSink === "paste") return COLORS.sink.paste;
    if (lowerSink === "clipboard") return COLORS.sink.clipboard;
    if (lowerSink === "file") return COLORS.sink.file;
    return COLORS.sink.paste;
  };

  return (
    <div
      ref={containerRef}
      className={exiting ? "toast exiting" : "toast"}
    >
      {sinkVisible ? (
        <div
          key={sinkAnimationKey}
          className={bgFlash ? "toast-bg-text flash" : "toast-bg-text"}
          style={{ color: getSinkColor(sink) }}
        >
          {sink.toUpperCase()}
        </div>
      ) : null}
      <div className="toast-content">
        <div className="toast-header">
          <span className="toast-title">
            keyless<span className="toast-carat">█</span>
          </span>
          <span className="toast-words">{wordCount} words</span>
        </div>
        <div className="toast-text">{text}</div>
      </div>
    </div>
  );
}
