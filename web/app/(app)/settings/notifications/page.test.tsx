import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_NOTIFICATION_SETTINGS } from "@/lib/api/settings";

const settingsApi = vi.hoisted(() => ({
  fetchNotificationSettings: vi.fn(),
  updateNotificationSettings: vi.fn(),
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

import NotificationsSettingsPage from "./page";

describe("settings notifications page", () => {
  beforeEach(() => {
    settingsApi.fetchNotificationSettings.mockResolvedValue({
      value: { ...DEFAULT_NOTIFICATION_SETTINGS },
      source: "mock-bypass",
    });
    settingsApi.updateNotificationSettings.mockResolvedValue({
      value: { ...DEFAULT_NOTIFICATION_SETTINGS, daily_digest_time: "09:15" },
      source: "mock-bypass",
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders controlled inputs hydrated from fetchNotificationSettings", async () => {
    render(<NotificationsSettingsPage />);
    await waitFor(() => {
      expect(screen.getByTestId("settings-notifications-escalation")).toHaveValue(
        DEFAULT_NOTIFICATION_SETTINGS.approval_escalation_minutes,
      );
    });
    expect(screen.getByTestId("settings-notifications-digest")).toHaveValue(
      DEFAULT_NOTIFICATION_SETTINGS.daily_digest_time,
    );
    expect(screen.getByTestId("settings-notifications-email")).toBeChecked();
    expect(screen.getByTestId("settings-notifications-push")).not.toBeChecked();
  });

  it("updates escalation minutes and toggles email", async () => {
    render(<NotificationsSettingsPage />);
    const escalation = await screen.findByTestId("settings-notifications-escalation");
    fireEvent.change(escalation, { target: { value: "30" } });
    expect(escalation).toHaveValue(30);

    fireEvent.click(screen.getByTestId("settings-notifications-email"));
    expect(screen.getByTestId("settings-notifications-email")).not.toBeChecked();

    expect(screen.getByTestId("settings-notifications-save-bar")).toBeInTheDocument();
  });

  it("calls updateNotificationSettings on save", async () => {
    render(<NotificationsSettingsPage />);
    const digest = await screen.findByTestId("settings-notifications-digest");
    fireEvent.change(digest, { target: { value: "09:15" } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("settings-notifications-save"));
    });

    expect(settingsApi.updateNotificationSettings).toHaveBeenCalledTimes(1);
    const [prev, patch] = settingsApi.updateNotificationSettings.mock.calls[0];
    expect(prev).toEqual(DEFAULT_NOTIFICATION_SETTINGS);
    expect(patch.daily_digest_time).toBe("09:15");
    expect(toastApi.success).toHaveBeenCalled();
  });
});
