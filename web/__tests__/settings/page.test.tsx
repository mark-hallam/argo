import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import SettingsLandingPage from "@/app/(app)/settings/page";

describe("/settings landing page", () => {
  it("renders the settings stage on a fresh workspace", () => {
    render(<SettingsLandingPage />);

    expect(screen.getByTestId("settings-landing")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: /^settings$/i }),
    ).toBeInTheDocument();
  });

  it("renders the canonical settings tiles with live routes as links", () => {
    render(<SettingsLandingPage />);

    expect(
      screen.getByRole("link", { name: /team & roles/i }),
    ).toHaveAttribute("href", "/settings/members");
    expect(
      screen.getByRole("link", { name: /api keys/i }),
    ).toHaveAttribute("href", "/settings/api-keys");
    expect(
      screen.getByRole("link", { name: /data export/i }),
    ).toHaveAttribute("href", "/settings/data-export");
  });

  it("renders coming-soon tiles as non-link disabled placeholders", () => {
    render(<SettingsLandingPage />);

    expect(
      screen.queryByRole("link", { name: /workspace/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("link", { name: /notifications/i }),
    ).not.toBeInTheDocument();
  });

  it("mounts without throwing on a fresh workspace (no providers required)", () => {
    expect(() => render(<SettingsLandingPage />)).not.toThrow();
  });
});
