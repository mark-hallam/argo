import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => true,
  recordingModeReadinessWarning: () => null,
}));

vi.mock("next/navigation", () => ({
  usePathname: () => "/team",
}));

import { DemoBanner } from "./DemoBanner";
import {
  __resetDataSourceForTests,
  publishDataSource,
} from "@/lib/skycot/data-source-context";

function installMemoryStorage(): { local: Storage; session: Storage } {
  function make(): Storage {
    const data = new Map<string, string>();
    return {
      get length() {
        return data.size;
      },
      clear: () => data.clear(),
      getItem: (k) => (data.has(k) ? (data.get(k) as string) : null),
      key: (i) => Array.from(data.keys())[i] ?? null,
      removeItem: (k) => {
        data.delete(k);
      },
      setItem: (k, v) => {
        data.set(k, String(v));
      },
    };
  }
  const local = make();
  const session = make();
  Object.defineProperty(window, "localStorage", { configurable: true, value: local });
  Object.defineProperty(window, "sessionStorage", { configurable: true, value: session });
  return { local, session };
}

beforeEach(() => {
  installMemoryStorage();
  __resetDataSourceForTests();
  delete process.env.NEXT_PUBLIC_HIDE_DEMO_BANNER;
});

afterEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  vi.clearAllMocks();
  delete process.env.NEXT_PUBLIC_HIDE_DEMO_BANNER;
});

describe("DemoBanner — copy branches on data source (UX-AUDIT-T0015 F8)", () => {
  it("uses the investor-safe 'every action you see is real' copy when no fetcher has published yet", async () => {
    render(<DemoBanner />);
    const banner = await screen.findByTestId("demo-banner");
    expect(banner.getAttribute("data-source")).toBe("unknown");
    expect(banner).toHaveTextContent(/Hosted POC tenant/);
    expect(banner).toHaveTextContent(/every action you see is real/);
  });

  it("says 'every action is real' and offers a sign-in CTA when the live gateway has answered", async () => {
    publishDataSource("live");
    render(<DemoBanner />);
    const banner = await screen.findByTestId("demo-banner");
    expect(banner.getAttribute("data-source")).toBe("live");
    expect(banner).toHaveTextContent(/Hosted POC tenant/);
    expect(banner).toHaveTextContent(/every action is real/);
    expect(banner).toHaveTextContent(/Sign in to talk to your own workspace/);
    expect(banner).not.toHaveTextContent(/sample data/);
    expect(banner).not.toHaveTextContent(/seed data/);
  });

  it("says 'backend unreachable, showing seed data' on the mock-offline fallback path", async () => {
    publishDataSource("mock-offline");
    render(<DemoBanner />);
    const banner = await screen.findByTestId("demo-banner");
    expect(banner.getAttribute("data-source")).toBe("mock-offline");
    expect(banner).toHaveTextContent(/Hosted POC tenant/);
    expect(banner).toHaveTextContent(/backend unreachable, showing seed data/);
  });

  it("says 'showing curated sample data' on the mock-bypass auth-short-circuit path", async () => {
    publishDataSource("mock-bypass");
    render(<DemoBanner />);
    const banner = await screen.findByTestId("demo-banner");
    expect(banner.getAttribute("data-source")).toBe("mock-bypass");
    expect(banner).toHaveTextContent(/Hosted POC tenant/);
    expect(banner).toHaveTextContent(/showing curated sample data/);
  });

  it("never uses the pre-rebase 'Demo mode — showing sample data. Sign in to see your own workspace.' copy on any data source", async () => {
    const sources: Array<"live" | "mock-offline" | "mock-bypass" | null> = [
      "live",
      "mock-offline",
      "mock-bypass",
      null,
    ];
    for (const src of sources) {
      __resetDataSourceForTests();
      if (src !== null) publishDataSource(src);
      const { unmount } = render(<DemoBanner />);
      const banner = await screen.findByTestId("demo-banner");
      expect(banner.textContent).not.toMatch(/^Demo mode —/);
      expect(banner.textContent).not.toMatch(
        /showing sample data\. Sign in to see your own workspace\./,
      );
      unmount();
    }
  });
});

describe("DemoBanner — NEXT_PUBLIC_HIDE_DEMO_BANNER suppress (UX-AUDIT-T0015 F13)", () => {
  it("renders nothing when NEXT_PUBLIC_HIDE_DEMO_BANNER=1", () => {
    process.env.NEXT_PUBLIC_HIDE_DEMO_BANNER = "1";
    const { container } = render(<DemoBanner />);
    expect(container.firstChild).toBeNull();
  });

  it("renders normally when NEXT_PUBLIC_HIDE_DEMO_BANNER is unset", async () => {
    render(<DemoBanner />);
    expect(await screen.findByTestId("demo-banner")).toBeTruthy();
  });
});

describe("DemoBanner — dismissal persists in sessionStorage", () => {
  it("clicking × writes the dismiss flag to sessionStorage (not localStorage)", async () => {
    const user = userEvent.setup();
    render(<DemoBanner />);
    const dismiss = await screen.findByTestId("demo-banner-dismiss");
    await user.click(dismiss);
    expect(window.sessionStorage.getItem("skycot:demo-banner-dismissed")).toBe("1");
    expect(window.localStorage.getItem("skycot:demo-banner-dismissed")).toBeNull();
    expect(screen.queryByTestId("demo-banner")).toBeNull();
  });

  it("a pre-seeded sessionStorage dismiss flag suppresses the banner on mount", () => {
    window.sessionStorage.setItem("skycot:demo-banner-dismissed", "1");
    const { container } = render(<DemoBanner />);
    // Hydration happens in useEffect; the banner reads sessionStorage
    // before rendering anything, so the first paint after hydrate stays
    // empty.
    expect(container.querySelector("[data-testid='demo-banner']")).toBeNull();
  });
});

describe("DemoBanner — z-index regression guard", () => {
  // Regression pin for fix-demo-banner-dismiss-button-z-index-freeze-
  // button-overlap. On narrow viewports the worker page's FreezeButton
  // could shift up into the banner's y-band and intercept clicks meant
  // for the X dismiss button. The fix gives the dismiss button explicit
  // `relative z-40` so its hit-target sits above any sibling that
  // scrolls into the banner's y-band. Without `relative` the z-index is
  // a no-op (z-index only applies to positioned elements), so this test
  // pins BOTH classes.
  it("renders the dismiss button with relative z-40 above the banner's z-30 stacking context", async () => {
    render(<DemoBanner />);
    const dismiss = await screen.findByTestId("demo-banner-dismiss");
    expect(dismiss.className).toMatch(/\brelative\b/);
    expect(dismiss.className).toMatch(/\bz-40\b/);
    expect(dismiss.className).toMatch(/\bpointer-events-auto\b/);
  });

  it("renders the banner wrapper with sticky top-0 z-30 pointer-events-none so non-dismiss elements remain interactive", async () => {
    render(<DemoBanner />);
    const banner = await screen.findByTestId("demo-banner");
    expect(banner.className).toMatch(/\bsticky\b/);
    expect(banner.className).toMatch(/\btop-0\b/);
    expect(banner.className).toMatch(/\bz-30\b/);
    expect(banner.className).toMatch(/\bpointer-events-none\b/);
  });
});
