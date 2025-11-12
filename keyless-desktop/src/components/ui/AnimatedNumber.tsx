import { useEffect, useRef } from 'react';

export interface AnimatedNumberProps {
	value: number;
	className?: string;
}

/**
 * Animated number component that smoothly transitions between values.
 * Uses easing animation for a polished feel.
 * Stable, top-level component so it doesn't remount on parent re-renders (e.g., scrolling).
 */
export function AnimatedNumber({ value, className = '' }: AnimatedNumberProps) {
	const ref = useRef<HTMLSpanElement>(null);
	useEffect(() => {
		const el = ref.current;
		if (!el) return;
		const from = Number(el.dataset.value || '0');
		const to = value;
		const start = performance.now();
		const duration = 300;
		let raf = 0;
		const step = (t: number) => {
			const k = Math.min(1, (t - start) / duration);
			const eased = 1 - Math.pow(1 - k, 3);
			const v = Math.round(from + (to - from) * eased);
			el.textContent = v.toLocaleString();
			if (k < 1) raf = requestAnimationFrame(step);
			else el.dataset.value = String(to);
		};
		raf = requestAnimationFrame(step);
		return () => cancelAnimationFrame(raf);
	}, [value]);
	return <span ref={ref} data-value="0" className={className}>{value.toLocaleString()}</span>;
}

