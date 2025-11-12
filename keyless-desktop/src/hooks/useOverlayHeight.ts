/**
 * Hook to measure overlay height for scroll offset calculations.
 * 
 * Tracks the height of an overlay element (e.g., DownloadOverlay) and updates
 * when the element resizes. Used to calculate scroll container max-height
 * to account for overlay space.
 * 
 * @param overlayRef - Ref to the overlay element to measure
 * @returns Height of the overlay in pixels (0 if element not found)
 */
import { useState, useEffect, RefObject } from 'react';

export function useOverlayHeight(overlayRef: RefObject<HTMLDivElement | null>): number {
	// Current overlay height in pixels.
	const [overlayHeight, setOverlayHeight] = useState<number>(0);

	/**
	 * Measure overlay height and update on resize.
	 * 
	 * Uses ResizeObserver to automatically update height when overlay content
	 * changes (e.g., downloads appear/disappear).
	 */
	useEffect(() => {
		const el = overlayRef.current;
		if (!el) {
			setOverlayHeight(0);
			return;
		}
		/**
		 * Measure and update overlay height.
		 */
		const measure = () => setOverlayHeight(el.offsetHeight);
		// Initial measurement.
		measure();
		// Set up ResizeObserver to track height changes.
		let ro: ResizeObserver | null = null;
		if (typeof ResizeObserver !== 'undefined') {
			ro = new ResizeObserver(measure);
			ro.observe(el);
		}
		// Cleanup: disconnect ResizeObserver on unmount.
		return () => {
			if (ro) ro.disconnect();
		};
	}, [overlayRef]);

	return overlayHeight;
}
