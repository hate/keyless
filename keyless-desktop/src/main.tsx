/**
 * Main entry point for the keyless-desktop React application.
 * 
 * Sets up the React root, wraps the app in StrictMode for development checks,
 * and provides an ErrorBoundary to catch and handle component errors gracefully.
 */

import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import "./index.css";

// Create React root and render the application.
// ErrorBoundary catches any unhandled errors in the component tree and displays
// a fallback UI instead of crashing the entire app.
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
