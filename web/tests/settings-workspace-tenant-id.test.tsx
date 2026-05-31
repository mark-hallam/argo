import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Vitest pin for rebase-fix-settings-workspace-tenant-identity-surface.
 *
 * Source: ux-audits/2026-05-26-2152 scene-6-claim-6-trust-and-approvals F7.
 * Before this brief, /settings/workspace and the sidebar UserPill never
 * surfaced the active `tenantId`, so Claim 6 (multi-channel + multi-tenant)
 * had no visible signal in the SPA. This file pins:
 *
 *  - The read-only Workspace ID row on /settings/workspace renders the
 *    `tenantId` from `useSkycotSession()` and has a working click-to-copy.
 *  - The UserPill popover renders the tenant chip beneath the email line
 *    and click-to-copy works there too.
 *
 * The chip + row both rely on `navigator.clipboard.writeText`, which
 * happy-dom doesn't ship — we stub it per-test.
 */

const sessionRef = vi.hoisted(() => ({
  current: {
    status: "authenticated" as const,
    isDemoMode: false,
    email: "owner@acme.example",
    displayName: "Acme Owner",
    tenantId: "tenant_acme" as string | null,
    workspaceLabel: "Acme · Owner",
    role: "owner" as const,
  },
}));

vi.mock("@/lib/skycot/use-session", () => ({
  useSkycotSession: () => sessionRef.current,
}));

vi.mock("@/components/skycot/ThemePicker", () => ({
  ThemePicker: () => <div data-testid="theme-picker-stub" />,
}));

vi.mock("@/lib/ui/use-toast", () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  }),
}));

vi.mock("@/lib/api/settings", () => ({
  DEFAULT_WORKSPACE_SETTINGS: {
    name: "Acme",
    description: "Sample",
    default_vertical: "marketing",
    timezone: "Australia/Melbourne",
    billing_contact: "billing@acme.example",
  },
  TIMEZONE_OPTIONS: ["Australia/Melbourne", "UTC"],
  VERTICAL_OPTIONS: [
    { value: "marketing", label: "Marketing" },
    { value: "sales", label: "Sales" },
  ],
  fetchWorkspaceSettings: vi.fn().mockResolvedValue({
    value: {
      name: "Acme",
      description: "Sample",
      default_vertical: "marketing",
      timezone: "Australia/Melbourne",
      billing_contact: "billing@acme.example",
    },
  }),
  updateWorkspaceSettings: vi.fn().mockResolvedValue(undefined),
}));

// Stubs the auth + router surfaces the UserPill imports so the popover
// can mount inside happy-dom without a real NextAuth provider.
vi.mock("next-auth/react", () => ({
  signOut: vi.fn().mockResolvedValue(undefined),
  useSession: () => ({ data: null, status: "authenticated" as const }),
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace: vi.fn(), push: vi.fn() }),
  usePathname: () => "/settings/workspace",
}));

vi.mock("@/lib/api/auth", () => ({
  signOut: vi.fn().mockResolvedValue({ ok: true }),
}));

vi.mock("@/components/ui/Avatar", () => ({
  Avatar: ({ alt }: { alt: string }) => <div data-testid="avatar-stub">{alt}</div>,
}));

import WorkspaceSettingsPage from "@/app/(app)/settings/workspace/page";
import { UserPill } from "@/components/skycot/UserPill";

function stubClipboard() {
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText },
    configurable: true,
  });
  return writeText;
}

beforeEach(() => {
  sessionRef.current = {
    status: "authenticated",
    isDemoMode: false,
    email: "owner@acme.example",
    displayName: "Acme Owner",
    tenantId: "tenant_acme",
    workspaceLabel: "Acme · Owner",
    role: "owner",
  };
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("/settings/workspace tenant-id row", () => {
  it("renders the tenant id from useSkycotSession in a click-to-copy row", async () => {
    const writeText = stubClipboard();
    await act(async () => {
      render(<WorkspaceSettingsPage />);
    });

    await waitFor(() => {
      expect(screen.getByTestId("settings-workspace-tenant-id-row")).toBeInTheDocument();
    });
    expect(screen.getByTestId("settings-workspace-tenant-id-value")).toHaveTextContent(
      "tenant_acme",
    );

    const copyBtn = screen.getByTestId("settings-workspace-tenant-id-copy");
    expect(copyBtn).not.toBeDisabled();
    expect(copyBtn).toHaveTextContent(/copy/i);

    await act(async () => {
      fireEvent.click(copyBtn);
    });

    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("tenant_acme");
    });
    expect(screen.getByTestId("settings-workspace-tenant-id-copy")).toHaveTextContent(
      /copied/i,
    );
  });

  it("falls back to an em-dash and disables copy when no tenantId is present", async () => {
    sessionRef.current = { ...sessionRef.current, tenantId: null };
    stubClipboard();
    await act(async () => {
      render(<WorkspaceSettingsPage />);
    });

    await waitFor(() => {
      expect(screen.getByTestId("settings-workspace-tenant-id-value")).toHaveTextContent(
        "—",
      );
    });
    expect(screen.getByTestId("settings-workspace-tenant-id-copy")).toBeDisabled();
  });
});

describe("UserPill tenant chip", () => {
  it("renders a tenant chip in the popover and copies the id on click", async () => {
    const writeText = stubClipboard();
    await act(async () => {
      render(<UserPill />);
    });

    const trigger = screen.getByTestId("appshell-user-menu-trigger");
    await act(async () => {
      fireEvent.click(trigger);
    });

    const chip = await screen.findByTestId("appshell-user-tenant-chip");
    expect(chip).toHaveTextContent(/tenant_acme/i);
    expect(chip).toHaveAttribute("title", expect.stringContaining("tenant_acme"));

    await act(async () => {
      fireEvent.click(chip);
    });
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("tenant_acme");
    });
  });

  it("does not render the chip when no tenantId is present", async () => {
    sessionRef.current = { ...sessionRef.current, tenantId: null };
    stubClipboard();
    await act(async () => {
      render(<UserPill />);
    });

    const trigger = screen.getByTestId("appshell-user-menu-trigger");
    await act(async () => {
      fireEvent.click(trigger);
    });
    expect(screen.queryByTestId("appshell-user-tenant-chip")).not.toBeInTheDocument();
  });
});
