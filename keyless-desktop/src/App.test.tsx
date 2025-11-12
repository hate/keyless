import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import App from "./App";

// Mock tauri window API to control label
vi.mock("@tauri-apps/api/webviewWindow", () => {
  return {
    getCurrentWebviewWindow: () => ({ label: "popover" }),
  };
});

// Mock tauri invoke to return bootstrap data with a model
vi.mock("@tauri-apps/api/core", () => {
  return {
    invoke: vi.fn((cmd: string) => {
      if (cmd === "load_settings_bootstrap") {
        return Promise.resolve({
          config: { modelPath: "openai/whisper-base" },
          hotkey: "control+option",
          autostart: false,
          inputDevices: [],
          installedModels: ["openai/whisper-base"],
          models: [{ id: "openai/whisper-base", downloaded: true, downloading: false }],
        });
      }
      return Promise.resolve({});
    }),
  };
});

// Mock all Tauri event listeners to prevent errors
vi.mock("@tauri-apps/api/event", () => {
  return {
    listen: vi.fn(() => Promise.resolve(() => { })),
  };
});

describe("App view selection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders idle popover by default", async () => {
    render(<App />);
    await waitFor(
      () => {
        expect(screen.getByText(/idle/i)).toBeInTheDocument();
      },
      { timeout: 3000 }
    );
    expect(screen.getByText(/to start listening/i)).toBeInTheDocument();
  });
});


