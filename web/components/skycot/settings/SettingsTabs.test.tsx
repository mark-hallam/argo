import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const navMock = vi.hoisted(() => ({ pathname: "/settings" }));

vi.mock("next/navigation", () => ({
  usePathname: () => navMock.pathname,
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
    navMock.pathname = "/settings";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Workspace"]);
  });

  it("highlights exactly one tab (Workspace) on /settings/workspace", () => {
    navMock.pathname = "/settings/workspace";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Workspace"]);
  });

  it("highlights exactly one tab (Billing) on /settings/billing — no double-highlight with Workspace", () => {
    navMock.pathname = "/settings/billing";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Billing"]);
  });

  it("highlights exactly one tab (Data export) on /settings/data-export", () => {
    navMock.pathname = "/settings/data-export";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Data export"]);
  });

  it("highlights exactly one tab (Security) on /settings/security", () => {
    navMock.pathname = "/settings/security";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Security"]);
  });

  it("highlights exactly one tab (Connectors) on a nested /settings/connectors/<id> sub-route", () => {
    navMock.pathname = "/settings/connectors/zendesk";
    render(<SettingsTabs />);
    const active = tabsWithAriaCurrent();
    expect(active).toEqual(["Connectors"]);
  });
});

afterEach(() => {
  navMock.pathname = "/settings";
  vi.restoreAllMocks();
});

const TAB_LABELS = [
  "Workspace",
  "Team",
  "Notifications",
  "Security",
  "Providers",
  "API keys",
  "Channels",
  "Knowledge",
  "Connectors",
  "Billing",
  "Data export",
];

describe("SettingsTabs", () => {
  it("renders all 11 settings tabs plus the Danger zone tab", () => {
    render(<SettingsTabs />);
    const nav = screen.getByTestId("settings-tabs");
    for (const label of TAB_LABELS) {
      expect(within(nav).getByText(label)).toBeInTheDocument();
    }
    expect(screen.getByTestId("settings-danger-tab")).toHaveTextContent("Danger zone");
  });

  it("marks the matching tab active via aria-current=page on path prefix match", () => {
    navMock.pathname = "/settings/billing/invoices";
    render(<SettingsTabs />);
    expect(screen.getByTestId("settings-tab-billing")).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByTestId("settings-tab-team")).not.toHaveAttribute(
      "aria-current",
    );
  });

  it("hides both scroll buttons when the tab bar fits its container (no overflow)", () => {
    const nav = stubNavMetrics({ scrollWidth: 800, clientWidth: 800, scrollLeft: 0 });
    render(<SettingsTabs />);
    act(() => {
      window.dispatchEvent(new Event("resize"));
    });
    expect(screen.getByTestId("settings-tabs-scroll-left")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    expect(screen.getByTestId("settings-tabs-scroll-right")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    expect(screen.getByTestId("settings-tabs-overflow")).toHaveAttribute(
      "data-overflow-left",
      "false",
    );
    expect(screen.getByTestId("settings-tabs-overflow")).toHaveAttribute(
      "data-overflow-right",
      "false",
    );
    nav.restore();
  });

  it("reveals the right scroll button when content overflows to the right", () => {
    const nav = stubNavMetrics({ scrollWidth: 1700, clientWidth: 1200, scrollLeft: 0 });
    render(<SettingsTabs />);
    act(() => {
      window.dispatchEvent(new Event("resize"));
    });
    expect(screen.getByTestId("settings-tabs-overflow")).toHaveAttribute(
      "data-overflow-right",
      "true",
    );
    expect(screen.getByTestId("settings-tabs-overflow")).toHaveAttribute(
      "data-overflow-left",
      "false",
    );
    expect(screen.getByTestId("settings-tabs-scroll-right")).toHaveAttribute(
      "aria-hidden",
      "false",
    );
    expect(screen.getByTestId("settings-tabs-scroll-right")).toHaveAttribute(
      "tabindex",
      "0",
    );
    expect(screen.getByTestId("settings-tabs-scroll-left")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
    nav.restore();
  });

  it("reveals both scroll buttons when scrolled to the middle of an overflowing bar", () => {
    const nav = stubNavMetrics({ scrollWidth: 1700, clientWidth: 1200, scrollLeft: 240 });
    render(<SettingsTabs />);
    act(() => {
      fireEvent.scroll(screen.getByTestId("settings-tabs"));
    });
    expect(screen.getByTestId("settings-tabs-overflow")).toHaveAttribute(
      "data-overflow-left",
      "true",
    );
    expect(screen.getByTestId("settings-tabs-overflow")).toHaveAttribute(
      "data-overflow-right",
      "true",
    );
    expect(screen.getByTestId("settings-tabs-scroll-left")).toHaveAttribute(
      "aria-hidden",
      "false",
    );
    expect(screen.getByTestId("settings-tabs-scroll-right")).toHaveAttribute(
      "aria-hidden",
      "false",
    );
    nav.restore();
  });

  it("calls scrollBy on the nav when the right arrow is clicked", () => {
    const nav = stubNavMetrics({ scrollWidth: 1700, clientWidth: 1200, scrollLeft: 0 });
    render(<SettingsTabs />);
    act(() => {
      window.dispatchEvent(new Event("resize"));
    });
    const button = screen.getByTestId("settings-tabs-scroll-right");
    fireEvent.click(button);
    expect(nav.scrollBy).toHaveBeenCalledWith(
      expect.objectContaining({ left: expect.any(Number), behavior: "smooth" }),
    );
    const [arg] = nav.scrollBy.mock.calls[0] as [{ left: number }];
    expect(arg.left).toBeGreaterThan(0);
    nav.restore();
  });
});

interface NavMetricsStub {
  scrollBy: ReturnType<typeof vi.fn>;
  restore: () => void;
}

function stubNavMetrics(metrics: {
  scrollWidth: number;
  clientWidth: number;
  scrollLeft: number;
}): NavMetricsStub {
  const proto = HTMLElement.prototype;
  const previous = {
    scrollWidth: Object.getOwnPropertyDescriptor(proto, "scrollWidth"),
    clientWidth: Object.getOwnPropertyDescriptor(proto, "clientWidth"),
    scrollLeft: Object.getOwnPropertyDescriptor(proto, "scrollLeft"),
    scrollBy: Object.getOwnPropertyDescriptor(proto, "scrollBy"),
  };
  Object.defineProperty(proto, "scrollWidth", {
    configurable: true,
    get: () => metrics.scrollWidth,
  });
  Object.defineProperty(proto, "clientWidth", {
    configurable: true,
    get: () => metrics.clientWidth,
  });
  Object.defineProperty(proto, "scrollLeft", {
    configurable: true,
    get: () => metrics.scrollLeft,
    set: () => undefined,
  });
  const scrollBy = vi.fn();
  Object.defineProperty(proto, "scrollBy", {
    configurable: true,
    writable: true,
    value: scrollBy,
  });
  return {
    scrollBy,
    restore: () => {
      for (const [key, desc] of Object.entries(previous)) {
        if (desc) {
          Object.defineProperty(proto, key, desc);
        } else {
          delete (proto as unknown as Record<string, unknown>)[key];
        }
      }
    },
  };
}
