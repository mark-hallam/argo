import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const pathnameRef = { current: "/settings" };

vi.mock("next/navigation", () => ({
  usePathname: () => pathnameRef.current,
}));

import { SettingsHeading } from "./SettingsHeading";

const ROUTE_CASES: Array<{ pathname: string; label: string }> = [
  { pathname: "/settings", label: "Workspace" },
  { pathname: "/settings/workspace", label: "Workspace" },
  { pathname: "/settings/members", label: "Team" },
  { pathname: "/settings/notifications", label: "Notifications" },
  { pathname: "/settings/security", label: "Security" },
  { pathname: "/settings/providers", label: "Providers" },
  { pathname: "/settings/api-keys", label: "API keys" },
  { pathname: "/settings/notify-channels", label: "Channels" },
  { pathname: "/settings/knowledge", label: "Knowledge" },
  { pathname: "/settings/knowledge/brand-voice", label: "Knowledge" },
  { pathname: "/settings/connectors", label: "Connectors" },
  { pathname: "/settings/billing", label: "Billing" },
  { pathname: "/settings/data-export", label: "Data export" },
];

describe("SettingsHeading", () => {
  it.each(ROUTE_CASES)(
    "renders exactly one sr-only h1 for $pathname → 'Settings: $label'",
    ({ pathname, label }) => {
      pathnameRef.current = pathname;
      const { container } = render(<SettingsHeading />);
      const h1s = container.querySelectorAll("h1");
      expect(h1s).toHaveLength(1);
      const heading = screen.getByRole("heading", { level: 1 });
      expect(heading).toHaveTextContent(`Settings: ${label}`);
      expect(heading).toHaveClass("sr-only");
    },
  );

  it("falls back to Workspace label for unknown sub-paths", () => {
    pathnameRef.current = "/settings/totally-unknown";
    render(<SettingsHeading />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Settings: Workspace",
    );
  });
});
