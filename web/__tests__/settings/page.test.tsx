import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("next/navigation", () => ({
  usePathname: () => "/settings",
}));

// The workspace panel now reads `useSkycotSession()` to surface the
// active tenant id (rebase-fix-settings-workspace-tenant-identity-surface).
// Stub the hook here so this layout test doesn't need a NextAuth provider.
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

import SettingsLayout from "@/app/(app)/settings/layout";
import SettingsLandingPage from "@/app/(app)/settings/page";
import { ThemeProvider } from "@/components/skycot/ThemeProvider";

function renderSettingsPage() {
  return render(
    <ThemeProvider>
      <SettingsLayout>
        <SettingsLandingPage />
      </SettingsLayout>
    </ThemeProvider>,
  );
}

describe("/settings landing page", () => {
  it("renders the settings shell and workspace panel on a fresh workspace", () => {
    renderSettingsPage();

    expect(screen.getByTestId("settings-shell")).toBeInTheDocument();
    expect(screen.getByTestId("settings-workspace-panel")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: /^settings: workspace$/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 2, name: /workspace identity/i }),
    ).toBeInTheDocument();
  });

  it("renders the canonical settings tabs with live routes as links", () => {
    renderSettingsPage();

    expect(
      screen.getByRole("link", { name: /^team$/i }),
    ).toHaveAttribute("href", "/settings/members");
    expect(
      screen.getByRole("link", { name: /api keys/i }),
    ).toHaveAttribute("href", "/settings/api-keys");
    expect(
      screen.getByRole("link", { name: /data export/i }),
    ).toHaveAttribute("href", "/settings/data-export");
  });

  it("renders the danger-zone tab as a security anchor link", () => {
    renderSettingsPage();

    expect(
      screen.getByRole("link", { name: /danger zone/i }),
    ).toHaveAttribute("href", "/settings/security#danger-zone");
  });

  it("mounts without throwing on a fresh workspace", () => {
    expect(() => renderSettingsPage()).not.toThrow();
  });
});
