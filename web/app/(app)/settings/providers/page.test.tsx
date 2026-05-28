import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_PROVIDER_SETTINGS } from "@/lib/api/settings";

const settingsApi = vi.hoisted(() => ({
  fetchProviderSettings: vi.fn(),
  updateProviderSettings: vi.fn(),
}));

const providerKeysApi = vi.hoisted(() => ({
  fetchProviderKeys: vi.fn(),
  saveProviderKey: vi.fn(),
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

vi.mock("@/lib/api/provider-keys", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/api/provider-keys")>();
  return { ...actual, ...providerKeysApi };
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
    providerKeysApi.fetchProviderKeys.mockResolvedValue({
      keys: [],
      source: "mock-bypass",
    });
    providerKeysApi.saveProviderKey.mockImplementation(async ({ provider, key }) => ({
      status: {
        provider,
        configured: true,
        masked_preview: `sk-...${key.slice(-4)}`,
        updated_at: new Date().toISOString(),
      },
      source: "mock-bypass" as const,
    }));
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

  it("renders a provider-key row per supported provider with mock status", async () => {
    render(<ProvidersSettingsPage />);
    await waitFor(() => {
      expect(screen.getByTestId("provider-keys-section")).toBeInTheDocument();
    });
    for (const provider of ["anthropic", "openai", "google", "azure", "bedrock"]) {
      const status = screen.getByTestId(`provider-key-${provider}-status`);
      expect(status).toHaveTextContent("Mock provider");
      expect(screen.getByTestId(`provider-key-${provider}-action`)).toHaveTextContent("Add key");
    }
  });

  it("expands the inline form when Add key is clicked", async () => {
    render(<ProvidersSettingsPage />);
    const trigger = await screen.findByTestId("provider-key-anthropic-action");
    fireEvent.click(trigger);
    expect(screen.getByTestId("provider-key-anthropic-input")).toBeInTheDocument();
    expect(screen.getByTestId("provider-key-anthropic-save")).toBeInTheDocument();
    expect(screen.getByTestId("provider-key-anthropic-cancel")).toBeInTheDocument();
  });

  it("flips status to Configured after a successful save", async () => {
    render(<ProvidersSettingsPage />);
    const trigger = await screen.findByTestId("provider-key-anthropic-action");
    fireEvent.click(trigger);
    const input = screen.getByTestId("provider-key-anthropic-input");
    fireEvent.change(input, { target: { value: "sk-ant-test-1234" } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("provider-key-anthropic-save"));
    });

    expect(providerKeysApi.saveProviderKey).toHaveBeenCalledWith({
      provider: "anthropic",
      key: "sk-ant-test-1234",
    });
    await waitFor(() => {
      expect(screen.getByTestId("provider-key-anthropic-status")).toHaveTextContent(
        "Configured",
      );
    });
    expect(screen.getByTestId("provider-key-anthropic-preview")).toHaveTextContent("1234");
    expect(screen.getByTestId("provider-key-anthropic-action")).toHaveTextContent("Rotate");
    expect(toastApi.success).toHaveBeenCalledWith(
      "Anthropic key saved",
      expect.objectContaining({ description: expect.any(String) }),
    );
  });
});
