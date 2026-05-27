/**
 * Acceptance test for the demo-mode pick-vertical pattern.
 *
 * Pins:
 * - the honest-relabel from
 *   `fix-onboarding-pick-vertical-demo-mode-honest-install-relabel`
 *   (subtitle does NOT claim "We install ..." as a present-tense
 *   action in demo mode);
 * - the chip-clutter cleanup from
 *   `fix-onboarding-pick-vertical-demo-mode-chip-clutter` (one page-
 *   level banner replaces the per-card chip, so the buyer still sees
 *   the install is deferred but the 7-card grid stops shouting it).
 */

import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const demoModeMock = vi.hoisted(() => ({
  isDemoMode: vi.fn((): boolean => false),
}));

const verticalsMock = vi.hoisted(() => ({
  useVerticals: vi.fn(() => ({
    verticals: [],
    source: "mock-offline" as const,
    isLoading: false,
    error: null,
  })),
}));

const draftMock = vi.hoisted(() => {
  const state = {
    verticalId: null as string | null,
    setStep: vi.fn(),
    setVertical: vi.fn(),
  };
  type Selector = (s: typeof state) => unknown;
  const hook = vi.fn((selector: Selector) => selector(state));
  return { state, hook };
});

const apiClientMock = vi.hoisted(() => ({
  apiPost: vi.fn(),
  ApiError: class ApiError extends Error {
    status = 0;
  },
}));

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
  }),
}));

vi.mock("@/lib/skycot/demo-mode", () => demoModeMock);
vi.mock("@/lib/api/verticals", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/api/verticals")>(
      "@/lib/api/verticals",
    );
  return { ...actual, useVerticals: verticalsMock.useVerticals };
});
vi.mock("@/lib/onboarding/use-onboarding-draft", () => ({
  useOnboardingDraft: draftMock.hook,
}));
vi.mock("@/lib/api/client", () => apiClientMock);

import OnboardingPickVerticalPage from "../page";

describe("onboarding pick-vertical demo-mode relabel", () => {
  beforeEach(() => {
    demoModeMock.isDemoMode.mockReset();
    verticalsMock.useVerticals.mockReset();
    verticalsMock.useVerticals.mockReturnValue({
      verticals: [],
      source: "mock-offline" as const,
      isLoading: false,
      error: null,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("ships the real-install subtitle outside demo mode", () => {
    demoModeMock.isDemoMode.mockReturnValue(false);

    render(<OnboardingPickVerticalPage />);

    expect(
      screen.getByTestId("onboarding-vertical-subtitle-real"),
    ).toHaveTextContent("We install the");
    expect(
      screen.queryByTestId("onboarding-vertical-subtitle-demo"),
    ).not.toBeInTheDocument();
    // No demo banner and no per-card demo chip in real mode.
    expect(
      screen.queryByTestId("pick-vertical-demo-banner"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("vertical-gtm-engineer-demo-chip"),
    ).not.toBeInTheDocument();
  });

  it("ships the honest demo-mode subtitle when NEXT_PUBLIC_SKYCOT_DEMO_MODE=1", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);

    render(<OnboardingPickVerticalPage />);

    const demoSub = screen.getByTestId("onboarding-vertical-subtitle-demo");
    expect(demoSub).toHaveTextContent(/Demo mode/i);
    expect(demoSub.textContent ?? "").toMatch(
      /once you create a real workspace/i,
    );
    // Demo-mode subtitle must NOT claim install is happening present-tense.
    expect(demoSub.textContent ?? "").not.toMatch(/^We install /);
    expect(
      screen.queryByTestId("onboarding-vertical-subtitle-real"),
    ).not.toBeInTheDocument();
  });

  it("renders one demo-mode banner above the grid and NO per-card demo chips", () => {
    demoModeMock.isDemoMode.mockReturnValue(true);

    render(<OnboardingPickVerticalPage />);

    // One page-level banner explains install-deferred state.
    const banner = screen.getByTestId("pick-vertical-demo-banner");
    expect(banner).toHaveTextContent(/Demo mode/i);
    expect(banner).toHaveTextContent(/install deferred/i);
    expect(banner.textContent ?? "").toMatch(
      /connect a real provider in Settings/i,
    );

    // None of the 7 fallback tiles carry the old per-card chip.
    const expectedTiles = [
      "gtm-engineer",
      "marketing-intern",
      "marketplace-seller",
      "recruiter",
      "sales-sdr",
      "customer-support",
      "finance-ar",
    ];
    for (const id of expectedTiles) {
      expect(
        screen.queryByTestId(`vertical-${id}-demo-chip`),
      ).not.toBeInTheDocument();
    }
  });
});
