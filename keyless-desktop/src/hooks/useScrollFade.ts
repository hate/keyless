
/**
 * Hook to calculate scroll fade alpha value based on remaining scrollable content.
 * 
 * Returns a value between 0 and 1 representing the opacity of the fade effect.
 * Used to show a visual indicator when content is scrollable (fade appears at bottom
 * when there's more content below).
*
* @param ref - Ref to the scrollable element
* @param triggerDistance - Distance from bottom (in pixels) at which fade starts appearing (default: 48)
* @returns Alpha value between 0 and 1 (0 = no fade, 1 = full fade)
*/
import { useState, useEffect, RefObject } from 'react';
export function useScrollFade(
	ref: RefObject<HTMLElement | null>,
	triggerDistance: number = 48
): number {
	// Alpha value for fade effect (0 = transparent, 1 = opaque).
	const [alpha, setAlpha] = useState<number>(0);

	/**
	 * Set up scroll listener to calculate fade alpha.
	 * 
	 * Calculates remaining scrollable distance and maps it to alpha value.
	 * Alpha increases as user scrolls down (more content remaining = more fade).
	 */
	useEffect(() => {
		const el = ref.current;
		if (!el) {
			setAlpha(0);
			return;
		}

		/**
		 * Calculate fade alpha based on remaining scrollable content.
		 * 
		 * Alpha = remaining / triggerDistance, clamped to [0, 1].
		 * When remaining > triggerDistance, alpha = 1 (full fade).
		 * When remaining = 0, alpha = 0 (no fade).
		 */
		const handleScroll = () => {
			// Calculate remaining scrollable distance (pixels from bottom).
			const remaining = el.scrollHeight - el.scrollTop - el.clientHeight;
			// Map remaining distance to alpha (clamped to [0, 1]).
			const newAlpha = Math.max(0, Math.min(1, remaining / triggerDistance));
			setAlpha(newAlpha);
		};

		// Initial calculation (in case element is already scrollable).
		handleScroll();

		// Listen for scroll events (passive for better performance).
		el.addEventListener('scroll', handleScroll, { passive: true });
		return () => el.removeEventListener('scroll', handleScroll);
	}, [ref, triggerDistance]);

	return alpha;
}

