import { useEffect, useMemo, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./App.css";

function Popover() {
	return (
		<div className="popover">
			<div className="title">idle</div>
			<div className="hint">hold your hotkey to start listening</div>
		</div>
	);
}

function Settings() {
	return (
		<div className="settings">
			<div className="title">settings</div>
			<div className="hint">this is a placeholder. sinks and options will appear here.</div>
		</div>
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
		if (label === "settings") return Settings;
		if (label === "pill") return Pill;
		return Popover;
	}, [label]);

	return <View />;
}
