/**
 * Listening view component.
 * 
 * Displays the active listening screen when PTT is held.
 * Shows:
 * - Runtime status (listening indicator)
 * - VAD status (speaking/not speaking)
 * - Audio visualization (EQ bars)
 * - Real-time transcript preview (auto-scrolls to show latest text)
 * 
 * The transcript preview automatically scrolls to the right to show the most recent
 * transcription text as it's being generated.
 */

import { useEffect, useRef, useState } from 'react';
import { BrandTitle } from '../components/BrandTitle';
import { AudioVisualizer } from '../components/AudioVisualizer';
import { Card } from '../components/layout/Card';
import { LinkButton } from '../components/buttons/LinkButton';
import { StatusPill } from '../components/StatusPill';

export interface ListeningViewProps {
  speaking: boolean;
  bars: number[];
  transcript: string;
  onOpenSettings?: () => void;
  stats?: {
    sessionWords: number;
    sessionTalkMs: number;
    lifetimeWords: number;
    lifetimeTalkMs: number;
  };
};

export default function ListeningView({ speaking, bars, transcript, onOpenSettings }: ListeningViewProps) {
  // VAD speaking state (true when speech is detected).
  const vadOpen = Boolean(speaking);
  // Refs for transcript auto-scroll: container (viewport) and text (scrollable content).
  const containerRef = useRef<HTMLDivElement | null>(null);
  const textRef = useRef<HTMLDivElement | null>(null);
  // Scroll offset in pixels (negative translateX to show right edge of text).
  const [offsetPx, setOffsetPx] = useState(0);

  /**
   * Auto-scroll transcript to show the most recent text.
   * 
   * Calculates the scroll offset needed to show the right edge of the transcript
   * (most recent text). Uses CSS transform for smooth animation.
   */
  useEffect(() => {
    const el = textRef.current;
    const container = containerRef.current;
    if (!el || !container) return;
    // Calculate scroll width (full text width) and container width (visible area).
    const sw = el.scrollWidth;
    const cw = container.clientWidth;
    // Calculate offset to show right edge (most recent text).
    const next = Math.max(0, sw - cw);
    // Update offset (CSS transition handles smooth animation).
    setOffsetPx(next);
  }, [transcript]);

  return (
    <>
      <Card className="min-h-[240px] flex flex-col py-[30px] px-[22px]">
        {/* Brand title */}
        <div className="mb-6">
          <BrandTitle />
        </div>
        <div className="flex-1 w-full">
          {/* "LISTENING" heading */}
          <div className="text-center mb-0">
            <div className="text-[36px] font-bold text-textPrimary tracking-[0.01em] uppercase">LISTENING</div>
          </div>
          {/* VAD status pill: shows "Active" when speaking, "Idle" when silent */}
          <div className="flex justify-center -mt-1 mb-6">
            <StatusPill active={vadOpen} activeLabel="Active" inactiveLabel="Idle" type="success" />
          </div>
          {/* Audio visualization: EQ bars that respond to audio input */}
          <AudioVisualizer bars={bars} vadOpen={vadOpen} />
          {/* Transcript preview: auto-scrolling single-line text showing real-time transcription */}
          <div className="px-4 flex justify-center mt-8">
            <div
              className="w-full bg-bgPreview/85 border border-bgTrack rounded-2xl py-2 px-5 overflow-hidden backdrop-blur-[1px]"
            >
              {/* Container: clips overflow, provides viewport */}
              <div
                ref={containerRef}
                className="overflow-hidden whitespace-nowrap"
              >
                {/* Text: scrolls horizontally to show most recent content */}
                <div
                  ref={textRef}
                  className="inline-block text-[12px] text-textSecondary transition-transform duration-200 ease-out"
                  style={{ transform: `translateX(-${offsetPx}px)` }}
                >
                  {transcript}
                </div>
              </div>
            </div>
          </div>
        </div>
        {/* Settings link */}
        <div className="mt-6 text-center">
          <LinkButton onClick={onOpenSettings}>[settings]</LinkButton>
        </div>
      </Card>
    </>
  );
}
