import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

// Mock tauri window API to control label
vi.mock("@tauri-apps/api/webviewWindow", () => {
  return {
    getCurrentWebviewWindow: () => ({ label: "popover" }),
  };
});

describe("App view selection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders idle popover by default", () => {
    render(<App />);
    expect(screen.getByText(/idle/i)).toBeInTheDocument();
    expect(screen.getByText(/to start listening/i)).toBeInTheDocument();
  });
});


