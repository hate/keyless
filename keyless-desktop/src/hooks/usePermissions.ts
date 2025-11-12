/**
 * Permissions management hook.
 * 
 * Handles system permission checking and runtime initialization.
 * Automatically navigates to onboarding view if permissions are missing,
 * and initializes the runtime thread once permissions are granted.
 * 
 * Features:
 * - Initial permission check on mount
 * - Automatic navigation to onboarding if permissions missing
 * - Runtime initialization after permissions granted
 * - Force recheck capability (for onboarding completion)
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { logInfo, logError } from '../utils/logger';
import type { PermissionsState, ViewKey } from '../types';

interface UsePermissionsOptions {
	activeView: ViewKey;
	switchView: (view: ViewKey, direction?: "down" | "up") => void;
	homeViewForStatus: () => ViewKey;
}

interface UsePermissionsResult {
	permissions: PermissionsState | null;
	runtimeReady: boolean;
	checkPermissions: (options?: { force?: boolean }) => Promise<void>;
}

export function usePermissions({
	activeView,
	switchView,
	homeViewForStatus,
}: UsePermissionsOptions): UsePermissionsResult {
	// Current permissions state (null until first check completes).
	const [permissions, setPermissions] = useState<PermissionsState | null>(null);
	// Runtime ready state (true when runtime thread is initialized).
	const [runtimeReady, setRuntimeReady] = useState<boolean>(false);

	// Refs for accessing current state in callbacks (avoids stale closures).
	const permissionsRef = useRef<PermissionsState | null>(null);
	const runtimeReadyRef = useRef<boolean>(false);
	const initializingRuntimeRef = useRef<boolean>(false);
	const initialCheckRunRef = useRef<boolean>(false);
	const needsRuntimeKickstartRef = useRef<boolean>(false);

	// Keep refs in sync with state (for callbacks to access current values).
	useEffect(() => {
		permissionsRef.current = permissions;
	}, [permissions]);

	useEffect(() => {
		runtimeReadyRef.current = runtimeReady;
	}, [runtimeReady]);

	/**
	 * Check system permissions and initialize runtime if needed.
	 * 
	 * Checks microphone and accessibility permissions, navigates to onboarding
	 * if missing, and initializes runtime thread once permissions are granted.
	 * 
	 * @param options - Optional force flag to recheck even if already checked
	 */
	const checkPermissions = useCallback(async (options?: { force?: boolean }) => {
		const force = options?.force ?? false;
		const currentPerms = permissionsRef.current;
		// Skip if already checked and permissions OK (unless forced).
		if (!force && (runtimeReadyRef.current || initializingRuntimeRef.current) && currentPerms && !currentPerms.needsOnboarding) {
			return;
		}
		try {
			// Check permissions via backend.
			const perms = await invoke<PermissionsState>('check_permissions');
			logInfo('[Onboarding] Permission check result:', perms);
			permissionsRef.current = perms;
			setPermissions(perms);

			if (perms.needsOnboarding) {
				// Permissions missing: navigate to onboarding view.
				logInfo('[Onboarding] Showing onboarding - permissions missing');
				if (activeView !== "onboarding") {
					switchView("onboarding", "down");
				}
				setRuntimeReady(false);
				runtimeReadyRef.current = false;
				needsRuntimeKickstartRef.current = true;
			} else {
				// Permissions OK: navigate away from onboarding if needed.
				logInfo('[Onboarding] Permissions OK');
				if (activeView === "onboarding") {
					switchView(homeViewForStatus(), "up");
				}
				// Initialize runtime if not already ready.
				if (!runtimeReadyRef.current) {
					// Check if runtime is already initialized (e.g., from previous session).
					let alreadyReady = false;
					try {
						alreadyReady = await invoke<boolean>('runtime_is_ready');
					} catch (error) {
						logError('Failed to check runtime ready status:', error);
						alreadyReady = false;
					}
					if (alreadyReady) {
						// Runtime already initialized: just update state.
						runtimeReadyRef.current = true;
						needsRuntimeKickstartRef.current = false;
						setRuntimeReady(true);
						return;
					}
					// Only initialize if we need to kickstart (prevents duplicate initialization).
					if (!needsRuntimeKickstartRef.current) {
						return;
					}
					logInfo('[Onboarding] Initializing runtime');
					initializingRuntimeRef.current = true;
					try {
						// Initialize runtime thread (starts audio pipeline, PTT listener, etc.).
						await invoke('initialize_runtime');
						runtimeReadyRef.current = true;
						needsRuntimeKickstartRef.current = false;
						setRuntimeReady(true);
					} catch (e) {
						logError("Failed to initialize runtime:", e);
						runtimeReadyRef.current = false;
					} finally {
						initializingRuntimeRef.current = false;
					}
				}
			}
		} catch (e) {
			logError("Permission check failed:", e);
		}
	}, [activeView, homeViewForStatus, switchView]);

	/**
	 * Run initial permission check on mount (guarded for React StrictMode).
	 * 
	 * StrictMode double-invokes effects, so we use a ref to ensure we only
	 * check once on initial mount.
	 */
	useEffect(() => {
		if (initialCheckRunRef.current) {
			return;
		}
		initialCheckRunRef.current = true;
		checkPermissions({ force: true });
	}, [checkPermissions]);

	return {
		permissions,
		runtimeReady,
		checkPermissions,
	};
}
