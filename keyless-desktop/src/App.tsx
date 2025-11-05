import { useEffect, useMemo, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./index.css";
import "./App.css";
import IdleView, { Arrow } from "./views/Idle";
import ListeningView from "./views/Listening";
import SettingsView from "./views/Settings";
import { useTauriStreams } from "./hooks/useTauriStreams";
import { listen } from "@tauri-apps/api/event";

function Popover() {
	const { status, eqBars, transcript } = useTauriStreams();
	const [showSettings, setShowSettings] = useState<boolean>(false);
	// Allow Rust to request navigation (e.g., tray menu "settings")
	useEffect(() => {
		const w = globalThis as any;
		if (!w.__TAURI_INTERNALS__) return;
		let un: (() => void) | undefined;
		listen<string>("navigate", (e) => {
			const dest = String(e.payload || "").toLowerCase();
			if (dest === "settings") {
				setShowSettings(true);
			}
			if (dest === "idle" || dest === "listening") {
				setShowSettings(false);
			}
		}).then((u) => (un = u)).catch(() => { });
		return () => {
			if (un) un();
		};
	}, []);

	// Reset back to status page when the window is hidden
	useEffect(() => {
		const w = globalThis as any;
		let un: (() => void) | undefined;
		if (w.__TAURI_INTERNALS__) {
			// Reset on blur so next show starts at main view (unless navigate event overrides)
			listen("tauri://blur", () => {
				setShowSettings(false);
			}).then((u) => (un = u)).catch(() => { });
		}
		const onVis = () => {
			if (document.visibilityState === "hidden") setShowSettings(false);
		};
		document.addEventListener("visibilitychange", onVis);
		return () => {
			if (un) un();
			document.removeEventListener("visibilitychange", onVis);
		};
	}, [status]);

	return (
		<>
			<Arrow />
			{showSettings ? (
				<SettingsView onBack={() => setShowSettings(false)} />
			) : status === "listening" ? (
				<ListeningView bars={eqBars} transcript={transcript} onOpenSettings={() => setShowSettings(true)} />
			) : (
				<IdleView onOpenSettings={() => setShowSettings(true)} />
			)}
		</>
	);
}

function Pill() {
	return (
		<div className="pill">
			<div className="bars" />
			<div className="hint">press esc to cancel</div>
		</div>
	);
}

export default function App() {
	const [label, setLabel] = useState<string>("popover");
	useEffect(() => {
		const w = getCurrentWebviewWindow();
		setLabel(w.label);
	}, []);

	const View = useMemo(() => {
		if (label === "pill") return Pill;
		return Popover; // settings is now inside popover
	}, [label]);

	return <View />;
}
