/**
 * Animation hook for mount animations.
 * 
 * Controls the fade-in/slide-up animation when the app window gains focus.
 * Automatically stops animation after duration, and restarts when window regains focus.
 * 
 * Used by App.tsx to apply mount animations to the main content area.
 */

import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { safeExecute } from '../utils/errorHandling';

// Animation duration in milliseconds (matches CSS animation duration).
const ANIMATION_DURATION_MS = 250;

export function useAnimation(): boolean {
	// Animation flag: true when animation should be active.
	const [animateOnMount, setAnimateOnMount] = useState<boolean>(true);

	/**
	 * Stop animation (sets flag to false).
	 */
	const stopAnimation = useCallback(() => {
		setAnimateOnMount(false);
	}, []);

	/**
	 * Start animation (sets flag to true, then stops after duration).
	 */
	const startAnimation = useCallback(() => {
		setAnimateOnMount(true);
		// Auto-stop after animation duration.
		setTimeout(stopAnimation, ANIMATION_DURATION_MS);
	}, [stopAnimation]);

	/**
	 * Stop animation after initial mount (prevents animation from staying active).
	 */
	useEffect(() => {
		const t = setTimeout(stopAnimation, ANIMATION_DURATION_MS);
		return () => clearTimeout(t);
	}, [stopAnimation]);

	/**
	 * Listen for window focus events (restart animation when window regains focus).
	 * 
	 * When the window gains focus (e.g., user switches back to the app),
	 * restart the mount animation for a polished feel.
	 */
	useEffect(() => {
		let un: (() => void) | undefined;
		safeExecute(
			async () => {
				const unsubscribe = await listen('tauri://focus', startAnimation);
				un = unsubscribe;
			},
			'useAnimation focus listener'
		);
		return () => {
			if (un) un();
		};
	}, [startAnimation]);

	return animateOnMount;
}
