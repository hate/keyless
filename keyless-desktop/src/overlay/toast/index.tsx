/**
 * Entry point for the Toast overlay window.
 * 
 * Renders the Toast component (transcription display overlay) in a separate window.
 * This window is positioned at the top-right of the primary monitor and displays
 * transcribed text after PTT is released, with word count and output sink indicator.
 */

import React from "react";
import ReactDOM from "react-dom/client";
import Toast from "./Toast";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Toast />
  </React.StrictMode>,
);

