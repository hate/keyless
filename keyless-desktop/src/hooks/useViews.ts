/**
 * View management hook for the popover window.
 * 
 * Handles view state, transitions, and navigation. Manages enter/exit animations
 * and automatically switches between idle/listening views based on runtime status.
 * Also listens for navigation events from the Rust backend (e.g., tray menu clicks).
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { logError } from '../utils/logger';
import { isTauriAvailable } from '../utils/tauriHelpers';
import type { ViewKey } from '../types';
import { TRANSITION_DURATION_MS, VIEW_HIERARCHY } from '../constants/views';

interface UseViewsResult {
	activeView: ViewKey;
	exitingView: ViewKey | null;
	transitionDirection: "down" | "up";
	switchView: (nextView: ViewKey, directionOverride?: "down" | "up") => void;
	homeViewForStatus: () => ViewKey;
	setSettingsInitialSection: (section: 'ai' | null) => void;
	settingsInitialSection: 'ai' | null;
}

export function useViews(status: string): UseViewsResult {
	// Current active view being displayed.
	const [activeView, setActiveView] = useState<ViewKey>("idle");
	// View that's currently animating out (exiting).
	const [exitingView, setExitingView] = useState<ViewKey | null>(null);
	// Direction of transition animation (down = forward, up = backward).
	const [transitionDirection, setTransitionDirection] = useState<"down" | "up">("down");
	// Initial section to show when opening settings (null = show all sections).
	const [settingsInitialSection, setSettingsInitialSection] = useState<'ai' | null>(null);
	// Timer reference for clearing exit animation timeout.
	const transitionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	/**
	 * Compute transition direction based on view hierarchy.
	 * Views higher in the hierarchy animate down (forward), lower views animate up (backward).
	 */
	const computeDirection = useCallback((from: ViewKey, to: ViewKey): "down" | "up" => {
		const fromIndex = VIEW_HIERARCHY.indexOf(from);
		const toIndex = VIEW_HIERARCHY.indexOf(to);
		// Default to "down" if views aren't in hierarchy (shouldn't happen).
		if (fromIndex === -1 || toIndex === -1) {
			return "down";
		}
		// Higher index = forward (down), lower index = backward (up).
		return toIndex >= fromIndex ? "down" : "up";
	}, []);

	/**
	 * Switch to a new view with transition animation.
	 * 
	 * Sets the exiting view (for exit animation), computes transition direction,
	 * and schedules cleanup of the exiting view after the transition duration.
	 */
	const switchView = useCallback((nextView: ViewKey, directionOverride?: "down" | "up") => {
		// No-op if already on the target view.
		if (activeView === nextView) {
			return;
		}
		// Use override direction if provided, otherwise compute from hierarchy.
		const direction = directionOverride ?? computeDirection(activeView, nextView);
		// Set exiting view (triggers exit animation).
		setExitingView(activeView);
		setTransitionDirection(direction);
		// Set new active view (triggers enter animation).
		setActiveView(nextView);
		// Clear any existing transition timer.
		if (transitionTimerRef.current) {
			clearTimeout(transitionTimerRef.current);
		}
		// Schedule cleanup: remove exiting view after transition completes.
		transitionTimerRef.current = window.setTimeout(() => {
			setExitingView(null);
			transitionTimerRef.current = null;
		}, TRANSITION_DURATION_MS);
	}, [activeView, computeDirection]);

	/**
	 * Determine the home view (idle or listening) based on runtime status.
	 */
	const homeViewForStatus = useCallback((): ViewKey => {
		return status === "listening" ? "listening" : "idle";
	}, [status]);

	/**
	 * Auto-switch between idle/listening views when status changes.
	 * 
	 * Only auto-switches if we're on a home view (not settings/models/onboarding).
	 * This allows users to stay in settings/models even when status changes.
	 */
	useEffect(() => {
		// Don't auto-switch if user is in a modal view.
		if (activeView === "settings" || activeView === "models" || activeView === "onboarding") {
			return;
		}
		// Switch to the appropriate home view based on status.
		const next = homeViewForStatus();
		if (next !== activeView) {
			// Listening = forward (down), idle = backward (up).
			switchView(next, next === "listening" ? "down" : "up");
		}
	}, [activeView, homeViewForStatus, switchView]);

	/**
	 * Listen for navigation events from Rust backend (e.g., tray menu clicks).
	 * 
	 * Handles navigation to settings, models, idle, and listening views.
	 * Also sets initial section for settings/models navigation.
	 */
	useEffect(() => {
		if (!isTauriAvailable()) return;
		let un: (() => void) | undefined;
		listen<string>("navigate", (e) => {
			const dest = String(e.payload || "").toLowerCase();
			if (dest === "settings") {
				// Navigate to settings: show all sections.
				setSettingsInitialSection(null);
				switchView("settings", "down");
			}
			if (dest === "models") {
				// Navigate to models: when returning to settings, show AI section.
				setSettingsInitialSection('ai');
				switchView("models", "down");
			}
			if (dest === "idle" || dest === "listening") {
				// Navigate to home view: always animate up (backward).
				const target: ViewKey = dest === "listening" ? "listening" : "idle";
				switchView(target, "up");
			}
		}).then((u) => (un = u)).catch((error) => {
			logError('Failed to attach navigate listener:', error);
		});
		return () => {
			// Cleanup: remove event listener on unmount.
			if (un) un();
		};
	}, [switchView]);

	/**
	 * Cleanup transition timer on unmount to prevent memory leaks.
	 */
	useEffect(() => {
		return () => {
			if (transitionTimerRef.current) {
				clearTimeout(transitionTimerRef.current);
				transitionTimerRef.current = null;
			}
		};
	}, []);

	return {
		activeView,
		exitingView,
		transitionDirection,
		switchView,
		homeViewForStatus,
		setSettingsInitialSection,
		settingsInitialSection,
	};
}
