/**
 * Entry point for the Pill overlay window.
 * 
 * Renders the Pill component (audio visualization overlay) in a separate window.
 * This window is positioned at the bottom-center of the primary monitor and displays
 * real-time audio visualization when PTT is held.
 */

import React from "react";
import ReactDOM from "react-dom/client";
import Pill from "./Pill";
import "./Pill.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Pill />
  </React.StrictMode>,
);

