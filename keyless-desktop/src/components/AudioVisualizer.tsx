type Props = { bars: number[] };

export function AudioVisualizer({ bars }: Props) {
  // Map normalized-like magnitudes (0..65535 or 0..100?) into 0..1 then color zones
  const normalized = bars.map((v) => {
    const n = Math.max(0, Math.min(1, typeof v === 'number' ? v / 65535 : 0));
    return n;
  });

  return (
    <div className="flex items-end justify-center gap-1 h-20 px-8">
      {normalized.map((n, i) => {
        const h = Math.max(0.1, n) * 100;
        // TUI thresholds: green (<=0.6), yellow (0.6..0.85), red (>0.85)
        const color = n > 0.85 ? 'from-[#ff5555] via-[#ff5555] to-[#ff5555]' : n > 0.6 ? 'from-[#f1fa8c] via-[#f1fa8c] to-[#f1fa8c]' : 'from-[#50fa7b] via-[#50fa7b] to-[#50fa7b]';
        return (
          <div
            key={i}
            className={`flex-1 rounded-t-sm transition-all duration-100 ease-out bg-gradient-to-t ${color}`}
            style={{ height: `${h}%`, opacity: 0.5 + n * 0.5 }}
          />
        );
      })}
    </div>
  );
}


