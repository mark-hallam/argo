import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const pathnameRef = vi.hoisted(() => ({ current: "/settings" as string }));

vi.mock("next/navigation", () => ({
  usePathname: () => pathnameRef.current,
}));

import { SettingsTabs } from "./SettingsTabs";

function tabsWithAriaCurrent(): string[] {
  return screen
    .getAllByRole("link")
    .filter((el) => el.getAttribute("aria-current") === "page")
    .map((el) => el.textContent?.trim() ?? "");
}

describe("SettingsTabs aria-current", () => {
  it("highlights exactly one tab (Workspace) on /settings", () => {
    pathnameRef.current = "/settings";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Workspace"]);
  });

  it("highlights exactly one tab (Workspace) on /settings/workspace", () => {
    pathnameRef.current = "/settings/workspace";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Workspace"]);
  });

  it("highlights exactly one tab (Billing) on /settings/billing — no double-highlight with Workspace", () => {
    pathnameRef.current = "/settings/billing";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Billing"]);
  });

  it("highlights exactly one tab (Data export) on /settings/data-export", () => {
    pathnameRef.current = "/settings/data-export";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Data export"]);
  });

  it("highlights exactly one tab (Security) on /settings/security", () => {
    pathnameRef.current = "/settings/security";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Security"]);
  });

  it("highlights exactly one tab (Connectors) on a nested /settings/connectors/<id> sub-route", () => {
    pathnameRef.current = "/settings/connectors/zendesk";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Connectors"]);
  });
});
