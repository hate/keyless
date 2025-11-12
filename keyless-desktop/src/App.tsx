/**
 * Main application component for the keyless-desktop popover window.
 * 
 * Orchestrates all application state, view management, and UI rendering.
 * Handles view transitions, permissions, model management, downloads, and settings.
 */

import { useCallback, useRef, useState, useEffect } from "react";
import "./index.css";
import IdleView from "./views/Idle";
import ListeningView from "./views/Listening";
import { Arrow } from "./components/layout/Arrow";
import SettingsView from "./views/Settings";
import ModelsView from "./views/Models";
import OnboardingView from "./views/Onboarding";
import { DownloadOverlay } from "./components/DownloadOverlay";
import { ErrorToast } from "./components/ui/ErrorToast";
import { useTauriStreams } from "./hooks/useTauriStreams";
import { usePermissions } from "./hooks/usePermissions";
import { useViews } from "./hooks/useViews";
import { useModels } from "./hooks/useModels";
import { useDownloads } from "./hooks/useDownloads";
import { useSettings } from "./hooks/useSettings";
import { useStats } from "./hooks/useStats";
import { useOverlayHeight } from "./hooks/useOverlayHeight";
import { useAnimation } from "./hooks/useAnimation";
import { attachListeners } from "./utils/eventListeners";
import { isTauriAvailable } from "./utils/tauriHelpers";

/**
 * Popover component - main UI container.
 * 
 * Manages view state, permissions, models, downloads, and settings.
 * Handles view transitions with animation and coordinates all sub-components.
 */
function Popover() {
	// Get runtime status, audio visualization data, and transcript from Tauri backend.
	const { status, eqBars, transcript, speaking } = useTauriStreams();
	// Get settings bootstrap data, hotkey, and output mode.
	const { settingsBootstrap, hotkey, outputMode } = useSettings();
	// Calculate statistics (word counts, talk time) from settings bootstrap.
	const stats = useStats(settingsBootstrap);

	// Arrow direction: "up" (points up) or "down" (points down), defaults to "up"
	const [arrowDirection, setArrowDirection] = useState<"up" | "down">("up");

	// Listen for arrow direction updates from backend (when popover is positioned)
	useEffect(() => {
		if (!isTauriAvailable()) return;

		let cleanup: (() => void) | undefined;

		attachListeners([
			{
				event: "arrow_direction",
				handler: (payload) => {
					const dir = String(payload || "").toLowerCase();
					if (dir === "up" || dir === "down") {
						setArrowDirection(dir as "up" | "down");
					}
				},
			},
		]).then((fn) => {
			cleanup = fn;
		}).catch((error) => {
			console.error("Failed to attach arrow_direction listener:", error);
		});

		return () => {
			cleanup?.();
		};
	}, []);

	// Manage view state and transitions (idle, listening, settings, models, onboarding).
	const {
		activeView,
		exitingView,
		transitionDirection,
		switchView,
		homeViewForStatus,
		setSettingsInitialSection,
		settingsInitialSection,
	} = useViews(status);

	// Check and manage system permissions (microphone, accessibility).
	const { permissions, checkPermissions } = usePermissions({
		activeView,
		switchView,
		homeViewForStatus,
	});

	// Get model catalog information and check if any models are installed.
	const { modelSizes, hasInstalledModel } = useModels(settingsBootstrap);

	// Manage active model downloads (pause, resume, cancel, dismiss).
	const {
		downloads,
		handlePause,
		handleResume,
		handleCancel,
		handleDismiss,
	} = useDownloads();

	// Reference to overlay container for height calculation (used for scroll offset).
	const overlayRef = useRef<HTMLDivElement | null>(null);
	const overlayHeight = useOverlayHeight(overlayRef);
	// Animation flag for mount animation (fade-in, slide-up).
	const animateOnMount = useAnimation();

	// Open settings view (resets initial section to show all sections).
	const openSettings = useCallback(() => {
		setSettingsInitialSection(null);
		switchView("settings", "down");
	}, [switchView, setSettingsInitialSection]);

	// Handle onboarding completion: force permission check to verify grants.
	const handleOnboardingComplete = async () => {
		await checkPermissions({ force: true });
	};

	/**
	 * Render the appropriate view component based on the current view state.
	 * 
	 * Handles view-specific props and conditional rendering (e.g., showing onboarding
	 * only when permissions are checked, showing idle instead of listening when no model is installed).
	 */
	const renderViewPanel = useCallback((view: typeof activeView) => {
		switch (view) {
			case "onboarding":
				// Show onboarding if permissions are loaded, otherwise show loading state.
				return permissions ? (
					<OnboardingView
						permissions={permissions}
						onComplete={handleOnboardingComplete}
					/>
				) : (
					<div className="mx-3 mb-1 rounded-xl border border-border bg-bgCard p-8 text-center">
						<p className="text-textSecondary lowercase">checking permissions...</p>
					</div>
				);
			case "models":
				// Models view: navigate back to settings AI section.
				return (
					<ModelsView
						onBack={() => {
							setSettingsInitialSection('ai');
							switchView("settings", "up");
						}}
						overlayOffset={overlayHeight}
					/>
				);
			case "settings":
				// Settings view: navigate back to home view (idle/listening).
				return (
					<SettingsView
						onBack={() => {
							setSettingsInitialSection(null);
							switchView(homeViewForStatus(), "up");
						}}
						onOpenModels={() => {
							setSettingsInitialSection('ai');
							switchView("models", "down");
						}}
						outputModeOverride={outputMode}
						stats={stats}
						overlayOffset={overlayHeight}
						initialSection={settingsInitialSection ?? undefined}
						bootstrap={settingsBootstrap ?? undefined}
						missingModel={!hasInstalledModel}
					/>
				);
			case "listening":
				// Listening view: only show if model is installed, otherwise show idle with missing model warning.
				return hasInstalledModel ? (
					<ListeningView
						speaking={speaking}
						bars={eqBars}
						transcript={transcript}
						onOpenSettings={openSettings}
					/>
				) : (
					<IdleView
						hotkey={hotkey}
						onOpenSettings={openSettings}
						onOpenModels={() => switchView("models", "down")}
						stats={stats}
						missingModel
					/>
				);
			case "idle":
			default:
				// Idle view: default home view when not listening.
				return (
					<IdleView
						hotkey={hotkey}
						onOpenSettings={openSettings}
						onOpenModels={() => switchView("models", "down")}
						stats={stats}
						missingModel={!hasInstalledModel}
					/>
				);
		}
	}, [
		permissions,
		handleOnboardingComplete,
		setSettingsInitialSection,
		switchView,
		overlayHeight,
		homeViewForStatus,
		outputMode,
		stats,
		settingsBootstrap,
		hasInstalledModel,
		speaking,
		eqBars,
		transcript,
		openSettings,
		hotkey,
	]);

	return (
		<div className="flex flex-col" style={{ height: '100vh' }}>
			{/* Main content area with mount animation */}
			<div className={animateOnMount ? "fade-in slide-up" : undefined}>
				{/* Scrollable view container */}
				<div className="scrollbar-hide" style={{ flex: '1 1 auto', minHeight: 0, overflowY: 'hidden', overflowX: 'hidden' }}>
					{/* View stack: handles enter/exit transitions */}
					<div className="view-stack">
						{/* Exiting view: animates out before being removed */}
						{exitingView && (
							<div
								key={`exit-${exitingView}-${activeView}`}
								className="view-transition"
								data-transition="exit"
								data-direction={transitionDirection === "down" ? "up" : "down"}
							>
								<Arrow direction={arrowDirection} />
								{renderViewPanel(exitingView)}
							</div>
						)}
						{/* Entering view: animates in */}
						<div
							key={`enter-${activeView}`}
							className="view-transition"
							data-transition="enter"
							data-direction={transitionDirection}
						>
							<Arrow direction={arrowDirection} />
							{renderViewPanel(activeView)}
						</div>
					</div>
				</div>
			</div>
			{/* Download overlay: shows active model downloads */}
			<div ref={overlayRef}>
				<DownloadOverlay
					downloads={downloads}
					modelSizes={modelSizes}
					onPause={handlePause}
					onResume={handleResume}
					onCancel={handleCancel}
					onDismiss={handleDismiss}
				/>
			</div>
			{/* Error toast: displays user-facing error notifications */}
			<ErrorToast />
		</div>
	);
}

/**
 * Root App component.
 * 
 * Simply renders the Popover component (main application UI).
 */
export default function App() {
	return <Popover />;
}
