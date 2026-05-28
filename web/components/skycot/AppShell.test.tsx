import * as React from "react";
import { act } from "react";
import { hydrateRoot, type Root } from "react-dom/client";
import { renderToString } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const pathnameRef = vi.hoisted(() => ({ current: "/team" }));
vi.mock("next/navigation", () => ({
  usePathname: () => pathnameRef.current,
}));
vi.mock("@/components/skycot/UserPill", () => ({ UserPill: () => null }));
vi.mock("@/components/skycot/ThemeToggle", () => ({ ThemeToggle: () => null }));
vi.mock("@/components/ui/Tooltip", () => ({
  HoverHint: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { AppNav } from "./AppShell";

describe("AppNav hydration", () => {
  let host: HTMLDivElement;
  let consoleError: ReturnType<typeof vi.spyOn>;
  let root: Root | null = null;

  beforeEach(() => {
    pathnameRef.current = "/team";
    consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    host = document.createElement("div");
    document.body.appendChild(host);
  });

  afterEach(async () => {
    if (root) {
      await act(async () => {
        root?.unmount();
      });
      root = null;
    }
    host.remove();
    consoleError.mockRestore();
  });

  it("server-renders every link without data-active so the client cannot disagree", () => {
    pathnameRef.current = "/audit";
    const html = renderToString(<AppNav />);
    expect(html).not.toContain("data-active");
  });

  it("does not log a hydration warning when the server pathname differs from the client pathname", async () => {
    // Simulates the cached-layout scenario from the brief: SSR HTML
    // was produced while the URL was /team, but the client navigates
    // to /audit before hydration kicks in.
    pathnameRef.current = "/team";
    const html = renderToString(<AppNav />);
    host.innerHTML = html;

    pathnameRef.current = "/audit";
    await act(async () => {
      root = hydrateRoot(host, <AppNav />);
    });

    const errors = consoleError.mock.calls.map((c) => c.join(" ")).join("\n");
    expect(errors).not.toContain("hydrated");
    expect(errors).not.toContain("Hydration failed");
    expect(errors).not.toContain("did not match");
  });

  it("after mount, marks only the current pathname's link as active", async () => {
    pathnameRef.current = "/audit";
    await act(async () => {
      root = hydrateRoot(host, <AppNav />);
    });
    const activeLinks = host.querySelectorAll('a[data-active="true"]');
    expect(activeLinks).toHaveLength(1);
    expect(activeLinks[0]?.getAttribute("href")).toBe("/audit");
  });

  it("marks Approvals (/) active only when the pathname is exactly '/'", async () => {
    pathnameRef.current = "/";
    await act(async () => {
      root = hydrateRoot(host, <AppNav />);
    });
    const activeLinks = host.querySelectorAll('a[data-active="true"]');
    expect(activeLinks).toHaveLength(1);
    expect(activeLinks[0]?.getAttribute("href")).toBe("/");
  });
});
