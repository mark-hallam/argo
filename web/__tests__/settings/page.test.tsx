import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import SettingsLayout from "@/app/(app)/settings/layout";
import SettingsLandingPage from "@/app/(app)/settings/page";

vi.mock("next/navigation", () => ({
  usePathname: () => "/settings",
}));

function renderSettingsPage() {
  return render(
    <SettingsLayout>
      <SettingsLandingPage />
    </SettingsLayout>,
  );
}

describe("/settings landing page", () => {
  it("renders the settings shell and workspace panel on a fresh workspace", () => {
    renderSettingsPage();

    expect(screen.getByTestId("settings-shell")).toBeInTheDocument();
    expect(screen.getByTestId("settings-workspace-panel")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: /^settings$/i }),
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

  it("mounts without throwing on a fresh workspace (no providers required)", () => {
    expect(() => renderSettingsPage()).not.toThrow();
  });
});
