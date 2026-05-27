import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_PROVIDER_SETTINGS } from "@/lib/api/settings";

const settingsApi = vi.hoisted(() => ({
  fetchProviderSettings: vi.fn(),
  updateProviderSettings: vi.fn(),
}));

const toastApi = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
  warning: vi.fn(),
  loading: vi.fn(),
  promise: vi.fn(),
  dismiss: vi.fn(),
}));

vi.mock("@/lib/api/settings", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/settings")>();
  return { ...actual, ...settingsApi };
});

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => toastApi,
}));

import ProvidersSettingsPage from "./page";

describe("settings providers page", () => {
  beforeEach(() => {
    settingsApi.fetchProviderSettings.mockResolvedValue({
      value: { ...DEFAULT_PROVIDER_SETTINGS },
      source: "mock-bypass",
    });
    settingsApi.updateProviderSettings.mockResolvedValue({
      value: { ...DEFAULT_PROVIDER_SETTINGS, primary_provider: "openai" },
      source: "mock-bypass",
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders controlled inputs hydrated from fetchProviderSettings", async () => {
    render(<ProvidersSettingsPage />);
    await waitFor(() => {
      expect(screen.getByTestId("settings-providers-primary")).toHaveValue(
        DEFAULT_PROVIDER_SETTINGS.primary_provider,
      );
    });
    expect(screen.getByTestId("settings-providers-fallback")).toHaveValue(
      DEFAULT_PROVIDER_SETTINGS.fallback_behavior,
    );
    expect(screen.getByTestId("settings-providers-allow-overrides")).toBeChecked();
  });

  it("changes primary provider and toggles overrides on user input", async () => {
    render(<ProvidersSettingsPage />);
    const primary = await screen.findByTestId("settings-providers-primary");
    fireEvent.change(primary, { target: { value: "openai" } });
    expect(primary).toHaveValue("openai");

    const allow = screen.getByTestId("settings-providers-allow-overrides");
    fireEvent.click(allow);
    expect(allow).not.toBeChecked();

    expect(screen.getByTestId("settings-providers-save-bar")).toBeInTheDocument();
  });

  it("calls updateProviderSettings on save", async () => {
    render(<ProvidersSettingsPage />);
    const primary = await screen.findByTestId("settings-providers-primary");
    fireEvent.change(primary, { target: { value: "openai" } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("settings-providers-save"));
    });

    expect(settingsApi.updateProviderSettings).toHaveBeenCalledTimes(1);
    const [prev, patch] = settingsApi.updateProviderSettings.mock.calls[0];
    expect(prev).toEqual(DEFAULT_PROVIDER_SETTINGS);
    expect(patch.primary_provider).toBe("openai");
    expect(toastApi.success).toHaveBeenCalled();
  });
});
