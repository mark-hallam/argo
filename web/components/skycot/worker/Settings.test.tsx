import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  FetchWorkerSettingsResult,
  WorkerSettings,
} from "@/lib/api/worker-settings";
import {
  budgetPercent,
  budgetTone,
} from "@/lib/api/worker-settings";
import type { WorkerPoliciesResult } from "@/lib/api/worker-policies";
import {
  DEFAULT_DELEGATOR_PERMISSIONS,
  type Worker,
  type WorkerPermissions,
} from "@/lib/types/worker";

vi.mock("next/navigation", () => ({
  useParams: () => ({ workerId: "wkr_01J0MARIAH" }),
}));

type WorkerQueryShape = {
  data: Worker | undefined;
  isPending: boolean;
  error?: Error | null;
};
const useWorkerMock = vi.fn<() => WorkerQueryShape>();
vi.mock("@/lib/hooks/use-worker", () => ({
  useWorker: () => useWorkerMock(),
}));

type SettingsQueryShape = {
  data: FetchWorkerSettingsResult | undefined;
  isPending: boolean;
  isError: boolean;
  error: Error | null;
  refetch: () => void;
};
const useSettingsMock = vi.fn<() => SettingsQueryShape>();
const onUpdateChannelsMock = vi.fn();
const onUpdateQuietHoursMock = vi.fn();
const onUpdateBudgetMock = vi.fn();
const onUpdateAutonomyMock = vi.fn();
const onPauseMock = vi.fn();
const onArchiveMock = vi.fn();
const onDeleteMock = vi.fn();
vi.mock("@/lib/hooks/use-worker-settings", () => ({
  useWorkerSettings: () => useSettingsMock(),
  useSettingsActions: () => ({
    onUpdateChannels: onUpdateChannelsMock,
    onUpdateQuietHours: onUpdateQuietHoursMock,
    onUpdateBudget: onUpdateBudgetMock,
    onUpdateAutonomy: onUpdateAutonomyMock,
    onPause: onPauseMock,
    onArchive: onArchiveMock,
    onDelete: onDeleteMock,
  }),
  workerSettingsQueryKey: () => ["worker-settings"],
}));

type PoliciesQueryShape = {
  data: WorkerPoliciesResult | undefined;
  isPending: boolean;
  isError: boolean;
  error: Error | null;
};
const usePoliciesMock = vi.fn<() => PoliciesQueryShape>();
const onUpdatePolicyMock = vi.fn();
vi.mock("@/lib/hooks/use-worker-policies", () => ({
  useWorkerPolicies: () => usePoliciesMock(),
  usePolicyActions: () => ({
    onUpdatePolicy: onUpdatePolicyMock,
    isUpdating: false,
  }),
  workerPoliciesQueryKey: () => ["worker-policies"],
}));

function policiesState(): PoliciesQueryShape {
  return {
    data: {
      workerId: "wkr_01J0MARIAH",
      readOnly: false,
      source: "mock-offline",
      rows: [
        { actionType: "draft_listing", currentPolicy: "auto-approve", inferredTrustGrade: "low", isDefault: false },
        { actionType: "post_listing", currentPolicy: "risk-gated", inferredTrustGrade: "medium", isDefault: true },
      ],
    },
    isPending: false,
    isError: false,
    error: null,
  };
}

import WorkerSettingsPage from "@/app/(app)/workers/[workerId]/settings/page";

function makeSettings(overrides: Partial<WorkerSettings> = {}): WorkerSettings {
  return {
    channels: { enabled: ["slack", "imessage", "email"], primary: "slack" },
    quietHours: { start: "21:00", end: "08:00", timezone: "America/Los_Angeles" },
    budget: {
      dailyCapUsd: 5,
      monthlyCapUsd: 75,
      usedTodayUsd: 4.32,
      usedMonthUsd: 41.6,
    },
    autonomy: { level: "balanced" },
    advanced: {
      manifestToml: "version = \"0.1.0\"",
      tenantId: "tnt_workspace",
      namespaceId: "ns_default",
      versionHash: "v0.1.0+sha.abc",
    },
    ...overrides,
  };
}

function settingsState(overrides: Partial<SettingsQueryShape> = {}): SettingsQueryShape {
  return {
    data: { settings: makeSettings(), source: "mock-offline" },
    isPending: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    ...overrides,
  };
}

function operatorPerms(overrides: Partial<WorkerPermissions> = {}): WorkerPermissions {
  return {
    ...DEFAULT_DELEGATOR_PERMISSIONS,
    canViewSettings: true,
    canDelete: true,
    ...overrides,
  };
}

function makeWorker(overrides: Partial<Worker> = {}): Worker {
  return {
    id: "wkr_01J0MARIAH",
    name: "Mariah",
    role: { tagline: "lists items", vertical: "marketplace" },
    status: "working",
    permissions: operatorPerms(),
    ...overrides,
  };
}

beforeEach(() => {
  useWorkerMock.mockReset();
  useWorkerMock.mockReturnValue({ data: makeWorker(), isPending: false, error: null });
  useSettingsMock.mockReset();
  usePoliciesMock.mockReset();
  usePoliciesMock.mockReturnValue(policiesState());
  onUpdateChannelsMock.mockReset();
  onUpdateQuietHoursMock.mockReset();
  onUpdateBudgetMock.mockReset();
  onUpdateAutonomyMock.mockReset();
  onUpdatePolicyMock.mockReset();
  onPauseMock.mockReset();
  onArchiveMock.mockReset();
  onDeleteMock.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkerSettingsPage", () => {
  it("renders the seven sections in canonical order (operator view)", () => {
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    const panel = screen.getByTestId("worker-tab-panel-settings");
    const titles = within(panel)
      .getAllByRole("heading", { level: 3 })
      .map((h) => h.textContent);
    expect(titles).toEqual([
      "Channels",
      "Quiet hours",
      "Budget",
      "Autonomy level",
      "Approval policy",
      "Advanced",
      "Danger zone",
    ]);
  });

  it("hides Advanced section when permissions.canViewSettings is false", () => {
    useWorkerMock.mockReturnValue({
      data: makeWorker({ permissions: operatorPerms({ canViewSettings: false }) }),
      isPending: false,
      error: null,
    });
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(screen.queryByTestId("card-settings-advanced")).not.toBeInTheDocument();
  });

  it("Channels: toggling a channel and saving fires onUpdateChannels with the new set", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("channel-toggle-sms"));
    await user.click(screen.getByTestId("channels-save"));
    expect(onUpdateChannelsMock).toHaveBeenCalledTimes(1);
    const arg = onUpdateChannelsMock.mock.calls[0][0];
    expect(arg.enabled).toEqual(expect.arrayContaining(["slack", "imessage", "email", "sms"]));
    expect(arg.primary).toBe("slack");
  });

  it("Channels: selecting a new primary auto-enables that channel and pins it", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("channel-primary-push"));
    await user.click(screen.getByTestId("channels-save"));
    const arg = onUpdateChannelsMock.mock.calls[0][0];
    expect(arg.primary).toBe("push");
    expect(arg.enabled).toEqual(expect.arrayContaining(["push"]));
  });

  it("Quiet hours: changing the timezone and saving fires onUpdateQuietHours", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(screen.getByTestId("quiet-hours-summary")).toHaveTextContent(
      /Mariah won't be reachable between 21:00 and 08:00 \(America\/Los_Angeles\)/,
    );
    const tz = screen.getByTestId("quiet-hours-tz") as HTMLSelectElement;
    await user.selectOptions(tz, "Europe/London");
    await user.click(screen.getByTestId("quiet-hours-save"));
    expect(onUpdateQuietHoursMock).toHaveBeenCalledWith({
      start: "21:00",
      end: "08:00",
      timezone: "Europe/London",
    });
  });

  it("Budget: summary string surfaces caps + usage + percent", () => {
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    const summary = screen.getByTestId("budget-summary");
    expect(summary).toHaveTextContent(/Daily cap: \$5\.00/);
    expect(summary).toHaveTextContent(/\$4\.32 used today \(86%\)/);
    expect(summary).toHaveTextContent(/Monthly cap: \$75\.00/);
  });

  it("Budget: daily-usage bar gets the warning tone at 80-95% utilization", () => {
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(screen.getByTestId("budget-usage-daily")).toHaveAttribute("data-tone", "warning");
  });

  it("Budget: when daily usage is 100%+, hard-pause notice renders and tone is destructive", () => {
    useSettingsMock.mockReturnValue(
      settingsState({
        data: {
          settings: makeSettings({
            budget: { dailyCapUsd: 5, monthlyCapUsd: 75, usedTodayUsd: 5, usedMonthUsd: 41.6 },
          }),
          source: "mock-offline",
        },
      }),
    );
    render(<WorkerSettingsPage />);
    expect(screen.getByTestId("budget-paused-notice")).toBeInTheDocument();
    expect(screen.getByTestId("budget-usage-daily")).toHaveAttribute("data-tone", "destructive");
  });

  it("Budget: editing daily + monthly caps and saving fires onUpdateBudget with numbers", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    const daily = screen.getByTestId("budget-daily-input") as HTMLInputElement;
    const monthly = screen.getByTestId("budget-monthly-input") as HTMLInputElement;
    await user.clear(daily);
    await user.type(daily, "10");
    await user.clear(monthly);
    await user.type(monthly, "150");
    await user.click(screen.getByTestId("budget-save"));
    expect(onUpdateBudgetMock).toHaveBeenCalledWith({ dailyCapUsd: 10, monthlyCapUsd: 150 });
  });

  it("Autonomy: switching from balanced to conservative requires confirm before save", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("autonomy-radio-conservative"));
    await user.click(screen.getByTestId("autonomy-save"));
    // Confirm dialog: trust-ratchet reset warning surfaces.
    expect(screen.getByTestId("alert-dialog")).toHaveTextContent(/trust ratchet/i);
    expect(onUpdateAutonomyMock).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onUpdateAutonomyMock).toHaveBeenCalledWith({ level: "conservative" });
  });

  it("Autonomy: switching balanced → aggressive saves without the ratchet-reset dialog", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("autonomy-radio-aggressive"));
    await user.click(screen.getByTestId("autonomy-save"));
    expect(onUpdateAutonomyMock).toHaveBeenCalledWith({ level: "aggressive" });
  });

  it("Advanced: View manifest opens a read-only TOML modal that can be closed", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("advanced-view-manifest"));
    const dialog = screen.getByTestId("manifest-dialog");
    expect(within(dialog).getByTestId("manifest-toml")).toHaveTextContent(/version/);
    await user.click(screen.getByTestId("manifest-close"));
    expect(screen.queryByTestId("manifest-dialog")).not.toBeInTheDocument();
  });

  it("Advanced: audit-the-audit link points at /audit?worker=…&action_type=settings_change", () => {
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(screen.getByTestId("advanced-audit-link")).toHaveAttribute(
      "href",
      "/audit?worker=wkr_01J0MARIAH&action_type=settings_change",
    );
  });

  it("Danger: Delete is gated on permissions.canDelete; gated copy renders for delegators", () => {
    useWorkerMock.mockReturnValue({
      data: makeWorker({
        permissions: operatorPerms({ canDelete: false }),
      }),
      isPending: false,
      error: null,
    });
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(screen.getByTestId("danger-delete")).toBeDisabled();
    expect(screen.getByTestId("danger-delete-gated")).toBeInTheDocument();
  });

  it("Danger: Delete CTA stays disabled until typed name matches; firing on match calls onDelete", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("danger-delete"));
    const input = screen.getByTestId("type-name-input") as HTMLInputElement;
    const confirm = screen.getByTestId("type-name-confirm") as HTMLButtonElement;
    expect(confirm).toBeDisabled();
    await user.type(input, "mariah");
    expect(confirm).toBeDisabled();
    await user.clear(input);
    await user.type(input, "Mariah");
    expect(confirm).not.toBeDisabled();
    await user.click(confirm);
    expect(onDeleteMock).toHaveBeenCalledTimes(1);
  });

  it("Danger: Archive confirm describes 30-day recovery window", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("danger-archive"));
    expect(screen.getByTestId("alert-dialog")).toHaveTextContent(/recover her within 30 days/);
    await user.click(screen.getByTestId("alert-dialog-confirm"));
    expect(onArchiveMock).toHaveBeenCalledTimes(1);
  });

  it("Approval policy: renders one row per action type with policy + trust grade", () => {
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(screen.getByTestId("approval-policy-row-draft_listing")).toBeInTheDocument();
    expect(screen.getByTestId("approval-policy-row-post_listing")).toBeInTheDocument();
    const draftRow = screen.getByTestId("approval-policy-row-draft_listing");
    expect(within(draftRow).getByTestId("approval-policy-badge-auto-approve")).toBeInTheDocument();
    expect(within(draftRow).getByTestId("approval-policy-trust-low")).toBeInTheDocument();
  });

  it("Approval policy: clicking Edit opens the modal; Save fires onUpdatePolicy with new policy + note", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("approval-policy-edit-draft_listing"));
    const modal = screen.getByTestId("approval-policy-edit-modal");
    expect(within(modal).getByTestId("approval-policy-radio-auto-approve")).toBeChecked();
    await user.click(within(modal).getByTestId("approval-policy-radio-always-ask"));
    await user.type(within(modal).getByTestId("approval-policy-note"), "audit-evidence test");
    await user.click(within(modal).getByTestId("approval-policy-save"));
    expect(onUpdatePolicyMock).toHaveBeenCalledTimes(1);
    expect(onUpdatePolicyMock).toHaveBeenCalledWith({
      actionType: "draft_listing",
      policy: "always-ask",
      operatorNote: "audit-evidence test",
    });
  });

  it("Approval policy: Save stays disabled until the operator note has non-whitespace content", async () => {
    const user = userEvent.setup();
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    await user.click(screen.getByTestId("approval-policy-edit-draft_listing"));
    const save = screen.getByTestId("approval-policy-save") as HTMLButtonElement;
    // Same-policy + empty note: disabled.
    expect(save).toBeDisabled();
    // Whitespace-only note: still disabled even after picking a new policy.
    await user.click(screen.getByTestId("approval-policy-radio-always-refuse"));
    await user.type(screen.getByTestId("approval-policy-note"), "    ");
    expect(save).toBeDisabled();
    // Real note: enables Save.
    await user.clear(screen.getByTestId("approval-policy-note"));
    await user.type(screen.getByTestId("approval-policy-note"), "rationale");
    expect(save).not.toBeDisabled();
  });

  it("Approval policy: Edit buttons disable + banner renders when the worker is paused", () => {
    useWorkerMock.mockReturnValue({
      data: makeWorker({ status: "paused" }),
      isPending: false,
      error: null,
    });
    useSettingsMock.mockReturnValue(settingsState());
    render(<WorkerSettingsPage />);
    expect(
      screen.getByTestId("approval-policy-readonly-banner"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("approval-policy-edit-draft_listing")).toBeDisabled();
  });

  it("renders an error state with Try again that refetches", async () => {
    const user = userEvent.setup();
    const refetch = vi.fn();
    useSettingsMock.mockReturnValue(
      settingsState({
        isError: true,
        data: undefined,
        error: new Error("boom"),
        refetch,
      }),
    );
    render(<WorkerSettingsPage />);
    const err = screen.getByTestId("worker-settings-error");
    expect(within(err).getByText(/boom/)).toBeInTheDocument();
    await user.click(within(err).getByRole("button", { name: /Try again/ }));
    expect(refetch).toHaveBeenCalledTimes(1);
  });
});

describe("budget helpers", () => {
  it("budgetPercent clamps to [0,100] and treats cap<=0 as 0%", () => {
    expect(budgetPercent(2, 5)).toBeCloseTo(40);
    expect(budgetPercent(6, 5)).toBe(100);
    expect(budgetPercent(-1, 5)).toBe(0);
    expect(budgetPercent(2, 0)).toBe(0);
    expect(budgetPercent(Number.NaN, 5)).toBe(0);
  });

  it("budgetTone maps <50→muted, 50-80→default, 80-95→warning, ≥95→destructive", () => {
    expect(budgetTone(20)).toBe("muted");
    expect(budgetTone(60)).toBe("default");
    expect(budgetTone(85)).toBe("warning");
    expect(budgetTone(99)).toBe("destructive");
  });
});
