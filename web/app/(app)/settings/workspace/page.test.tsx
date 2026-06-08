import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_WORKSPACE_SETTINGS } from "@/lib/api/settings";

const settingsApi = vi.hoisted(() => ({
  fetchWorkspaceSettings: vi.fn(),
  updateWorkspaceSettings: vi.fn(),
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

vi.mock("@/components/skycot/ThemePicker", () => ({
  ThemePicker: () => <div data-testid="theme-picker-mock" />,
}));

// The workspace panel now reads `useSkycotSession()` for the tenant-id row
// (rebase-fix-settings-workspace-tenant-identity-surface). Stub the hook
// so the existing controlled-input assertions don't need a NextAuth provider.
vi.mock("@/lib/skycot/use-session", () => ({
  useSkycotSession: () => ({
    status: "authenticated" as const,
    isDemoMode: false,
    email: "owner@acme.example",
    displayName: "Acme Owner",
    tenantId: "tenant_acme",
    workspaceLabel: "Acme · Owner",
    role: "owner",
  }),
}));

import WorkspaceSettingsPage from "./page";

describe("settings workspace page", () => {
  beforeEach(() => {
    settingsApi.fetchWorkspaceSettings.mockResolvedValue({
      value: { ...DEFAULT_WORKSPACE_SETTINGS },
      source: "mock-bypass",
    });
    settingsApi.updateWorkspaceSettings.mockResolvedValue({
      value: { ...DEFAULT_WORKSPACE_SETTINGS, name: "Renamed" },
      source: "mock-bypass",
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders controlled inputs hydrated from fetchWorkspaceSettings", async () => {
    render(<WorkspaceSettingsPage />);
    await waitFor(() => {
      expect(screen.getByTestId("settings-workspace-name")).toHaveValue(
        DEFAULT_WORKSPACE_SETTINGS.name,
      );
    });
    expect(screen.getByTestId("settings-workspace-timezone")).toHaveValue(
      DEFAULT_WORKSPACE_SETTINGS.timezone,
    );
    expect(screen.getByTestId("settings-workspace-billing")).toHaveValue(
      DEFAULT_WORKSPACE_SETTINGS.billing_contact,
    );
  });

  it("fires onChange for name and surfaces the save bar", async () => {
    render(<WorkspaceSettingsPage />);
    const nameInput = await screen.findByTestId("settings-workspace-name");
    expect(screen.queryByTestId("settings-workspace-save-bar")).not.toBeInTheDocument();

    fireEvent.change(nameInput, { target: { value: "Renamed Tenant" } });

    expect(nameInput).toHaveValue("Renamed Tenant");
    expect(screen.getByTestId("settings-workspace-save-bar")).toBeInTheDocument();
  });

  it("calls updateWorkspaceSettings when Save is clicked, sending a ONE-KEY patch", async () => {
    render(<WorkspaceSettingsPage />);
    const nameInput = await screen.findByTestId("settings-workspace-name");
    fireEvent.change(nameInput, { target: { value: "Renamed Tenant" } });

    const saveBtn = screen.getByTestId("settings-workspace-save");
    await act(async () => {
      fireEvent.click(saveBtn);
    });

    expect(settingsApi.updateWorkspaceSettings).toHaveBeenCalledTimes(1);
    const [prev, patch] = settingsApi.updateWorkspaceSettings.mock.calls[0];
    expect(prev).toEqual(DEFAULT_WORKSPACE_SETTINGS);
    // A one-field edit must send only that field (last-writer-wins guard),
    // not the whole draft object.
    expect(patch).toEqual({ name: "Renamed Tenant" });
    expect(Object.keys(patch)).toHaveLength(1);
    expect(toastApi.success).toHaveBeenCalledWith(
      "Workspace saved",
      expect.objectContaining({ description: expect.any(String) }),
    );
  });

  it("reconciles inputs from the gateway's returned value, not the local draft", async () => {
    // The gateway normalises `name` (trims + title-cases here). The UI must
    // adopt the server's canonical value, not keep showing the raw local
    // draft that would silently change on next reload.
    settingsApi.updateWorkspaceSettings.mockResolvedValue({
      value: { ...DEFAULT_WORKSPACE_SETTINGS, name: "Normalised Name" },
      source: "gateway",
    });

    render(<WorkspaceSettingsPage />);
    const nameInput = await screen.findByTestId("settings-workspace-name");
    fireEvent.change(nameInput, { target: { value: "  messy name  " } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("settings-workspace-save"));
    });

    // Input now shows the SERVER value, not the messy local draft.
    await waitFor(() => {
      expect(screen.getByTestId("settings-workspace-name")).toHaveValue(
        "Normalised Name",
      );
    });
    // And the save bar is gone — draft === persisted === server value, so
    // the panel is no longer dirty.
    expect(
      screen.queryByTestId("settings-workspace-save-bar"),
    ).not.toBeInTheDocument();
  });

  it("reverts persisted state and shows an error toast on mutation failure", async () => {
    settingsApi.updateWorkspaceSettings.mockRejectedValue(new Error("gateway angry"));

    render(<WorkspaceSettingsPage />);
    const nameInput = await screen.findByTestId("settings-workspace-name");
    fireEvent.change(nameInput, { target: { value: "Bad name" } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("settings-workspace-save"));
    });

    expect(toastApi.error).toHaveBeenCalledWith(
      "Couldn't save workspace",
      expect.objectContaining({ description: "gateway angry" }),
    );
  });
});
