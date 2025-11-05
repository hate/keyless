type Props = { bars: number[]; hint?: string };

export function PillOverlay({ bars, hint = '[esc] cancel' }: Props) {
  const normalized = bars.map((v) => Math.max(0.1, Math.min(1, v / 65535)));
  return (
    <div
      className="inline-flex flex-col items-center gap-1 px-5 py-2.5 rounded-full border"
      style={{ background: '#0f0f0f', borderColor: '#2a2a2a' }}
    >
      <div className="flex items-end justify-center gap-1 h-4">
        {normalized.map((n, i) => (
          <div
            key={i}
            className="w-0.5 rounded-t-sm transition-all duration-100 ease-out bg-gradient-to-t from-[#50fa7b] via-[#c8ff6e] to-[#ff5555]"
            style={{ height: `${n * 100}%`, opacity: 0.5 + n * 0.5 }}
          />
        ))}
      </div>
      <p className="text-[10px] text-[#a0a0a0] lowercase whitespace-nowrap">{hint}</p>
    </div>
  );
}


