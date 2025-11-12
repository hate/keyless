/**
 * Centralized type definitions for the keyless-desktop application.
 * 
 * This module exports all shared types and interfaces used across components,
 * hooks, utilities, and views. Types are organized by domain (downloads, models,
 * settings, views, permissions).
 */

/**
 * Download status for model downloads.
 * Tracks the lifecycle of a download operation.
 */
export type DownloadStatus = "in-progress" | "complete" | "error" | "cancelled" | "paused";

/**
 * Desktop application statistics (word counts and talk time).
 * Used for displaying session and lifetime statistics in the UI.
 */
export interface DesktopStats {
	sessionWords: number;
	sessionTalkMs: number;
	lifetimeWords: number;
	lifetimeTalkMs: number;
}

/**
 * Download progress information.
 * Used for displaying download progress bars and ETA.
 */
export interface DownloadProgress {
	bytes: number;
	total?: number;
	mbps: number;
	etaSeconds: number;
}

/**
 * Download overlay state for displaying active downloads.
 * Used by the DownloadOverlay component to show download status.
 */
export interface DownloadOverlayState {
	modelId: string;
	status: DownloadStatus;
	stage?: string;
	progress?: DownloadProgress;
	error?: string;
	updatedAt: number;
}

/**
 * Model catalog entry (basic model information).
 * Used for model listing and catalog display.
 */
export interface ModelCatalogEntry {
	id: string;
	downloaded: boolean;
	downloading: boolean;
	sizeBytes?: number;
}

/**
 * Settings bootstrap payload (initial data loaded on app start).
 * Contains config, hotkey, autostart, devices, and models for fast initial render.
 */
export interface SettingsBootstrapPayload {
	config: Record<string, unknown>;
	hotkey: string;
	autostart: boolean;
	inputDevices: string[];
	installedModels: string[];
	models: ModelCatalogEntry[];
}

/**
 * View key type for navigation between app views.
 * Used by the view management system to track current and transitioning views.
 */
export type ViewKey = "idle" | "listening" | "settings" | "models" | "onboarding";

/**
 * Permissions state for system permission tracking.
 * Used by the permissions hook and onboarding view.
 */
export interface PermissionsState {
	needsOnboarding: boolean;
	microphone?: boolean;
	accessibility?: boolean;
	[key: string]: unknown;
}

/**
 * Push-to-talk listener type (hotkey combination).
 * Used across settings, hotkey parsing, and config.
 */
export type PttListener = 'ctrlOption' | 'ctrlShift';

/**
 * Backend model information (basic model state).
 * Used across Models view, ModelRow component, and model state helpers.
 */
export interface BackendModelInfo {
	id: string;
	downloaded: boolean;
	downloading: boolean;
}

/**
 * Backend model status (detailed model state including progress, errors, etc.).
 * Used across Models view, ModelRow component, and model state helpers.
 */
export interface BackendModelStatus {
	downloaded: boolean;
	downloading: boolean;
	stage?: string;
	progress?: DownloadProgress;
	error?: string;
	paused?: boolean;
}

/**
 * Model identifier type (string alias for model IDs).
 * Used across Models view and model state helpers to ensure consistent typing.
 */
export type ModelId = string;

/**
 * Raw model status response from backend API.
 * Used for type-safe parsing of backend responses.
 */
export interface ModelStatusResponse {
	downloaded?: boolean;
	downloading?: boolean;
	stage?: string;
	progress?: {
		bytes?: number;
		total?: number;
		mbps?: number;
		etaSeconds?: number;
	};
	error?: string;
	paused?: boolean;
}

/**
 * Backend model info with size information.
 * Used for list_models_with_sizes API response.
 */
export interface BackendModelInfoWithSize extends BackendModelInfo {
	sizeBytes?: number;
}

/**
 * Raw config object from backend API.
 * Used for type-safe parsing of config responses.
 */
export type ConfigResponse = Record<string, unknown>;

/**
 * Configuration values interface for Settings view.
 * Used across Settings state management and config patches utility.
 */
export interface ConfigValues {
	language: string | null;
	modelPath: string;
	vadStartDb: number;
	vadStopDb: number;
	vadMinDuration: number;
	vadMaxSilence: number;
}
