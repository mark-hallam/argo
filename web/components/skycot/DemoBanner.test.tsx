import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/skycot/demo-mode", () => ({
  isDemoMode: () => true,
  recordingModeReadinessWarning: () => null,
}));

import { DemoBanner } from "./DemoBanner";

function installMemoryStorage(): Storage {
  const data = new Map<string, string>();
  const storage: Storage = {
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
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
  return storage;
}

beforeEach(() => {
  installMemoryStorage();
});

afterEach(() => {
  window.localStorage.clear();
  vi.clearAllMocks();
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
