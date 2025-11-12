import { useEffect, useMemo, useRef, useState } from 'react';
import { COLORS } from '../constants/colors';

export interface AudioVisualizerProps {
	bars: number[];
	vadOpen: boolean;
}

/**
 * Canvas-based EQ visualizer component.
 * Renders bars from 0..100 range coming from backend.
 * Features:
 * - HiDPI aware (devicePixelRatio scaling)
 * - Always shows a subtle baseline
 * - Guarantees minimum visible bar height
 * - Animated decay when VAD closes
 */
export function AudioVisualizer({ bars, vadOpen }: AudioVisualizerProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [size, setSize] = useState<{ w: number; h: number }>({ w: 0, h: 0 });

  // Visualization tuning
  const TARGET_BARS = 24;         // fewer, chunked bars for a cleaner look
  const MIN_BAR_WIDTH = 6;        // ensure visually thick bars
  const GAP = 4;                  // spacing between bars
  const RADIUS = 3;               // rounded corners radius (px)
  const LEFT_PAD = 8;
  const RIGHT_PAD = 8;
  const MIN_FRAC = 0.10;          // 10% min height when active

  const hasData = Array.isArray(bars) && bars.length > 0;
  const data = useMemo<number[]>(() => {
    if (hasData) return bars.map((v) => Math.max(0, Math.min(100, Number(v) || 0)));
    // Placeholder 32 bars when no data yet
    return Array.from({ length: 32 }, () => 0);
  }, [bars, hasData]);

  // Downsample bars to target count by max-pooling to preserve peaks
  const pooled = useMemo<number[]>(() => {
    const src = data;
    const n = src.length;
    if (n === 0) return [];
    if (n <= TARGET_BARS) return src;
    const out: number[] = [];
    for (let i = 0; i < TARGET_BARS; i++) {
      const start = Math.floor((i / TARGET_BARS) * n);
      const end = Math.floor(((i + 1) / TARGET_BARS) * n);
      let maxv = 0;
      for (let j = start; j < Math.max(end, start + 1) && j < n; j++) {
        if (src[j] > maxv) maxv = src[j];
      }
      out.push(maxv);
    }
    return out;
  }, [data]);

  // Observe container size for responsive canvas
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const cr = entry.contentRect;
        setSize({ w: Math.max(0, Math.floor(cr.width)), h: Math.max(0, Math.floor(cr.height)) });
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Animated draw: bars decay to zero and turn grey when VAD closes
  const displayRef = useRef<number[]>([]);
  const rafRef = useRef<number | null>(null);

  // Keep display vector length in sync to avoid pops
  useEffect(() => {
    if (displayRef.current.length !== pooled.length) {
      const prev = displayRef.current;
      const next = new Array(pooled.length).fill(0);
      const m = Math.min(prev.length, next.length);
      for (let i = 0; i < m; i++) next[i] = prev[i];
      displayRef.current = next;
    }
  }, [pooled.length]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const draw = (values: number[], active: boolean) => {
      const dpr = Math.max(1, Math.floor(window.devicePixelRatio || 1));
      const width = Math.max(1, size.w);
      const height = Math.max(1, size.h);
      if (canvas.width !== width * dpr || canvas.height !== height * dpr) {
        canvas.width = width * dpr;
        canvas.height = height * dpr;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
      }
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.resetTransform();
      ctx.scale(dpr, dpr);

      // Clear
      ctx.clearRect(0, 0, width, height);

      // Baseline
      const baselineY = height - 6;

      // Layout
      const count = values.length;
      const usableWidth = Math.max(0, width - LEFT_PAD - RIGHT_PAD - GAP * Math.max(0, count - 1));
      const barWidth = count > 0 ? Math.max(MIN_BAR_WIDTH, Math.floor(usableWidth / Math.max(1, count))) : MIN_BAR_WIDTH;

      const drawRoundRect = (x: number, y: number, w: number, h: number, r: number) => {
        const rr = Math.min(r, w / 2, h / 2);
        ctx.beginPath();
        ctx.moveTo(x + rr, y);
        ctx.lineTo(x + w - rr, y);
        ctx.quadraticCurveTo(x + w, y, x + w, y + rr);
        ctx.lineTo(x + w, y + h - rr);
        ctx.quadraticCurveTo(x + w, y + h, x + w - rr, y + h);
        ctx.lineTo(x + rr, y + h);
        ctx.quadraticCurveTo(x, y + h, x, y + h - rr);
        ctx.lineTo(x, y + rr);
        ctx.quadraticCurveTo(x, y, x + rr, y);
        ctx.closePath();
      };

      // Gradient for active; grey for inactive/decay
      let activeGradient: CanvasGradient | null = null;
      if (active) {
        activeGradient = ctx.createLinearGradient(0, 0, 0, Math.max(1, height - 10));
        activeGradient.addColorStop(0.0, COLORS.canvas.gradientTop);
        activeGradient.addColorStop(0.5, COLORS.canvas.gradientMiddle);
        activeGradient.addColorStop(1.0, COLORS.canvas.gradientBottom);
      }

      for (let i = 0; i < count; i++) {
        const v = Math.max(0, Math.min(1, values[i] / 100));
        const eased = Math.sqrt(v);
        const usableH = height - 10;
        const barH = Math.max(MIN_FRAC, eased) * usableH;
        const x = LEFT_PAD + i * (barWidth + GAP);
        const y = baselineY - barH;

        if (active) {
          ctx.fillStyle = activeGradient as CanvasGradient;
          ctx.globalAlpha = 0.85;
          drawRoundRect(x, y, barWidth, barH, RADIUS);
          ctx.fill();
          ctx.globalAlpha = 1;
        } else {
          // Inactive: draw grey circles at bar bases
          const cx = x + barWidth / 2;
          const r = Math.max(2, Math.min(4, barWidth / 2));
          const cy = baselineY - 1;
          ctx.beginPath();
          ctx.fillStyle = COLORS.canvas.inactive;
          ctx.globalAlpha = 0.9;
          ctx.arc(cx, cy, r, 0, Math.PI * 2);
          ctx.fill();
          ctx.globalAlpha = 1;
        }
      }
    };

    const step = () => {
      const current = displayRef.current;
      const target = (vadOpen && hasData) ? pooled : new Array(pooled.length).fill(0);
      // Sync length again defensively
      if (current.length !== target.length) {
        displayRef.current = new Array(target.length).fill(0);
      }
      let maxDiff = 0;
      for (let i = 0; i < target.length; i++) {
        const c = displayRef.current[i] ?? 0;
        const t = target[i];
        const rising = t > c;
        const alpha = (vadOpen && hasData) ? (rising ? 0.45 : 0.30) : 0.15;
        const next = c + (t - c) * alpha;
        displayRef.current[i] = next;
        const d = Math.abs(next - t);
        if (d > maxDiff) maxDiff = d;
      }
      const active = vadOpen && hasData;
      draw(displayRef.current, active);

      const keepGoing = active || maxDiff > 0.5;
      if (keepGoing) {
        rafRef.current = requestAnimationFrame(step);
      } else {
        rafRef.current = null;
      }
    };

    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    rafRef.current = requestAnimationFrame(step);
    return () => {
      if (rafRef.current) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };
  }, [pooled, vadOpen, hasData, size.w, size.h]);

  return (
    <div ref={containerRef} className="relative h-20 px-8 overflow-hidden">
      <canvas ref={canvasRef} />
    </div>
  );
}
